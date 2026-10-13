#!/bin/bash
# Isolated synthetic ARM guest qualification only. No host Binder/adb/USB access.
set -euo pipefail
cd "$(dirname "$0")/../.."
test -f AGENTS.md
base=artifacts/native-arm64-binder
mkdir -p "$base/target" "$base/vm" "$base/runtime"
exec 9>"$base/task.lock"
flock -n 9
builder=sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6
simulator=sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6
limits=(--rm --network none --cpus 2 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)")
case "${1:-run}" in
 prepare-public)
  python3 tools/native-arm64-binder/prepare-public.py
  ;;
 ndk-build)
  python3 tools/native-arm64-binder/prepare-public.py
  python3 tools/native-arm64-binder/verify-runtime.py
  python3 tools/native-arm64-binder/verify-link-inputs.py
  docker run "${limits[@]}" -v "$PWD:/workspace:ro" -v "$PWD/$base:/test" "$builder" bash /workspace/tools/native-arm64-binder/build-ndk-in-container.sh
  ;;
 build)
  docker run "${limits[@]}" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target \
    -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/$base/target:/target" \
    -w /workspace/tools/native-arm64-binder/fixture "$builder" bash -ec \
    'cargo fmt --check; cargo clippy --all-targets --locked --offline --target aarch64-unknown-linux-gnu -- -D warnings; cargo build --locked --offline --target aarch64-unknown-linux-gnu'
  ;;
 clone)
  test ! -e "$base/vm/root.img"
  test ! -e "$base/vm/root.img.partial"
  echo '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b  sim/boot/Image' | sha256sum -c -
  echo 'a9bb29d29abda94748c9948f903c3c1704f829eea477652b9a4201a61f36a647  sim/boot/initramfs-linux.img' | sha256sum -c -
  cp --reflink=auto --sparse=always sim/rootfs.img "$base/vm/root.img.partial"
  docker run "${limits[@]}" -v "$PWD:/workspace:ro" -v "$PWD/$base:/test" "$simulator" bash -ec '
    debugfs -w -R "rm /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial
    debugfs -w -R "write /workspace/tools/native-arm64-binder/boot.sh /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial
    debugfs -w -R "set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755" /test/vm/root.img.partial
    debugfs -R "cat /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial > /test/vm/installed-boot.sh
    cmp /test/vm/installed-boot.sh /workspace/tools/native-arm64-binder/boot.sh'
  mv "$base/vm/root.img.partial" "$base/vm/root.img"
  ;;
 run|driver-red|driver-green|ndk-run|ndk-red)
  phase=${1:-run}; test "$phase" != run || phase=green
  test "$phase" != ndk-run || phase=ndk-green
  label=${2:-"$phase-$(date -u +%Y%m%dT%H%M%SZ)"}
  docker run "${limits[@]}" --name pokelinux-arm64-binder-test \
    -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" "$simulator" \
    python3 /workspace/tools/native-arm64-binder/vm.py "$phase" "$label"
  ;;
 *) echo 'Usage: run.sh prepare-public|build|ndk-build|clone|run|ndk-run [unique-result-label]|driver-red|driver-green|ndk-red' >&2; exit 2;;
esac
