#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-}" in red|green) phase=$1;; *) echo 'Usage: native-theme-worker-suite.sh red|green' >&2; exit 64;; esac
out=artifacts/native-theme-worker/$phase
[[ ! -e $out ]] || { echo "Preserve prior worker evidence: $out" >&2; exit 1; }
mkdir -p "$out"
name=pokelinux-native-theme-worker-b472
docker container inspect "$name" >/dev/null 2>&1 && { echo 'Owned worker container exists; inspect first' >&2; exit 1; }
trap 'docker rm -f "$name" >/dev/null 2>&1 || true' EXIT INT TERM
set +e
docker run --name "$name" --network none --cpus 2 --memory 2g --pids-limit 128 --cap-drop ALL \
 --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
 -v "$PWD:/workspace:ro" -v "$PWD/$out:/results:rw" --entrypoint python3 \
 pixel-phone-qt-native:20261012-b472 /workspace/tools/native-theme-worker-test.py "$phase" > "$out/host.log" 2>&1
status=$?
set -e
printf '%s\n' "$status" > "$out/exit-code"
cat "$out/host.log"
exit "$status"
