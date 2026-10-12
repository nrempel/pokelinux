#!/usr/bin/env bash
# Isolated native audio validation; no host audio, NIC, D-Bus or device access.
set -euo pipefail
cd "$(dirname "$0")/.."
image=pokelinux-native-audio:20261012
mkdir -p artifacts/native-audio/target sim/rust-cargo
args=(--rm --cpus 2 --memory 2g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
  --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
  -v "$PWD/native-audio:/workspace/native-audio" -v "$PWD/sim/rust-cargo:/cargo"
  -v "$PWD/artifacts/native-audio/target:/target")
case "${1:-build}" in
  prepare)
    docker build -f build-support/Dockerfile.native-audio -t "$image" build-support
    ca_args=()
    if [[ -n "${CODEX_PROXY_CERT:-}" ]]; then
      ca_args=(--mount "type=bind,src=$CODEX_PROXY_CERT,dst=/run/proxy-ca.pem,readonly" -e CARGO_HTTP_CAINFO=/run/proxy-ca.pem)
    fi
    docker run "${args[@]}" "${ca_args[@]}" "$image" cargo fetch --locked
    ;;
  build)
    docker run "${args[@]}" --network none "$image" cargo fmt --check
    docker run "${args[@]}" --network none "$image" cargo clippy --locked --offline --all-targets -- -D warnings
    docker run "${args[@]}" --network none "$image" cargo test --locked --offline
    docker run "${args[@]}" --network none "$image" cargo build --locked --offline
    docker run "${args[@]}" --network none "$image" cargo build --locked --offline --release
    docker run "${args[@]}" --network none "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu
    docker run "${args[@]}" --network none "$image" cargo build --locked --offline --release --target aarch64-unknown-linux-gnu
    ;;
  integration)
    python3 tools/native-audio-suite.py --output "${2:?choose a NEW ignored evidence directory}"
    ;;
  *) echo 'Usage: build-native-audio.sh prepare|build|integration NEW_OUTPUT_DIR' >&2; exit 2 ;;
esac
