#!/bin/sh
# Run inside build-support/Dockerfile on an x86-64 Linux host.
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project"
python3 - <<'PY'
import hashlib,json,pathlib
root=pathlib.Path('.')
lock=json.loads((root/'sources.lock.json').read_text())
for name,item in lock.items():
 archive=root/'artifacts/sources'/('libhybris.tar.gz' if name=='libhybris' else 'android-headers.tar.gz')
 if hashlib.sha256(archive.read_bytes()).hexdigest()!=item['archive_sha256']:
  raise SystemExit('Source checksum mismatch: '+str(archive))
PY
mkdir -p build/source/libhybris build/source/headers artifacts/logs
if [ ! -f build/source/libhybris/hybris/configure.ac ]; then
 tar -xzf artifacts/sources/libhybris.tar.gz --strip-components=1 -C build/source/libhybris
 tar -xzf artifacts/sources/android-headers.tar.gz --strip-components=1 -C build/source/headers
fi
export HYBRIS_SOURCE="$project/build/source/libhybris/hybris"
export ANDROID_HEADERS="$project/build/source/headers"
export PREFIX="$project/artifacts/linux-aarch64"
export CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++
export PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig
sh tools/build-hybris.sh --host=aarch64-linux-gnu --build=x86_64-linux-gnu > artifacts/logs/harbour-hybris-build.log 2>&1
cmake -S . -B build/bridge -DCMAKE_TOOLCHAIN_FILE="$project/build-support/aarch64.cmake" \
 -DHYBRIS_SOURCE="$HYBRIS_SOURCE" -DPIXEL_FACTORY_RUNTIME="$project/artifacts/runtime" \
 -DCMAKE_BUILD_TYPE=RelWithDebInfo > artifacts/logs/harbour-cmake.log 2>&1
cmake --build build/bridge -j"${JOBS:-4}" >> artifacts/logs/harbour-cmake.log 2>&1
ctest --test-dir build/bridge --output-on-failure > artifacts/logs/harbour-ctest.log 2>&1
cmake --install build/bridge --prefix "$PREFIX" > artifacts/logs/harbour-install.log 2>&1
dpkg-query -W > artifacts/logs/harbour-build-packages.txt
