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
`src/main.rs` attached the Ctrl-arrow `EventControllerKey` to the window with
GTK's default propagation phase. A focused `gtk::Scale` processes arrow keys
first, so the controller never reached the code that calls `folder_videos` and
`wrapped_neighbor`. The seek action at a media boundary produced the reported
failure rather than the intended wrapped folder navigation.

## Fix
Set the window key controller's propagation phase to `Capture`. It now handles
Ctrl-arrow before a focused seek bar can consume the event, allowing the
existing wrapped folder-navigation path to load the adjacent video.

## Verification
`cargo test --locked` passed: 5 tests passed, 0 failed. This includes the
existing regression coverage for wrapped previous/next folder navigation.

`cargo build --locked` passed.

`git diff --check` passed.