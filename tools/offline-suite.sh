#!/bin/bash
# Portable temporary validation environment; Harbour is only one possible Docker host.
set -euo pipefail
cd "$(dirname "$0")/.."
mode=${1:-all}
case "$mode" in all|build|provision|test) ;; *) echo 'usage: offline-suite.sh [all|build|provision|test]' >&2; exit 2;; esac
mkdir -p sim artifacts/logs
exec 9>sim/suite.lock
flock -n 9 || { echo 'Another suite owns this VM disk.' >&2; exit 1; }
builder=pixel-graphics-build:20261011-b472
simulator=pixel-graphics-sim:20261011-b472
resources=(--rm --cpus 4 --memory 6g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges)
mounts=(-v "$PWD:/workspace")
identity=(--user "$(id -u):$(id -g)")
if [[ "$mode" == all || "$mode" == build || "$mode" == provision ]]; then
  docker build -f build-support/Dockerfile -t "$builder" build-support > artifacts/logs/build-image.log 2>&1
  docker build -f build-support/Dockerfile.sim -t "$simulator" build-support > artifacts/logs/sim-image.log 2>&1
fi
if [[ "$mode" == all || "$mode" == build ]]; then
  docker run "${resources[@]}" --network none "${identity[@]}" "${mounts[@]}" "$builder" sh tools/cross-build.sh
  docker run "${resources[@]}" --network none "${identity[@]}" "${mounts[@]}" "$simulator" python3 tools/test-factory-boundary.py
fi
if [[ "$mode" == all || "$mode" == provision ]]; then
  if [[ ! -f sim/rootfs.img ]]; then
    docker run "${resources[@]}" "${identity[@]}" "${mounts[@]}" "$simulator" sh tools/download-sim.sh
    docker run --rm --network none --cpus 4 --memory 6g --pids-limit 512 \
      -e SIM_UID="$(id -u)" -e SIM_GID="$(id -g)" "${mounts[@]}" "$simulator" sh tools/prepare-sim.sh
  fi
  if [[ ! -f sim/provisioned.json ]]; then
    docker run "${resources[@]}" "${identity[@]}" "${mounts[@]}" "$simulator" python3 tools/sim.py provision
  fi
fi
if [[ "$mode" == all || "$mode" == test ]]; then
  test -f sim/provisioned.json || { echo 'Provision the VM first.' >&2; exit 1; }
  docker run "${resources[@]}" --network none "${identity[@]}" "${mounts[@]}" "$simulator" python3 tools/sim.py test
fi
