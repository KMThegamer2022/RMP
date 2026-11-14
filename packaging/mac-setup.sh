#!/bin/bash
set -eu
src=$1
out=$2
tmp=$(mktemp -d)
setup="$tmp/image/RMP Setup.app/Contents"
mkdir -p "$setup/MacOS" "$setup/Resources"
ditto "$src" "$setup/Resources/RMP.app"
cp "$src/Contents/Resources/RMP.icns" "$setup/Resources/"
cp "$src/Contents/Info.plist" "$setup/"
for pair in 'CFBundleIdentifier io.github.kmthegamer2022.rmp.setup' 'CFBundleExecutable Setup' 'CFBundleName RMP-Setup'; do
 /usr/libexec/PlistBuddy -c "Set :${pair%% *} ${pair#* }" "$setup/Info.plist"
done
/usr/libexec/PlistBuddy -c 'Delete :CFBundleDocumentTypes' "$setup/Info.plist"
clang -fobjc-arc packaging/mac-setup.m -framework Cocoa -o "$setup/MacOS/Setup"
codesign -fs - "$tmp/image/RMP Setup.app"
hdiutil create -quiet -volname 'Install RMP' -srcfolder "$tmp/image" -format UDZO "$out"
