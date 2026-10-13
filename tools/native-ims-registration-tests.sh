#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
phase=${1:-green}
case "$phase" in red|green) ;; *) exit 2;; esac
out="artifacts/native-ims-registration/$phase"
test ! -e "$out" || { echo "Evidence already exists" >&2;exit 1; }
mkdir -p "$out" sim/rust-cargo
args=(--rm --init --network none --cpus 2 --memory 2g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges
 --user "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target
 -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/$out:/target" -w /workspace/native-ims)
image=pixel-phone-rust:20261012-b472
sha256sum native-ims/src/registration.rs native-ims/tests/registration.rs native-ims/tests/fixtures/shannon-registration.tsv > "$out/source-sha256.txt"
if [ "$phase" = red ]; then
 docker run "${args[@]}" "$image" cargo test --locked --offline --target x86_64-unknown-linux-gnu --test registration > "$out/test.log" 2>&1
else
 docker run "${args[@]}" "$image" cargo fmt --check
 docker run "${args[@]}" "$image" cargo clippy --locked --offline --all-targets -- -D warnings
 for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  docker run "${args[@]}" "$image" cargo test --locked --offline --target "$target"
  docker run "${args[@]}" "$image" cargo build --locked --offline --release --target "$target"
 done
fi
