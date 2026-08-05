# Packaging

Rumpel ships as a Debian package, an RPM, and a Flatpak. The `.deb`, `.rpm`, and a bare
`x86_64` binary are built automatically by
[`.github/workflows/release.yml`](../.github/workflows/release.yml) on every `v*` tag; the
instructions below reproduce those builds locally.

## Debian and RPM

```bash
cargo install cargo-deb cargo-generate-rpm
cargo build --release
cargo deb --no-build
cargo generate-rpm
```

The packages are written to `target/debian/` and `target/generate-rpm/`. Runtime dependencies
are declared in [`Cargo.toml`](../Cargo.toml) under `package.metadata.deb` and
`package.metadata.generate-rpm`.

Fedora users may need to install `gstreamer1-libav` separately because it is distributed by
[RPM Fusion](https://rpmfusion.org/) rather than the Fedora repositories, so it cannot be
declared as a hard dependency.

## Flatpak

The manifest is [`packaging/io.lies.rumpel.yml`](../packaging/io.lies.rumpel.yml). It uses
pinned sources in [`packaging/cargo-sources.json`](../packaging/cargo-sources.json);
regenerate that file whenever `Cargo.lock` changes, using the
[Flatpak cargo generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo).

Flatpak builds require `flatpak` and `flatpak-builder`. Install the builder from the host
distribution:

```bash
sudo dnf install flatpak flatpak-builder
# or: sudo apt install flatpak flatpak-builder
```

For an offline build, install the manifest's SDK and Rust extension first:

```bash
flatpak install flathub org.gnome.Sdk//50 org.freedesktop.Sdk.Extension.rust-stable//50
flatpak-builder --force-clean --disable-download \
  --state-dir=target/flatpak/state target/flatpak/build packaging/io.lies.rumpel.yml
```

The Flatpak-distributed Builder application is an alternative to a host installation. With
`org.flatpak.Builder` installed, run the same build through its bundled `flatpak-builder`
binary:

```bash
flatpak run --filesystem="$PWD" org.flatpak.Builder --force-clean --disable-download \
  --state-dir=target/flatpak/state target/flatpak/build packaging/io.lies.rumpel.yml
```

Neither command installs or deploys the resulting Flatpak.

## Desktop integration

Three files drive desktop integration, and all three are installed by the deb and RPM:

| File | Installed to |
| --- | --- |
| [`io.lies.rumpel.desktop`](../packaging/io.lies.rumpel.desktop) | `/usr/share/applications/` |
| [`io.lies.rumpel.metainfo.xml`](../packaging/io.lies.rumpel.metainfo.xml) | `/usr/share/metainfo/` |
| [`io.lies.rumpel.svg`](../packaging/io.lies.rumpel.svg) | `/usr/share/icons/hicolor/scalable/apps/` |

The AppStream metainfo carries a `<releases>` list that has to be extended for each release,
and it currently has no `<screenshots>` element. Flathub requires at least one screenshot, so
that element must be added before a Flathub submission.

## Cutting a release

1. Update `version` in [`Cargo.toml`](../Cargo.toml) and run a build so `Cargo.lock` picks up
   the new version.
2. Move the `[Unreleased]` entries in [`CHANGELOG.md`](../CHANGELOG.md) into a new version
   heading.
3. Add a `<release>` entry to
   [`packaging/io.lies.rumpel.metainfo.xml`](../packaging/io.lies.rumpel.metainfo.xml).
4. Merge to `main`, then tag and push:

   ```bash
   git tag -a v0.0.0 -m "Rumpel v0.0.0"
   git push origin v0.0.0
   ```

The tag push triggers the release workflow, which builds the packages and attaches them to a
GitHub Release. The publish step creates the release if it does not exist and uploads assets
with `--clobber` otherwise, so a re-run against an existing tag is safe.

To rebuild an existing tag, run the workflow manually from the Actions tab with the tag name
as the `tag` input; the workflow checks out that tag rather than the default branch.

> [!IMPORTANT]
> A tag push runs the workflow file **as it exists at that tag**. If the release workflow
> itself was fixed after a tag was created, move the tag to a commit that contains the fix
> (`git tag -f`, then `git push --force origin <tag>`) or use the manual `workflow_dispatch`
> run, which always uses the default branch's workflow file.
