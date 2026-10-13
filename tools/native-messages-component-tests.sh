#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-messages/components sim/rust-cargo
args=(--rm --pull=never --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -v "$PWD:/workspace:ro" -v "$PWD/native-messages:/workspace/native-messages" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-messages/components:/target" -w /workspace/native-messages pixel-phone-rust:20261012-b472)
if [ ! -f native-messages/Cargo.lock ];then docker run "${args[@]}" cargo generate-lockfile --offline;fi
case "${1:-test}" in
 red) docker run "${args[@]}" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test projection service_owned_projection_runs_with_no_window_and_preserves_local_receipt_id ;;
 test) docker run "${args[@]}" cargo fmt --check;docker run "${args[@]}" cargo clippy --locked --offline --all-targets -- -D warnings;for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu;do docker run "${args[@]}" cargo test --locked --offline --target "$target";done ;;
 *) echo 'Usage: native-messages-component-tests.sh red|test' >&2;exit 2;;
esac
