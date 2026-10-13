#!/bin/bash
# Dedicated offline fixture. No host Binder/adb/USB or physical phone access.
set -euo pipefail
cd "$(dirname "$0")/../.."
base=artifacts/native-messages-incoming-binder
mkdir -p "$base/target" "$base/vm" "$base/runtime"
exec 9>"$base/task.lock";flock -n 9
builder=sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6
simulator=sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
limits=(--rm --network none --cpus 2 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)")
case "${1:-smoke}" in
 build)
  python3 tools/native-messages-incoming-binder/verify-inputs.py
  "${docker_local[@]}" run "${limits[@]}" -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target     -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo:ro" -v "$PWD/$base/target:/target"     -v "$PWD/native-kernel-identity:/workspace/native-kernel-identity" -v "$PWD/native-messages:/workspace/native-messages" -v "$PWD/native-lifecycle:/workspace/native-lifecycle" -v "$PWD/tools/native-messages-incoming-binder/fixture:/workspace/tools/native-messages-incoming-binder/fixture"     -w /workspace/tools/native-messages-incoming-binder/fixture "$builder" bash -ec     'cargo generate-lockfile --offline; cargo generate-lockfile --manifest-path /workspace/native-messages/Cargo.toml --offline; cargo generate-lockfile --manifest-path /workspace/native-kernel-identity/Cargo.toml --offline; cargo fmt --manifest-path /workspace/native-kernel-identity/Cargo.toml; cargo fmt --manifest-path /workspace/native-messages/Cargo.toml; cargo fmt --manifest-path /workspace/native-lifecycle/Cargo.toml; cargo fmt; cargo clippy --all-targets --locked --offline --target aarch64-unknown-linux-gnu -- -D warnings; cargo build --locked --offline --target aarch64-unknown-linux-gnu; cargo build --manifest-path /workspace/native-lifecycle/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --bins; cargo build --manifest-path /workspace/native-messages/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --bins; cargo clippy --manifest-path /workspace/native-kernel-identity/Cargo.toml --locked --offline --all-targets --target aarch64-unknown-linux-gnu -- -D warnings; cargo test --manifest-path /workspace/native-kernel-identity/Cargo.toml --locked --offline --target aarch64-unknown-linux-gnu --no-run --message-format json > /target/kernel-boundary-arm-build.jsonl'
  "${docker_local[@]}" run "${limits[@]}" -v "$PWD:/workspace:ro" -v "$PWD/$base:/test" "$builder" bash /workspace/tools/native-messages-incoming-binder/build-ndk.sh
  python3 - <<'PYTEST'
import hashlib,json
from pathlib import Path
base=Path('artifacts/native-messages-incoming-binder');rows=[]
for line in (base/'target/kernel-boundary-arm-build.jsonl').read_text().splitlines():
    value=json.loads(line)
    if value.get('reason')=='compiler-artifact' and value.get('executable') and value['profile']['test']:
        relative=Path('target')/Path(value['executable']).relative_to('/target');path=base/relative
        rows.append({'name':value['target']['name'],'path':str(relative),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
assert len(rows)==2
(base/'kernel-test-artifacts.json').write_text(json.dumps(rows,indent=2)+'\n')
PYTEST
  ;;
 clone)
  test ! -e "$base/vm/root.img";test ! -e "$base/vm/root.img.partial"
  echo '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b  sim/boot/Image' | sha256sum -c -
  echo 'a9bb29d29abda94748c9948f903c3c1704f829eea477652b9a4201a61f36a647  sim/boot/initramfs-linux.img' | sha256sum -c -
  cp --reflink=auto --sparse=always sim/rootfs.img "$base/vm/root.img.partial"
  "${docker_local[@]}" run "${limits[@]}" -v "$PWD:/workspace:ro" -v "$PWD/$base:/test" "$simulator" bash -ec '
    debugfs -w -R "rm /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial
    debugfs -w -R "write /workspace/tools/native-messages-incoming-binder/boot.sh /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial
    debugfs -w -R "set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755" /test/vm/root.img.partial
    debugfs -R "cat /usr/local/bin/pixel-sim-boot" /test/vm/root.img.partial > /test/vm/installed-boot.sh
    cmp /test/vm/installed-boot.sh /workspace/tools/native-messages-incoming-binder/boot.sh'
  mv "$base/vm/root.img.partial" "$base/vm/root.img"
  ;;
 smoke|incoming-red|incoming-green|incoming-systemd|incoming-stop-red|incoming-stop-green|incoming-systemd-stop-red|incoming-systemd-stop-green)
  phase=${1:-smoke};label=${2:-"$phase-$(date -u +%Y%m%dT%H%M%SZ)"}
  "${docker_local[@]}" run "${limits[@]}" --name pokelinux-messages-incoming-binder-test     -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" "$simulator"     python3 /workspace/tools/native-messages-incoming-binder/vm.py "$phase" "$label"
  ;;
 *) echo 'Usage: run.sh build|clone|smoke|incoming-green|incoming-systemd|incoming-stop-green|incoming-systemd-stop-green [unique-label]; strict red phases require preserved pre-fix binaries' >&2;exit 2;;
esac
