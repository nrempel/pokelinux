#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
phase=${1:?red or green}; [[ $phase == red || $phase == green ]] || exit 64
label=${2:?new result label}; [[ $label =~ ^[a-z0-9-]+$ ]] || exit 64
adapter=${3:-tools/native-session-compositor/adapter.cpp}
observer=${4:-}
root=artifacts/native-session-compositor
mkdir -p "$root/vm"
exec 9>"$root/owned.lock"
flock -n 9 || { echo 'Own compositor fixture is busy' >&2; exit 1; }
name=pokelinux-session-compositor-sol-20261012
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
"${docker_local[@]}" container inspect "$name" >/dev/null 2>&1 && { echo 'Own compositor container exists' >&2; exit 1; }
[[ -f $adapter ]] || { echo 'Selected adapter source missing' >&2; exit 1; }
out="$root/$label"; [[ ! -e $out ]] || { echo 'Preserve existing compositor proof' >&2; exit 1; }
mkdir -p "$out/source" "$out/bins"
python3 - "$out" "$phase" "$adapter" "$observer" <<'SNAPSHOT'
import hashlib,json,shutil,sys
from pathlib import Path
out=Path(sys.argv[1]);source=out/'source';bins=out/'bins'
for name in ['guest.py','run.sh','run-vm.sh','README.md']:
 shutil.copyfile(Path('tools/native-session-compositor')/name,source/name)
shutil.copyfile(sys.argv[3],source/'adapter.cpp')
shutil.copyfile('artifacts/native-session-build-probe/run-4/source/lock-client.c',source/'lock-client-original.c')
shutil.copyfile('artifacts/native-session-build-probe/run-4/probe.so',bins/'probe.so')
if sys.argv[4]:
 shutil.copyfile(sys.argv[4],bins/'session-observer-fixture');(bins/'session-observer-fixture').chmod(0o755)
observer_build=Path(sys.argv[4]).parent.parent.parent.parent if sys.argv[4] else None
selected_crates=observer_build/'source' if observer_build and (observer_build/'selection.json').is_file() else Path('.')
for name in ('native-session','native-lifecycle','native-kernel-identity'):
 shutil.copytree(selected_crates/name,source/name,ignore=shutil.ignore_patterns('target','._*'))
if observer_build:
 shutil.copyfile(observer_build/'selection.json',source/'observer-build-selection.json')
 shutil.copyfile(observer_build/'build.log',source/'observer-build.log')
 if (observer_build/'bins/process-tests').is_file():
  shutil.copyfile(observer_build/'bins/process-tests',bins/'observer-process-tests')
  peer=out/'target/aarch64-unknown-linux-gnu/debug/session-peer-fixture';peer.parent.mkdir(parents=True)
  shutil.copyfile(observer_build/'bins/session-peer-fixture',peer);peer.chmod(0o755)
  leaf=Path('artifacts/pidfd-identity')
  proof=json.loads((leaf/'proof.json').read_text())
  for selected in proof['arm64_static']['executables']:
   candidate=Path(selected['path']);assert hashlib.sha256(candidate.read_bytes()).hexdigest()==selected['sha256']
   shutil.copyfile(candidate,bins/('kernel-'+candidate.name))
  shutil.copyfile(leaf/'proof.json',source/'leaf-build-proof.json')
 check=json.loads((source/'observer-build-selection.json').read_text())
 for name,digest in check['sources'].items():
  assert hashlib.sha256((source/name).read_bytes()).hexdigest()==digest,name
files={str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for folder in [source,bins,*([out/'target'] if (out/'target').exists() else [])] for p in sorted(folder.rglob('*')) if p.is_file()}
selection={'phase':sys.argv[2],'adapter_selected_path':sys.argv[3],'observer_selected_path':sys.argv[4] or None,'files':files,'kernel_image_sha256':hashlib.sha256(Path('sim/boot/Image').read_bytes()).hexdigest(),'initrd_sha256':hashlib.sha256(Path('sim/boot/initramfs-linux.img').read_bytes()).hexdigest(),'expected_hyprland_elf_sha256':'ae9a9e4129263e74df3f8a0a62beefd00f9bf4224caf51e96ebf0482ab74a85a','expected_probe_sha256':'ac673f13ee21d1ea42b7d509e04703bac7bc40661c08d064172c4177d9a5e60b','owned_kernel_test_binaries':(bins/'observer-process-tests').is_file()}
(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
SNAPSHOT
if [[ ! -f "$root/vm/root.img" ]]; then
 cp --reflink=auto --sparse=always sim/rootfs.img "$root/vm/root.img"
 cat > "$root/vm/entry" <<'BOOT'
#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python3 /results/source/guest.py
BOOT
 debugfs -w -R 'rm /usr/local/bin/pixel-sim-boot' "$root/vm/root.img" >/dev/null
 debugfs -w -R "write $root/vm/entry /usr/local/bin/pixel-sim-boot" "$root/vm/root.img" >/dev/null
 debugfs -w -R 'set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755' "$root/vm/root.img" >/dev/null
fi
container_id=$("${docker_local[@]}" create --name "$name" --network none --cpus 10 --memory 8g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace:ro" -v "$PWD/$root/vm:/vm:ro" -v "$PWD/$out:/results" pixel-graphics-sim:20261011-b472 bash /results/source/run-vm.sh)
cleanup() { "${docker_local[@]}" rm -f "$container_id" >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM
printf '%s\n' "$container_id" > "$out/container-id"
"${docker_local[@]}" image inspect pixel-graphics-sim:20261011-b472 --format '{{.Id}}' > "$out/image-id"
set +e
"${docker_local[@]}" start -a "$container_id" > "$out/host.log" 2>&1
status=$?
set -e
printf '%s\n' "$status" > "$out/exit-code"
cat "$out/host.log"
exit "$status"
