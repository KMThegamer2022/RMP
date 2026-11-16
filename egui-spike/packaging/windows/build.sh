#!/usr/bin/env bash
# Builds the egui RMP for Windows into build/RMP (same layout as windows/build.sh).
# Usage: build.sh <extracted vlc-3.0.24 win64 dir> [output dir]   (run from the repo root, in Git Bash)
set -euo pipefail
cd "$(dirname "$0")/../../.."
runtime=${1:?Pass the extracted VLC 3.0.24 win64 directory}
out=${2:-build/RMP}
mkdir -p "$out"
export VLC_DIR="$(cd "$runtime" && pwd)"
(cd egui-spike && cargo build --locked --release)
cp egui-spike/target/release/egui-spike.exe "$out/RMP.exe"
cp "$runtime"/*.dll "$out/"
cp -R "$runtime/plugins" "$runtime/lua" notices "$out/"
cp packaging/midi/TimGM6mb.sf2 "$out/"
./packaging/fetch-ytdlp.sh "$out"
