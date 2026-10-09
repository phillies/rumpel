#!/usr/bin/env bash
# Assemble the portable Windows zip from a release build.
#
# Run from the MSYS2 MINGW64 shell after `cargo build --release`:
#   packaging/windows/bundle.sh [output-dir]
#
# Produces <output-dir>/rumpel-windows-x86_64.zip containing a rumpel/ folder.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PREFIX="${MINGW_PREFIX:-/mingw64}"
OUT="${1:-$ROOT/target/windows-bundle}"
DIST="$OUT/rumpel"

rm -rf "$OUT"
mkdir -p "$DIST"

cp "$ROOT/target/release/rumpel.exe" "$ROOT/LICENSE" "$DIST/"

# Copy every MinGW DLL that the given PE files need, then the DLLs those need, and so on.
copy_deps() {
    local -a queue=("$@")
    while ((${#queue[@]})); do
        local file="${queue[0]}"
        queue=("${queue[@]:1}")
        while read -r dll; do
            [[ -f "$DIST/$dll" ]] && continue
            [[ -f "$PREFIX/bin/$dll" ]] || continue
            cp "$PREFIX/bin/$dll" "$DIST/"
            queue+=("$DIST/$dll")
        done < <(objdump -p "$file" | awk '/DLL Name/ {print $3}')
    done
}

# GStreamer plugins and the scanner helper, which GStreamer runs as a separate process
mkdir -p "$DIST/lib/gstreamer-1.0" "$DIST/libexec"
cp "$PREFIX"/lib/gstreamer-1.0/*.dll "$DIST/lib/gstreamer-1.0/"
cp "$PREFIX/libexec/gstreamer-1.0/gst-plugin-scanner.exe" "$DIST/libexec/"
copy_deps "$DIST/rumpel.exe" "$DIST"/lib/gstreamer-1.0/*.dll "$DIST/libexec/gst-plugin-scanner.exe"

# gdk-pixbuf loaders (SVG and PNG icons) and the cache that lists them.
# The cache must name the loaders by their install-prefix paths: gdk-pixbuf relocates
# paths under the prefix it was built with to wherever rumpel.exe actually lives.
PIXBUF_DIR="lib/gdk-pixbuf-2.0/2.10.0"
mkdir -p "$DIST/$PIXBUF_DIR"
cp -r "$PREFIX/$PIXBUF_DIR/loaders" "$DIST/$PIXBUF_DIR/"
copy_deps "$DIST/$PIXBUF_DIR"/loaders/*.dll
mapfile -t pixbuf_loaders < <(cygpath -m "$PREFIX/$PIXBUF_DIR"/loaders/*.dll)
"$PREFIX/bin/gdk-pixbuf-query-loaders.exe" "${pixbuf_loaders[@]}" > "$DIST/$PIXBUF_DIR/loaders.cache"

# GTK modules loaded on demand (print backends, where the MSYS2 package ships them)
if [[ -d "$PREFIX/lib/gtk-4.0" ]]; then
    cp -r "$PREFIX/lib/gtk-4.0" "$DIST/lib/"
    while IFS= read -r -d '' module; do
        copy_deps "$module"
    done < <(find "$DIST/lib/gtk-4.0" -name '*.dll' -print0)
fi

# Compiled GSettings schemas (GTK needs org.gtk.Settings.*) and icon themes
mkdir -p "$DIST/share/glib-2.0" "$DIST/share/icons"
cp -r "$PREFIX/share/glib-2.0/schemas" "$DIST/share/glib-2.0/"
cp -r "$PREFIX/share/icons/Adwaita" "$PREFIX/share/icons/hicolor" "$DIST/share/icons/"

(cd "$OUT" && bsdtar -a -cf rumpel-windows-x86_64.zip rumpel)
echo "Wrote $OUT/rumpel-windows-x86_64.zip"
