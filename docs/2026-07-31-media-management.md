# Media information and folder management

## Behavior

Rumpel provides an `i` control in the player toolbar. It toggles a transparent
information overlay in the upper-right corner of the video. The overlay shows
the available video codec, resolution, frame rate, and every audio stream's
codec and channel count.

`Left` and `Right` open the preceding or following video in the
current file's directory. The directory is filtered to common video extensions,
sorted alphabetically by filename without case sensitivity, and wraps at either
end.

`Ctrl+X` moves the playing file to the desktop Trash using Gio's standard file
operation. After a successful move, Rumpel opens the next alphabetical video,
wrapping when needed. When the removed file was the sole video, playback stops
and the player remains black and empty. A failed Trash operation leaves the
current file playing.

## Compatibility

Metadata is populated from GStreamer's stream-collection messages. Fields that
the active demuxer or decoder does not report are simply omitted. Native builds
use Gio's freedesktop-compatible Trash integration. The Flatpak is granted host
filesystem write access because deletion must be able to move host videos to
Trash.