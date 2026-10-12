#!/bin/bash
# Run from the package root on a Linux KVM host. No phone access or host adb.
set -euo pipefail
cd "$(dirname "$0")/.."
action=${1:-test}
image=pixel-android-framework:20261011-b472
name=pixel-framework-b472
mkdir -p android-framework/results
exec 9>android-framework/suite.lock
flock -n 9 || { echo 'Another framework suite owns this workspace' >&2; exit 1; }
start_transport_vm(){
  if docker container inspect pixel-transport-vm-b472 >/dev/null 2>&1; then echo 'Transport VM already exists' >&2; exit 1; fi
  mkdir -p android-framework/transport-vm
  rm -f android-framework/transport-vm/{ready,request,response}.json
  docker run -d --name pixel-transport-vm-b472 --label org.omarchy.test=transport --network "container:$name" --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" pixel-graphics-sim:20261011-b472 python3 tools/transport-vm.py >/dev/null
  vm_created=1
  docker inspect pixel-transport-vm-b472 > android-framework/results/transport-container.json
}
case "$action" in
 prepare)
  python3 tools/download-audit-tools.py
  python3 tools/prepare-framework.py
  ;;
 build)
  if ! docker image inspect pixel-graphics-build:20261011-b472 >/dev/null 2>&1; then
   docker build -t pixel-graphics-build:20261011-b472 -f build-support/Dockerfile build-support
  fi
  if ! docker image inspect pixel-hardware-audit:20261011-b472 >/dev/null 2>&1; then
   docker build -t pixel-hardware-audit:20261011-b472 -f build-support/Dockerfile.audit build-support
  fi
  docker build -t "$image" -f build-support/Dockerfile.framework build-support
  docker run --rm --network none --cpus 4 --memory 6g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" "$image" bash tools/build-framework.sh
  ;;
 test|enrollment|callui|gui|plugin|direct|journal|radio-baseline|rust|shannon-codec|sms-codec|binder-kernel|radio-ipc)
  if [ "$action" = rust ]; then bash tools/build-qt-android-tests.sh; fi
  if docker container inspect "$name" >/dev/null 2>&1 || docker network inspect "$name" >/dev/null 2>&1; then
   echo 'The dedicated test container/network already exists; inspect its owner before cleanup.' >&2;exit 1
  fi
  # The workspace flock and absent dedicated container establish exclusive ownership.
  # A killed emulator can leave this PID lock referring to a reused container PID.
  avd_lock=android-framework/home/avd/pixel-test.avd/hardware-qemu.ini.lock
  if [ -f "$avd_lock" ]; then mv "$avd_lock" "$avd_lock.previous-$(date +%s)"; fi
  network_created=0
  container_created=0
  vm_created=0
  gui_created=0
  cleanup(){
   if [ "$gui_created" = 1 ]; then docker stop -t 10 pixel-phone-vm-b472 >/dev/null || true; docker rm pixel-phone-vm-b472 >/dev/null || true; fi
   if [ "$vm_created" = 1 ]; then docker stop -t 10 pixel-transport-vm-b472 >/dev/null || true; docker rm pixel-transport-vm-b472 >/dev/null || true; fi
   if [ "$container_created" = 1 ]; then
    docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 logcat -d -s OmarchyBridge:V AndroidRuntime:E > android-framework/results/app-runtime.log 2>&1 || true
    docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 shell dumpsys activity services org.omarchy.bridge > android-framework/results/app-services.txt 2>&1 || true
    docker inspect "$name" > android-framework/results/container-final.json 2>/dev/null || true
    docker logs "$name" > android-framework/results/emulator.log 2>&1 || true
    docker stop -t 10 "$name" >/dev/null || true
    docker rm "$name" >/dev/null || true
   fi
   if [ "$network_created" = 1 ]; then docker network rm "$name" >/dev/null || true; fi
  }
  trap cleanup EXIT
  docker network create --internal "$name" >/dev/null
  network_created=1
  docker run -d --name "$name" --label org.omarchy.test=framework --network "$name" --sysctl net.ipv6.conf.all.disable_ipv6=1 --cpus 4 --memory 10g --pids-limit 1024 --cap-drop ALL --security-opt no-new-privileges --device /dev/kvm --group-add "$(stat -c %g /dev/kvm)" --user "$(id -u):$(id -g)" -v "$PWD:/workspace" "$image" bash tools/boot-framework.sh >/dev/null
  container_created=1
  docker inspect "$name" > android-framework/results/container.json
  docker network inspect "$name" > android-framework/results/network.json
  if [ "$action" = sms-codec ]; then
   docker exec "$name" bash tools/build-shannon-oracle.sh
   docker exec "$name" python3 framework/tests/sms_codec.py
   exit 0
  fi
  if [ "$action" = radio-ipc ]; then
   docker exec "$name" bash tools/build-radio-peer.sh
   docker exec "$name" bash tools/build-shannon-oracle.sh
   docker exec "$name" python3 framework/tests/radio_ipc.py
   exit 0
  fi
  if [ "$action" = binder-kernel ]; then
   docker exec "$name" python3 framework/tests/binder_kernel.py
   exit 0
  fi
  if [ "$action" = shannon-codec ]; then
   docker exec "$name" bash tools/build-shannon-oracle.sh
   docker exec "$name" python3 framework/tests/shannon_oracle.py
   exit 0
  fi
  if [ "$action" = radio-baseline ]; then
   docker exec "$name" python3 framework/tests/radio_baseline.py
   exit 0
  fi
  if [ "$action" = journal ]; then
   docker exec "$name" python3 framework/tests/dial_journal.py
   exit 0
  fi
  if [ "$action" = callui ]; then
   docker exec "$name" python3 framework/tests/callui.py
   exit 0
  fi
  if [ "$action" = rust ]; then
   mkdir -p artifacts/rust-android
   printf '%s\n' '{"passed":false,"phone_accessed":false}' > artifacts/rust-android/result.json
   docker exec "$name" python3 framework/tests/enrollment.py
   docker exec "$name" python3 framework/tests/ui.py
   docker exec -e RUST_TEST_UID="$(id -u)" -e RUST_TEST_GID="$(id -g)" "$name" python3 framework/tests/rust_setup.py
   for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
    docker run --rm --network "container:$name" --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
     -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e PHONE_EMULATOR_ENROLLMENT="$(cat android-framework/rust-enrollment-path.txt)" \
     -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/sim/rust-target:/target" \
     pixel-phone-rust:20261012-b472 cargo test --locked --offline --target "$target" --test android_emulator -- --ignored \
     > "artifacts/rust-android/$target.log" 2>&1
    for expected in 1 0; do
     radio_action=call
     if [ "$expected" = 0 ]; then radio_action=cancel; fi
     # This command addresses the emulator console, never a carrier number.
     docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 emu gsm "$radio_action" 5550100
     docker run --rm --network "container:$name" --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
      -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e PHONE_EMULATOR_ENROLLMENT="$(cat android-framework/rust-enrollment-path.txt)" -e PHONE_EXPECT_RINGING="$expected" \
      -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/sim/rust-target:/target" \
      pixel-phone-rust:20261012-b472 cargo test --locked --offline --target "$target" --test android_ringing -- --ignored \
      > "artifacts/rust-android/$target-radio-$expected.log" 2>&1
    done
    docker exec "$name" python3 framework/tests/rust_controls_setup.py enable
    docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 emu gsm call 5550100
    docker run --rm --network "container:$name" --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
     -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e PHONE_EMULATOR_ENROLLMENT="$(cat android-framework/rust-enrollment-path.txt)" \
     -v "$PWD:/workspace:ro" -v "$PWD/sim/rust-cargo:/cargo" -v "$PWD/sim/rust-target:/target" \
     pixel-phone-rust:20261012-b472 cargo test --locked --offline --target "$target" --test android_controls -- --ignored \
     > "artifacts/rust-android/$target-controls.log" 2>&1
    docker exec "$name" python3 framework/tests/rust_controls_setup.py disable
   done
   docker exec "$name" python3 framework/tests/rust_controls_setup.py enable
   docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 emu gsm call 5550100
   mkdir -p artifacts/rust-android/qt-results
   docker run --rm --network "container:$name" --cpus 3 --memory 3g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
    -e PHONE_EMULATOR_ENROLLMENT="$(cat android-framework/rust-enrollment-path.txt)" \
    -e PHONE_TEST_BINARIES=/workspace/sim/rust-target/x86_64-unknown-linux-gnu/debug \
    -e QT_QPA_PLATFORM=offscreen -e QT_QUICK_BACKEND=software -e QT_QUICK_CONTROLS_STYLE=Basic \
    -v "$PWD:/workspace:ro" -v "$PWD/artifacts/rust-android/qt-results:/results" \
    pixel-phone-qt-native:20261012-b472 timeout 60 /workspace/sim/native-qt-build/test-phone-android-view \
    > artifacts/rust-android/qt-android-test.log 2>&1
   docker exec "$name" python3 framework/tests/rust_controls_setup.py disable
   docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 shell getenforce > artifacts/rust-android/selinux.txt
   (cd runtime && sha256sum Cargo.toml Cargo.lock src/*.rs src/bin/*.rs tests/*.rs) > artifacts/rust-android/sources.sha256
   printf '%s\n' '{"passed":true,"scope":"Rust Android framework observations, recovery, radio watch and accepted Answer/End with Telecom callback verification","native_linux":true,"arm64_qemu_user":true,"adb_forwarded":true,"phone_accessed":false,"accepted_answer_end_tested":true,"emulator_incoming_and_removal":true,"carrier_ims_audio_verified":false}' > artifacts/rust-android/result.json
   exit 0
  fi
  if [ "$action" = direct ]; then
   docker exec "$name" python3 framework/tests/enrollment.py
   docker exec "$name" python3 framework/tests/ui.py
   docker exec "$name" python3 framework/tests/direct_transport.py
   docker exec "$name" python3 framework/tests/lifecycle.py
   docker exec "$name" python3 framework/tests/release_provenance.py direct
   exit 0
  fi
  if [ "$action" = gui ] || [ "$action" = plugin ]; then
   docker exec "$name" python3 framework/tests/enrollment.py
   docker exec "$name" python3 framework/tests/ui.py
   if docker container inspect pixel-phone-vm-b472 >/dev/null 2>&1; then echo 'Phone VM already exists' >&2; exit 1; fi
   mkdir -p android-framework/phone-vm
   rm -f android-framework/phone-vm/{ready,request,response,result,gui-ready,gui-request,gui-response,active,active-verified,outgoing,outgoing-verified}.json
   guest_args=()
   if [ "$action" = plugin ]; then guest_args+=(--plugin); fi
   docker run -d --name pixel-phone-vm-b472 --label org.omarchy.test=phone-ui --network "container:$name" --cpus 4 --memory 6g --pids-limit 512 --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" -v "$PWD:/workspace" pixel-graphics-sim:20261011-b472 python3 tools/phone-vm.py "${guest_args[@]}" >/dev/null
   gui_created=1
   docker inspect pixel-phone-vm-b472 > android-framework/results/phone-container.json
   if [ "$action" = plugin ]; then
    docker exec "$name" python3 framework/tests/plugin_bridge.py
   else
    docker exec "$name" python3 framework/tests/gui_bridge.py
   fi
   docker exec "$name" python3 framework/tests/release_provenance.py "$action"
   docker wait pixel-phone-vm-b472 | grep -qx 0
   python3 -c 'import json; assert json.load(open("android-framework/phone-vm/result.json"))["passed"]'
   exit 0
  fi
  if [ "$action" = enrollment ]; then
   docker exec "$name" python3 framework/tests/enrollment.py
   docker exec "$name" python3 framework/tests/ui.py
   docker exec "$name" python3 framework/tests/callui.py
   start_transport_vm
   docker exec "$name" python3 framework/tests/native_enrollment.py
   docker exec "$name" python3 framework/tests/release_provenance.py enrollment
   docker wait pixel-transport-vm-b472 | grep -qx 0
   docker rm pixel-transport-vm-b472 >/dev/null
   vm_created=0
   exit 0
  fi
  docker exec "$name" python3 framework/tests/suite.py
  docker exec "$name" python3 framework/tests/telecom.py
  start_transport_vm
  docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 install --no-incremental -r android-framework/apks/bridge-legacy-fixture.apk
  docker exec "$name" python3 framework/tests/transport.py
  docker wait pixel-transport-vm-b472 | grep -qx 0
  docker rm pixel-transport-vm-b472 >/dev/null
  vm_created=0
  docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 install --no-incremental android-framework/apks/ims.apk
  docker exec "$name" python3 framework/tests/ims.py
  docker exec "$name" python3 framework/tests/ims_negative.py
  docker exec "$name" python3 framework/tests/stock_activation.py
  docker exec "$name" android-framework/sdk/platform-tools/adb -s emulator-5554 install --no-incremental -r android-framework/apks/bridge.apk
  docker exec "$name" python3 framework/tests/provenance.py
  python3 tools/verify-transport.py
  python3 tools/verify-framework.py
  ;;
 *) echo 'Usage: tools/framework-suite.sh prepare|build|test' >&2;exit 2;;
esac
