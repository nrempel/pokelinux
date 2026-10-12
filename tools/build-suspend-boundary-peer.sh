#!/bin/bash
# Prepared framework image; public generated NDK code and unchanged public libpower only.
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
bt=$sdk/build-tools/android-37.0
jar=$sdk/platform/android-37.0/android.jar
out=artifacts/suspend-boundary
src=framework/suspendpeer
shared=artifacts/camera-boundary
ndk=$shared/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64
cxx=$ndk/bin/x86_64-linux-android31-clang++
python3 tools/prepare-suspend-boundary.py --verify-only
mkdir -p "$out/ndk" "$out/headers" "$out/classes" "$out/dex" "$out/apk/lib/x86_64" "$out/public/hardware_legacy"
cp "$out/public/libpower17-power.h" "$out/public/hardware_legacy/power.h"
while IFS= read -r schema; do
 flags=(--lang=ndk --structured --min_sdk_version=31)
 if [[ "$schema" == *android/system/suspend/* ]]; then flags+=(--stability=vintf --version=1 --hash=082a58c2505b703702f34ab8abcb1d1b5e890be4); fi
 "$bt/aidl" "${flags[@]}" -I "$src/aidl" -I "$out/frozen-aidl" -h "$out/headers" -o "$out/ndk" "$schema"
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for root in [Path("artifacts/suspend-boundary/frozen-aidl"),Path("framework/suspendpeer/aidl")] for p in sorted(root.rglob("*.aidl"))))')
mapfile -t common < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("artifacts/suspend-boundary/ndk").rglob("*.cpp"))))')
flags=(-std=c++17 -O2 -g -Wall -Wextra -Werror -I "$out/headers" -I "$shared/public-headers" -static-libstdc++)
"$cxx" "${flags[@]}" -fPIC -shared "$src/peer.cpp" "${common[@]}" -lbinder_ndk -ldl -llog -o "$out/apk/lib/x86_64/libsuspend_boundary_peer.so"
for branch in locked unlocked; do
 branch_args=(); if [ "$branch" = unlocked ]; then branch_args+=(-DENABLE_NO_LOCK_BINDER_TXN); fi
 "$cxx" "${flags[@]}" "${branch_args[@]}" -I "$src/platform-adapters" -I "$out/public" "$src/client.cpp" "$out/public/libpower17-power.cpp" "${common[@]}" -Wl,--wrap=AServiceManager_isDeclared -Wl,--wrap=AServiceManager_waitForService -lbinder_ndk -ldl -llog -o "$out/suspend-ndk-client-$branch"
done
javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$out/classes" "$src/Peer.java" "$src/FlagOracle.java"
"$bt/d8" --lib "$jar" --min-api 31 --output "$out/dex" "$out"/classes/org/pokelinux/suspendpeer/*.class
"$bt/aapt2" link -I "$jar" --manifest "$src/AndroidManifest.xml" -o "$out/unsigned.apk"
(cd "$out/dex" && zip -q ../unsigned.apk classes.dex)
(cd "$out/apk" && zip -q ../unsigned.apk lib/x86_64/libsuspend_boundary_peer.so)
"$bt/zipalign" -f 4 "$out/unsigned.apk" "$out/aligned.apk"
"$bt/apksigner" sign --ks android-framework/apks/trusted.jks --ks-pass pass:android --out "$out/peer.apk" "$out/aligned.apk"
"$bt/apksigner" verify "$out/peer.apk"
python3 - <<'EVIDENCE'
import hashlib,json
from pathlib import Path
root=Path('/workspace'); out=root/'artifacts/suspend-boundary'
files=[root/'android-framework/sdk/build-tools/android-37.0/aidl',*sorted((root/'framework/suspendpeer').rglob('*')),*sorted((out/'frozen-aidl').rglob('*.aidl')),*sorted((out/'ndk').rglob('*.cpp')),out/'public/libpower17-power.cpp',out/'suspend-ndk-client-locked',out/'suspend-ndk-client-unlocked',out/'peer.apk']
(out/'ndk-build-evidence.json').write_text(json.dumps({'scope':'original generated NDK proxies/dispatchers, unchanged public power.cpp with logging/tracing and discovery adapters; live Binder execution recorded separately','files':{str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files if p.is_file()}},indent=2)+'\n')
EVIDENCE
