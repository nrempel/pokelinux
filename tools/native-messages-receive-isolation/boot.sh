#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results /private/artifacts/runtime
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro android-runtime /private/artifacts/runtime
exec python3 /workspace/tools/native-messages-receive-isolation/guest.py
