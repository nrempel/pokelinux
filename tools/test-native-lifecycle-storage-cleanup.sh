#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
proof=${1:-cleanup-green}
[[ "$proof" == cleanup-red || "$proof" == cleanup-green ]]
failed=0
for mode in error withheld; do
mkdir -p "artifacts/native-lifecycle-storage/$proof-$mode"
docker run --rm --init --network none --cpus 2 --memory 256m --pids-limit 32 --ulimit core=0 \
 --cap-drop ALL --cap-add DAC_OVERRIDE --security-opt no-new-privileges --read-only -e NATIVE_LIFECYCLE_ISOLATED=1 \
 --tmpfs /opt:rw,exec,nosuid,nodev --tmpfs /run:rw,nosuid,nodev --tmpfs /var/lib:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev \
 -v "$PWD/artifacts/native-lifecycle-storage/target/x86_64-unknown-linux-gnu/debug:/bins:ro" \
 -v "$PWD/tools/native-lifecycle-storage-cleanup-fixture.py:/cleanup-fixture.py:ro" \
 -v "$PWD/artifacts/native-lifecycle-storage/$proof-$mode:/evidence" \
 pixel-graphics-build:20261011-b472 python3 /cleanup-fixture.py --binary-root /bins --results-dir /evidence --mode "$mode" || failed=1
done
exit "$failed"
