#!/bin/bash
# Full native app regression using actual Rust test fixtures in owned containers.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-theme
[[ ! -e artifacts/native-theme/qt-final ]] || { echo 'Preserve existing qt-final evidence before a new run' >&2; exit 1; }
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL \
  --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/results/rust-fixtures \
  -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" \
  -v "$PWD/artifacts/native-theme:/results:rw" -w /workspace/runtime \
  pixel-native-radio:20261012 cargo build --offline --locked --bins --examples -j3 \
  > artifacts/native-theme/rust-fixture-build.log 2>&1
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL \
  --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace:ro" -v "$PWD/artifacts/native-theme:/results:rw" \
  --entrypoint python3 pixel-phone-qt-native:20261012-b472 /workspace/tools/test-native-theme-qt.py
