#!/usr/bin/env bash
# Downloads the pinned yt-dlp helper into a bundle directory after checking its SHA-256 digest.
set -euo pipefail
dest=${1:?Pass the directory to place the helper in}
version=2026.10.26
if [[ "${OSTYPE:-}" == msys* || "${OSTYPE:-}" == cygwin* ]]; then
 asset=yt-dlp.exe
 digest=f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d
else
 asset=yt-dlp_macos
 digest=0f922c15ccc0b83eefa1c8bc9104e434318c4bf16e408e1ee93b7745a6efe0aa
fi
mkdir -p "$dest"
tmp=$(mktemp)
curl -sS --fail --location --retry 3 "https://github.com/yt-dlp/yt-dlp/releases/download/$version/$asset" -o "$tmp"
actual=$(shasum -a 256 "$tmp" 2>/dev/null | cut -d' ' -f1 || sha256sum "$tmp" | cut -d' ' -f1)
if [ "$actual" != "$digest" ]; then rm -f "$tmp"; echo "Unexpected yt-dlp digest: $actual" >&2; exit 1; fi
name=yt-dlp; [[ "$asset" == *.exe ]] && name=yt-dlp.exe
mv "$tmp" "$dest/$name"
chmod +x "$dest/$name"
