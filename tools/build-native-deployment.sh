#!/bin/bash
# Read-only staging admission checks; no installed or physical service mutations.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-deployment/correction-elf-resident
image=pixel-phone-rust:20261012-b472
args=(--rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/artifacts/native-deployment/correction-elf-resident:/target"
 -w /workspace/native-deployment)
case "${1:-build}" in
 lock) docker run "${args[@]}" "$image" cargo generate-lockfile --offline ;;
 red) docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test admission ;;
 build)
  docker run "${args[@]}" "$image" cargo fmt --check
  docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
   docker run "${args[@]}" "$image" rustc --edition 2024 --crate-name deployment_fixture_probe --target "$target" -C linker="$([ "$target" = aarch64-unknown-linux-gnu ] && echo aarch64-linux-gnu-gcc || echo cc)" -C opt-level=2 -C strip=symbols fixtures/elf-probe.rs -o "/target/$target/native-elf-fixture"
   docker run "${args[@]}" -e DEPLOYMENT_TEST_ELF="/target/$target/native-elf-fixture" "$image" cargo test --locked --offline --target "$target"
   docker run "${args[@]}" "$image" cargo build --locked --offline --release --target "$target"
  done
  ;;
 fmt) docker run "${args[@]}" "$image" cargo fmt ;;
 *) echo 'Usage: build-native-deployment.sh lock|red|build|fmt' >&2; exit 2 ;;
esac
