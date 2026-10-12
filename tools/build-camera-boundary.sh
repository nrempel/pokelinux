#!/bin/bash
# In the prepared framework container. Only public/generated synthetic fixtures.
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
bt=$sdk/build-tools/android-37.0
jar=$sdk/platform/android-37.0/android.jar
out=artifacts/camera-boundary
src=framework/camerapeer
ndk=$out/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64
cxx=$ndk/bin/x86_64-linux-android31-clang++
[ -x "$cxx" ] || { echo 'Run tools/prepare-camera-boundary.py on Harbour first' >&2; exit 1; }
python3 - <<'CHECK'
import hashlib,pathlib
pins={
 'android-framework/sdk/build-tools/android-37.0/aidl':'a4ad150a7cf6ea297c299cb779da7a58c537cfc07923d1e15ccbc554a30cd4ba',
 'framework/camerapeer/aidl/android/hardware/common/NativeHandle.aidl':'bda89ace51cfb22a84c6a9d6836776a459706debb2f2a4f398a768fcc5935976',
}
for name,digest in pins.items():
 assert hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest()==digest, 'Unqualified camera probe input: '+name
CHECK
mkdir -p "$out/ndk" "$out/headers" "$out/classes" "$out/dex" "$out/apk/lib/x86_64"
for schema in android/hardware/common/NativeHandle org/pokelinux/cameratest/ICameraBoundaryProbe; do
 "$bt/aidl" --lang=ndk --structured --stability=vintf --min_sdk_version=31 --version=1 --hash=0000000000000000000000000000000000000000 -I "$src/aidl" -h "$out/headers" -o "$out/ndk" "$src/aidl/$schema.aidl"
done
common=("$out/ndk/android/hardware/common/NativeHandle.cpp" "$out/ndk/org/pokelinux/cameratest/ICameraBoundaryProbe.cpp")
flags=(-std=c++17 -O2 -g -Wall -Wextra -Werror -I "$out/headers" -I "$out/public-headers" -static-libstdc++)
"$cxx" "${flags[@]}" -fPIC -shared "$src/peer.cpp" "${common[@]}" -lbinder_ndk -ldl -llog -o "$out/apk/lib/x86_64/libcamera_boundary_peer.so"
"$cxx" "${flags[@]}" "$src/client.cpp" "${common[@]}" -lbinder_ndk -ldl -llog -o "$out/camera-ndk-client"
javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$out/classes" "$src/Peer.java"
"$bt/d8" --lib "$jar" --min-api 31 --output "$out/dex" "$out"/classes/org/pokelinux/camerapeer/*.class
"$bt/aapt2" link -I "$jar" --manifest "$src/AndroidManifest.xml" -o "$out/unsigned.apk"
(cd "$out/dex" && zip -q ../unsigned.apk classes.dex)
(cd "$out/apk" && zip -q ../unsigned.apk lib/x86_64/libcamera_boundary_peer.so)
"$bt/zipalign" -f 4 "$out/unsigned.apk" "$out/aligned.apk"
"$bt/apksigner" sign --ks android-framework/apks/trusted.jks --ks-pass pass:android --out "$out/peer.apk" "$out/aligned.apk"
"$bt/apksigner" verify "$out/peer.apk"
python3 - <<'PY'
import hashlib,json,pathlib
root=pathlib.Path('/workspace'); out=root/'artifacts/camera-boundary'
files=[root/'android-framework/sdk/build-tools/android-37.0/aidl',out/'toolchain.json',out/'public-headers.json',*sorted((out/'public-headers').rglob('*.h')),*sorted((root/'framework/camerapeer').rglob('*')), *sorted((out/'ndk').rglob('*.cpp')),out/'camera-ndk-client',out/'apk/lib/x86_64/libcamera_boundary_peer.so',out/'peer.apk']
(out/'build-evidence.json').write_text(json.dumps({'scope':'original public compiler plus compiled original generated NDK dispatcher/proxy; runtime evidence separate','files':{str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files if p.is_file()}},indent=2)+'\n')
PY
