#!/bin/bash
# Separate outputs preserve the original 21-test frozen checkpoint byte-for-byte.
set -euo pipefail
cd "$(dirname "$0")/.."
correction=artifacts/native-deployment/correction-elf-resident
mkdir -p "$correction"
image=pixel-phone-rust:20261012-b472
args=(--rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/$correction:/target"
 -w /workspace/native-deployment)
case "${1:-build}" in
 red)
  mkdir -p "$correction/x86_64-unknown-linux-gnu"
  docker run "${args[@]}" "$image" rustc --edition 2024 --crate-name deployment_fixture_probe -C opt-level=2 -C strip=symbols fixtures/elf-probe.rs -o /target/x86_64-unknown-linux-gnu/native-elf-fixture
  docker run "${args[@]}" -e DEPLOYMENT_TEST_ELF=/target/x86_64-unknown-linux-gnu/native-elf-fixture "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test admission
  ;;
 build)
  docker run "${args[@]}" "$image" cargo fmt --check
  docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
   mkdir -p "$correction/$target"
   linker=cc
   if [ "$target" = aarch64-unknown-linux-gnu ]; then linker=aarch64-linux-gnu-gcc; fi
   docker run "${args[@]}" "$image" rustc --edition 2024 --crate-name deployment_fixture_probe --target "$target" -C linker="$linker" -C opt-level=2 -C strip=symbols fixtures/elf-probe.rs -o "/target/$target/native-elf-fixture"
   docker run "${args[@]}" -e DEPLOYMENT_TEST_ELF="/target/$target/native-elf-fixture" "$image" cargo test --locked --offline --target "$target"
   docker run "${args[@]}" "$image" cargo build --locked --offline --release --target "$target"
  done
  ;;
 fmt) docker run "${args[@]}" "$image" cargo fmt ;;
 *) echo 'Usage: build-native-deployment-correction.sh red|build|fmt' >&2; exit 2 ;;
esac
