#!/bin/bash
set -euo pipefail
cd /workspace
out=artifacts/sensors-boundary
src=framework/sensorpeer
public=$out/public
sdk=android-framework/sdk
bt=$sdk/build-tools/android-37.0
jar=$sdk/platform/android-37.0/android.jar
ndk=artifacts/camera-boundary/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64
cxx=$ndk/bin/x86_64-linux-android31-clang++
# Reuse immutable pinned toolchain/helper inputs; camera source/build artifacts are never edited.
[ -x "$cxx" ]
python3 tools/prepare-sensors-boundary.py
mkdir -p "$out/headers" "$out/ndk" "$out/classes" "$out/dex" "$out/apk/lib/x86_64"
while IFS= read -r schema; do
 version=1;hash=0000000000000000000000000000000000000000
 if [[ "$schema" == *android/hardware/sensors/* ]]; then version=3;hash=b051523be857524c99057b69f253a7d0bd295400; fi
 "$bt/aidl" --lang=ndk --structured --stability=vintf --min_sdk_version=31 --version="$version" --hash="$hash" -I "$public/frozen-aidl" -I "$src/aidl" -h "$out/headers" -o "$out/ndk" "$schema"
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for root in [Path("artifacts/sensors-boundary/public/frozen-aidl"),Path("framework/sensorpeer/aidl")] for p in sorted(root.rglob("*.aidl"))))')
flags=(-std=c++17 -O2 -g -Wall -Wextra -Werror -Wno-nontrivial-memcall -Wno-deprecated-copy-with-user-provided-copy -I "$out/headers" -I "$public/include" -I "$src/support" -I artifacts/camera-boundary/public-headers -static-libstdc++)
"$cxx" "${flags[@]}" -Wno-unused-command-line-argument -c "$src/event_abi.cpp" -o "$out/event-abi.o"
"$ndk/bin/llvm-nm" -S "$out/event-abi.o" > "$out/event-abi-symbols.txt"
python3 - <<'PY'
from pathlib import Path
p=Path('artifacts/sensors-boundary')
row=next(r.split() for r in (p/'event-abi-symbols.txt').read_text().splitlines() if r.endswith('pokelinux_sensor_event_size'))
size=int(row[1],16);assert size%8==0 and 16<size<=256
(p/'event-size.rs').write_text('pub const EVENT_SIZE: usize = '+str(size)+';\n')
print('Original generated NDK sensor Event quantum:',size,flush=True)
PY
mapfile -t generated < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("artifacts/sensors-boundary/ndk").rglob("*.cpp"))))')
"$cxx" "${flags[@]}" -fPIC -shared "$src/peer.cpp" "$src/support.cpp" "$public/EventFlag.cpp" "$public/native_handle.cpp" "${generated[@]}" -lbinder_ndk -llog -o "$out/apk/lib/x86_64/libsensor_boundary_peer.so"
javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$out/classes" "$src/Peer.java"
"$bt/d8" --lib "$jar" --min-api 31 --output "$out/dex" "$out"/classes/org/pokelinux/sensorpeer/*.class
"$bt/aapt2" link -I "$jar" --manifest "$src/AndroidManifest.xml" -o "$out/unsigned.apk"
(cd "$out/dex" && zip -q ../unsigned.apk classes.dex)
(cd "$out/apk" && zip -q ../unsigned.apk lib/x86_64/libsensor_boundary_peer.so)
"$bt/zipalign" -f 4 "$out/unsigned.apk" "$out/aligned.apk"
"$bt/apksigner" sign --ks android-framework/apks/trusted.jks --ks-pass pass:android --out "$out/peer.apk" "$out/aligned.apk"
"$bt/apksigner" verify "$out/peer.apk"
