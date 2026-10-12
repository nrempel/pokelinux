#!/bin/bash
# Harbour component and codec comparison build; no radio or physical device access.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-messaging sim/rust-cargo
image=pixel-phone-rust:20261012-b472
args=(--rm --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-messaging:/target"
 -w /workspace/native-messaging)
case "${1:-build}" in
 prepare) docker run "${args[@]}" "$image" cargo fetch --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu ;;
 build)
  docker run "${args[@]}" --network none "$image" cargo fmt --check
  docker run "${args[@]}" --network none "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
   docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target "$target"
   docker run "${args[@]}" --network none "$image" cargo build --locked --offline --release --target "$target"
  done
  ;;
 *) echo 'Usage: build-native-messaging.sh prepare|build' >&2; exit 2 ;;
esac
