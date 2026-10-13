#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
base=artifacts/native-messages-enumeration
frozen=artifacts/native-messages-incoming-binder/selections/incoming-green-final-v2
mkdir -p "$base/target" "$base/runtime" "$base/vm"
exec 9>"$base/task.lock";flock -n 9
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
builder=sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6
simulator=sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6
limits=(--rm --network none --cpus 2 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)")
case "${1:-red}" in
 component-build)
  "${docker_local[@]}" run "${limits[@]}" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/$base/target:/target" -v "$PWD/$frozen/sources/native-lifecycle:/workspace/native-lifecycle:ro" -v "$PWD/tools/native-messages-enumeration/fixture:/workspace/tools/native-messages-enumeration/fixture" -w /workspace/tools/native-messages-enumeration/fixture "$builder" bash -ec 'cargo generate-lockfile --offline;cargo fmt;cargo build --locked --offline --target x86_64-unknown-linux-gnu'
  ;;
 green-build)
  "${docker_local[@]}" run "${limits[@]}" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/$base/target:/target" -v "$PWD/$frozen/sources/native-lifecycle:/workspace/native-lifecycle:ro" -v "$PWD/$frozen/sources/tools/native-messages-incoming-binder/fixture:/workspace/tools/native-messages-incoming-binder/fixture:ro" -v "$PWD/tools/native-messages-enumeration/fixture:/workspace/tools/native-messages-enumeration/fixture" -w /workspace/tools/native-messages-enumeration/fixture "$builder" bash -ec 'cargo fmt;cargo clippy --locked --offline --all-targets --target aarch64-unknown-linux-gnu -- -D warnings;cargo build --locked --offline --target aarch64-unknown-linux-gnu;cargo build --manifest-path /workspace/tools/native-messages-incoming-binder/fixture/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --bin messages-incoming-fixture-service;cargo build --manifest-path /workspace/native-messages/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --bins;cargo test --manifest-path /workspace/native-messaging/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --test receipt --no-run --message-format json > /target/receipt-arm-tests.jsonl;cargo test --manifest-path /workspace/native-messages/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --test projection --no-run --message-format json > /target/projection-arm-tests.jsonl'
  ;;
 component-red)
  label=${2:?};mkdir "$base/runtime/$label"
  "${docker_local[@]}" run "${limits[@]}" --tmpfs /tmp:rw,nosuid,nodev -v "$PWD/$base/target/x86_64-unknown-linux-gnu/debug:/bins:ro" -v "$PWD/$base/runtime/$label:/evidence" "$builder" /bins/enumeration-component /tmp/enum-component /evidence/result.json > "$base/runtime/$label/log" 2>&1
  ;;
 prepare-red)
  python3 - "$frozen" "$base" <<'PYINIT'
import hashlib,json,shutil,sys
from pathlib import Path
frozen,base=map(Path,sys.argv[1:]);manifest=json.loads((frozen/'selection.json').read_text())
for name,value in manifest['binaries'].items():
    source=frozen/'binaries'/Path(name).name;assert hashlib.sha256(source.read_bytes()).hexdigest()==value
    target=base/name;assert not target.exists();target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target);target.chmod(0o755)
PYINIT
  test ! -e "$base/vm/root.img";cp --reflink=auto --sparse=always artifacts/native-messages-incoming-binder/vm/root.img "$base/vm/root.img.partial"
  "${docker_local[@]}" run "${limits[@]}" -v "$PWD:/workspace:ro" -v "$PWD/$base:/test" "$simulator" bash -ec 'debugfs -w -R "rm /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial;debugfs -w -R "write /workspace/tools/native-messages-enumeration/boot.sh /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial;debugfs -w -R "set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755" /test/vm/root.img.partial'
  mv "$base/vm/root.img.partial" "$base/vm/root.img"
  ;;
 red|green)
  label=${2:?}
  "${docker_local[@]}" run "${limits[@]}" --name pokelinux-messages-enumeration-test -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" "$simulator" python3 /workspace/tools/native-messages-enumeration/vm.py "$1" "$label"
  ;;
 *) exit 2;;
esac
