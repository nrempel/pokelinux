#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
base=artifacts/native-messages-receive-isolation
mkdir -p "$base/target"
exec 9>"$base/build.lock";flock -n 9
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
"${docker_local[@]}" run --rm --name pokelinux-receive-isolation-sol-build --network none --cpus 8 --memory 12g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e CARGO_BUILD_JOBS=8 -v "$PWD:/workspace:ro" -v /home/ubuntu/Projects/pokelinux/sim/rust-cargo:/cargo:ro -v /home/ubuntu/Projects/pokelinux/artifacts/sms-binder/frozen-aidl:/workspace/artifacts/sms-binder/frozen-aidl:ro -v "$PWD/$base/target:/target" -w /workspace/tools/native-messages-incoming-binder/fixture sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6 bash -ec 'cargo build --locked --offline --target aarch64-unknown-linux-gnu --bin messages-incoming-fixture-service; cargo build --manifest-path /workspace/native-messages/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --bins; cargo build --manifest-path /workspace/native-lifecycle/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --bin lifecycle-launcher --bin lifecycle-client'
