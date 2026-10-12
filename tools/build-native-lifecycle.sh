#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-lifecycle/target
args=(--rm --init --cpus 2 --memory 2g --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-lifecycle/target:/target"
 -w /workspace/native-lifecycle --network none)
image=pixel-phone-rust:20261012-b472
case "${1:-build}" in
 lock) docker run "${args[@]}" "$image" cargo generate-lockfile --offline ;;
 fmt) docker run "${args[@]}" "$image" cargo fmt ;;
 compile)
  docker run "${args[@]}" "$image" cargo build --locked --offline --target x86_64-unknown-linux-gnu --bins
  docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --no-run --message-format=json --lib > artifacts/native-lifecycle/x86-lib-cargo.jsonl
  ;;
 build)
  docker run "${args[@]}" "$image" cargo fmt --check
  docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  docker run "${args[@]}" "$image" cargo build --locked --offline --target x86_64-unknown-linux-gnu --bins
  docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --no-run --message-format=json --lib > artifacts/native-lifecycle/x86-lib-cargo.jsonl
  docker run "${args[@]}" "$image" cargo build --locked --offline --target aarch64-unknown-linux-gnu --bins --message-format=json > artifacts/native-lifecycle/arm64-bins-cargo.jsonl
  docker run "${args[@]}" "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu --no-run --message-format=json --lib > artifacts/native-lifecycle/arm64-lib-cargo.jsonl
  python3 tools/index-native-lifecycle.py
  ;;
 *) echo 'Usage: build-native-lifecycle.sh lock|fmt|compile|build' >&2; exit 2 ;;
esac
