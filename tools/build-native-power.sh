#!/bin/bash
# Component tests only; never opens a power, suspend or device control interface.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-power sim/rust-cargo
image=pixel-phone-rust:20261012-b472
args=(--rm --cpus 2 --memory 2g --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-power:/target"
 -w /workspace/native-power)
case "${1:-build}" in
 lock) docker run "${args[@]}" --network none "$image" cargo generate-lockfile --offline ;;
 test) docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu ;;
 build)
  docker run "${args[@]}" --network none "$image" cargo fmt --check
  docker run "${args[@]}" --network none "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
   docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target "$target"
  done
  ;;
 *) echo 'Usage: build-native-power.sh lock|test|build' >&2; exit 2 ;;
esac
