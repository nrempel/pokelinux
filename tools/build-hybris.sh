#!/bin/sh
# Build only the glibc-side loader and EGL/GLES wrappers; no Android service runs.
set -eu
: "${HYBRIS_SOURCE:?Path to pinned source hybris/ directory}"
: "${ANDROID_HEADERS:?Path to pinned Droidian android-headers-30}"
: "${PREFIX:?Installation prefix (use a staging directory)}"
case "$HYBRIS_SOURCE:$ANDROID_HEADERS:$PREFIX" in /*:/*:/*) ;; *) echo 'Use absolute paths' >&2; exit 2;; esac
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$HYBRIS_SOURCE/.."
for fix in "$project"/patches/*.patch; do
  if patch --dry-run -p1 < "$fix" >/dev/null 2>&1; then
    patch -p1 < "$fix"
  elif ! patch --dry-run -R -p1 < "$fix" >/dev/null 2>&1; then
    echo "Patch does not match the source lock: $fix" >&2; exit 1
  fi
done
cd "$HYBRIS_SOURCE"
./autogen.sh --prefix="$PREFIX" --with-android-headers="$ANDROID_HEADERS" --enable-arch=arm64 --disable-lindroid-drm --disable-glvnd --with-default-egl-platform=pixel "$@"
make -C common -j"${JOBS:-4}"
make -C hardware -j"${JOBS:-4}"
make -C egl libEGL.la -j"${JOBS:-4}"
make -C glesv2 -j"${JOBS:-4}"
# Deliberately avoid installing other hardware adapters and Android UI wrappers.
mkdir -p "$PREFIX/lib/libhybris/linker"
cp -P common/.libs/libhybris-common.so* hardware/.libs/libhardware.so* egl/.libs/libEGL.so* glesv2/.libs/libGLESv2.so* "$PREFIX/lib/"
cp common/q/.libs/q.so "$PREFIX/lib/libhybris/linker/"
