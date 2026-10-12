#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/suspend-boundary/rust sim/rust-cargo
python3 tools/prepare-suspend-boundary.py --verify-only
args=(--rm --init --network none --cpus 2 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/suspend-boundary/rust:/target"
 -w /workspace/native-suspend-boundary pixel-native-radio:20261012)
case "${1:-build}" in
 lock) docker run "${args[@]}" cargo generate-lockfile --offline ;;
 arm64-check)
  docker run "${args[@]}" cargo clippy --all-targets --locked --offline --target aarch64-unknown-linux-gnu -- -D warnings
  docker run "${args[@]}" cargo build --locked --offline --release --target aarch64-unknown-linux-gnu
  ;;
 check) docker run "${args[@]}" cargo check --locked --offline --target x86_64-unknown-linux-musl ;;
 build)
  docker run "${args[@]}" cargo fmt --check
  docker run "${args[@]}" cargo clippy --all-targets --locked --offline --target x86_64-unknown-linux-musl -- -D warnings
  docker run "${args[@]}" cargo build --locked --offline --release --target x86_64-unknown-linux-musl
  ;;
 *) echo 'Usage: build-native-suspend-boundary.sh lock|check|build|arm64-check' >&2;exit 2 ;;
esac
