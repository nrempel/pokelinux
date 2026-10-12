#!/bin/bash
# Run on the temporary Linux validation host. Preparation needs network; build does not.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-radio sim/rust-cargo
image=pixel-native-radio:20261012
args=(--rm --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-radio:/target")
case "${1:-build}" in
 prepare)
  if ! docker image inspect pixel-phone-rust:20261012-b472 >/dev/null 2>&1; then
   docker build -t pixel-phone-rust:20261012-b472 -f build-support/Dockerfile.rust build-support
  fi
  docker build -t "$image" -f build-support/Dockerfile.native-radio build-support
  docker run "${args[@]}" "$image" cargo fetch --locked --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-gnu
  ;;
 build)
  docker run "${args[@]}" --network none "$image" cargo fmt --check
  docker run "${args[@]}" --network none "$image" cargo clippy --all-targets --locked --offline --target x86_64-unknown-linux-musl -- -D warnings
  for target in x86_64-unknown-linux-musl aarch64-unknown-linux-gnu; do
   docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target "$target"
  done
  docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target x86_64-unknown-linux-musl --test response_receiver --no-run --message-format=json > artifacts/native-radio/receiver-test-build.json
  python3 tools/collect-receiver-test.py
  docker run "${args[@]}" --network none "$image" cargo build --locked --offline --release --target x86_64-unknown-linux-musl
  docker run "${args[@]}" --network none "$image" readelf -l /target/x86_64-unknown-linux-musl/release/pokelinux-radio > artifacts/native-radio/elf-program-headers.txt
  if grep -q INTERP artifacts/native-radio/elf-program-headers.txt; then
   echo 'Probe unexpectedly requires a dynamic ELF interpreter' >&2; exit 1
  fi
  ;;
 *) echo 'Usage: build-native-radio.sh prepare|build' >&2; exit 2;;
esac
