# Ctrl-arrow seeks instead of changing videos

## Symptom
Pressing `Ctrl+Left` or `Ctrl+Right` while the seek bar has focus seeks within
the current video. At the beginning or end of the video this can leave the
player in an invalid playback state instead of opening the previous or next
video.

## Reproduction
Open a video, focus the seek bar, and press `Ctrl+Left` or `Ctrl+Right`. The
seek bar handles the key before Rumpel's window-level folder-navigation handler
runs.

## Root cause
GTK controls can consume Ctrl-arrow shortcuts before the application's key
controller receives them. Setting the controller to capture phase did not make
the shortcut reliable in the installed application, so Ctrl-arrow cannot be the
folder-navigation binding.

## Fix
Use unmodified `Left` and `Right` in the capture-phase key controller. The
handler ignores modifier shortcuts, preserves `Ctrl+X`, and sends the plain
arrows through the existing wrapped folder-navigation path.

## Verification
`cargo fmt --check` passed.

`cargo test --locked` passed: 6 tests passed, 0 failed, including the plain
Left/Right shortcut regression test and wrapped folder-navigation test.

`cargo build --locked --release` passed.

`cargo generate-rpm`, `desktop-file-validate packaging/io.lies.rumpel.desktop`,
and `git diff --check` passed.