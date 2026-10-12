#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
image=pokelinux-native-connectivity:20261012
mkdir -p artifacts/native-connectivity/target sim/rust-cargo
args=(--rm --cpus 2 --memory 2g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD/native-connectivity:/workspace/native-connectivity" -v "$PWD/native-power:/workspace/native-power:ro" -v "$PWD/sim/rust-cargo:/cargo"
 -v "$PWD/artifacts/native-connectivity/target:/target")
case "${1:-build}" in
 prepare) docker build -f build-support/Dockerfile.native-connectivity -t "$image" build-support ;;
 lock) docker run "${args[@]}" --network none "$image" cargo generate-lockfile --offline ;;
 format) docker run "${args[@]}" --network none "$image" cargo fmt ;;
 build)
  docker run "${args[@]}" --network none "$image" cargo fmt --check
  docker run "${args[@]}" --network none "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  docker run "${args[@]}" --network none "$image" cargo test --locked --offline
  docker run "${args[@]}" --network none "$image" cargo build --locked --offline
  docker run "${args[@]}" --network none "$image" cargo build --locked --offline --release
  docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu
  docker run "${args[@]}" --network none "$image" cargo build --locked --offline --release --target aarch64-unknown-linux-gnu ;;
 *) echo 'Usage: build-native-connectivity.sh prepare|lock|format|build' >&2; exit 2 ;;
esac
