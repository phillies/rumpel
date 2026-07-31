use gst::prelude::*;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

const VIDEO_EXTENSIONS: &[&str] = &[
    "3gp", "3g2", "asf", "avi", "divx", "flv", "m2ts", "m4v", "mkv", "mov", "mp4", "mpe", "mpeg",
    "mpg", "mpv", "mts", "mxf", "ogm", "ogv", "ts", "vob", "webm", "wmv",
];

fn is_video_extension(extension: &str) -> bool {
    VIDEO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

fn is_video_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(is_video_extension)
}

fn folder_videos(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut videos = std::fs::read_dir(path.parent().unwrap_or(path))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry| is_video_path(entry))
        .collect::<Vec<_>>();
    videos.sort_by_cached_key(|entry| entry.file_name().unwrap_or_default().to_ascii_lowercase());
    Ok(videos)
}

fn wrapped_neighbor(files: &[PathBuf], current: &Path, offset: isize) -> Option<PathBuf> {
    let current_index = files.iter().position(|file| file == current)?;
    let index = (current_index as isize + offset).rem_euclid(files.len() as isize) as usize;
    files.get(index).cloned()
}

fn stream_details(collection: &gst::StreamCollection) -> String {
    let mut details = Vec::new();
    for index in 0..collection.size() {
        let Some(stream) = collection.stream(index) else {
            continue;
        };
        let Some(caps) = stream.caps() else {
            continue;
        };
        let Some(structure) = caps.structure(0) else {
            continue;
        };
        let codec = structure.name();
        if stream.stream_type().contains(gst::StreamType::VIDEO) {
            let width = structure.get::<i32>("width").ok();
            let height = structure.get::<i32>("height").ok();
            let frame_rate = structure.get::<gst::Fraction>("framerate").ok();
            let mut line = format!("Video: {codec}");
            if let (Some(width), Some(height)) = (width, height) {
                line.push_str(&format!("\nResolution: {width}x{height}"));
            }
            if let Some(frame_rate) = frame_rate {
                line.push_str(&format!("\nFrame rate: {frame_rate} fps"));
            }
            details.push(line);
        } else if stream.stream_type().contains(gst::StreamType::AUDIO) {
            let channels = structure.get::<i32>("channels").ok();
            details.push(channels.map_or_else(
                || format!("Audio: {codec}"),
                |channels| format!("Audio: {codec} ({channels} channels)"),
            ));
        }
    }
    details.join("\n\n")
}

fn fmt_time(t: gst::ClockTime) -> String {
    let s = t.seconds();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

fn config_path() -> std::path::PathBuf {
    glib::user_config_dir().join("rumpel.conf")
}

fn load_config() -> (bool, bool) {
    let kf = glib::KeyFile::new();
    let _ = kf.load_from_file(config_path(), glib::KeyFileFlags::NONE);
    (
        kf.boolean("state", "mute").unwrap_or(false),
        kf.boolean("state", "loop").unwrap_or(true),
    )
}

fn save_config(mute: bool, looping: bool) {
    let _ = std::fs::create_dir_all(glib::user_config_dir());
    let kf = glib::KeyFile::new();
    kf.set_boolean("state", "mute", mute);
    kf.set_boolean("state", "loop", looping);
    let _ = kf.save_to_file(config_path());
}

fn load_file(
    playbin: &gst::Element,
    window: &gtk::ApplicationWindow,
    play_btn: &gtk::ToggleButton,
    size_done: &Cell<bool>,
    current_file: &RefCell<Option<PathBuf>>,
    info_label: &gtk::Label,
    file: &gio::File,
) {
    let _ = playbin.set_state(gst::State::Null);
    size_done.set(false);
    current_file.replace(file.path());
    info_label.set_text("Loading media information...");
    playbin.set_property("uri", file.uri().as_str());
    if let Some(name) = file.basename() {
        window.set_title(Some(&name.to_string_lossy()));
    }
    play_btn.set_active(true);
    let _ = playbin.set_state(gst::State::Playing);
}

fn update_sink_size(sink: &gst::Element, picture: &gtk::Picture) {
    let scale = picture.scale_factor() as u32;
    sink.set_property("window-width", (picture.width().max(0) as u32) * scale);
    sink.set_property("window-height", (picture.height().max(0) as u32) * scale);
}

fn build_window(app: &gtk::Application, file: Option<&gio::File>) {
    let playbin = gst::ElementFactory::make("playbin3").build().unwrap();
    let sink = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .unwrap();
    let paintable = sink.property::<gdk::Paintable>("paintable");

    // GL path when available, per gtk4paintablesink docs; falls back to software
    let video_sink = if paintable
        .property::<Option<gdk::GLContext>>("gl-context")
        .is_some()
    {
        gst::ElementFactory::make("glsinkbin")
            .property("sink", &sink)
            .build()
            .unwrap()
    } else {
        sink.clone()
    };
    playbin.set_property("video-sink", &video_sink);

    let picture = gtk::Picture::new();
    picture.set_paintable(Some(&paintable));
    picture.set_content_fit(gtk::ContentFit::Contain);
    {
        let sink = sink.clone();
        picture.connect_notify_local(Some("width"), move |picture, _| {
            update_sink_size(&sink, picture);
        });
    }
    {
        let sink = sink.clone();
        picture.connect_notify_local(Some("height"), move |picture, _| {
            update_sink_size(&sink, picture);
        });
    }

    // --- controls ---
    let play_btn = gtk::ToggleButton::builder()
        .icon_name("media-playback-start-symbolic")
        .tooltip_text("Play/Pause")
        .build();
    let stop_btn = gtk::Button::from_icon_name("media-playback-stop-symbolic");
    stop_btn.set_tooltip_text(Some("Stop"));
    let (mute0, loop0) = load_config();
    let loop_btn = gtk::ToggleButton::builder()
        .icon_name("media-playlist-repeat-symbolic")
        .tooltip_text("Loop")
        .active(loop0)
        .build();
    let pos_lbl = gtk::Label::new(Some("0:00"));
    let seek = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 1.0);
    seek.set_hexpand(true);
    seek.set_draw_value(false);
    let dur_lbl = gtk::Label::new(Some("0:00"));
    let mute_btn = gtk::ToggleButton::builder()
        .icon_name("audio-volume-high-symbolic")
        .tooltip_text("Mute")
        .build();
    mute_btn.connect_toggled(|b| {
        b.set_icon_name(if b.is_active() {
            "audio-volume-muted-symbolic"
        } else {
            "audio-volume-high-symbolic"
        });
    });
    let vol = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.05);
    vol.set_size_request(100, -1);
    vol.set_tooltip_text(Some("Volume"));
    let fit_dd = gtk::DropDown::from_strings(&["Fit", "Stretch", "Cover"]);
    fit_dd.set_tooltip_text(Some("Scaling"));
    let open_btn = gtk::Button::from_icon_name("folder-open-symbolic");
    open_btn.set_tooltip_text(Some("Open video"));
    let info_btn = gtk::ToggleButton::builder()
        .icon_name("dialog-information-symbolic")
        .tooltip_text("Media information")
        .build();

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    for w in [
        play_btn.upcast_ref::<gtk::Widget>(),
        stop_btn.upcast_ref(),
        loop_btn.upcast_ref(),
        mute_btn.upcast_ref(),
        vol.upcast_ref(),
        fit_dd.upcast_ref(),
        open_btn.upcast_ref(),
        info_btn.upcast_ref(),
    ] {
        buttons.append(w);
    }
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    buttons.append(&spacer);
    buttons.append(&pos_lbl);
    buttons.append(&gtk::Label::new(Some("/")));
    buttons.append(&dur_lbl);

    let controls = gtk::Box::new(gtk::Orientation::Vertical, 0);
    controls.add_css_class("toolbar");
    controls.add_css_class("osd");
    controls.set_valign(gtk::Align::End);
    controls.set_margin_start(12);
    controls.set_margin_end(12);
    controls.set_margin_bottom(12);
    controls.append(&seek);
    controls.append(&buttons);

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&picture));
    overlay.add_overlay(&controls);
    let info_label = gtk::Label::new(None);
    info_label.set_xalign(0.0);
    info_label.set_wrap(true);
    info_label.set_max_width_chars(42);
    info_label.set_margin_top(18);
    info_label.set_margin_end(18);
    info_label.add_css_class("media-info");
    info_label.set_halign(gtk::Align::End);
    info_label.set_valign(gtk::Align::Start);
    info_label.set_visible(false);
    overlay.add_overlay(&info_label);
    {
        let info_label = info_label.clone();
        info_btn.connect_toggled(move |button| info_label.set_visible(button.is_active()));
    }

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rumpel")
        .default_width(854)
        .default_height(480)
        .child(&overlay)
        .build();
    window.add_css_class("video");

    // single click on the video toggles play/pause, double-click fullscreen
    let click = gtk::GestureClick::new();
    {
        let (window, play_btn) = (window.clone(), play_btn.clone());
        click.connect_pressed(move |_, n_press, _, _| {
            // every press toggles; the second press of a double-click undoes
            // the first one's toggle, so net effect is fullscreen only
            play_btn.set_active(!play_btn.is_active());
            if n_press == 2 {
                if window.is_fullscreen() {
                    window.unfullscreen();
                } else {
                    window.fullscreen();
                }
            }
        });
    }
    picture.add_controller(click);

    // controls hide after 5 s; mouse in the bottom zone brings them back
    let hide_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let schedule_hide = {
        let (controls, hide_timer) = (controls.clone(), hide_timer.clone());
        Rc::new(move || {
            if let Some(id) = hide_timer.borrow_mut().take() {
                id.remove();
            }
            let id = glib::timeout_add_local_once(std::time::Duration::from_secs(5), {
                let (controls, hide_timer) = (controls.clone(), hide_timer.clone());
                move || {
                    hide_timer.borrow_mut().take();
                    controls.set_visible(false);
                }
            });
            *hide_timer.borrow_mut() = Some(id);
        })
    };
    let motion = gtk::EventControllerMotion::new();
    motion.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let (controls, hide_timer, schedule_hide, overlay) = (
            controls.clone(),
            hide_timer.clone(),
            schedule_hide.clone(),
            overlay.clone(),
        );
        motion.connect_motion(move |_, _, y| {
            // ponytail: fixed 100 px hot zone instead of tracking control bounds
            if y >= overlay.height() as f64 - 100.0 {
                if let Some(id) = hide_timer.borrow_mut().take() {
                    id.remove();
                }
                controls.set_visible(true);
            } else if controls.is_visible() && hide_timer.borrow().is_none() {
                schedule_hide();
            }
        });
    }
    overlay.add_controller(motion);
    schedule_hide();

    // size the window to the video's pixel size on load, capped to the monitor
    let size_done = Rc::new(Cell::new(false));
    let current_file = Rc::new(RefCell::new(file.and_then(gio::File::path)));
    {
        let (window, size_done) = (window.clone(), size_done.clone());
        paintable.connect_invalidate_size(move |p| {
            let (w, h) = (p.intrinsic_width(), p.intrinsic_height());
            if size_done.get() || w <= 0 || h <= 0 {
                return;
            }
            size_done.set(true);
            // ponytail: cap at 90% of the monitor to leave room for panels
            let (mut max_w, mut max_h) = (f64::MAX, f64::MAX);
            if let Some(display) = gdk::Display::default() {
                // window may not be mapped yet: fall back to the first monitor
                let mon = window
                    .surface()
                    .and_then(|s| display.monitor_at_surface(&s))
                    .or_else(|| display.monitors().item(0).and_downcast::<gdk::Monitor>());
                if let Some(mon) = mon {
                    let g = mon.geometry();
                    max_w = g.width() as f64 * 0.9;
                    max_h = g.height() as f64 * 0.9;
                }
            }
            let scale = (max_w / w as f64).min(max_h / h as f64).min(1.0);
            window.set_default_size((w as f64 * scale) as i32, (h as f64 * scale) as i32);
            // sizing before the first present lets the compositor place the
            // window fully on screen (Wayland offers no client-side move)
            if !window.is_visible() {
                window.present();
            }
        });
    }

    // --- wiring ---
    playbin
        .bind_property("volume", &vol.adjustment(), "value")
        .bidirectional()
        .sync_create()
        .build();
    playbin
        .bind_property("mute", &mute_btn, "active")
        .bidirectional()
        .sync_create()
        .build();
    mute_btn.set_active(mute0);

    // persist mute/loop on every change
    {
        let l = loop_btn.clone();
        mute_btn.connect_toggled(move |b| save_config(b.is_active(), l.is_active()));
    }
    {
        let m = mute_btn.clone();
        loop_btn.connect_toggled(move |b| save_config(m.is_active(), b.is_active()));
    }

    {
        let picture = picture.clone();
        fit_dd.connect_selected_notify(move |dd| {
            picture.set_content_fit(match dd.selected() {
                1 => gtk::ContentFit::Fill,
                2 => gtk::ContentFit::Cover,
                _ => gtk::ContentFit::Contain,
            });
        });
    }

    {
        let playbin = playbin.clone();
        play_btn.connect_toggled(move |b| {
            if b.is_active() {
                b.set_icon_name("media-playback-pause-symbolic");
                let _ = playbin.set_state(gst::State::Playing);
            } else {
                b.set_icon_name("media-playback-start-symbolic");
                let _ = playbin.set_state(gst::State::Paused);
            }
        });
    }

    {
        let (playbin, play_btn, seek, pos_lbl) = (
            playbin.clone(),
            play_btn.clone(),
            seek.clone(),
            pos_lbl.clone(),
        );
        stop_btn.connect_clicked(move |_| {
            play_btn.set_active(false);
            let _ = playbin.set_state(gst::State::Ready);
            seek.set_value(0.0);
            pos_lbl.set_text("0:00");
        });
    }

    {
        let playbin = playbin.clone();
        seek.connect_change_value(move |_, _, v| {
            let _ = playbin.seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_nseconds((v * 1e9) as u64),
            );
            glib::Propagation::Proceed
        });
    }

    // position/duration poll; dies with the window (root() gone)
    glib::timeout_add_local(std::time::Duration::from_millis(500), {
        let (playbin, seek, pos_lbl, dur_lbl) = (
            playbin.clone(),
            seek.clone(),
            pos_lbl.clone(),
            dur_lbl.clone(),
        );
        move || {
            if seek.root().is_none() {
                return glib::ControlFlow::Break;
            }
            if let Some(dur) = playbin.query_duration::<gst::ClockTime>() {
                seek.set_range(0.0, (dur.nseconds() as f64 / 1e9).max(0.1));
                dur_lbl.set_text(&fmt_time(dur));
            }
            if let Some(pos) = playbin.query_position::<gst::ClockTime>() {
                seek.set_value(pos.nseconds() as f64 / 1e9);
                pos_lbl.set_text(&fmt_time(pos));
            }
            glib::ControlFlow::Continue
        }
    });

    let bus_watch = playbin
        .bus()
        .unwrap()
        .add_watch_local({
            let (playbin, play_btn, loop_btn, info_label) = (
                playbin.clone(),
                play_btn.clone(),
                loop_btn.clone(),
                info_label.clone(),
            );
            move |_, msg| {
                match msg.view() {
                    gst::MessageView::StreamCollection(streams) => {
                        let details = stream_details(&streams.stream_collection());
                        info_label.set_text(if details.is_empty() {
                            "Media information is unavailable."
                        } else {
                            &details
                        });
                    }
                    gst::MessageView::Eos(_) => {
                        if loop_btn.is_active() {
                            let _ =
                                playbin.seek_simple(gst::SeekFlags::FLUSH, gst::ClockTime::ZERO);
                        } else {
                            play_btn.set_active(false);
                            let _ = playbin.set_state(gst::State::Ready);
                        }
                    }
                    gst::MessageView::Error(e) => {
                        eprintln!("Playback error: {}", e.error());
                        play_btn.set_active(false);
                        let _ = playbin.set_state(gst::State::Null);
                    }
                    _ => {}
                }
                glib::ControlFlow::Continue
            }
        })
        .unwrap();

    {
        let (playbin, window, play_btn, size_done, current_file, info_label) = (
            playbin.clone(),
            window.clone(),
            play_btn.clone(),
            size_done.clone(),
            current_file.clone(),
            info_label.clone(),
        );
        let key = gtk::EventControllerKey::new();
        let key_window = window.clone();
        key.connect_key_pressed(move |_, key, _, modifiers| {
            if !modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            if key == gdk::Key::x || key == gdk::Key::X {
                let Some(current) = current_file.borrow().clone() else {
                    return glib::Propagation::Stop;
                };
                let successor = folder_videos(&current)
                    .ok()
                    .and_then(|files| wrapped_neighbor(&files, &current, 1))
                    .filter(|next| next != &current);
                let file = gio::File::for_path(&current);
                let (playbin, play_btn, size_done, current_file, info_label, window) = (
                    playbin.clone(),
                    play_btn.clone(),
                    size_done.clone(),
                    current_file.clone(),
                    info_label.clone(),
                    key_window.clone(),
                );
                file.trash_async(
                    glib::Priority::DEFAULT,
                    gio::Cancellable::NONE,
                    move |result| {
                        if let Err(error) = result {
                            eprintln!("Could not move {} to Trash: {error}", current.display());
                        } else if let Some(successor) = successor {
                            load_file(
                                &playbin,
                                &window,
                                &play_btn,
                                &size_done,
                                &current_file,
                                &info_label,
                                &gio::File::for_path(successor),
                            );
                        } else {
                            let _ = playbin.set_state(gst::State::Null);
                            play_btn.set_active(false);
                            current_file.replace(None);
                            info_label.set_text("No video is loaded.");
                            window.set_title(Some("Rumpel"));
                        }
                    },
                );
                return glib::Propagation::Stop;
            }
            let offset = match key {
                gdk::Key::Left => -1,
                gdk::Key::Right => 1,
                _ => return glib::Propagation::Proceed,
            };
            if let Some(current) = current_file.borrow().as_deref() {
                if let Ok(files) = folder_videos(current) {
                    if let Some(next) = wrapped_neighbor(&files, current, offset) {
                        let next = gio::File::for_path(next);
                        load_file(
                            &playbin,
                            &key_window,
                            &play_btn,
                            &size_done,
                            &current_file,
                            &info_label,
                            &next,
                        );
                    }
                }
            }
            glib::Propagation::Stop
        });
        window.add_controller(key);
    }

    {
        let (app, window, playbin, play_btn, size_done, current_file, info_label) = (
            app.clone(),
            window.clone(),
            playbin.clone(),
            play_btn.clone(),
            size_done.clone(),
            current_file.clone(),
            info_label.clone(),
        );
        open_btn.connect_clicked(move |_| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("video/*");
            filter.set_name(Some("Videos"));
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            let dialog = gtk::FileDialog::builder().filters(&filters).build();
            let (app, win, playbin, play_btn, size_done, current_file, info_label) = (
                app.clone(),
                window.clone(),
                playbin.clone(),
                play_btn.clone(),
                size_done.clone(),
                current_file.clone(),
                info_label.clone(),
            );
            dialog.open(Some(&window), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    // empty window: load here; otherwise a new window per video
                    if playbin.property::<Option<String>>("uri").is_none() {
                        load_file(
                            &playbin,
                            &win,
                            &play_btn,
                            &size_done,
                            &current_file,
                            &info_label,
                            &file,
                        );
                    } else {
                        build_window(&app, Some(&file));
                    }
                }
            });
        });
    }

    {
        let playbin = playbin.clone();
        window.connect_close_request(move |_| {
            let _ = &bus_watch; // keep bus watch alive for the window's lifetime
            let _ = playbin.set_state(gst::State::Null);
            glib::Propagation::Proceed
        });
    }

    if let Some(f) = file {
        // present happens in the invalidate-size handler, once the video's
        // size is known; fallback covers audio-only or broken files
        load_file(
            &playbin,
            &window,
            &play_btn,
            &size_done,
            &current_file,
            &info_label,
            f,
        );
        let window = window.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
            if !window.is_visible() {
                window.present();
            }
        });
    } else {
        window.present();
    }
}

fn main() -> glib::ExitCode {
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "opengl");
    }
    gst::init().expect("failed to init GStreamer");
    gstgtk4::plugin_register_static().expect("failed to register gtk4paintablesink");

    let app = gtk::Application::builder()
        .application_id("io.lies.rumpel")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| {
        let css = gtk::CssProvider::new();
        css.load_from_string("window.video { background: black; }");
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    app.connect_activate(|app| build_window(app, None));
    app.connect_open(|app, files, _| {
        for f in files {
            build_window(app, Some(f));
        }
    });
    app.run()
}

#[cfg(test)]
mod tests {
    use super::{fmt_time, is_video_extension, wrapped_neighbor};
    use std::path::PathBuf;

    #[test]
    fn config_roundtrip() {
        // must run before anything else touches glib's cached config dir
        std::env::set_var(
            "XDG_CONFIG_HOME",
            std::env::temp_dir().join("rumpel-test-config"),
        );
        super::save_config(true, false);
        assert_eq!(super::load_config(), (true, false));
        super::save_config(false, true);
        assert_eq!(super::load_config(), (false, true));
    }

    #[test]
    fn time_formatting() {
        assert_eq!(fmt_time(gst::ClockTime::from_seconds(0)), "0:00");
        assert_eq!(fmt_time(gst::ClockTime::from_seconds(65)), "1:05");
        assert_eq!(fmt_time(gst::ClockTime::from_seconds(3725)), "1:02:05");
    }

    #[test]
    fn folder_navigation_wraps_in_both_directions() {
        let files = ["a.mkv", "b.mp4", "c.webm"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        assert_eq!(
            wrapped_neighbor(&files, &files[0], -1),
            Some(files[2].clone())
        );
        assert_eq!(
            wrapped_neighbor(&files, &files[2], 1),
            Some(files[0].clone())
        );
    }

    #[test]
    fn common_video_extensions_are_case_insensitive() {
        assert!(is_video_extension("MKV"));
        assert!(is_video_extension("mpv"));
        assert!(is_video_extension("wmv"));
        assert!(!is_video_extension("txt"));
    }
}
