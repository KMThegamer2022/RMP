#!/bin/bash
# Builds RMP.app (egui) with VLC's runtime bundled, then an installer .dmg using the shipping
# packaging/mac-setup.sh. Usage: build-app.sh [output dir]   (VLC_APP overrides /Applications/VLC.app)
set -eu
here="$(cd "$(dirname "$0")/.." && pwd)"; root="$here/.."
out="${1:-$here/build}"
# Build outside synced folders (iCloud/File Provider adds metadata that codesign rejects).
work=$(mktemp -d)
dest="$work/RMP.app"; bin="$dest/Contents/MacOS"
runtime="${VLC_APP:-/Applications/VLC.app}/Contents/MacOS"
cd "$here"
cargo build --locked --release
mkdir -p "$bin" "$dest/Contents/Resources"
cp target/release/egui-spike "$bin/RMP"
ditto --noextattr --norsrc "$runtime/lib" "$bin/lib"
ditto --noextattr --norsrc "$runtime/plugins" "$bin/plugins"
ditto --noextattr --norsrc "$runtime/share" "$bin/share"
"$root/packaging/fetch-ytdlp.sh" "$bin"
ditto --noextattr --norsrc "$root/notices" "$dest/Contents/Resources/Notices"
cp "$root/assets/RMP.icns" "$root/assets/vlc-cone.png" "$dest/Contents/Resources/"
cp "$root/Info.plist" "$dest/Contents/"
xattr -cr "$dest"
cachegen=$(mktemp -d)
clang "$root/packaging/cache.c" -L"$bin/lib" -lvlc -Wl,-rpath,"$bin/lib" -o "$cachegen/cache"
"$cachegen/cache" "$bin/plugins"; test -s "$bin/plugins/plugins.dat"
rm "$cachegen/cache"; rmdir "$cachegen"
codesign --force --deep --sign - "$dest"
(cd "$root" && ./packaging/mac-setup.sh "$dest" "$work/RMP-egui.dmg")
mkdir -p "$out"; rm -rf "$out/RMP.app"
ditto --noextattr --norsrc "$dest" "$out/RMP.app"; cp "$work/RMP-egui.dmg" "$out/"
echo "Built $out/RMP.app and $out/RMP-egui.dmg (signed copy: open the .dmg)"
