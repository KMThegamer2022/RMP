#!/bin/bash
# The source was verified and staged alongside the destination on the same volume.
set -eu
parent=$1; stage=$2; destination=$3
while kill -0 "$parent" 2>/dev/null; do sleep 0.2; done
if ! mv "$destination" "$stage/previous.app"; then exit 1; fi
if mv "$stage/RMP.app" "$destination" && /usr/bin/open "$destination"; then
 /bin/rm -r "$stage"
else
 if [ -e "$destination" ]; then mv "$destination" "$stage/failed.app"; fi
 mv "$stage/previous.app" "$destination"
 /usr/bin/open "$destination"
 /usr/bin/osascript -e 'display alert "RMP update failed" message "The previous version was restored."'
fi
