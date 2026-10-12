#!/bin/bash
# Build only the production APK. Does not create or use signing credentials.
set -euo pipefail
cd "$(dirname "$0")/.."
sdk=${1:-android-framework/sdk}
output=${2:-artifacts/android-controls/bridge-unsigned.apk}
bt="$sdk/build-tools/android-37.0"
jar="$sdk/platform/android-37.0/android.jar"
for tool in javac zip python3; do command -v "$tool" >/dev/null; done
for tool in d8 aapt2 zipalign; do test -x "$bt/$tool"; done
test -f "$jar"
test ! -e "$output" || { echo 'Output already exists; choose a new path' >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/classes" "$tmp/dex"
javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$tmp/classes" framework/bridge/*.java
"$bt/d8" --lib "$jar" --min-api 31 --output "$tmp/dex" "$tmp"/classes/org/omarchy/bridge/*.class
"$bt/aapt2" link -I "$jar" --manifest framework/bridge/AndroidManifest.xml -o "$tmp/base.apk"
(cd "$tmp/dex" && zip -q "$tmp/base.apk" classes.dex)
"$bt/zipalign" -f 4 "$tmp/base.apk" "$tmp/aligned.apk"
python3 tools/verify-release-apk.py "$tmp/aligned.apk"
"$bt/zipalign" -c 4 "$tmp/aligned.apk"
mkdir -p "$(dirname "$output")"
# Exclusive create prevents concurrent builds from replacing an existing artifact.
python3 - "$tmp/aligned.apk" "$output" <<'PY'
import sys
from pathlib import Path
with Path(sys.argv[2]).open('xb') as dest:dest.write(Path(sys.argv[1]).read_bytes())
PY
printf '%s\n' "$output"
