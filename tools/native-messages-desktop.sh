#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
phase=${1:-green};case "$phase" in prepare|red|green|green-[0-9]*|red-[0-9]*) ;; *) exit 2;; esac
mkdir -p artifacts/native-messages/vm artifacts/native-messages/desktop
build_root=${NATIVE_MESSAGES_BUILD_ROOT:-artifacts/native-messages/build}
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
exec 9>artifacts/native-messages/desktop.lock
flock -n 9 || { echo 'Owned Messages desktop is busy' >&2;exit 2; }
if [[ ! -f artifacts/native-messages/vm/root.img ]];then
 cp --reflink=auto --sparse=always sim/rootfs.img artifacts/native-messages/vm/root.img
 cat > artifacts/native-messages/vm/entry <<'BOOT'
#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python3 /results/guest.py
BOOT
 debugfs -w -R 'rm /usr/local/bin/pixel-sim-boot' artifacts/native-messages/vm/root.img >/dev/null
 debugfs -w -R 'write artifacts/native-messages/vm/entry /usr/local/bin/pixel-sim-boot' artifacts/native-messages/vm/root.img >/dev/null
 debugfs -w -R 'set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755' artifacts/native-messages/vm/root.img >/dev/null
fi
[[ $phase != prepare ]] || exit 0
out="artifacts/native-messages/desktop/$phase";[[ ! -e "$out" ]]||{ echo 'Preserve prior proof' >&2;exit 2; };mkdir -p "$out"
python3 - "$out" "$build_root" <<'PIN'
import hashlib,json,shutil,sys
from pathlib import Path
out=Path(sys.argv[1]);build=Path(sys.argv[2]);source=out/'source';source.mkdir()
for name in ['native-messages','native-lifecycle','native-messages-ui']:
 shutil.copytree(name,source/name,ignore=shutil.ignore_patterns('target'))
(source/'tools').mkdir()
for name in ['native-messages-guest.py','native-messages-service-fixture.py','native-messages-recovery-order-fixture.py','native-messages-retained-fd-app.py','build-native-messages-qt-guest.py','omarchy_theme_patch.py','native-messages-desktop.sh','run-native-messages-vm.sh','native-messages-recovery-order.sh']:
 shutil.copyfile(Path('tools')/name,source/'tools'/name)
shutil.copyfile('tools/native-messages-guest.py',out/'guest.py')
(source/'native-app').mkdir()
for name in ['PhoneThemeFiles.cpp','PhoneThemeFiles.h','PhoneTheme.qml','ThemePalette.js']:
 shutil.copyfile(Path('native-app')/name,source/'native-app'/name)
(out/'bins').mkdir()
for name in ['messages-service','messages-launch','messages-stop','messages-seed-fixture','lifecycle-launcher','lifecycle-client']:
 shutil.copyfile(build/'aarch64-unknown-linux-gnu/debug'/name,out/'bins'/name)
 (out/'bins'/name).chmod(0o755)
files={str(f.relative_to(out)):hashlib.sha256(f.read_bytes()).hexdigest()for folder in [source,out/'bins']for f in folder.rglob('*')if f.is_file()}
(out/'run-config.json').write_text(json.dumps({'source':str(source),'bins':str(out/'bins'),'files':files},indent=2)+'\n')
PIN
name=pokelinux-messages-desktop-b472
"${docker_local[@]}" container inspect "$name" >/dev/null 2>&1&&{ echo 'Inspect owned container before launch' >&2;exit 2; }
cleanup(){ "${docker_local[@]}" rm -f "$name" >/dev/null 2>&1||true; }
trap cleanup EXIT INT TERM
set +e
"${docker_local[@]}" run --name "$name" --network none --cpus 3 --memory 5g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace:ro" -v "$PWD/artifacts/native-messages/vm:/vm:ro" -v "$PWD/$out:/results" pixel-graphics-sim:20261011-b472 bash /workspace/tools/run-native-messages-vm.sh > "$out/host.log" 2>&1
status=$?
set -e
printf '%s\n' "$status">"$out/exit-code"
cat "$out/host.log"
exit "$status"
