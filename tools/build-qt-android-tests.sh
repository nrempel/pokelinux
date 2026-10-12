#!/bin/bash
# Build the native test from current source before the owned emulator run.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p sim/native-qt-build artifacts/rust-android
image=pixel-phone-qt-native:20261012-b472
docker image inspect "$image" >/dev/null
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 \
 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
 -v "$PWD:/workspace:ro" -v "$PWD/sim/native-qt-build:/build" "$image" \
 sh -ec 'cmake -S /workspace/native-app -B /build -G Ninja && cmake --build /build --target test-phone-android-view -j3' \
 > artifacts/rust-android/qt-android-build.log 2>&1
sha256sum sim/native-qt-build/test-phone-android-view > artifacts/rust-android/qt-test-binary.sha256
find native-app -type f -not -path '*/build/*' -print0 | sort -z | xargs -0 sha256sum > artifacts/rust-android/qt-test-sources.sha256
docker image inspect "$image" --format '{{.Id}}' > artifacts/rust-android/qt-test-image.txt
