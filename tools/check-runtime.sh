#!/bin/sh
# Google's linker --list only. No service startup, constructors, or GPU probe.
set -eu
: "${PIXEL_RUNTIME:?Absolute path to assembled runtime}"
case "$PIXEL_RUNTIME" in /*) ;; *) echo 'Use an absolute runtime path' >&2; exit 2;; esac
for target in vendor/lib64/egl/libEGL_powervr.so vendor/lib64/egl/libGLESv2_powervr.so vendor/lib64/hw/mapper.pixel.so vendor/bin/hw/pixel.gralloc.allocator-service system/bin/servicemanager; do
  case "$target" in
    system/*) search="$PIXEL_RUNTIME/system/lib64:$PIXEL_RUNTIME/vendor/lib64";;
    *) search="$PIXEL_RUNTIME/vendor/lib64:$PIXEL_RUNTIME/system/lib64";;
  esac
  search="$search:$PIXEL_RUNTIME/apex/com.android.runtime/lib64/bionic:$PIXEL_RUNTIME/apex/com.android.runtime/lib64"
  qemu-aarch64 -cpu max -L "$PIXEL_RUNTIME" -E "LD_LIBRARY_PATH=$search" "$PIXEL_RUNTIME/apex/com.android.runtime/bin/linker64" --list "$PIXEL_RUNTIME/$target"
done
