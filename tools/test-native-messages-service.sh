#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
phase=${1:-green};case "$phase" in red|reopen-red|green|green-[0-9]*) ;; *) exit 2;; esac
out="artifacts/native-messages/service-$phase"
[[ ! -e "$out" ]] || { echo "Preserve existing proof $out" >&2;exit 2; }
mkdir -p "$out"
docker run --rm --init --network none --cpus 2 --memory 512m --pids-limit 64 \
 --cap-drop ALL --cap-add SETUID --cap-add SETGID --cap-add CHOWN --cap-add KILL --cap-add DAC_OVERRIDE \
 --security-opt no-new-privileges --read-only -e NATIVE_MESSAGES_ISOLATED=1 \
 --tmpfs /opt:rw,exec,nosuid,nodev --tmpfs /run:rw,nosuid,nodev --tmpfs /var/lib:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev \
 -v "$PWD/artifacts/native-messages/build/x86_64-unknown-linux-gnu/debug:/bins:ro" \
 -v "$PWD/tools/native-messages-service-fixture.py:/fixture.py:ro" -v "$PWD/$out:/evidence" \
 pixel-graphics-build:20261011-b472 python3 /fixture.py --phase "$phase" > "$out/log" 2>&1
