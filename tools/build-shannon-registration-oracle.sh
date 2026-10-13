#!/bin/bash
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
out=artifacts/ims-registration-rea/oracle
mkdir -p "$out/classes" "$out/dex"
javac -cp "$sdk/platform/android-37.0/android.jar" -source 8 -target 8 -Xlint:-options -d "$out/classes" framework/shannon-registration-oracle/RegistrationIndicationOracle.java
"$sdk/build-tools/android-37.0/d8" --lib "$sdk/platform/android-37.0/android.jar" --min-api 31 --output "$out/dex" "$out/classes/org/omarchy/audit/RegistrationIndicationOracle.class"
(cd "$out/dex" && zip -q ../oracle.jar classes.dex)
