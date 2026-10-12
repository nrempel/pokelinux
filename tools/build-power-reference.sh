#!/bin/bash
# Executes unmodified public libperfmgr source, never a factory vendor target.
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD/artifacts/native-power/reference"
case "${1:-build}" in
 fetch) exec python3 tools/fetch-power-reference.py ;;
 build)
  python3 - <<'PY'
import hashlib,json,pathlib
root=pathlib.Path('artifacts/native-power/reference/upstream')
for f in json.loads(pathlib.Path('tools/power-reference/fetch-manifest.json').read_text())['files']:
 p=root/f['path'];data=p.read_bytes();assert len(data)==f['bytes'] and hashlib.sha256(data).hexdigest()==f['sha256'],p
PY
  mkdir -p "$root/shims/android-base" "$root/shims/utils"
  for header in unique_fd chrono_utils file logging properties stringprintf strings; do
   printf '#include "/harness/platform.h"\n' > "$root/shims/android-base/$header.h"
  done
  printf '#include "/harness/platform.h"\n' > "$root/shims/utils/Trace.h"
  docker run --rm --network none --memory 1g --cpus 1 --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges \
   --user "$(id -u):$(id -g)" --mount "type=bind,src=$root,dst=/results" \
   --mount "type=bind,src=$PWD/tools/power-reference,dst=/harness,readonly" \
   --workdir /results --entrypoint sh pixel-graphics-build:20261011-b472 \
   -c 'g++ -std=c++17 -O0 -g0 -Wall -Wextra -I shims -I upstream/include /harness/oracle.cpp upstream/RequestGroup.cc upstream/Node.cc upstream/FileNode.cc -o oracle && ./oracle /results/temporary-node-work > reference.tsv'
  python3 tools/verify-power-reference.py
  ;;
 *) echo 'Usage: build-power-reference.sh fetch|build' >&2; exit 2 ;;
esac
