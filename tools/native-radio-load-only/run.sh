#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
base=artifacts/native-radio-load-only
registry=/home/ubuntu/Projects/pokelinux/sim/rust-cargo/registry
target=/dev/shm/pokelinux-radio-load-owner-sol-target
cargo_home=/dev/shm/pokelinux-radio-load-owner-sol-cargo
builder=sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6
runtime=sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6
backing=/home/ubuntu/Projects/pokelinux/artifacts/native-arm64-binder/vm/root.img
kernel=/home/ubuntu/Projects/pokelinux/sim/boot
case "${1:-}" in
 build)
  mkdir -p "$target" "$cargo_home" "$base"
  "${docker_local[@]}" run --rm --network none --cpus "${POKELINUX_RADIO_BUILD_CPUS:-8}" --memory 12g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
   -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e CARGO_INCREMENTAL=0 -e RUSTFLAGS="-C target-feature=+crt-static" \
   -v "$PWD:/workspace:ro" -v "$PWD/tools/native-radio-load-only:/workspace/tools/native-radio-load-only" \
   -v "$cargo_home:/cargo" -v "$registry:/cargo/registry:ro" -v "$target:/target" -w /workspace/tools/native-radio-load-only "$builder" \
   bash -ec 'cargo fmt; cargo clippy --all-targets --locked --offline --target aarch64-unknown-linux-gnu -- -D warnings; cargo build --locked --offline --target aarch64-unknown-linux-gnu'
  ;;
 freeze)
  freeze_label=${2:-freeze};[[ "$freeze_label" =~ ^[a-zA-Z0-9-]+$ ]]
  exec 8</home/ubuntu/Projects/pokelinux/artifacts/native-arm64-binder/vm.lock;flock -s -n 8
  "${docker_local[@]}" run --rm --network none --cpus 1 --memory 512m --pids-limit 64 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
   -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" -v "$target:/target:ro" \
   -v "$kernel/Image:/inputs/Image:ro" -v "$kernel/initramfs-linux.img:/inputs/initramfs-linux.img:ro" -v "$backing:/inputs/root.img:ro" \
   -w /workspace "$runtime" python3 tools/native-radio-load-only/freeze.py "$freeze_label"
  ;;
 red|green)
  test "$#" -ge 2;case=$1;label=$2;[[ "$label" =~ ^[a-zA-Z0-9-]+$ ]];freeze_label=${3:-freeze};[[ "$freeze_label" =~ ^[a-zA-Z0-9-]+$ ]]
  exec 9>"$base/guest.lock";flock -n 9
  exec 8</home/ubuntu/Projects/pokelinux/artifacts/native-arm64-binder/vm.lock;flock -s -n 8
  mkdir -p "$base/runtime";mkdir "$base/runtime/$label";out="$base/runtime/$label"
  cat "/proc/$$/fdinfo/8" > "$out/canonical-lock-before.txt"
  sha256sum "$base/$freeze_label/selection.json" > "$out/selection-before.sha256"
  command=("${docker_local[@]}" run --rm --name pokelinux-radio-load-public-"$case" --network none --cpus 6 --memory 8g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --read-only --tmpfs /tmp:rw,exec,nosuid,nodev,size=1g --user "$(id -u):$(id -g)" \
   -v "$PWD:/workspace:ro" -v "$PWD/$out:/workspace/$out" \
   -v "$kernel/Image:/inputs/Image:ro" -v "$kernel/initramfs-linux.img:/inputs/initramfs-linux.img:ro" -v "$backing:/inputs/root.img:ro" \
   -w /workspace "$runtime" python3 /workspace/"$base"/"$freeze_label"/source/tools/native-radio-load-only/vm.py "$label" "$case" "$freeze_label")
  printf '%s\0' "${command[@]}" > "$out/docker-command.argv0"
  set +e;"${command[@]}" > "$out/docker-console.log" 2>&1;docker_exit=$?;set -e
  printf '%s\n' "$docker_exit" > "$out/docker-exit-code.txt"
  sha256sum "$kernel/Image" "$kernel/initramfs-linux.img" "$backing" > "$out/input-after.sha256"
  cat "/proc/$$/fdinfo/8" > "$out/canonical-lock-after.txt"
  sha256sum "$base/$freeze_label/selection.json" > "$out/selection-after.sha256"
  exit "$docker_exit"
  ;;
 *) echo 'Usage: run.sh build|freeze|red unique-label' >&2;exit 2;;
esac
