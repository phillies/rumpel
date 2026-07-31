# Rumpel
Rust Media Player for Linux

A minimal video player for Linux (GTK4 + GStreamer). One slim control bar, multiple
windows, no fuzz.

**Features:** seek bar, play/pause/stop, loop, volume/mute, media information,
folder navigation, Trash, three scaling modes (fit / stretch / cover), one window
per video.

## For users

### 1. Install the system libraries

Fedora:

```bash
sudo dnf install gtk4 gstreamer1 gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-libav gstreamer1-vaapi
```

Debian/Ubuntu:

```bash
sudo apt install libgtk-4-1 gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-vaapi
```

`*-libav` provides the H.264/H.265 software decoders most videos need (on Fedora
it comes from [RPM Fusion](https://rpmfusion.org/)). `*-vaapi` enables hardware
decoding for supported GPUs, which is important for smooth 4K playback. Fedora
AMD systems additionally need RPM Fusion's `mesa-va-drivers-freeworld` for
patent-encumbered H.264/H.265 VA-API profiles.

### 2. Install Rumpel

With an rpm or deb package (see "Packaging" below for how to build one), the
system libraries above are installed automatically and you can skip step 1:

```bash
sudo dnf install ./rumpel-*.rpm      # Fedora
sudo apt install ./rumpel_*.deb      # Debian/Ubuntu
```

Alternatively, install with cargo (comes with [rustup](https://rustup.rs/));
this also needs the dev headers from the "For developers" section below:

```bash
cargo install --path .
```

This puts `rumpel` in `~/.cargo/bin` (on your PATH if you use rustup).

### 3. Desktop integration (optional)

Package installs (rpm/deb) already include the desktop entry — skip this step.
For cargo installs, to open videos with Rumpel from your file manager
(Dolphin, Nautilus, …), install a desktop entry:

```bash
cat > ~/.local/share/applications/rumpel.desktop <<EOF
[Desktop Entry]
Type=Application
Name=Rumpel
Comment=Minimal video player
Exec=$HOME/.cargo/bin/rumpel %U
Icon=multimedia-player
Terminal=false
Categories=AudioVideo;Video;Player;
MimeType=video/mp4;video/x-matroska;video/webm;video/x-msvideo;video/quicktime;video/mpeg;video/ogg;video/x-flv;video/x-ms-wmv;
EOF
update-desktop-database ~/.local/share/applications
```

Rumpel then shows up in the "Open With" menu for video files. The absolute
path in `Exec` matters: `~/.cargo/bin` is usually not on the desktop
session's PATH.

### 4. Play something

```bash
rumpel movie.mp4            # one window
rumpel a.mp4 b.mkv          # one window per video
rumpel                      # empty window, open a file from the folder button
```

The folder button loads into an empty window, or opens a new window if the
current one is already playing.

### Keyboard shortcuts

`Ctrl+Left` and `Ctrl+Right` move to the previous or next video in the current
file's folder. Files are ordered alphabetically by name and navigation wraps at
each end. Common video formats, including Matroska, MP4, WebM, AVI, MPEG, FLV,
and WMV, are included.

`Ctrl+X` moves the playing file to the desktop Trash and opens the next video
with wrapping. With no files remaining, the player becomes empty. The `i`
control displays a transparent upper-right overlay with the available video and
audio stream codec details, resolution, frame rate, and audio channel counts.

## For developers

### Setup

Rust (via [rustup](https://rustup.rs/)) plus the GTK4 and GStreamer dev headers:

Fedora:

```bash
sudo dnf install gtk4-devel gstreamer1-devel gstreamer1-plugins-base-devel gstreamer1-libav gstreamer1-vaapi
```

Debian/Ubuntu:

```bash
sudo apt install libgtk-4-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-libav gstreamer1.0-vaapi
```

### Build, run, test

```bash
cargo run -- video.mp4      # debug build and run
cargo test                  # unit tests
cargo build --release       # optimized binary in target/release/rumpel
```

The whole player is a single file: [`src/main.rs`](src/main.rs). Playback is a
GStreamer `playbin3` rendered into a `GtkPicture` via `gtk4paintablesink`; every
control is a property binding or a one-liner on the pipeline. Each window owns
its own pipeline, which is what makes multi-window free.

### Packaging (rpm / deb)

Package metadata lives in [`Cargo.toml`](Cargo.toml)
(`[package.metadata.generate-rpm]` and `[package.metadata.deb]`); the desktop
entry ships from [`packaging/rumpel.desktop`](packaging/rumpel.desktop).
Runtime dependencies (GTK4, GStreamer plugins) are declared there, so
`dnf`/`apt` installs them along with the package.

```bash
cargo install cargo-generate-rpm cargo-deb   # once

cargo build --release
cargo generate-rpm     # -> target/generate-rpm/rumpel-*.rpm
cargo deb              # -> target/debian/rumpel_*.deb
```

Install with `sudo dnf install ./rumpel-*.rpm` or
`sudo apt install ./rumpel_*.deb`. Note: the deb hard-depends on
`gstreamer1.0-libav`; the rpm can't, because on Fedora the codec pack comes
from RPM Fusion — users without it need
`sudo dnf install gstreamer1-libav` for H.264/H.265. Build each package on
its own distro family (the deb on Debian/Ubuntu) so the binary links against
the right library versions.

### Packaging (Flatpak)

The manifest is [`packaging/io.lies.rumpel.yml`](packaging/io.lies.rumpel.yml).
Flatpak builds are offline, so crate downloads are pinned in
`packaging/cargo-sources.json` — regenerate it whenever `Cargo.lock` changes:

```bash
# one-time: generator script + its deps
python3 -m venv .venv && .venv/bin/pip install aiohttp tomlkit
curl -LO https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
.venv/bin/python flatpak-cargo-generator.py Cargo.lock -o packaging/cargo-sources.json
```

Build and install (no root needed):

```bash
flatpak install --user flathub org.flatpak.Builder \
    org.gnome.Platform//50 org.gnome.Sdk//50 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08

flatpak run org.flatpak.Builder --user --force-clean \
    --state-dir=target/flatpak/state target/flatpak/build \
    packaging/io.lies.rumpel.yml

flatpak run io.lies.rumpel video.mp4
```

H.264/H.265 come from the `org.freedesktop.Platform.ffmpeg-full` runtime
extension (flatpak offers it on install). For a Flathub submission you'd
additionally need an appstream metainfo file and an app icon — not included
yet.
