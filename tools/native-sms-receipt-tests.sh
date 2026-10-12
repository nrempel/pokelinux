#!/bin/bash
# Original native SQLite/process ownership only; no radio, vendor or phone access.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/native-sms-receipt/window-fixed sim/rust-cargo
image=pixel-phone-rust:20261012-b472
args=(--rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/native-sms-receipt/window-fixed:/target"
 -w /workspace/native-messaging)
case "${1:-test}" in
 red) docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test receipt sqlite_raw_and_uncertainty_precede_injected_ack_dispatch ;;
 cancel-red) docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test receipt cancellation_revokes_the_incoming_source_until_explicit_test_reattachment ;;
 process-red) docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test receipt sigkill_releases_single_owner_without_replaying_claimed_ack_or_deduplicating_redelivery ;;
 test)
  docker run "${args[@]}" "$image" cargo fmt --check
  docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
   docker run "${args[@]}" "$image" cargo test --locked --offline --target "$target"
   docker run "${args[@]}" "$image" cargo build --locked --offline --release --target "$target"
  done
  docker run "${args[@]}" "$image" cargo test --locked --offline --target aarch64-unknown-linux-gnu --test receipt --no-run --message-format=json > artifacts/native-sms-receipt/window-fixed/receipt-test-build-arm64.json
  ;;
 *) echo 'Usage: native-sms-receipt-tests.sh red|process-red|cancel-red|test' >&2;exit 2 ;;
esac
