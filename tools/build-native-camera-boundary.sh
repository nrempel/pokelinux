#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/camera-boundary sim/rust-cargo
python3 tools/verify-camera-schemas.py
args=(--rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/artifacts/camera-boundary:/target"
 -w /workspace/native-camera-boundary pixel-native-radio:20261012)
docker run "${args[@]}" cargo generate-lockfile --offline
docker run "${args[@]}" cargo fmt --check
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" -w /workspace/native-camera pixel-native-radio:20261012 cargo fmt --check
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" -w /workspace/native-camera pixel-native-radio:20261012 cargo clippy --all-targets --offline -- -D warnings
docker run "${args[@]}" cargo clippy --all-targets --locked --offline --target x86_64-unknown-linux-musl -- -D warnings
docker run "${args[@]}" cargo build --locked --offline --release --target x86_64-unknown-linux-musl

for client in camera-binder-client camera-owner-client; do
 docker run "${args[@]}" readelf -l "/target/x86_64-unknown-linux-musl/release/$client" > "artifacts/camera-boundary/$client-elf-program-headers.txt"
 if rg -q INTERP "artifacts/camera-boundary/$client-elf-program-headers.txt"; then
  echo 'Camera client unexpectedly requires a dynamic ELF interpreter' >&2; exit 1
 fi
done
python3 - <<'EVIDENCE'
import hashlib,json,pathlib
root=pathlib.Path('.'); out=root/'artifacts/camera-boundary'
files=[*sorted((root/'native-camera-boundary').rglob('*.rs')),*sorted((root/'native-camera').rglob('*.rs')),root/'native-camera/Cargo.toml',root/'native-camera-boundary/Cargo.toml',root/'native-camera-boundary/Cargo.lock',out/'x86_64-unknown-linux-musl/release/camera-binder-client',out/'x86_64-unknown-linux-musl/release/camera-owner-client',*sorted((out/'x86_64-unknown-linux-musl/release/build').glob('pokelinux-camera-boundary-*/out/camera.rs'))]
(out/'rust-build-evidence.json').write_text(json.dumps({'scope':'generated rsbinder proxy and original Rust adapter; live NDK comparison in runtime evidence','files':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}},indent=2)+'\n')
EVIDENCE
