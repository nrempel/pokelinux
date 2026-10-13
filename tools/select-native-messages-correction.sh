#!/bin/bash
# Compiler-selected current artifacts; never writes the historical build tree.
set -euo pipefail
cd "$(dirname "$0")/.."
root=artifacts/native-messages/recovery-order-correction
out=$root/cargo-json
[[ ! -e "$out" ]] || { echo 'Preserve prior compiler selection' >&2; exit 2; }
mkdir -p "$out"
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
args=(--rm --pull=never --network none --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" -w /workspace)
for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
 for kind in bins lifecycle release projection; do
  target_root=$root/build
  manifest=native-messages/Cargo.toml
  extra=(--bins)
  case "$kind" in
   bins) extra+=(--features isolated-fixture);;
   lifecycle) manifest=native-lifecycle/Cargo.toml;;
   release) extra+=(--release);;
   projection) target_root=$root/components;extra=(--test projection --no-run);;
  esac
  command=build;[[ $kind != projection ]] || command=test
  "${docker_local[@]}" run "${args[@]}" -v "$PWD/$target_root:/target" pixel-phone-rust:20261012-b472 cargo "$command" --manifest-path "$manifest" --target "$target" --locked --offline --message-format=json "${extra[@]}" > "$out/$target-$kind.jsonl" 2> "$out/$target-$kind.stderr"
 done
done
