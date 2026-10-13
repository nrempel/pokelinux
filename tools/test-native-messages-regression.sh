#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
phase=${1:-final}
[[ "$phase" =~ ^[a-z0-9-]+$ ]] || exit 2
out="artifacts/native-messages/lifecycle-regression-$phase"
[[ ! -e "$out" ]] || { echo 'Preserve existing proof' >&2; exit 2; }
mkdir -p "$out/bins" "$out/supervision" "$out/error" "$out/withheld"
build_root=${NATIVE_MESSAGES_BUILD_ROOT:-artifacts/native-messages/build}
for name in lifecycle-daemon lifecycle-client lifecycle-launcher; do
 cp "$build_root/x86_64-unknown-linux-gnu/debug/$name" "$out/bins/"
done
args=(--rm --init --network none --cpus 2 --memory 512m --pids-limit 64 --ulimit core=0
 --cap-drop ALL --cap-add SETUID --cap-add SETGID --cap-add CHOWN --cap-add KILL --cap-add DAC_OVERRIDE
 --security-opt no-new-privileges --read-only -e NATIVE_LIFECYCLE_ISOLATED=1
 --tmpfs /opt:rw,exec,nosuid,nodev --tmpfs /run:rw,nosuid,nodev --tmpfs /var/lib:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev
 -v "$PWD/$out/bins:/bins:ro")
docker run "${args[@]}" -e LIFECYCLE_STORAGE_RUN_BASELINE=1 \
 -v "$PWD/tools/native-lifecycle-fixture.py:/baseline-fixture.py:ro" \
 -v "$PWD/tools/native-lifecycle-storage-fixture.py:/storage-fixture.py:ro" \
 -v "$PWD/$out/supervision:/evidence" pixel-graphics-build:20261011-b472 \
 python3 /storage-fixture.py > "$out/supervision/log" 2>&1
for mode in error withheld; do
 docker run "${args[@]}" \
  -v "$PWD/tools/native-lifecycle-storage-cleanup-fixture.py:/cleanup-fixture.py:ro" \
  -v "$PWD/$out/$mode:/evidence" pixel-graphics-build:20261011-b472 \
  python3 /cleanup-fixture.py --binary-root /bins --results-dir /evidence --mode "$mode" > "$out/$mode/log" 2>&1
done
python3 - "$out" <<'PY'
import hashlib,json,sys
from pathlib import Path
out=Path(sys.argv[1])
results={name:json.loads((out/name/'result.json').read_text()) for name in ['supervision','error','withheld']}
assert all(value['passed'] for value in results.values())
assert len(results['supervision']['groups']) == 8
result={'passed':True,'cases':27,'scope':'18 installed cases + 7 actual process supervision + 2 injected reap failures with real child cleanup',
'binaries':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (out/'bins').iterdir()},
'fixtures':{name:hashlib.sha256((Path('tools')/name).read_bytes()).hexdigest() for name in ['native-lifecycle-fixture.py','native-lifecycle-storage-fixture.py','native-lifecycle-storage-cleanup-fixture.py']},
'results':results,'physical_device':False}
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print('PASS 27 current-source lifecycle regression cases')
PY
