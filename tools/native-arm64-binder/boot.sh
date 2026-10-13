#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python3 /workspace/tools/native-arm64-binder/guest.py
