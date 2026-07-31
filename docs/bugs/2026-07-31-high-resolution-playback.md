# High-resolution video playback stutters

## Symptom
Playing high-resolution video, including a 2160p HDR HEVC Matroska file, is
stuttery in Rumpel while VLC plays it smoothly.

## Reproduction
Ran Rumpel with
`/home/phil/Downloads/tmp2/Project.Hail.Mary.2026.2160p.4K.WEB.x265.10bit.AAC5.1-[YTS.BZ].mkv`.
It is 3840x2160, Main 10 HEVC, and 24 fps. GStreamer selected `avdec_h265` and
reported repeated `Dropping frame due to QoS` warnings from both the decoder and
`gtk4paintablesink`.

For comparison, FFmpeg software-decodes the same file at about 85 fps on this
host. This rules out HEVC decoding throughput as the primary bottleneck.

## Root cause
Rumpel depended on `gst-plugin-gtk4` with its default features, which exclude
all Linux GL backends. Consequently `gtk4paintablesink` compiled without
Wayland EGL or X11 EGL support, did not initialize a GDK GL context, and only
accepted full-resolution system-memory frames. That slow CPU presentation
back-pressured `playbin3`, which sent QoS events to `avdec_h265` and dropped
frames despite the CPU being able to decode the stream. Rumpel also did not
propagate the displayed picture size to the sink, unlike the plugin's supplied
`RenderWidget`.

The host also has no GStreamer H.265 hardware decoder factory. Its VA plugin
registers only MPEG-2, JPEG, and VP9 decoders on the AMD GPU; Fedora AMD systems
need RPM Fusion's `mesa-va-drivers-freeworld` for H.264/H.265 VA-API profiles.

## Fix
Enable `waylandegl` and `x11egl` features on `gst-plugin-gtk4`, which compiles
the Linux GL backends that the existing `glsinkbin` setup requires. Set
`gtk4paintablesink`'s `window-width` and `window-height` properties whenever the
picture is resized, matching the plugin's supplied `RenderWidget` behavior.
Request GTK's supported `opengl` renderer before GTK or GStreamer initialize
unless the user has explicitly set `GSK_RENDERER`. Native package dependencies
continue to install the VA-API GStreamer plugin, and the Fedora documentation
identifies the AMD codec-driver prerequisite.

## Verification
`cargo +stable test --locked` passed: 3 tests passed, 0 failed.

`cargo +stable build --locked --release` passed.

`cargo +stable fmt --check` reports formatting changes in pre-existing
folder-navigation code outside this fix; those changes were left untouched.

FFmpeg decoded the reproduction file at about 85 fps. Before the fix,
GStreamer diagnostics showed `avdec_h265` QoS frame drops caused by a late
`gtk4paintablesink`. After enabling the EGL features, the same 4K run logged a
successfully realized `GdkWaylandGLContext`, an activated GL context, negotiated
`video/x-raw(memory:GLMemory)`, and zero late-frame or QoS-drop warnings over
eight seconds.

`packaging/cargo-sources.json` must be regenerated before the Flatpak build is
released because it does not yet pin the new EGL backend crates; the generator
is not installed in this environment.