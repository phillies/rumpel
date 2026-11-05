# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3] - 2026-11-05

### Added

- Homebrew formula for macOS and Linux: `brew tap phillies/rumpel https://github.com/phillies/rumpel && brew install rumpel`.

### Fixed

- Windows builds use MSYS2 UCRT64, since MINGW64 no longer ships GTK4 and the GStreamer plugin sets; this unblocks the release workflow.
- macOS builds no longer request the Linux-only Wayland/X11 GL backends.

## [0.4.2] - 2026-11-01

### Fixed

- Closing a window within 2 seconds of opening it (while another window is open) no longer brings it back as a black window that cannot be closed.

## [0.4.1] - 2026-11-01

### Added

- `S`/`D` jump distance is configurable in `rumpel.conf` (`[seek] backward`/`forward`, as `N%` or `Ns`).
- Hardware video decoders (VA, NVDEC, D3D11/12, V4L2, QSV/MSDK, AMF) are preferred by default; set `[decoding] hardware=false` to disable.
- Folder navigation also recognises `.qt`, `.f4v`, `.rm` and `.rmvb`.

### Changed

- Saving mute/loop preserves other settings in `rumpel.conf`.

## [0.4.0] - 2026-10-09

### Added

- Releases are created automatically when a version bump in `Cargo.toml` merges to `main`.
- Windows support: build with the MSYS2 MINGW64 toolchain and ship a self-contained `rumpel-windows-x86_64.zip` with each release.
- Keyboard shortcuts: `Ctrl+O` opens a video, `Ctrl+S` and `Ctrl+D` go to the previous and next video, and `S` and `D` jump back and forward by 20% of the video's length.

### Changed

- Persist mute and loop settings in `%LOCALAPPDATA%\rumpel.conf` on Windows.
- Release builds on Windows run without a console window.

### Fixed

- Moving to the previous or next video with the arrow keys no longer crashes the player with a `RefCell already borrowed` panic.
- `Ctrl+X` stops playback and releases the file before moving it to Trash, which Windows requires. If the move fails, the video keeps playing.
- Opening a video in a window whose last video was trashed loads it there instead of opening a second window.
- Mute stays on when the next video loads on Windows; previously the next video played with sound while the button still showed muted.
- Video on Windows no longer plays slowly or jerkily: Rumpel enables GTK's Direct Composition (`GDK_DEBUG=dcomp`) so GTK draws with the GPU instead of falling back to CPU rendering.

## [0.3.1] - 2026-08-05

### Fixed

- Use `Left` and `Right` for folder video navigation because GTK controls can consume Ctrl-arrow shortcuts.
- Check out the repository in the release publish job so the GitHub Release is created.
- Publish releases idempotently, so re-running the workflow for an existing tag uploads the packages instead of failing.

## [0.3.0] - 2026-08-01

### Added

- `--version` command-line argument that prints the installed Rumpel version without starting the player.

### Changed

- License the project under GPL-3.0-or-later and add contributor, conduct, and security policies.
- Add AppStream metadata, an application icon, and tagged GitHub Release publishing.

## [0.2.0] - 2026-07-31

### Added

- A toggleable upper-right media-information overlay with video and audio stream details.
- Wrapped alphabetical folder navigation with `Ctrl+Left` and `Ctrl+Right`.
- `Ctrl+X` support to move the playing video to Trash and advance to the next video.