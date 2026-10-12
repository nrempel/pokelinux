#!/bin/bash
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
bt=$sdk/build-tools/android-37.0
jar=$sdk/platform/android-37.0/android.jar
out=artifacts/native-sms-receipt/binder/peer
src=framework/smsreceiptpeer
ndk=artifacts/camera-boundary/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64
mkdir -p "$out/ndk" "$out/headers" "$out/classes" "$out/dex" "$out/apk/lib/x86_64"
while IFS= read -r schema; do
 version=3; hash=30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb
 if [[ "$schema" == framework/smsreceiptpeer/* ]]; then version=1; hash=0000000000000000000000000000000000000000; fi
 "$bt/aidl" --lang=ndk --structured --stability=vintf --min_sdk_version=31 --version="$version" --hash="$hash" -I artifacts/sms-binder/frozen-aidl -I "$src/aidl" -h "$out/headers" -o "$out/ndk" "$schema"
done < <(python3 -c 'from pathlib import Path;print("\n".join(str(p) for root in [Path("artifacts/sms-binder/frozen-aidl"),Path("framework/smsreceiptpeer/aidl")] for p in sorted(root.rglob("*.aidl"))))')
if [ "${1:-build}" = generate ]; then exit 0; fi
mapfile -t common < <(python3 -c 'from pathlib import Path;print("\n".join(str(p) for p in sorted(Path("artifacts/native-sms-receipt/binder/peer/ndk").rglob("*.cpp"))))')
"$ndk/bin/x86_64-linux-android31-clang++" -std=c++17 -O2 -g -Wall -Wextra -Werror -fPIC -shared -static-libstdc++ -I "$out/headers" -I artifacts/camera-boundary/public-headers "$src/peer.cpp" "${common[@]}" -lbinder_ndk -ldl -llog -o "$out/apk/lib/x86_64/libsms_receipt_peer.so"
javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$out/classes" "$src/Peer.java"
"$bt/d8" --lib "$jar" --min-api 31 --output "$out/dex" "$out"/classes/org/pokelinux/smsreceiptpeer/*.class
"$bt/aapt2" link -I "$jar" --manifest "$src/AndroidManifest.xml" -o "$out/unsigned.apk"
(cd "$out/dex" && zip -q ../unsigned.apk classes.dex)
(cd "$out/apk" && zip -q ../unsigned.apk lib/x86_64/libsms_receipt_peer.so)
"$bt/zipalign" -f 4 "$out/unsigned.apk" "$out/aligned.apk"
"$bt/apksigner" sign --ks android-framework/apks/trusted.jks --ks-pass pass:android --out "$out/peer.apk" "$out/aligned.apk"
"$bt/apksigner" verify "$out/peer.apk"
