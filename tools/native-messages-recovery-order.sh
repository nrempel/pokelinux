#!/bin/bash
# Correction outputs are separate from the historical ff8d206c checkpoint.
set -euo pipefail
cd "$(dirname "$0")/.."
correction=artifacts/native-messages/recovery-order-correction
phase=${1:-build}
run_id=${2:-}
[[ -z "$run_id" || "$run_id" =~ ^[a-z0-9-]+$ ]] || exit 2
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
build_args=(--rm --pull=never --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/$correction/build:/target" -w /workspace)
case "$phase" in
 build)
  [[ ! -e "$correction/build-checks.log" ]] || { echo 'Preserve correction build proof' >&2; exit 2; }
  {
   "${docker_local[@]}" run "${build_args[@]}" pixel-phone-rust:20261012-b472 cargo fmt --manifest-path native-lifecycle/Cargo.toml --check
   "${docker_local[@]}" run "${build_args[@]}" pixel-phone-rust:20261012-b472 cargo clippy --manifest-path native-lifecycle/Cargo.toml --locked --offline --all-targets -- -D warnings
   "${docker_local[@]}" run "${build_args[@]}" pixel-phone-rust:20261012-b472 cargo clippy --manifest-path native-messages/Cargo.toml --locked --offline --all-targets -- -D warnings
   for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
    "${docker_local[@]}" run "${build_args[@]}" pixel-phone-rust:20261012-b472 cargo build --manifest-path native-messages/Cargo.toml --locked --offline --target "$target" --features isolated-fixture --bins
    "${docker_local[@]}" run "${build_args[@]}" pixel-phone-rust:20261012-b472 cargo build --manifest-path native-lifecycle/Cargo.toml --locked --offline --target "$target" --bins
    "${docker_local[@]}" run "${build_args[@]}" pixel-phone-rust:20261012-b472 cargo build --manifest-path native-messages/Cargo.toml --locked --offline --target "$target" --release --bins
   done
  } > "$correction/build-checks.log" 2>&1
  ;;
 green-ordering|green-retained-fd|service-green)
  out="$correction/$phase${run_id:+-$run_id}"
  [[ ! -e "$out/log" ]] || { echo 'Preserve correction process proof' >&2; exit 2; }
  mkdir -p "$out"
  python3 - "$out" "$correction" <<'PIN'
import hashlib,json,sys
from pathlib import Path
out,root=map(Path,sys.argv[1:])
source=[f for name in ['native-messages','native-lifecycle'] for f in Path(name).rglob('*') if f.is_file() and 'target' not in f.parts]
source.extend(Path('tools')/name for name in ['native-messages-service-fixture.py','native-messages-recovery-order-fixture.py','native-messages-retained-fd-app.py','native-messages-recovery-order.sh'])
binaries=[root/'build/x86_64-unknown-linux-gnu/debug'/name for name in ['messages-service','messages-stop','messages-launch','messages-seed-fixture','lifecycle-launcher','lifecycle-client']]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
(out/'run-selection.json').write_text(json.dumps({'source_sha256':{str(p):sha(p) for p in sorted(source)},'binary_sha256':{str(p):sha(p) for p in binaries},'mechanism':'actual Linux UID/PID/seqpacket and SIGSTOP; no credential or provider mocks'},indent=2)+'\n')
PIN
  option=--retained-order
  fixture_phase=recovery-only
  [[ "$phase" != green-retained-fd ]] || option=--retained-fd
  [[ "$phase" != service-green ]] || fixture_phase=green
  set +e
  "${docker_local[@]}" run --rm --init --network none --cpus 2 --memory 512m --pids-limit 64 --ulimit core=0 \
   --cap-drop ALL --cap-add SETUID --cap-add SETGID --cap-add CHOWN --cap-add KILL --cap-add DAC_OVERRIDE \
   --security-opt no-new-privileges --read-only -e NATIVE_MESSAGES_ISOLATED=1 \
   --tmpfs /opt:rw,exec,nosuid,nodev --tmpfs /run:rw,nosuid,nodev --tmpfs /var/lib:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev \
   -v "$PWD:/workspace:ro" -v "$PWD/$correction/build/x86_64-unknown-linux-gnu/debug:/bins:ro" -v "$PWD/$out:/evidence" \
   pixel-graphics-build:20261011-b472 python3 /workspace/tools/native-messages-service-fixture.py --phase "$fixture_phase" "$option" > "$out/log" 2>&1
  status=$?
  set -e
  printf '%s\n' "$status" > "$out/exit-code"
  cat "$out/log"
  exit "$status"
  ;;
 *) echo 'Usage: native-messages-recovery-order.sh build|green-ordering|green-retained-fd|service-green [evidence-suffix]' >&2; exit 2 ;;
esac
