# Rumpel
**RU**st **M**edia **P**lay**E**r for **L**inux

Rumpel is a minimal video player for Linux, built with GTK4 and GStreamer. It
opens one window per video and keeps the player interface to a single control
bar.

Project home: [github.com/phillies/rumpel](https://github.com/phillies/rumpel)

## Features

- Play one or more video files, with one player window for each file.
- Seek, play, pause, stop, loop, mute, and change volume.
- Scale video to fit, stretch, or cover the window.
- Browse neighbouring videos in the active file's directory.
- Move the active video to the desktop Trash.
- Inspect available audio and video stream details.

## Installation

Release packages are not published yet. Build a package from source, or run
Rumpel directly with Cargo.

### Runtime dependencies

Fedora:

```bash
sudo dnf install gtk4 gstreamer1 gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-libav gstreamer1-vaapi
```

Debian/Ubuntu:

```bash
sudo apt install libgtk-4-1 gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-vaapi
```

`*-libav` provides common H.264/H.265 software decoders. On Fedora it is
available from [RPM Fusion](https://rpmfusion.org/). `*-vaapi` enables hardware
decoding on supported GPUs. Codec availability can vary by distribution and
region.

### Build from source

Rumpel requires Rust 1.92 or newer, GTK4 development files, and GStreamer
development files.

Fedora:

```bash
sudo dnf install gtk4-devel gstreamer1-devel gstreamer1-plugins-base-devel pkgconf-pkg-config
```

Debian/Ubuntu:

```bash
sudo apt install libgtk-4-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev pkg-config
```

```bash
cargo build --release
./target/release/rumpel video.mp4
```

## Usage

```bash
rumpel movie.mp4
rumpel first.mp4 second.mkv
rumpel
rumpel --version
```

With no argument, use the folder control to select a video. Opening a video
from an occupied window creates a new player window.

`rumpel --version` prints the installed Rumpel version and exits without
starting the player.

| Shortcut | Action |
| --- | --- |
| `Left` | Open the previous video in the current directory |
| `Right` | Open the next video in the current directory |
| `Ctrl+X` | Move the active video to Trash and open the next video |
| `i` | Toggle media information |

Directory navigation supports common Matroska, MP4, WebM, AVI, MPEG, FLV, and
WMV extensions. Files are ordered alphabetically without case sensitivity and
navigation wraps at either end.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo run -- video.mp4
```

The player implementation is in [src/main.rs](src/main.rs). See the
[changelog](CHANGELOG.md) for released changes.

## Packaging

Build Debian and RPM packages with the corresponding packagers installed:

```bash
cargo install cargo-deb cargo-generate-rpm
cargo build --release
cargo deb --no-build
cargo generate-rpm
```

The packages are written to `target/debian/` and `target/generate-rpm/`.
Runtime dependencies are declared in [Cargo.toml](Cargo.toml). Fedora users may
need to install `gstreamer1-libav` separately because it is distributed by RPM
Fusion.

The Flatpak manifest is [packaging/io.lies.rumpel.yml](packaging/io.lies.rumpel.yml).
It uses pinned sources in [packaging/cargo-sources.json](packaging/cargo-sources.json);
regenerate that file whenever `Cargo.lock` changes using the
[Flatpak cargo generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo).

### Flatpak

Flatpak builds require `flatpak` and `flatpak-builder`. Install the builder from
the host distribution, for example:

```bash
sudo dnf install flatpak flatpak-builder
# or: sudo apt install flatpak flatpak-builder
```

For an offline build, install the manifest's SDK and Rust extension first:

```bash
flatpak install flathub org.gnome.Sdk//50 org.freedesktop.Sdk.Extension.rust-stable//50
flatpak-builder --force-clean --disable-download --state-dir=target/flatpak/state target/flatpak/build packaging/io.lies.rumpel.yml
```

The Flatpak-distributed Builder application is an alternative to a host
installation. With `org.flatpak.Builder` installed, run the same build through
its bundled `flatpak-builder` binary:

```bash
flatpak run --filesystem="$PWD" org.flatpak.Builder --force-clean --disable-download --state-dir=target/flatpak/state target/flatpak/build packaging/io.lies.rumpel.yml
```

Neither command installs or deploys the resulting Flatpak.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before
opening an issue or pull request. Participation is governed by the
[Code of Conduct](CODE_OF_CONDUCT.md).

To report a security vulnerability, follow [SECURITY.md](SECURITY.md) rather
than filing a public issue.

## License

Copyright (C) 2026 Philipp Lies.

Rumpel is licensed under the [GNU General Public License, version 3 or later](LICENSE).
