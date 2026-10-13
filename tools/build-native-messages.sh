#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-messages/build sim/rust-cargo
args=(--rm --pull=never --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-messages/build:/target" -w /workspace pixel-phone-rust:20261012-b472)
for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu;do
 docker run "${args[@]}" cargo build --manifest-path native-messages/Cargo.toml --locked --offline --target "$target" --features isolated-fixture --bins
 docker run "${args[@]}" cargo build --manifest-path native-lifecycle/Cargo.toml --locked --offline --target "$target" --bins
done
