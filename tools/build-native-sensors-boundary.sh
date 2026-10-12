#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/sensors-boundary sim/rust-cargo
python3 tools/prepare-sensors-boundary.py
args=(--rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/sensors-boundary:/target"
 -w /workspace/native-sensors-boundary pixel-native-radio:20261012)
docker run "${args[@]}" cargo generate-lockfile --offline
docker run "${args[@]}" cargo fmt --check
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" -w /workspace/native-sensors pixel-native-radio:20261012 cargo fmt --check
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" -w /workspace/native-sensors pixel-native-radio:20261012 cargo clippy --all-targets --offline -- -D warnings
docker run "${args[@]}" cargo clippy --all-targets --locked --offline --target x86_64-unknown-linux-musl -- -D warnings
docker run "${args[@]}" cargo build --locked --offline --release --target x86_64-unknown-linux-musl
docker run "${args[@]}" readelf -l /target/x86_64-unknown-linux-musl/release/sensor-fmq-client > artifacts/sensors-boundary/client-elf-program-headers.txt
if rg -q INTERP artifacts/sensors-boundary/client-elf-program-headers.txt; then echo 'Sensor client unexpectedly requires dynamic ELF interpreter' >&2;exit 1;fi
python3 - <<'PY'
import hashlib,json,pathlib
root=pathlib.Path('.');out=root/'artifacts/sensors-boundary'
files=[*sorted((root/'native-sensors-boundary').rglob('*.rs')),*sorted((root/'native-sensors').rglob('*.rs')),root/'native-sensors/Cargo.toml',root/'native-sensors-boundary/Cargo.toml',root/'native-sensors-boundary/Cargo.lock',out/'x86_64-unknown-linux-musl/release/sensor-fmq-client',*sorted((out/'x86_64-unknown-linux-musl/release/build').glob('pokelinux-sensors-boundary-*/out/sensor.rs'))]
(out/'rust-build-evidence.json').write_text(json.dumps({'scope':'generated exact SensorsV3 proxy and original Rust owner; live original libfmq comparison in runtime evidence','files':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}},indent=2)+'\n')
PY
