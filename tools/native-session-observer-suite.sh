#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
phase=${1:?red or green}
[[ $phase == red || $phase == green ]] || exit 64
label=${2:-$phase}
[[ $label =~ ^[a-z0-9-]+$ ]] || exit 64
root=artifacts/native-session-observer
mkdir -p "$root"
exec 9>"$root/owned.lock"
flock -n 9 || { echo 'Own observer runner is busy' >&2; exit 1; }
name=pokelinux-native-session-observer-20261012
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
"${docker_local[@]}" container inspect "$name" >/dev/null 2>&1 && { echo 'Own observer container exists' >&2; exit 1; }
out=$root/$label
[[ ! -e $out ]] || { echo 'Preserve existing observer proof' >&2; exit 1; }
mkdir -p "$out/source" "$out/target"
python3 - "$out" "$phase" <<'SNAPSHOT'
from pathlib import Path
import hashlib,json,shutil,sys
out=Path(sys.argv[1])
for name in ('native-session','native-lifecycle'):
 shutil.copytree(name,out/'source'/name,ignore=shutil.ignore_patterns('target','._*'))
sources={str(p.relative_to(out/'source')):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((out/'source').rglob('*')) if p.is_file()}
(out/'selection.json').write_text(json.dumps({'phase':sys.argv[2],'executed_source_mount':'/source:ro','sources':sources},indent=2)+'\n')
SNAPSHOT
container_id=$("${docker_local[@]}" create --name "$name" --network none --cpus 2 --memory 3g --pids-limit 128 --cap-drop ALL \
 --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
 -v "$PWD/$out/source:/source:ro" -v "$PWD/sim/rust-cargo:/cargo:rw" -v "$PWD/$out:/results:rw" \
 -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/results/target --entrypoint bash \
 pixel-phone-rust:20261012-b472 -c 'cargo test --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --test process -- --test-threads=1')
# Only this successful create transfers ownership. Refusal and name-collision
# failures cannot remove an existing process/container belonging to another run.
cleanup() { "${docker_local[@]}" rm -f "$container_id" >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM
"${docker_local[@]}" image inspect pixel-phone-rust:20261012-b472 --format '{{.Id}}' > "$out/image-id"
printf '%s\n' "$container_id" > "$out/container-id"
set +e
"${docker_local[@]}" start -a "$container_id" > "$out/log" 2>&1
status=$?
set -e
printf '%s\n' "$status" > "$out/exit-code"
cat "$out/log"
exit "$status"
