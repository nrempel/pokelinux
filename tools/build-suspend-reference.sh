#!/bin/bash
# Executes unmodified public Android source. Never runs a retained vendor binary.
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-build}" in
 fetch) exec python3 tools/fetch-suspend-reference.py ;;
 build)
  python3 tools/fetch-suspend-reference.py --verify-only
  python3 tools/suspend-reference/prepare-shims.py
  mkdir -p artifacts/native-suspend/reference
  docker run --rm --init --network none --memory 1g --cpus 1 --pids-limit 128 --cap-drop ALL --security-opt no-new-privileges \
   --user "$(id -u):$(id -g)" --mount "type=bind,src=$PWD/artifacts/native-suspend,dst=/results" \
   --mount "type=bind,src=$PWD/tools/suspend-reference,dst=/harness,readonly" \
   --workdir /results --entrypoint sh pixel-graphics-build:20261011-b472 \
   -c 'g++ -std=c++17 -O0 -g0 -pthread -I reference/shims -I public -I /harness -include /harness/platform.h /harness/oracle.cpp /harness/support.cpp public/SystemSuspend.cpp public/SystemSuspendAidl.cpp -o reference/oracle && : > reference/reference.tsv && : > reference/source-stderr.log && for scenario in ordering idempotent-release destructor-release concurrent-acquire count-reject state-write-failure dead-controller dead-client; do timeout 10 reference/oracle "$scenario" "/results/reference/$scenario-work" >> reference/reference.tsv 2>> reference/source-stderr.log || exit $?; done'
  python3 tools/verify-suspend-reference.py
  ;;
 *) echo 'Usage: build-suspend-reference.sh fetch|build' >&2; exit 2 ;;
esac
