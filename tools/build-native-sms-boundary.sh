#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/sms-binder sim/rust-cargo
python3 - <<'CHECK'
import hashlib,json,pathlib
root=pathlib.Path('artifacts/sms-binder')
for row in json.loads((root/'frozen-schema-manifest.json').read_text())['sources']:
 if 'path' in row:assert hashlib.sha256((root/row['path']).read_bytes()).hexdigest()==row['sha256'],row['path']
CHECK
args=(--rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e CC_x86_64_unknown_linux_musl=musl-gcc
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/sms-binder:/target"
 -w /workspace/native-sms-boundary pixel-native-radio:20261012)
if [ ! -f native-sms-boundary/Cargo.lock ]; then docker run "${args[@]}" cargo generate-lockfile --offline; fi
docker run "${args[@]}" cargo fmt --check
docker run "${args[@]}" cargo clippy --all-targets --locked --offline --target x86_64-unknown-linux-musl -- -D warnings
docker run "${args[@]}" cargo build --release --locked --offline --target x86_64-unknown-linux-musl
