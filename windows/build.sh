#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
runtime=${1:?Pass the extracted VLC 3.0.24 win64 directory}
out=${2:-build/RMP}
mkdir -p "$out"
cargo build --locked --release --target x86_64-pc-windows-gnu
cc=${CC:-x86_64-w64-mingw32-gcc}
rc=${WINDRES:-x86_64-w64-mingw32-windres}
"$rc" -I . windows/rmp.rc -O coff -o "$out/icon.o"
"$cc" -Wall -O2 -municode -mwindows -static windows/player.c "$out/icon.o" \
 "${CARGO_TARGET_DIR:-target}/x86_64-pc-windows-gnu/release/librmp.a" "$runtime/sdk/lib/libvlc.lib" \
 -lcomctl32 -lcomdlg32 -lshell32 -luserenv -lws2_32 -lntdll -lbcrypt -o "$out/RMP.exe"
rm "$out/icon.o"
cp "$runtime"/*.dll "$out/"
cp -R "$runtime/plugins" "$runtime/lua" notices "$out/"
