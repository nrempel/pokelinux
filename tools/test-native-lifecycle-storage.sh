#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
proof=${1:-green}
case "$proof" in red) baseline=0 ;; green) baseline=1 ;; *) exit 2 ;; esac
mkdir -p "artifacts/native-lifecycle-storage/$proof"
docker run --rm --init --network none --cpus 2 --memory 512m --pids-limit 64 \
 --cap-drop ALL --cap-add SETUID --cap-add SETGID --cap-add CHOWN --cap-add KILL --cap-add DAC_OVERRIDE \
 --security-opt no-new-privileges --read-only -e NATIVE_LIFECYCLE_ISOLATED=1 -e "LIFECYCLE_STORAGE_RUN_BASELINE=$baseline" \
 --tmpfs /opt:rw,exec,nosuid,nodev --tmpfs /run:rw,nosuid,nodev --tmpfs /var/lib:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev \
 -v "$PWD/artifacts/native-lifecycle-storage/target/x86_64-unknown-linux-gnu/debug:/bins:ro" \
 -v "$PWD/tools/native-lifecycle-fixture.py:/baseline-fixture.py:ro" \
 -v "$PWD/tools/native-lifecycle-storage-fixture.py:/storage-fixture.py:ro" \
 -v "$PWD/artifacts/native-lifecycle-storage/$proof:/evidence" \
 pixel-graphics-build:20261011-b472 python3 /storage-fixture.py
