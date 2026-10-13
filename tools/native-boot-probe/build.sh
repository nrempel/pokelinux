#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
target=${NATIVE_BOOT_PROBE_TARGET:?select a private RAM build directory}
cargo_home=${NATIVE_BOOT_PROBE_CARGO:?select a private Cargo metadata directory}
registry=${NATIVE_BOOT_PROBE_REGISTRY:?select the retained offline registry}
mkdir -p "$target" "$cargo_home"
env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
docker --host=unix:///var/run/docker.sock run --rm --network none \
  --cpus 8 --memory 4g --pids-limit 512 --user "$(id -u):$(id -g)" --cap-drop ALL \
  -v "$root:/workspace:ro" -v "$target:/target" -v "$cargo_home:/cargo" -v "$registry:/cargo/registry:ro" \
  -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e 'RUSTFLAGS=-C target-feature=+crt-static' \
  -w /workspace/native-boot-probe \
  0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6 \
  bash -c 'set -e
    cargo fmt --check
    cargo clippy --all-targets --target aarch64-unknown-linux-gnu --locked --offline -- -D warnings
    cargo build --release --target aarch64-unknown-linux-gnu --locked --offline'
