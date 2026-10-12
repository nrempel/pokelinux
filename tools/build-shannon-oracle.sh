#!/bin/bash
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
out=artifacts/ims-native-audit/oracle
mkdir -p "$out/classes" "$out/dex"
javac -source 8 -target 8 -Xlint:-options -d "$out/classes" framework/shannon-oracle/*.java
"$sdk/build-tools/android-37.0/d8" --lib "$sdk/platform/android-37.0/android.jar" --min-api 31 --output "$out/dex" "$out"/classes/org/omarchy/audit/*.class
(cd "$out/dex" && zip -q ../oracle.jar classes.dex)
