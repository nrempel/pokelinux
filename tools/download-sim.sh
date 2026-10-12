#!/bin/sh
# This base image is pinned. A rolling upstream change must not silently replace it.
set -eu
cd "$(dirname "$0")/.."
mkdir -p sim/cache
base=https://de3.mirror.archlinuxarm.org/os/multi
source=ArchLinuxARM-2026.08-aarch64-rootfs.tar.gz
archive=sim/cache/ArchLinuxARM-aarch64-latest.tar.gz
if [ ! -f "$archive" ]; then
  curl --fail --location --retry 3 "$base/$source" -o "$archive.partial"
  mv "$archive.partial" "$archive"
fi
echo '42a4eeaa038994ffd31fa173256ef2f0ef511358eeb41b9ea1f8626391b9b319  sim/cache/ArchLinuxARM-aarch64-latest.tar.gz' | sha256sum -c -
if [ ! -f "$archive.sig" ]; then
  curl --fail --location --retry 3 "$base/$source.sig" -o "$archive.sig"
fi
if [ ! -f sim/cache/archlinuxarm.asc ]; then
  curl --fail --location --retry 3 \
    'https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x68B3537F39A313B3E574D06777193F152BDBE6A6' \
    -o sim/cache/archlinuxarm.asc
fi
