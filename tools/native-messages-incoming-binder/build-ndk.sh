#!/bin/bash
set -euo pipefail
cd /workspace
out=/test/ndk
mkdir -p "$out/headers" "$out/generated"
aidl=android-framework/sdk/build-tools/android-37.0/aidl
shopt -s globstar nullglob
for schema in artifacts/sms-binder/frozen-aidl/**/*.aidl tools/native-messages-incoming-binder/aidl/**/*.aidl; do
 version=3; hash=30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb
 if [[ "$schema" == tools/native-messages-incoming-binder/aidl/* ]]; then version=1; hash=0000000000000000000000000000000000000000; fi
 "$aidl" --lang=ndk --structured --stability=vintf --min_sdk_version=35 --version="$version" --hash="$hash"    -I artifacts/sms-binder/frozen-aidl -I tools/native-messages-incoming-binder/aidl    -h "$out/headers" -o "$out/generated" "$schema"
done
generated=("$out"/generated/**/*.cpp)
compiler=/workspace/artifacts/camera-boundary/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++
"$compiler" -std=c++17 -O0 -g -Wall -Wextra -Werror -Wno-unused-parameter -static-libstdc++   -I "$out/headers" -I /workspace/artifacts/camera-boundary/public-headers -I /workspace/artifacts/native-arm64-binder/public   tools/native-messages-incoming-binder/peer.cpp "${generated[@]}" -lbinder_ndk -ldl -o /test/incoming-peer
public=/workspace/artifacts/native-arm64-binder/public/property
"$compiler" -std=c++20 -O0 -g -Wall -Wextra -Werror -static-libstdc++ -ffunction-sections -fdata-sections -Wl,--gc-sections   -I"$public/libpropertyinfoserializer/include" -I"$public/libpropertyinfoparser/include" -I"$public/libbase/include"   /workspace/tools/native-arm64-binder/property-info.cpp "$public/libpropertyinfoserializer/property_info_serializer.cpp"   "$public/libpropertyinfoserializer/trie_builder.cpp" "$public/libpropertyinfoserializer/trie_serializer.cpp"   "$public/libbase/strings.cpp" "$public/libbase/stringprintf.cpp" "$public/libpropertyinfoparser/property_info_parser.cpp" -o /test/property-info
"${compiler%++}" -x c -O0 -g -Wall -Wextra -Werror -Wno-deprecated-declarations   /workspace/tools/native-arm64-binder/property-init.c -ldl -o /test/property-init
"$compiler" --version > /test/ndk-compiler.txt
for binary in incoming-peer property-info property-init;do readelf -h -d -l "/test/$binary" > "/test/$binary-elf.txt";done
