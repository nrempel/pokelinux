#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
test "$#" = 2
base=$1;label=$2
[[ "$base" =~ ^artifacts/native-camera-service-boundary-[a-z0-9-]+$ && "$label" =~ ^[a-zA-Z0-9_-]+$ ]]
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
exec 9>"$base/guest.lock";flock -n 9
# Canonical reader lock spans actual Docker wait and post-input closure.
# Historical VM reader takes exclusive; prepare refuses an existing backing separately.
exec 8</home/ubuntu/Projects/pokelinux/artifacts/native-arm64-binder/vm.lock;flock -s -n 8
out="$base/runtime/$label";mkdir -p "$base/runtime";mkdir "$out"
cat "/proc/$$/fdinfo/8" > "$out/canonical-lock-before.txt"
guest=eagain-red-guest.py
if [[ "$base" == artifacts/native-camera-service-boundary-green ]];then guest=green-guest.py;fi
command=("${docker_local[@]}" run --rm --name "pokelinux-$label" --network none --cpus 3 --memory 3g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --read-only --tmpfs /tmp:rw,exec,nosuid,nodev,size=1g --user 1000:1000 -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" -v "$PWD/$base/freeze/sources/tools/native-camera-service-boundary/$guest:/workspace/tools/native-arm64-binder/guest.py:ro" -v /home/ubuntu/Projects/pokelinux/sim/boot/Image:/inputs/Image:ro -v /home/ubuntu/Projects/pokelinux/sim/boot/initramfs-linux.img:/inputs/initramfs-linux.img:ro -v /home/ubuntu/Projects/pokelinux/artifacts/native-arm64-binder/vm/root.img:/inputs/root.img:ro -w /workspace sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6 python3 "/workspace/$base/freeze/sources/tools/native-camera-service-boundary/composed-vm.py" "$base" "$label")
printf '%s\0' "${command[@]}" > "$out/docker-command.argv0"
set +e
"${command[@]}" > "$out/docker-console.log" 2>&1
docker_exit=$?
set -e
printf '%s\n' "$docker_exit" > "$out/docker-exit-code.txt"
sha256sum /home/ubuntu/Projects/pokelinux/sim/boot/Image /home/ubuntu/Projects/pokelinux/sim/boot/initramfs-linux.img /home/ubuntu/Projects/pokelinux/artifacts/native-arm64-binder/vm/root.img > "$out/input-after.sha256"
cat "/proc/$$/fdinfo/8" > "$out/canonical-lock-after.txt"
"${docker_local[@]}" run --rm --network none --cpus 1 --memory 512m --pids-limit 64 --cap-drop ALL --security-opt no-new-privileges --user 1000:1000 -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" -w /workspace sha256:07b2b220c62ebe15a90af7e10d703297b511508891492a292ae986f0521c448a python3 "/workspace/$base/freeze/sources/tools/native-camera-service-boundary/composed-receipt.py" "$base" "$label"
exit "$docker_exit"
