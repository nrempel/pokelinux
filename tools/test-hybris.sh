#!/bin/sh
set -eu
ulimit -c 0
: "${HYBRIS_SOURCE:?Set HYBRIS_SOURCE to the configured pinned source}"
: "${BRIDGE_BUILD:?Set BRIDGE_BUILD to an absolute build directory}"
export LD_LIBRARY_PATH="$HYBRIS_SOURCE/common/.libs:$HYBRIS_SOURCE/hardware/.libs:$HYBRIS_SOURCE/egl/.libs"
export HYBRIS_ANDROID_SDK_VERSION=37 HYBRIS_LINKER_DIR="$HYBRIS_SOURCE/common/q/.libs"
export HYBRIS_EGLPLATFORM=pixel HYBRIS_EGLPLATFORM_DIR="$BRIDGE_BUILD"
export LIBEGL="$BRIDGE_BUILD/fake-android-egl.so" LIBGLESV2="$BRIDGE_BUILD/fake-android-egl.so"
exec "$BRIDGE_BUILD/test-hybris"
