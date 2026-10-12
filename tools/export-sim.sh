#!/bin/bash
# Preserve the provisioned guest so Harbour is not a dependency for future work.
set -euo pipefail
cd "$(dirname "$0")/.."
exec 9>sim/suite.lock
flock -n 9 || { echo 'A suite is using the VM disk.' >&2; exit 1; }
test -f sim/provisioned.json
test ! -e artifacts/vm/arch-arm64.qcow2 || { echo 'Export already exists; move it aside explicitly.' >&2; exit 1; }
mkdir -p artifacts/vm
docker run --rm --network none --cpus 2 --memory 2g --pids-limit 128 \
  --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace" pixel-graphics-sim:20261011-b472 sh -eu -c '
    qemu-img convert -c -O qcow2 sim/rootfs.img artifacts/vm/arch-arm64.qcow2.partial
    qemu-img check -f qcow2 artifacts/vm/arch-arm64.qcow2.partial
    qemu-img compare -f raw -F qcow2 sim/rootfs.img artifacts/vm/arch-arm64.qcow2.partial
    mv artifacts/vm/arch-arm64.qcow2.partial artifacts/vm/arch-arm64.qcow2
    cp sim/boot/Image sim/boot/initramfs-linux.img artifacts/vm/
    cp sim/provisioned.json artifacts/vm/
    sha256sum artifacts/vm/arch-arm64.qcow2 artifacts/vm/Image artifacts/vm/initramfs-linux.img artifacts/vm/provisioned.json > artifacts/vm/SHA256SUMS
  '
