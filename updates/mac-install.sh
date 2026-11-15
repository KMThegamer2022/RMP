#!/bin/bash
# Both the verified new bundle and any backup live on the destination volume.
set -eu
parent=$1; stage=$2; destination=$3
alert(){ /usr/bin/osascript -e 'on run argv' -e 'display alert "RMP update failed" message (item 1 of argv)' -e 'end run' "$1"; }
while kill -0 "$parent" 2>/dev/null; do sleep 0.2; done
if [ -e "$destination" ] || [ -L "$destination" ]; then
 info="$destination/Contents/Info.plist"
 bundle_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$info" 2>/dev/null || true)
 if [ -L "$destination" ] || [ "$bundle_id" != io.github.kmthegamer2022.rmp ]; then
  alert "Another item already occupies $destination. It was left in place."
  exit 1
 fi
 if ! mv "$destination" "$stage/previous.app"; then
  alert "RMP could not replace $destination. The existing copy was left in place."
  exit 1
 fi
fi
if mv "$stage/RMP.app" "$destination"; then
 if /usr/bin/open -n "$destination"; then
  desktop="$HOME/Desktop/RMP.app"
  if [ "$(/usr/bin/defaults read io.github.kmthegamer2022.rmp DesktopShortcut 2>/dev/null || echo 1)" != 0 ]; then
   [ -L "$desktop" ] && /bin/rm -f "$desktop"
   [ -e "$desktop" ] || /bin/ln -s "$destination" "$desktop" || true
  fi
  /bin/rm -r "$stage"; exit 0
 fi
 mv "$destination" "$stage/failed.app"
fi
if [ -e "$stage/previous.app" ]; then
 mv "$stage/previous.app" "$destination"
 /usr/bin/open "$destination" || true
 alert "The previous version was restored."
else
 alert "The new copy could not be installed and opened. You can still use your original copy."
fi
exit 1
