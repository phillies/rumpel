# Windows support

## Behavior

Rumpel builds with the MSYS2 MINGW64 toolchain and ships as a self-contained
`rumpel-windows-x86_64.zip`. Playback, folder navigation, the control bar, and
Ctrl+X (Recycle Bin) work as on Linux. Mute and loop persist in
`%LOCALAPPDATA%\rumpel.conf`. Release builds are GUI-subsystem binaries, so no
console window opens.

## Audio

`wasapi2sink` has its own `volume` and `mute` properties, so playbin hands both
to the sink instead of using a software `volume` element. Each new video gets a
new WASAPI ring buffer, and a mute applied before it opens is lost: after muting
and moving to the next video, the audio session's peak meter showed full output
while the button still read muted. Rumpel reapplies the slider volume and the
mute button state on every `StreamStart` message, which keeps the next video
silent.

## Video path

Windows does not use GStreamer GL. `gtk4paintablesink` wraps GTK's WGL context
on Windows without any extra feature, so the Linux branch would wrap the sink in
`glsinkbin`. GStreamer's GL context then has to share GTK's WGL context, and
`wglShareLists` fails with `ERROR_BUSY` (`0xaa`). Every frame is dropped and the
picture stays black.

Windows therefore passes frames to the sink in system memory and GTK uploads
them. With the plugin's default features the sink accepts only RGB, which forces
a CPU colour conversion: a 3840x2160 60 fps H.264 file produced 424 QoS drop
warnings in 15 seconds. Enabling `gtk_v4_20` lets the sink accept NV12 directly
and GTK converts it itself; the same file then plays with no drops. The decoder
is `d3d12h264dec`, so decoding stays in hardware.

GTK's OpenGL and Vulkan renderers need Direct Composition on Windows, which GTK
4.24 only enables with `GDK_DEBUG=dcomp`. Without it, GTK paints with cairo on
the CPU and converts YUV frames on its main thread: a 720p 10-bit HEVC file
painted at 18 fps instead of 30, and the sink logged "too many pending frames".
Rumpel therefore sets `GDK_DEBUG=dcomp` on Windows unless `GDK_DEBUG` is already
set. GTK then uses its OpenGL renderer, which plays that file at 30 fps
(median paint 5 ms) and a 4K60 H.264 file at 59 fps.

Sending RGB from GStreamer instead (no `gtk_v4_20`) is not a substitute: it keeps
720p at 30 fps under cairo, but GStreamer's CPU conversion cannot keep up with
4K60, which played at 2-3 fps with or without Direct Composition. If Direct
Composition is unavailable on a machine, GTK falls back to cairo and
high-bitrate or 10-bit video plays jerkily.

The plugin's `winegl` feature is not an alternative: it needs
`gstreamer-gl-egl`, which the MSYS2 GStreamer packages do not provide.

## Packaging

`packaging/windows/bundle.sh` collects the DLLs `rumpel.exe` and the GStreamer
plugins need, plus gdk-pixbuf loaders, GSettings schemas, and icon themes. The
`loaders.cache` keeps the prefix-relative paths that `gdk-pixbuf-query-loaders`
writes, which gdk-pixbuf resolves against its own DLL location. At startup
`rumpel.exe` points GStreamer and gdk-pixbuf at the bundle when a
`lib/gstreamer-1.0` folder sits next to it.

The bundle does not include `gdbus.exe`, so each launch runs as its own instance
instead of forwarding files to a running one. Opening a file still produces one
window per video.

## Verification

Run on Windows 11 with MSYS2 GTK 4.24.1 and GStreamer 1.28.7, from the unpacked
zip with only `C:\WINDOWS\system32` and `C:\WINDOWS` on `PATH`: GTK used its
OpenGL renderer; a 720p 10-bit HEVC episode and a 1080x1920 HEVC clip played
with no dropped frames, and a 4K60 H.264 clip dropped 5 frames in 15 seconds.
The control bar icons rendered, and `Ctrl+X` moved files to the Recycle Bin.
