#!/bin/bash
# Own cloned disk/guest entry; never writes the shared simulation image.
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-}" in prepare|red|red-font|green|green-worker) phase=$1;; *) echo 'Usage: tools/native-theme-suite.sh prepare|red|red-font|green|green-worker' >&2; exit 64;; esac
exec 9>artifacts/native-theme/suite.lock
flock -n 9 || { echo 'Native theme suite already owned' >&2; exit 1; }
name=pokelinux-native-theme-b472
docker container inspect "$name" >/dev/null 2>&1 && { echo 'Owned container exists; inspect first' >&2; exit 1; }
if [[ ! -f artifacts/native-theme/vm/root.img ]]; then
  cp --reflink=auto --sparse=always sim/rootfs.img artifacts/native-theme/vm/root.img
  cat > artifacts/native-theme/vm/entry <<'BOOT'
#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python3 /workspace/tools/native-theme-guest.py
BOOT
  debugfs -w -R 'rm /usr/local/bin/pixel-sim-boot' artifacts/native-theme/vm/root.img >/dev/null
  debugfs -w -R 'write artifacts/native-theme/vm/entry /usr/local/bin/pixel-sim-boot' artifacts/native-theme/vm/root.img >/dev/null
  debugfs -w -R 'set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755' artifacts/native-theme/vm/root.img >/dev/null
fi
[[ $phase != prepare ]] || exit 0
out=artifacts/native-theme/runtime/$phase
[[ ! -e $out ]] || { echo "Preserve prior evidence: $out exists" >&2; exit 1; }
mkdir -p "$out"
python3 - "$phase" "$out" <<'CONFIG'
import hashlib,json,sys
from pathlib import Path
mode='baseline' if sys.argv[1]=='red-font' else 'production'
m=Path('artifacts/native-theme/baseline/manifest.json')
if mode=='baseline' and not m.is_file():raise SystemExit('Create and pin the readonly pre-font consumer snapshot first')
Path(sys.argv[2]+'/run-config.json').write_text(json.dumps({'consumer':mode,'baseline_manifest_sha256':hashlib.sha256(m.read_bytes()).hexdigest() if mode=='baseline' else None})+'\n')
CONFIG
cleanup() { docker rm -f "$name" >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM
set +e
docker run --name "$name" --network none --cpus 3 --memory 5g --pids-limit 512 \
  --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace:ro" -v "$PWD/artifacts/native-theme/vm:/vm:ro" \
  -v "$PWD/$out:/results:rw" pixel-graphics-sim:20261011-b472 bash /workspace/tools/run-native-theme-vm.sh > "$out/host.log" 2>&1
status=$?
set -e
printf '%s\n' "$status" > "$out/exit-code"
cat "$out/host.log"
exit "$status"
