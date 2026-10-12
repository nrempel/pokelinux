#!/bin/bash
# Temporary x86_64 Arch runtime for the Android emulator's same-kernel transport test.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p android-framework/results
exec 8>android-framework/direct-runtime.lock
flock -n 8 || { echo 'Direct runtime build already active' >&2; exit 1; }
docker build -t pixel-arch-runtime:20261011-b472 -f build-support/Dockerfile.arch-runtime build-support
docker image inspect archlinux:base > android-framework/results/arch-runtime-base.json
docker image inspect pixel-arch-runtime:20261011-b472 > android-framework/results/arch-runtime-image.json
docker run --rm --network none --cpus 1 --memory 512m --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges pixel-arch-runtime:20261011-b472 cat /arch-runtime.tar.gz > android-framework/arch-runtime.tar.gz.tmp
mv android-framework/arch-runtime.tar.gz.tmp android-framework/arch-runtime.tar.gz
