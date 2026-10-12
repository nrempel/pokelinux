#!/bin/sh
# Run only in the documented read-only, network-disabled audit container.
# Diagnostics only: neither result establishes a running radio or IMS service.
set -eu
test -f /.dockerenv || { echo 'Run in the audit container' >&2; exit 2; }
cd "$(dirname "$0")/.."
root=/workspace/artifacts/radio-runtime
case "${1:-}" in
 stock-list)
  exec timeout 30 qemu-aarch64 -L "$root" \
   -E LD_LIBRARY_PATH=/vendor/lib64:/system/lib64:/apex/com.android.runtime/lib64/bionic:/apex/com.android.runtime/lib64 \
   "$root/apex/com.android.runtime/bin/linker64" --list /vendor/bin/hw/rild_exynos ;;
 hybris-load)
  export PIXEL_RUNTIME="$root"
  exec timeout 30 sh tools/with-runtime.sh qemu-aarch64 -L "$root" \
   artifacts/linux-aarch64/bin/runtime-probe "$root" --load /vendor/lib64/libsitril.so ;;
 *) echo 'usage: radio-load-probes.sh [stock-list|hybris-load]' >&2; exit 2 ;;
esac
