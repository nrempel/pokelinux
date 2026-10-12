#!/bin/bash
# Injected temporary descriptors only. Does not open host power interfaces.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-suspend/rust artifacts/native-suspend/target sim/rust-cargo
args=(--rm --init --cpus 2 --memory 2g --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-suspend/target:/target"
 -w /workspace/native-suspend --network none)
image=pixel-phone-rust:20261012-b472
case "${1:-build}" in
 lock) docker run "${args[@]}" "$image" cargo generate-lockfile --offline ;;
 arm64-process)
  docker run "${args[@]}" "$image" cargo build --locked --offline --target aarch64-unknown-linux-gnu --bin lifetime-peer --message-format=json > artifacts/native-suspend/arm64-helper-cargo.jsonl
  docker run "${args[@]}" "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu --no-run --message-format=json --lib --test coordination --test descriptors --test deadlines --test process_lifetime > artifacts/native-suspend/arm64-tests-cargo.jsonl
  python3 tools/index-suspend-arm64.py
  ;;
 test) docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu ;;
 build)
  docker run "${args[@]}" "$image" cargo fmt --check
  docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu
  docker run "${args[@]}" "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu --lib --test coordination --test descriptors --test deadlines
  ;;
 *) echo 'Usage: build-native-suspend.sh lock|test|build|arm64-process' >&2; exit 2 ;;
esac
