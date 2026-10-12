#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/sensor-wake sim/rust-cargo
python3 tools/prepare-sensors-boundary.py
python3 tools/prepare-suspend-boundary.py --verify-only
args=(--rm --init --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/sensor-wake:/target"
 -w /workspace/native-sensors-boundary pixel-native-radio:20261012)
docker run "${args[@]}" cargo generate-lockfile --offline
docker run "${args[@]}" cargo fmt --check
docker run "${args[@]}" cargo generate-lockfile --offline --manifest-path ../native-sensors/Cargo.toml
docker run "${args[@]}" cargo fmt --manifest-path ../native-sensors/Cargo.toml --check
docker run "${args[@]}" cargo clippy --all-targets --locked --offline --manifest-path ../native-sensors/Cargo.toml -- -D warnings
docker run "${args[@]}" cargo clippy --features wake-fault-test --all-targets --locked --offline --target x86_64-unknown-linux-musl -- -D warnings
docker run "${args[@]}" cargo build --features wake-fault-test --locked --offline --release --target x86_64-unknown-linux-musl --bin sensor-wake-client
docker run "${args[@]}" readelf -l /target/x86_64-unknown-linux-musl/release/sensor-wake-client > artifacts/sensor-wake/client-elf.txt
if rg -q INTERP artifacts/sensor-wake/client-elf.txt; then echo 'Sensor wake client requires dynamic interpreter' >&2;exit 1;fi
python3 - <<'PY'
import hashlib,json,pathlib
root=pathlib.Path('.');out=root/'artifacts/sensor-wake'
files=[*sorted((root/'native-sensors').rglob('*.rs')),*sorted((root/'native-sensors-boundary').rglob('*.rs')),root/'native-sensors/Cargo.toml',root/'native-sensors/Cargo.lock',root/'native-sensors-boundary/Cargo.toml',root/'native-sensors-boundary/Cargo.lock',out/'x86_64-unknown-linux-musl/release/sensor-wake-client',*sorted((out/'x86_64-unknown-linux-musl/release/build').glob('pokelinux-sensors-boundary-*/out/sensor.rs')),*sorted((out/'x86_64-unknown-linux-musl/release/build').glob('pokelinux-suspend-boundary-*/out/suspend.rs'))]
files=[p for p in files if 'target' not in p.parts]
(out/'build-evidence.json').write_text(json.dumps({'files':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}},indent=2)+'\n')
PY
