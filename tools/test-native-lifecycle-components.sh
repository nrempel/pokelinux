#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
test_name=$(python3 - <<'PY'
import json
from pathlib import Path
items=[json.loads(line) for line in Path('artifacts/native-lifecycle/x86-lib-cargo.jsonl').read_text().splitlines()]
paths=[x['executable'] for x in items if x.get('reason') == 'compiler-artifact' and x.get('profile',{}).get('test') and x.get('executable') and x['target']['kind'] == ['lib']]
assert len(paths) == 1, paths
name=Path(paths[0]).name
assert name.startswith('pokelinux_lifecycle-') and all(c.isalnum() or c in '-_' for c in name)
print(name)
PY
)
docker run --rm --init --network none --cpus 2 --memory 256m --pids-limit 32 \
 --cap-drop ALL --security-opt no-new-privileges --read-only --tmpfs /run:rw,nosuid,nodev \
 -e NATIVE_LIFECYCLE_TEST_ROOT=/run \
 -v "$PWD/artifacts/native-lifecycle/target/x86_64-unknown-linux-gnu/debug/deps/$test_name:/tests:ro" \
 pixel-graphics-build:20261011-b472 /tests --test-threads=1
