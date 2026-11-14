#!/bin/bash
set -eu
cd "$(dirname "$0")"
dest="${1:-$PWD/build/RMP.app}"
runtime="${VLC_APP:-/Applications/VLC.app}/Contents/MacOS"
bin="$dest/Contents/MacOS"
mkdir -p "$bin" "$dest/Contents/Resources"
cargo build --locked --release
ditto --noextattr --norsrc "$runtime/lib" "$bin/lib"
ditto --noextattr --norsrc "$runtime/plugins" "$bin/plugins"
ditto --noextattr --norsrc "$runtime/share" "$bin/share"
clang -fobjc-arc -O2 ui.m "${CARGO_TARGET_DIR:-target}/release/librmp.a" -framework Cocoa -framework QuartzCore -framework Security -L"$bin/lib" -lvlc -Wl,-rpath,@executable_path/lib -o "$bin/RMP"
ditto --noextattr --norsrc notices "$dest/Contents/Resources/Notices"
cp assets/RMP.icns "$dest/Contents/Resources/"
cp Info.plist "$dest/Contents/"
xattr -cr "$dest"
codesign --force --deep --sign - "$dest"
