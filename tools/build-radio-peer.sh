#!/bin/bash
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
bt=$sdk/build-tools/android-37.0
jar=$sdk/platform/android-37.0/android.jar
out=artifacts/native-radio/peer
mkdir -p "$out/classes" "$out/dex"
javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$out/classes" framework/radiopeer/Peer.java
"$bt/d8" --lib "$jar" --min-api 31 --output "$out/dex" "$out"/classes/org/pokelinux/radiopeer/*.class
"$bt/aapt2" link -I "$jar" --manifest framework/radiopeer/AndroidManifest.xml -o "$out/unsigned.apk"
(cd "$out/dex" && zip -q ../unsigned.apk classes.dex)
"$bt/zipalign" -f 4 "$out/unsigned.apk" "$out/aligned.apk"
"$bt/apksigner" sign --ks android-framework/apks/trusted.jks --ks-pass pass:android --out "$out/peer.apk" "$out/aligned.apk"
"$bt/apksigner" verify "$out/peer.apk"
