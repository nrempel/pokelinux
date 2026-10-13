#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-lifecycle-storage/target
args=(--rm --init --cpus 2 --memory 2g --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-lifecycle-storage/target:/target"
 -w /workspace/native-lifecycle --network none)
image=pixel-phone-rust:20261012-b472
case "${1:-build}" in
 fmt) docker run "${args[@]}" "$image" cargo fmt ;;
 fix) docker run "${args[@]}" "$image" cargo clippy --locked --offline --fix --allow-dirty --allow-no-vcs --all-targets -- -D warnings ;;
 compile)
  docker run "${args[@]}" "$image" cargo build --locked --offline --target x86_64-unknown-linux-gnu --bins --message-format=json > artifacts/native-lifecycle-storage/x86-bins-cargo.jsonl
  docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --lib --no-run --message-format=json > artifacts/native-lifecycle-storage/x86-tests-cargo.jsonl
  ;;
 build)
  docker run "${args[@]}" "$image" cargo fmt --check
  docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  for architecture in x86_64 aarch64; do
   docker run "${args[@]}" "$image" cargo build --locked --offline --target "$architecture-unknown-linux-gnu" --bins --message-format=json > "artifacts/native-lifecycle-storage/$architecture-bins-cargo.jsonl"
   docker run "${args[@]}" "$image" cargo test --locked --offline --target "$architecture-unknown-linux-gnu" --lib --no-run --message-format=json > "artifacts/native-lifecycle-storage/$architecture-tests-cargo.jsonl"
  done
  ;;
 *) echo 'Usage: build-native-lifecycle-storage.sh fmt|fix|compile|build' >&2; exit 2 ;;
esac
