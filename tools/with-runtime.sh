#!/bin/sh
# Select only this package's libraries. No default command and no device access.
set -eu
if [ "$#" -eq 0 ]; then echo 'usage: with-runtime.sh COMMAND [ARGS...]' >&2; exit 2; fi
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
export PIXEL_RUNTIME="${PIXEL_RUNTIME:-$project/artifacts/runtime}"
prefix="${PIXEL_BRIDGE_PREFIX:-$project/artifacts/linux-aarch64}"
export LD_LIBRARY_PATH="$prefix/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export HYBRIS_LINKER_DIR="$prefix/lib/libhybris/linker" HYBRIS_ANDROID_SDK_VERSION=37
export HYBRIS_EGLPLATFORM=pixel HYBRIS_EGLPLATFORM_DIR="$prefix/lib/libhybris"
export HYBRIS_LD_LIBRARY_PATH="$PIXEL_RUNTIME/vendor/lib64:$PIXEL_RUNTIME/system/lib64:$PIXEL_RUNTIME/apex/com.android.runtime/lib64/bionic:$PIXEL_RUNTIME/apex/com.android.runtime/lib64"
export LIBEGL="$PIXEL_RUNTIME/vendor/lib64/egl/libEGL_powervr.so"
export LIBGLESV2="$PIXEL_RUNTIME/vendor/lib64/egl/libGLESv2_powervr.so"
exec "$@"
