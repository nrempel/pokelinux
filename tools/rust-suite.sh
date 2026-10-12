#!/usr/bin/env bash
# Run in the owned Rust build image with /workspace read-only and /results writable.
set -euo pipefail
printf '%s\n' '{"passed":false,"scope":"Rust component verification in progress"}' > /results/result.json
cd /workspace/runtime
rustc -Vv > /results/toolchain.txt
cargo -V >> /results/toolchain.txt
sha256sum Cargo.toml Cargo.lock src/*.rs src/bin/*.rs tests/*.rs > /results/sources.sha256
cargo fmt --check > /results/format.log 2>&1
cargo clippy --locked --offline --all-targets -- -D warnings > /results/clippy.log 2>&1
cargo test --locked --offline > /results/linux-tests.log 2>&1
cargo test --locked --offline --target aarch64-unknown-linux-gnu > /results/arm64-tests.log 2>&1
printf '%s\n' '{"passed":true,"scope":"Rust protocol, local socket, durable journal and TLS transport components","linux_native":true,"arm64_qemu_user":true,"android_integration":false,"hardware_verified":false}' > /results/result.json
cat /results/result.json
