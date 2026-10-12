#!/usr/bin/env bash
# Run on the temporary Linux build host after rust-suite.sh completes.
set -euo pipefail
cd "$(dirname "$0")/.."
(cd runtime && sha256sum --check ../artifacts/rust-core/sources.sha256 > /dev/null)
executable=$(sed -n 's/.*Running tests\/authorization.rs (\(\/target\/debug\/deps\/authorization-[a-f0-9]*\)).*/\1/p' artifacts/rust-core/linux-tests.log)
[[ "$executable" =~ ^/target/debug/deps/authorization-[a-f0-9]+$ ]]
docker run --rm --network none --read-only --tmpfs /tmp:rw,nosuid,nodev \
  --cap-drop ALL --cap-add SETUID --cap-add SETGID --security-opt no-new-privileges \
  -v "$PWD/sim/rust-target:/target:ro" pixel-phone-rust:20261012-b472 \
  "$executable" --exact foreign_uid_is_rejected_before_reading_or_revealing_session --ignored \
  > artifacts/rust-core/foreign-uid.log 2>&1
cat artifacts/rust-core/foreign-uid.log
