#!/bin/sh
# Local isolated Qt/QML validation. No Android or physical-device connection.
set -eu
if [ "$#" -ne 5 ]; then
  echo 'usage: qt-native-suite.sh ABSOLUTE_BINARY_DIR ABSOLUTE_ENROLLMENT_DIR ABSOLUTE_SEED_EXECUTABLE ABSOLUTE_TLS_PEER_EXECUTABLE ABSOLUTE_ACTIVATOR_EXECUTABLE' >&2
  exit 2
fi
for input in "$@"; do
  case "$input" in /*) ;; *) echo 'absolute fixture paths required' >&2; exit 2 ;; esac
done
binary_dir=$1
test -x "$binary_dir/omarchy-phone"
test -x "$binary_dir/omarchy-phoned"
enrollment_dir=$2
seed_path=$3
peer_path=$4
activator_path=$5
test -x "$seed_path"
test -x "$peer_path"
test -x "$activator_path"
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
results="$root/artifacts/rust-native-local"
mkdir -p "$results"
printf '{"passed":false,"scope":"native Qt/QML isolated suite"}\n' > "$results/qt-run.json"
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 \
  --user "$(id -u):$(id -g)" --cap-drop ALL --security-opt no-new-privileges \
  -e PHONE_TEST_ACTIVATOR=/fixture/activate_phone \
  -e PHONE_TEST_PEER=/fixture/qt_android_peer \
  -e PHONE_TEST_SEED=/fixture/seed_pending \
  -e PHONE_TEST_BINARIES=/fixture/bin \
  -e PHONE_TEST_ENROLLMENT=/fixture/enrollment \
  -v "$root:/workspace:ro" -v "$binary_dir:/fixture/bin:ro" \
  -v "$enrollment_dir:/fixture/enrollment:ro" -v "$seed_path:/fixture/seed_pending:ro" \
  -v "$peer_path:/fixture/qt_android_peer:ro" \
  -v "$activator_path:/fixture/activate_phone:ro" \
  -v "$results:/results" pixel-phone-qt-native:20261011 sh -ec '
    cmake -S /workspace/native-app -B /tmp/build -G Ninja -DCMAKE_INSTALL_PREFIX=/tmp/stage > /results/qt-configure.log 2>&1
    cmake --build /tmp/build -j3 > /results/qt-build.log 2>&1
    cmake --install /tmp/build > /results/qt-install.log 2>&1
    cp /fixture/bin/omarchy-phone /tmp/stage/bin/omarchy-phone
    export PHONE_TEST_APP=/tmp/stage/bin/omarchy-phone-app
    export PHONE_TEST_SERVICES=/tmp/stage/share/dbus-1/services
    ctest --test-dir /tmp/build -V > /results/qt-tests.log 2>&1
    sha256sum /tmp/stage/bin/omarchy-phone-app /tmp/stage/share/applications/org.omarchy.Phone.desktop /tmp/stage/share/dbus-1/services/org.omarchy.Phone.service > /results/qt-installed-app.sha256
    mkdir -p /results/native-app
    cp /tmp/stage/bin/omarchy-phone-app /tmp/stage/share/applications/org.omarchy.Phone.desktop /tmp/stage/share/dbus-1/services/org.omarchy.Phone.service /results/native-app/
    sha256sum /fixture/bin/omarchy-phone /fixture/bin/omarchy-phoned /tmp/stage/bin/omarchy-phone-app /fixture/seed_pending /fixture/qt_android_peer /fixture/activate_phone > /results/qt-tested-binaries.sha256
  '
printf '{"passed":true,"scope":"native Qt/QML isolated suite","android_verified":false,"phone_accessed":false}\n' > "$results/qt-run.json"
