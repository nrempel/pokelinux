#!/bin/bash
# Owned stock emulator only; never host adb, physical phones or host power controls.
set -euo pipefail
cd "$(dirname "$0")/.."
phase=oracle
image=pixel-android-framework:20261011-b472
name=pokelinux-native-ims-reader-b472
mkdir -p android-framework/results "artifacts/ims-response-rea/runtime/$phase"
exec 9>android-framework/suite.lock
flock -n 9 || { echo 'Another framework suite owns this workspace' >&2;exit 1; }
if docker container inspect "$name" >/dev/null 2>&1 || docker network inspect "$name" >/dev/null 2>&1; then echo 'Owned receipt test resources already exist; inspect before cleanup' >&2;exit 1;fi
avd_lock=android-framework/home/avd/pixel-test.avd/hardware-qemu.ini.lock
if [ -f "$avd_lock" ]; then mv "$avd_lock" "$avd_lock.previous-$(date +%s)";fi
network_created=0
container_created=0
cleanup(){
 if [ "$container_created" = 1 ]; then
  docker logs "$name" > "artifacts/ims-response-rea/runtime/$phase/emulator.log" 2>&1 || true
  docker inspect "$name" > "artifacts/ims-response-rea/runtime/$phase/container.json" 2>/dev/null || true
  docker stop -t 10 "$name" >/dev/null || true
  docker rm "$name" >/dev/null || true
 fi
 if [ "$network_created" = 1 ]; then docker network rm "$name" >/dev/null || true;fi
}
trap cleanup EXIT
docker network create --internal "$name" >/dev/null
network_created=1
docker run -d --name "$name" --label org.pokelinux.test=native-ims-reader --network "$name" --sysctl net.ipv6.conf.all.disable_ipv6=1 --cpus 4 --memory 10g --pids-limit 1024 --cap-drop ALL --security-opt no-new-privileges --device /dev/kvm --group-add "$(stat -c %g /dev/kvm)" --user "$(id -u):$(id -g)" -v "$PWD:/workspace" "$image" bash tools/boot-framework.sh >/dev/null
container_created=1
docker exec "$name" bash tools/build-shannon-reader-oracle.sh
docker exec "$name" python3 framework/tests/shannon_reader.py > "artifacts/ims-response-rea/runtime/$phase/runner-output.txt" 2>&1
