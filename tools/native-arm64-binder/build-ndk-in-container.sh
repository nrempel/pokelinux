set -euo pipefail
compiler=/workspace/artifacts/camera-boundary/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++
public=/workspace/artifacts/native-arm64-binder/public/property
"$compiler" -std=c++20 -O0 -g -Wall -Wextra -Werror -static-libstdc++ -ffunction-sections -fdata-sections -Wl,--gc-sections -I"$public/libpropertyinfoserializer/include" -I"$public/libpropertyinfoparser/include" -I"$public/libbase/include" \
  /workspace/tools/native-arm64-binder/property-info.cpp "$public/libpropertyinfoserializer/property_info_serializer.cpp" "$public/libpropertyinfoserializer/trie_builder.cpp" "$public/libpropertyinfoserializer/trie_serializer.cpp" "$public/libbase/strings.cpp" "$public/libbase/stringprintf.cpp" "$public/libpropertyinfoparser/property_info_parser.cpp" -o /test/property-info
"${compiler%++}" -x c -O0 -g -Wall -Wextra -Werror -Wno-deprecated-declarations /workspace/tools/native-arm64-binder/property-init.c -ldl -o /test/property-init
"$compiler" -std=c++17 -O0 -g -Wall -Wextra -Werror -nostdlib++ -fno-exceptions -fno-rtti -I/workspace/artifacts/native-arm64-binder/public /workspace/tools/native-arm64-binder/ndk-peer.cpp -lbinder_ndk -ldl -o /test/ndk-peer
"$compiler" --version > /test/ndk-compiler.txt
for binary in ndk-peer property-info property-init; do readelf -h -d -l "/test/$binary" > "/test/$binary-elf.txt"; done
