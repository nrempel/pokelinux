#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
root=artifacts/native-session-build-probe
mkdir -p "$root/vm"
exec 9>"$root/owned.lock"
flock -n 9 || { echo 'Own build probe is busy' >&2; exit 2; }
if [[ ! -f "$root/vm/root.img" ]]; then
cp --reflink=auto --sparse=always sim/rootfs.img "$root/vm/root.img"
cat > "$root/vm/entry" <<'BOOT'
#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python3 /workspace/tools/native-session-build-probe/guest.py
BOOT
debugfs -w -R 'rm /usr/local/bin/pixel-sim-boot' "$root/vm/root.img" >/dev/null
debugfs -w -R "write $root/vm/entry /usr/local/bin/pixel-sim-boot" "$root/vm/root.img" >/dev/null
debugfs -w -R 'set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755' "$root/vm/root.img" >/dev/null
fi
run_id=${1:-run-1}
case "$run_id" in run-[0-9]*) ;; *) exit 2 ;; esac
out="$root/$run_id"
[[ ! -e "$out" ]] || { echo 'Preserve prior proof' >&2; exit 2; }
mkdir -p "$out/source"
cp tools/native-session-build-probe/{probe.cpp,guest.py,run.sh,run-vm.sh,lock-client.c} "$out/source/"
name=pokelinux-session-build-sol-20261012
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
"${docker_local[@]}" container inspect "$name" >/dev/null 2>&1 && { echo 'Own named container already exists' >&2; exit 2; }
cleanup() { "${docker_local[@]}" rm -f "$name" >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM
set +e
"${docker_local[@]}" run --name "$name" --network none --cpus 3 --memory 5g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace:ro" -v "$PWD/$root/vm:/vm:ro" -v "$PWD/$out:/results" pixel-graphics-sim:20261011-b472 bash /workspace/tools/native-session-build-probe/run-vm.sh > "$out/host.log" 2>&1
status=$?
set -e
printf '%s\n' "$status" > "$out/exit-code"
cat "$out/host.log"
exit "$status"
