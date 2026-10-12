#!/bin/bash
# Extend the owned ARM64 guest; preserve its prior disk and leave the host alone.
set -euo pipefail
cd "$(dirname "$0")/.."
exec 8>android-framework/suite.lock
flock -n 8 || { echo 'Framework suite is using the guest' >&2; exit 1; }
exec 9>sim/suite.lock
flock -n 9 || { echo 'Graphics suite is using the guest' >&2; exit 1; }
for name in pixel-framework-b472 pixel-phone-vm-b472 pixel-transport-vm-b472 pixel-shell-provision-b472; do
  if docker container inspect "$name" >/dev/null 2>&1; then
    echo "Existing owned test container requires inspection: $name" >&2
    exit 1
  fi
done
test -f sim/rootfs.img
backup=sim/before-shell-$(date -u +%Y%m%dT%H%M%SZ)
mkdir "$backup"
cp --reflink=auto --sparse=always sim/rootfs.img "$backup/rootfs.img"
cp sim/provisioned.json "$backup/provisioned.json"
printf 'Preserved guest: %s\n' "$backup"
docker run --rm --name pixel-shell-provision-b472 --cpus 4 --memory 6g --pids-limit 512 \
  --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace" pixel-graphics-sim:20261011-b472 python3 tools/sim.py provision
