#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
base=artifacts/native-messages-receive-isolation
simulator=sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6
docker_local=(env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock)
case "${1:?}" in
 clone)
  test ! -e "$base/vm/root.img"
  cp --reflink=auto --sparse=always /home/ubuntu/Projects/pokelinux/artifacts/native-messages-enumeration/vm/root.img "$base/vm/root.img.partial"
  "${docker_local[@]}" run --rm --name pokelinux-receive-isolation-sol-clone --network none --cpus 2 --memory 2g --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace:ro" -v "$PWD/$base/vm:/image" "$simulator" bash -ec 'debugfs -w -R "rm /usr/local/bin/pixel-sim-boot" /image/root.img.partial; debugfs -w -R "write /workspace/tools/native-messages-receive-isolation/boot.sh /usr/local/bin/pixel-sim-boot" /image/root.img.partial; debugfs -w -R "set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755" /image/root.img.partial'
  mv "$base/vm/root.img.partial" "$base/vm/root.img"
  ;;
 red|green|lock-red|lock-green|errors-green|enumeration-green)
  phase=$1;label=${2:?}
  "${docker_local[@]}" run --rm --name pokelinux-receive-isolation-sol-vm --network none --cpus 6 --memory 8g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace:ro" -v "$PWD/$base:/workspace/$base" -v /home/ubuntu/Projects/pokelinux/sim/boot:/kernel:ro -v /home/ubuntu/Projects/pokelinux/artifacts/runtime:/android-runtime:ro "$simulator" python3 /workspace/tools/native-messages-receive-isolation/vm.py "$phase" "$label"
  ;;
 *) exit 2;;
esac
