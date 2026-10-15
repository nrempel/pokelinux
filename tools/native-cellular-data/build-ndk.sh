#!/bin/bash
# SOURCE-ONLY PROPOSAL. Root supplies owned offline build execution and receipts.
set -euo pipefail
export LC_ALL=C
workspace=${1:?absolute selected workspace}
out=${2:?absolute empty output directory}
[[ "$workspace" == /* && "$out" == /* && ! -e "$out" ]]
fixture="$workspace/tools/native-cellular-data"
frozen="$workspace/native-cellular-boundary/aidl"
authored="$fixture/fixture-aidl"
mkdir -p "$out/headers" "$out/generated"
aidl="$workspace/android-framework/sdk/build-tools/android-37.0/aidl"
compiler="$workspace/artifacts/camera-boundary/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++"
control_hash=$(< "$fixture/fixture-interface-hash.txt")
[[ "$control_hash" =~ ^[0-9a-f]{40}$ ]]
shopt -s globstar nullglob
public_schemas=("$frozen"/**/*.aidl)
control_schemas=("$authored"/**/*.aidl)
[[ ${#public_schemas[@]} == 41 && ${#control_schemas[@]} == 2 ]]
for schema in "${public_schemas[@]}"; do
  version=3
  hash=58d15e9e2c355be7b3dda6d4d34effd672bfd1cb
  if [[ "$schema" == "$frozen/android/hardware/radio/data/"* ]]; then
    hash=cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df
  fi
  "$aidl" --lang=ndk --structured --stability=vintf --min_sdk_version=35 \
    --version="$version" --hash="$hash" -I "$frozen" -I "$authored" \
    -h "$out/headers" -o "$out/generated" "$schema"
done
for schema in "${control_schemas[@]}"; do
  "$aidl" --lang=ndk --structured --stability=vintf --min_sdk_version=35 \
    --version=1 --hash="$control_hash" -I "$frozen" -I "$authored" \
    -h "$out/headers" -o "$out/generated" "$schema"
done
generated=("$out"/generated/**/*.cpp)
[[ ${#generated[@]} -gt 0 ]]
# All compilation (including property helpers) uses max16 direct children.
# INT/TERM records a sticky failure and never exits across spawn/PID retention.
# Root's original outer cutoff/container custody owns a stuck build; no new timer.
mkdir -p "$out/objects" "$out/dependencies" "$out/compile-logs" "$out/link-logs"
: > "$out/compile-waits.tsv"
: > "$out/compile-sources.tsv"
compile_pids=()
compile_sources=()
peer_objects=()
info_objects=()
init_objects=()
compile_failed=0
abort_status=0
exiting=0
source_index=0
note_interrupt() {
  if [[ "$abort_status" == 0 ]]; then abort_status=$1; fi
}
arm_deferred_signals() {
  trap 'note_interrupt 130' INT
  trap 'note_interrupt 143' TERM
}
fail_if_interrupted() {
  if [[ "$abort_status" != 0 ]]; then exit "$abort_status"; fi
}
drain_compiles() {
  local index pid status
  for index in "${!compile_pids[@]}"; do
    pid=${compile_pids[$index]}
    if wait "$pid"; then status=0; else status=$?; compile_failed=1; fi
    if [[ "$abort_status" != 0 ]]; then
      # A signal may interrupt Bash wait before child completion. Wait again;
      # Bash also retains an already-waited child's status, so this is safe then.
      trap '' INT TERM
      if wait "$pid"; then status=0; else status=$?; compile_failed=1; fi
      if [[ "$exiting" == 0 ]]; then arm_deferred_signals; fi
    fi
    unset 'compile_pids[index]'
    if ! printf '%s\t%s\t%s\n' "$pid" "$status" "${compile_sources[$index]}" >> "$out/compile-waits.tsv"; then
      compile_failed=1
    fi
    unset 'compile_sources[index]'
  done
  compile_pids=()
  compile_sources=()
}
finish_compiles_on_exit() {
  local status=$?
  exiting=1
  trap - EXIT
  trap '' INT TERM
  drain_compiles
  if [[ "$status" == 0 && "$abort_status" != 0 ]]; then status=$abort_status; fi
  if [[ "$status" == 0 && "$compile_failed" != 0 ]]; then status=1; fi
  exit "$status"
}
trap finish_compiles_on_exit EXIT
arm_deferred_signals
queue_compile() {
  local program=$1 tool=$2 source=$3
  shift 3
  local object="$out/objects/$source_index.o"
  local dependency="$out/dependencies/$source_index.d"
  local log="$out/compile-logs/$source_index.log"
  fail_if_interrupted
  "$tool" "$@" -MD -MF "$dependency" -c "$source" -o "$object" > "$log" 2>&1 &
  compile_pids+=("$!")
  compile_sources+=("$source")
  # Deferred signal handling guarantees custody before any interruption exits.
  fail_if_interrupted
  case "$program" in
    data-peer) peer_objects+=("$object") ;;
    property-info) info_objects+=("$object") ;;
    property-init) init_objects+=("$object") ;;
    *) exit 2 ;;
  esac
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$source_index" "$program" "$source" \
    "$object" "$dependency" "$log" "$!" >> "$out/compile-sources.tsv"
  source_index=$((source_index + 1))
  if [[ ${#compile_pids[@]} == 16 ]]; then drain_compiles; fi
}
peer_sources=("$fixture/peer.cpp" "${generated[@]}")
for source in "${peer_sources[@]}"; do
  queue_compile data-peer "$compiler" "$source" \
    -std=c++17 -O0 -g -Wall -Wextra -Werror -Wno-unused-parameter \
    -I "$out/headers" -I "$workspace/artifacts/camera-boundary/public-headers" \
    -I "$workspace/artifacts/native-arm64-binder/public"
done
public="$workspace/artifacts/native-arm64-binder/public/property"
info_sources=(
  "$workspace/tools/native-arm64-binder/property-info.cpp"
  "$public/libpropertyinfoserializer/property_info_serializer.cpp"
  "$public/libpropertyinfoserializer/trie_builder.cpp"
  "$public/libpropertyinfoserializer/trie_serializer.cpp"
  "$public/libbase/strings.cpp"
  "$public/libbase/stringprintf.cpp"
  "$public/libpropertyinfoparser/property_info_parser.cpp"
)
for source in "${info_sources[@]}"; do
  queue_compile property-info "$compiler" "$source" \
    -std=c++20 -O0 -g -Wall -Wextra -Werror -ffunction-sections -fdata-sections \
    -I "$public/libpropertyinfoserializer/include" \
    -I "$public/libpropertyinfoparser/include" -I "$public/libbase/include"
done
queue_compile property-init "${compiler%++}" \
  "$workspace/tools/native-arm64-binder/property-init.c" \
  -x c -O0 -g -Wall -Wextra -Werror -Wno-deprecated-declarations
drain_compiles
fail_if_interrupted
if [[ "$compile_failed" != 0 ]]; then
  printf 'Compilation failed; every started compiler was waited.\n' >&2
  exit 1
fi
"$compiler" -Wall -Wextra -Werror -static-libstdc++ "${peer_objects[@]}" \
  -lbinder_ndk -ldl -o "$out/data-peer" > "$out/link-logs/data-peer.log" 2>&1
fail_if_interrupted
"$compiler" -Wall -Wextra -Werror -static-libstdc++ -Wl,--gc-sections \
  "${info_objects[@]}" -o "$out/property-info" > "$out/link-logs/property-info.log" 2>&1
fail_if_interrupted
"${compiler%++}" -Wall -Wextra -Werror "${init_objects[@]}" -ldl \
  -o "$out/property-init" > "$out/link-logs/property-init.log" 2>&1
fail_if_interrupted
"$compiler" --version > "$out/ndk-compiler.txt"
for binary in data-peer property-info property-init; do
  readelf -h -d -l "$out/$binary" > "$out/$binary-elf.txt"
done
# Root finalizer pins all generated headers/cpps/ELFs/dependencies/wait records,
# actual opened compiler/link inputs and exact tool/runtime source identities.
# NDK .so link stubs are compile inputs, never runtime substitutes.
