// release builds are GUI apps on Windows: no console window behind the player
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

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

#[derive(Debug, PartialEq)]
enum Shortcut {
    Open,
    Trash,
    /// previous (-1) or next (1) video in the current directory
    Navigate(isize),
    /// jump by this fraction of the duration
    Seek(f64),
}

const SEEK_FRACTION: f64 = 0.2;

fn shortcut(key: gdk::Key, modifiers: gdk::ModifierType) -> Option<Shortcut> {
    // lowercase so Caps Lock does not change the letter shortcuts
    let key = key.to_lower();
    if modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
        return match key {
            gdk::Key::o => Some(Shortcut::Open),
            gdk::Key::x => Some(Shortcut::Trash),
            gdk::Key::s => Some(Shortcut::Navigate(-1)),
            gdk::Key::d => Some(Shortcut::Navigate(1)),
            _ => None,
        };
    }
    let shortcut_modifiers = gdk::ModifierType::SHIFT_MASK
        | gdk::ModifierType::ALT_MASK
        | gdk::ModifierType::SUPER_MASK
        | gdk::ModifierType::HYPER_MASK
        | gdk::ModifierType::META_MASK;
    if modifiers.intersects(shortcut_modifiers) {
        return None;
    }
    match key {
        gdk::Key::Left => Some(Shortcut::Navigate(-1)),
        gdk::Key::Right => Some(Shortcut::Navigate(1)),
        gdk::Key::s => Some(Shortcut::Seek(-SEEK_FRACTION)),
        gdk::Key::d => Some(Shortcut::Seek(SEEK_FRACTION)),
        _ => None,
    }
}

fn seek_target(
    position: gst::ClockTime,
    duration: gst::ClockTime,
    fraction: f64,
) -> gst::ClockTime {
    let target = position.nseconds() as f64 + duration.nseconds() as f64 * fraction;
    gst::ClockTime::from_nseconds(target.clamp(0.0, duration.nseconds() as f64) as u64)
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

fn version_requested(arguments: impl IntoIterator<Item = String>) -> bool {
    arguments
        .into_iter()
        .any(|argument| argument == "--version")
}

fn config_path() -> std::path::PathBuf {
    glib::user_config_dir().join("rumpel.conf")
}

fn load_config_from(path: &Path) -> (bool, bool) {
    let kf = glib::KeyFile::new();
    let _ = kf.load_from_file(path, glib::KeyFileFlags::NONE);
    (
        kf.boolean("state", "mute").unwrap_or(false),
        kf.boolean("state", "loop").unwrap_or(true),
    )
}

fn save_config_to(path: &Path, mute: bool, looping: bool) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let kf = glib::KeyFile::new();
    kf.set_boolean("state", "mute", mute);
    kf.set_boolean("state", "loop", looping);
    let _ = kf.save_to_file(path);
}

fn load_config() -> (bool, bool) {
    load_config_from(&config_path())
}

fn save_config(mute: bool, looping: bool) {
    save_config_to(&config_path(), mute, looping);
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

    // GL path when available, per gtk4paintablesink docs; falls back to software.
    // Windows always takes the software path: GStreamer's GL context cannot share
    // GTK's WGL context (wglShareLists fails with ERROR_BUSY) and frames stay black.
    let video_sink = if cfg!(not(windows))
        && paintable
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
            let (playbin, play_btn, loop_btn, mute_btn, vol, info_label) = (
                playbin.clone(),
                play_btn.clone(),
                loop_btn.clone(),
                mute_btn.clone(),
                vol.clone(),
                info_label.clone(),
            );
            move |_, msg| {
                match msg.view() {
                    // playbin hands volume and mute to the audio sink when it has
                    // its own; on Windows each video gets a new WASAPI ring buffer
                    // that drops a mute set before it opened, so reapply both
                    gst::MessageView::StreamStart(_) => {
                        playbin.set_property("volume", vol.value());
                        playbin.set_property("mute", mute_btn.is_active());
                    }
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
        let (playbin, window, play_btn, size_done, current_file, info_label, open_btn) = (
            playbin.clone(),
            window.clone(),
            play_btn.clone(),
            size_done.clone(),
            current_file.clone(),
            info_label.clone(),
            open_btn.clone(),
        );
        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let key_window = window.clone();
        key.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(shortcut) = shortcut(key, modifiers) else {
                return glib::Propagation::Proceed;
            };
            match shortcut {
                Shortcut::Open => open_btn.emit_clicked(),
                Shortcut::Seek(fraction) => {
                    if let (Some(position), Some(duration)) = (
                        playbin.query_position::<gst::ClockTime>(),
                        playbin.query_duration::<gst::ClockTime>(),
                    ) {
                        let _ = playbin.seek_simple(
                            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                            seek_target(position, duration, fraction),
                        );
                    }
                }
                Shortcut::Navigate(offset) => {
                    // clone first: load_file replaces current_file, which must not
                    // still be borrowed (an if-let borrow lives through its body)
                    let current = current_file.borrow().clone();
                    if let Some(current) = current {
                        if let Ok(files) = folder_videos(&current) {
                            if let Some(next) = wrapped_neighbor(&files, &current, offset) {
                                load_file(
                                    &playbin,
                                    &key_window,
                                    &play_btn,
                                    &size_done,
                                    &current_file,
                                    &info_label,
                                    &gio::File::for_path(next),
                                );
                            }
                        }
                    }
                }
                Shortcut::Trash => {
                    let Some(current) = current_file.borrow().clone() else {
                        return glib::Propagation::Stop;
                    };
                    let successor = folder_videos(&current)
                        .ok()
                        .and_then(|files| wrapped_neighbor(&files, &current, 1))
                        .filter(|next| next != &current);
                    let file = gio::File::for_path(&current);
                    // release the file before trashing it: Windows refuses to move a
                    // file that is still open. Untoggle first, since the toggle
                    // handler pauses the pipeline, which would reopen the file.
                    play_btn.set_active(false);
                    let _ = playbin.set_state(gst::State::Null);
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
                            let next = match result {
                                Err(error) => {
                                    eprintln!(
                                        "Could not move {} to Trash: {error}",
                                        current.display()
                                    );
                                    // still there, so keep playing it
                                    Some(current)
                                }
                                Ok(()) => successor,
                            };
                            if let Some(next) = next {
                                load_file(
                                    &playbin,
                                    &window,
                                    &play_btn,
                                    &size_done,
                                    &current_file,
                                    &info_label,
                                    &gio::File::for_path(next),
                                );
                            } else {
                                current_file.replace(None);
                                info_label.set_text("No video is loaded.");
                                window.set_title(Some("Rumpel"));
                            }
                        },
                    );
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
                    // empty window (never loaded, or its last video was trashed):
                    // load here; otherwise a new window per video
                    let empty = current_file.borrow().is_none();
                    if empty {
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

// portable Windows bundles keep GStreamer and gdk-pixbuf files next to rumpel.exe;
// point the libraries there so file associations work without a wrapper script
#[cfg(windows)]
fn use_bundled_runtime() {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    else {
        return;
    };
    if !dir.join("lib").join("gstreamer-1.0").is_dir() {
        return;
    }
    std::env::set_var(
        "GST_PLUGIN_SYSTEM_PATH_1_0",
        dir.join("lib").join("gstreamer-1.0"),
    );
    std::env::set_var(
        "GST_PLUGIN_SCANNER",
        dir.join("libexec").join("gst-plugin-scanner.exe"),
    );
    std::env::set_var(
        "GDK_PIXBUF_MODULE_FILE",
        dir.join("lib")
            .join("gdk-pixbuf-2.0")
            .join("2.10.0")
            .join("loaders.cache"),
    );
    std::env::set_var("XDG_DATA_DIRS", dir.join("share"));
}

fn main() -> glib::ExitCode {
    if version_requested(std::env::args().skip(1)) {
        println!("rumpel {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }

    #[cfg(windows)]
    use_bundled_runtime();

    // GTK's GPU renderers need Direct Composition on Windows, which GTK only
    // enables on request; without it GTK paints with cairo on the CPU and
    // drops frames (720p HEVC managed 18 fps instead of 30)
    #[cfg(windows)]
    if std::env::var_os("GDK_DEBUG").is_none() {
        std::env::set_var("GDK_DEBUG", "dcomp");
    }

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
    use super::{
        fmt_time, is_video_extension, seek_target, shortcut, version_requested, wrapped_neighbor,
        Shortcut,
    };
    use gtk::gdk;
    use std::path::PathBuf;

    #[test]
    fn version_flag_is_recognized() {
        assert!(version_requested(vec!["--version".into()]));
        assert!(!version_requested(vec!["movie.mp4".into()]));
    }

    #[test]
    fn config_roundtrip() {
        // explicit path: GLib ignores XDG_CONFIG_HOME on Windows, so the real
        // config location must never be touched by tests
        let path = std::env::temp_dir()
            .join(format!("rumpel-test-{}", std::process::id()))
            .join("rumpel.conf");
        super::save_config_to(&path, true, false);
        assert_eq!(super::load_config_from(&path), (true, false));
        super::save_config_to(&path, false, true);
        assert_eq!(super::load_config_from(&path), (false, true));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
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
    fn plain_arrows_navigate_between_folder_videos() {
        assert_eq!(
            shortcut(gdk::Key::Left, gdk::ModifierType::empty()),
            Some(Shortcut::Navigate(-1))
        );
        assert_eq!(
            shortcut(gdk::Key::Right, gdk::ModifierType::empty()),
            Some(Shortcut::Navigate(1))
        );
        assert_eq!(
            shortcut(gdk::Key::Left, gdk::ModifierType::CONTROL_MASK),
            None
        );
    }

    #[test]
    fn control_shortcuts_open_trash_and_navigate() {
        let ctrl = gdk::ModifierType::CONTROL_MASK;
        assert_eq!(shortcut(gdk::Key::o, ctrl), Some(Shortcut::Open));
        assert_eq!(shortcut(gdk::Key::x, ctrl), Some(Shortcut::Trash));
        assert_eq!(shortcut(gdk::Key::X, ctrl), Some(Shortcut::Trash));
        assert_eq!(shortcut(gdk::Key::s, ctrl), Some(Shortcut::Navigate(-1)));
        assert_eq!(shortcut(gdk::Key::d, ctrl), Some(Shortcut::Navigate(1)));
        assert_eq!(shortcut(gdk::Key::o, gdk::ModifierType::empty()), None);
    }

    #[test]
    fn plain_s_and_d_seek_by_a_fifth() {
        let none = gdk::ModifierType::empty();
        assert_eq!(shortcut(gdk::Key::s, none), Some(Shortcut::Seek(-0.2)));
        assert_eq!(shortcut(gdk::Key::d, none), Some(Shortcut::Seek(0.2)));
        // Caps Lock reports the uppercase key without Shift
        assert_eq!(shortcut(gdk::Key::D, none), Some(Shortcut::Seek(0.2)));
        assert_eq!(shortcut(gdk::Key::D, gdk::ModifierType::SHIFT_MASK), None);
    }

    #[test]
    fn seek_target_clamps_to_the_video() {
        let s = gst::ClockTime::from_seconds;
        assert_eq!(seek_target(s(10), s(100), 0.2), s(30));
        assert_eq!(seek_target(s(50), s(100), -0.2), s(30));
        assert_eq!(seek_target(s(10), s(100), -0.2), s(0));
        assert_eq!(seek_target(s(90), s(100), 0.2), s(100));
    }

    #[test]
    fn common_video_extensions_are_case_insensitive() {
        assert!(is_video_extension("MKV"));
        assert!(is_video_extension("mpv"));
        assert!(is_video_extension("wmv"));
        assert!(!is_video_extension("txt"));
    }
}
