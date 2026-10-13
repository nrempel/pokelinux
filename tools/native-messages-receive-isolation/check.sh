#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
base=artifacts/native-messages-receive-isolation
exec 9>"$base/build.lock";flock -n 9
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
"${docker_local[@]}" run --rm --name pokelinux-receive-isolation-sol-check --network none --cpus 8 --memory 12g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e CARGO_BUILD_JOBS=8 -v "$PWD:/workspace" -v /home/ubuntu/Projects/pokelinux/sim/rust-cargo:/cargo:ro -v /home/ubuntu/Projects/pokelinux/artifacts/sms-binder/frozen-aidl:/workspace/artifacts/sms-binder/frozen-aidl:ro -v "$PWD/$base/target:/target" -w /workspace sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6 bash -ec '
for manifest in native-inbox/Cargo.toml native-messages/Cargo.toml native-lifecycle/Cargo.toml tools/native-messages-incoming-binder/fixture/Cargo.toml; do cargo fmt --manifest-path "$manifest"; done
cargo clippy --manifest-path native-messages/Cargo.toml --locked --offline --all-targets --all-features -- -D warnings
cargo clippy --manifest-path tools/native-messages-incoming-binder/fixture/Cargo.toml --locked --offline --all-targets -- -D warnings
cargo test --manifest-path native-messages/Cargo.toml --locked --offline --test projection
cargo test --manifest-path native-messaging/Cargo.toml --locked --offline --test receipt
cargo test --manifest-path native-inbox/Cargo.toml --locked --offline --lib --test inbox --test original_deliver
cargo test --manifest-path native-messages/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --test projection --no-run --message-format=json > /target/projection-arm-tests.jsonl
cargo test --manifest-path native-messaging/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --test receipt --no-run --message-format=json > /target/receipt-arm-tests.jsonl
cargo test --manifest-path native-inbox/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --lib --test inbox --test original_deliver --no-run --message-format=json > /target/inbox-arm-tests.jsonl
'
