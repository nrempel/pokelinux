#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${BOOT_REFERENCE_ROOT:?Set BOOT_REFERENCE_ROOT to the private prepared AOSP reference directory}"
: "${BOOT_BUILD_OUTPUT:?Set BOOT_BUILD_OUTPUT to a private output directory}"
mkdir -p "$BOOT_BUILD_OUTPUT"
image=0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6
args=(--rm --network none --cpus 8 --memory 3g --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" --tmpfs "/target:rw,exec,size=1g,uid=$(id -u),gid=$(id -g)"
 -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e CARGO_BUILD_JOBS=8
 -e 'RUSTFLAGS=-C link-arg=-fuse-ld=bfd' -e BOOT_REFERENCE_FIXTURES=/reference/fixtures
 -e CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
 -e 'CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER=qemu-aarch64 -L /usr/aarch64-linux-gnu'
 -v "$PWD:/workspace" -v "${BOOT_CARGO_HOME:-$PWD/sim/rust-cargo}:/cargo:ro"
 -v "$BOOT_REFERENCE_ROOT:/reference:ro" -v "$BOOT_BUILD_OUTPUT:/evidence" -w /workspace/native-boot-inputs)
docker run "${args[@]}" "$image" bash -c 'set -e
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline --target x86_64-unknown-linux-gnu
cargo test --locked --offline --target aarch64-unknown-linux-gnu
cargo build --locked --offline --release --target x86_64-unknown-linux-gnu
cargo build --locked --offline --release --target aarch64-unknown-linux-gnu
cp /target/x86_64-unknown-linux-gnu/release/pokelinux-boot-inputs /evidence/inspect-x86
cp /target/aarch64-unknown-linux-gnu/release/pokelinux-boot-inputs /evidence/inspect-arm64'
