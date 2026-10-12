#!/bin/bash
# Dedicated container only: no host identity, sysfs, device or network mutation.
set -euo pipefail
cd "$(dirname "$0")/.."
proof=${1:-green}
case "$proof" in red|green) ;; *) echo 'Usage: test-native-lifecycle.sh red|green' >&2; exit 2 ;; esac
mkdir -p "artifacts/native-lifecycle/$proof"
docker run --rm --init --network none --cpus 2 --memory 512m --pids-limit 64 \
 --cap-drop ALL --cap-add SETUID --cap-add SETGID --cap-add CHOWN --cap-add KILL --cap-add DAC_OVERRIDE \
 --security-opt no-new-privileges --read-only -e NATIVE_LIFECYCLE_ISOLATED=1 \
 --tmpfs /opt:rw,exec,nosuid,nodev --tmpfs /run:rw,nosuid,nodev --tmpfs /var/lib:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev \
 -v "$PWD/artifacts/native-lifecycle/target/x86_64-unknown-linux-gnu/debug:/bins:ro" \
 -v "$PWD/tools/native-lifecycle-fixture.py:/fixture.py:ro" \
 -v "$PWD/artifacts/native-lifecycle/$proof:/evidence" \
 pixel-graphics-build:20261011-b472 python3 /fixture.py
