#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
test_name=$(python3 - <<'PY'
import json
from pathlib import Path
records=[json.loads(line) for line in Path('artifacts/native-lifecycle-storage/x86_64-tests-cargo.jsonl').read_text().splitlines()]
selected=[r['executable'] for r in records if r.get('reason')=='compiler-artifact' and r.get('executable') and r.get('profile',{}).get('test') and r['target']['kind']==['lib']]
assert len(selected)==1
name=Path(selected[0]).name
assert name.startswith('pokelinux_lifecycle-') and all(c.isalnum() or c in '-_' for c in name)
print(name)
PY
)
docker run --rm --init --network none --cpus 2 --memory 256m --pids-limit 32 \
 --cap-drop ALL --security-opt no-new-privileges --read-only \
 --tmpfs /run:rw,nosuid,nodev --tmpfs /opt:rw,exec,nosuid,nodev \
 -e NATIVE_LIFECYCLE_TEST_ROOT=/run -e NATIVE_LIFECYCLE_STORAGE_HELPER=/worker-source \
 -v "$PWD/artifacts/native-lifecycle-storage/target/x86_64-unknown-linux-gnu/debug/deps/$test_name:/tests:ro" \
 -v "$PWD/artifacts/native-lifecycle-storage/target/x86_64-unknown-linux-gnu/debug/lifecycle-daemon:/worker-source:ro" \
 pixel-graphics-build:20261011-b472 /tests --test-threads=1
