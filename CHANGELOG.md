# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Use `Left` and `Right` for folder video navigation because GTK controls can consume Ctrl-arrow shortcuts.

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