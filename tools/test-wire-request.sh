#!/bin/sh
# Host parser test with hash-checked, unchanged AOSP Android 17 JSON sources.
# Not an Android lifecycle or emulator test. Argument: prepared source directory.
set -eu
cd "$(dirname "$0")/.."
sources=${1:?Supply the directory containing the five AOSP JSON source files}
python3 - "$sources" <<'PY'
import hashlib,json,sys,urllib.parse
from pathlib import Path
root=Path(sys.argv[1])
for entry in json.loads(Path('artifacts/rust-native-local/aosp-json-provenance.json').read_text()):
    name=Path(urllib.parse.urlsplit(entry['url']).path).name
    assert hashlib.sha256((root/name).read_bytes()).hexdigest()==entry['sha256'],name
PY
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
# This annotation only directs Android's Ravenwood build tooling. Its stand-in
# does not replace any parser behavior and is never part of the production APK.
cat > "$out/RavenwoodKeepWholeClass.java" <<'JAVA'
package android.ravenwood.annotation;
public @interface RavenwoodKeepWholeClass {}
JAVA
javac -source 8 -target 8 -Xlint:-options -d "$out/classes" \
 "$out/RavenwoodKeepWholeClass.java" \
 "$sources/JsonReader.java" "$sources/JsonToken.java" "$sources/JsonScope.java" \
 "$sources/MalformedJsonException.java" "$sources/StringPool.java" \
 framework/bridge/WireRequest.java framework/unit/WireRequestTest.java
java -cp "$out/classes" org.omarchy.bridge.WireRequestTest
