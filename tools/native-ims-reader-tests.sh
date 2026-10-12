#!/bin/bash
# Native scalar/opaque decoder only; original APK and real Binder are separate.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-ims-reader/green sim/rust-cargo
args=(--rm --init --network none --cpus 2 --memory 2g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-ims-reader/green:/target"
 -w /workspace/native-ims)
image=pixel-phone-rust:20261012-b472
docker run "${args[@]}" "$image" cargo fmt --check
docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
 docker run "${args[@]}" "$image" cargo test --locked --offline --target "$target"
 docker run "${args[@]}" "$image" cargo build --locked --offline --release --target "$target"
done
docker run "${args[@]}" "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu --test data_reader --no-run --message-format=json > artifacts/native-ims-reader/green/reader-test-build-arm64.json
