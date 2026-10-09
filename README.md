<div align="center">

<img src="packaging/io.lies.rumpel.svg" width="112" alt="Rumpel logo">

# Rumpel

**RU**st **M**edia **P**lay**E**r for **L**inux

A minimal GTK4 + GStreamer video player for Linux and Windows. One window per video, one control bar, nothing else.

[![CI](https://github.com/phillies/rumpel/actions/workflows/ci.yml/badge.svg)](https://github.com/phillies/rumpel/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/phillies/rumpel?color=blue)](https://github.com/phillies/rumpel/releases/latest)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](LICENSE)
[![Rust 1.92+](https://img.shields.io/badge/rust-1.92%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Platform: Linux and Windows](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey)](#installation)

<!-- Screenshot goes here once captured:
<img src="docs/screenshot.png" width="720" alt="Rumpel playing a video with the media-information overlay open">
-->

</div>

---

## Why Rumpel?

Most video players want to be a media library. Rumpel wants to show you the file you
double-clicked and get out of the way.

- **One window per video.** Open five files, get five windows. No playlist, no queue.
- **One control bar.** Seek, play/pause, stop, loop, mute, volume, scaling, open, info.
- **Folder-native.** <kbd>←</kbd> / <kbd>→</kbd> walk the videos next to the current file, and
  <kbd>Ctrl</kbd>+<kbd>X</kbd> trashes the current one and moves on — a fast way to triage a
  directory of clips.
- **Nothing to configure.** Mute and loop are remembered; everything else is a click.

Deliberately out of scope: playlists, a media library, a subtitle UI, streaming URLs, and a
settings dialog.

## Installation

### Download a release

Prebuilt packages are attached to
[the latest release](https://github.com/phillies/rumpel/releases/latest): `.deb`, `.rpm`, a bare
`x86_64` Linux binary, and `rumpel-windows-x86_64.zip` for Windows.

```bash
# Debian / Ubuntu
sudo apt install ./rumpel_*_amd64.deb

# Fedora
sudo dnf install ./rumpel-*.x86_64.rpm
```

### Windows: run a release

The Windows zip is self-contained. There is nothing to install beforehand:

1. Download `rumpel-windows-x86_64.zip` from the latest release.
2. Unzip it anywhere, for example `%LOCALAPPDATA%\Programs\Rumpel`.
3. Run `rumpel\rumpel.exe`, or right-click a video and choose **Open with** → `rumpel.exe`.

The zip bundles GTK4, GStreamer with the good, bad, and libav plugin sets (common H.264 and H.265
decoders), and the icon theme. Hardware decoding uses Direct3D 12 or 11 through the GPU driver you
already have. The first start takes a few seconds longer while GStreamer indexes its plugins.

Windows may show a SmartScreen warning because the executable is not code-signed: choose
**More info** → **Run anyway**.

### Linux runtime dependencies

<details>
<summary><b>Fedora</b></summary>

```bash
sudo dnf install gtk4 gstreamer1 gstreamer1-plugins-good \
  gstreamer1-plugins-bad-free gstreamer1-libav gstreamer1-vaapi
```

</details>

<details>
<summary><b>Debian / Ubuntu</b></summary>

```bash
sudo apt install libgtk-4-1 gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-vaapi
```

</details>

`*-libav` provides common H.264/H.265 software decoders; on Fedora it comes from
[RPM Fusion](https://rpmfusion.org/). `*-vaapi` enables hardware decoding on supported GPUs.
Codec availability varies by distribution and region.

### Build from source on Linux

Requires Rust 1.92 or newer, plus GTK4 and GStreamer development files.

<details>
<summary><b>Fedora</b></summary>

```bash
sudo dnf install gtk4-devel gstreamer1-devel \
  gstreamer1-plugins-base-devel pkgconf-pkg-config
```

</details>

<details>
<summary><b>Debian / Ubuntu</b></summary>

```bash
sudo apt install libgtk-4-dev libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev pkg-config
```

</details>

```bash
cargo build --release
./target/release/rumpel video.mp4
```

### Build from source on Windows

Rumpel builds on Windows with the MSYS2 MINGW64 environment, which provides the GCC toolchain,
Rust, and the GTK4 and GStreamer development files in one place. The steps below use only MSYS2;
no separate Rust installer is needed.

1. **Install MSYS2.** Download the installer from [msys2.org](https://www.msys2.org/) and keep the
   default location, `C:\msys64`.
2. **Open the MINGW64 shell.** From the Start menu, open **MSYS2 MINGW64**. Do not use the plain
   *MSYS2* or *UCRT64* shells; the commands below assume MINGW64.
3. **Update MSYS2.** Run `pacman -Syu`. If it asks you to close the terminal, close it, reopen
   **MSYS2 MINGW64**, and run `pacman -Su`.
4. **Install the toolchain and libraries:**

   ```bash
   pacman -S --needed base-devel \
     mingw-w64-x86_64-toolchain mingw-w64-x86_64-rust mingw-w64-x86_64-pkgconf \
     mingw-w64-x86_64-gtk4 mingw-w64-x86_64-gstreamer \
     mingw-w64-x86_64-gst-plugins-base mingw-w64-x86_64-gst-plugins-good \
     mingw-w64-x86_64-gst-plugins-bad mingw-w64-x86_64-gst-libav
   ```

   `mingw-w64-x86_64-rust` must provide Rust 1.92 or newer; check with `rustc --version`.
5. **Get the source.** Clone the repository inside the shell, for example
   `git clone https://github.com/phillies/rumpel && cd rumpel`.
6. **Build:**

   ```bash
   cargo build --release
   ```

7. **Run.** From the same MINGW64 shell:

   ```bash
   ./target/release/rumpel.exe video.mp4
   ```

   The MINGW64 shell puts the GTK and GStreamer DLLs on `PATH`. To start `rumpel.exe` from
   PowerShell or File Explorer instead, either build the portable zip (below) or add
   `C:\msys64\mingw64\bin` to your `PATH`.

**Portable zip.** To produce the same `rumpel-windows-x86_64.zip` that the releases ship, run
`packaging/windows/bundle.sh` from the MINGW64 shell after the release build. Details are in
[docs/PACKAGING.md](docs/PACKAGING.md#windows-portable-zip).

Release builds are Windows GUI applications without a console window, so `rumpel --version` may
print nothing when started from Explorer or PowerShell. Use `cargo run -- --version` in the MINGW64
shell to see the version.

## Usage

```bash
rumpel movie.mp4              # play one file
rumpel first.mp4 second.mkv   # one window per file
rumpel                        # start empty, pick a file with the folder button
rumpel --version              # print the version and exit
```

Opening a video from a window that already has one creates a new window.

### Controls

| Input | Action |
| --- | --- |
| <kbd>←</kbd> or <kbd>Ctrl</kbd>+<kbd>S</kbd> | Previous video in the current directory |
| <kbd>→</kbd> or <kbd>Ctrl</kbd>+<kbd>D</kbd> | Next video in the current directory |
| <kbd>S</kbd> | Jump back by 20% of the video's length |
| <kbd>D</kbd> | Jump forward by 20% of the video's length |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | Open a video |
| <kbd>Ctrl</kbd>+<kbd>X</kbd> | Move the current video to Trash, open the next one |
| Click on video | Play / pause |
| Double-click on video | Toggle fullscreen |
| Info button | Toggle the media-information overlay |

Folder navigation covers common Matroska, MP4, WebM, AVI, MPEG, FLV, and WMV extensions, orders
files case-insensitively, and wraps at either end.

Mute and loop persist across runs in `$XDG_CONFIG_HOME/rumpel.conf` on Linux, and in
`%LOCALAPPDATA%\rumpel.conf` on Windows. Rumpel sets `GSK_RENDERER=opengl` unless the variable is
already set in the environment.

## Contributing

Contributions are welcome — start with [CONTRIBUTING.md](CONTRIBUTING.md). Participation is
governed by the [Code of Conduct](CODE_OF_CONDUCT.md).

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo run -- video.mp4
```

The player is a single file: [src/main.rs](src/main.rs). Building the Debian, RPM, and Flatpak
packages is covered in [docs/PACKAGING.md](docs/PACKAGING.md).

| | |
| --- | --- |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |
| Security policy | [SECURITY.md](SECURITY.md) |
| Releases | [github.com/phillies/rumpel/releases](https://github.com/phillies/rumpel/releases) |

## Acknowledgements

Built on [GTK4](https://www.gtk.org/), [GStreamer](https://gstreamer.freedesktop.org/), and
[gst-plugin-gtk4](https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs).

## License

Copyright (C) 2026 Philipp Lies.

Rumpel is licensed under the
[GNU General Public License, version 3 or later](LICENSE).
