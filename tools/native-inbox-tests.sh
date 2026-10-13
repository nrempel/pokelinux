#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-inbox sim/rust-cargo
args=(--rm --pull=never --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -v "$PWD:/workspace:ro" -v "$PWD/native-inbox:/workspace/native-inbox" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-inbox:/target" -w /workspace/native-inbox pixel-phone-rust:20261012-b472)
if [ ! -f native-inbox/Cargo.lock ];then docker run "${args[@]}" cargo generate-lockfile --offline;fi
case "${1:-test}" in
 red) docker run "${args[@]}" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test inbox committed_local_receipt_is_projected_through_actual_sqlite ;;
 gate-red) docker run "${args[@]}" cargo test --locked --offline --target x86_64-unknown-linux-gnu --lib access_storage_tests ;;
 process-red) docker run "${args[@]}" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test inbox sigkill_releases_private_inbox_with_committed_projection_intact ;;
 test) docker run "${args[@]}" cargo fmt --check;docker run "${args[@]}" cargo clippy --locked --offline --all-targets -- -D warnings
  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu;do docker run "${args[@]}" cargo test --locked --offline --target "$target";docker run "${args[@]}" cargo build --locked --offline --release --target "$target";done
  docker run "${args[@]}" cargo test --locked --offline --target aarch64-unknown-linux-gnu --no-run --message-format=json > artifacts/native-inbox/inbox-test-build-arm64.next.json
  mv artifacts/native-inbox/inbox-test-build-arm64.next.json artifacts/native-inbox/inbox-test-build-arm64.json ;;
 *) echo 'Usage: native-inbox-tests.sh red|process-red|gate-red|test' >&2;exit 2;;
esac
