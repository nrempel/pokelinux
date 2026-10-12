#!/bin/sh
# Validate trusted locally built package members at /usr in a disposable image.
# No host installation, systemd activation, Android or physical-device access.
set -eu
if [ "$#" -ne 6 ]; then
  echo 'usage: qt-package-suite.sh ABS_PACKAGE ABS_QT_BUILD ABS_ENROLLMENT ABS_SEED ABS_TLS_PEER ABS_ACTIVATOR' >&2
  exit 2
fi
for input in "$@"; do
  case "$input" in /*) ;; *) echo 'absolute fixture paths required' >&2; exit 2 ;; esac
done
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
results="$root/artifacts/rust-native-local"
mkdir -p "$results"
printf '{"passed":false,"scope":"package extraction into isolated Linux image"}\n' > "$results/qt-package-run.json"
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 \
  --cap-drop ALL --cap-add SETUID --cap-add SETGID --security-opt no-new-privileges \
  -e FIXTURE_UID="$(id -u)" -e FIXTURE_GID="$(id -g)" \
  -e PHONE_TEST_BINARIES=/usr/bin -e PHONE_TEST_APP=/usr/bin/omarchy-phone-app \
  -e PHONE_TEST_SERVICES=/usr/share/dbus-1/services \
  -e PHONE_TEST_ENROLLMENT=/fixture/enrollment -e PHONE_TEST_SEED=/fixture/seed_pending \
  -e PHONE_TEST_PEER=/fixture/qt_android_peer -e PHONE_TEST_ACTIVATOR=/fixture/activate_phone \
  -v "$root:/workspace:ro" -v "$1:/package.tar:ro" -v "$2:/build" \
  -v "$3:/fixture/enrollment:ro" -v "$4:/fixture/seed_pending:ro" \
  -v "$5:/fixture/qt_android_peer:ro" -v "$6:/fixture/activate_phone:ro" \
  -v "$results:/results" pixel-phone-qt-native:20261011 sh -ec '
    tar --no-same-owner -xf /package.tar -C / \
      usr/bin/omarchy-phone usr/bin/omarchy-phoned usr/bin/omarchy-phone-app \
      usr/share/applications/org.omarchy.Phone.desktop \
      usr/share/dbus-1/services/org.omarchy.Phone.service \
      usr/lib/systemd/user/omarchy-phone.service
    exec setpriv --reuid="$FIXTURE_UID" --regid="$FIXTURE_GID" --clear-groups sh -ec '\''
      export HOME=/tmp/phone-package-home XDG_CACHE_HOME=/tmp/phone-package-cache
      mkdir -p "$HOME" "$XDG_CACHE_HOME"
      sha256sum /usr/bin/omarchy-phone /usr/bin/omarchy-phoned /usr/bin/omarchy-phone-app \
        /usr/share/dbus-1/services/org.omarchy.Phone.service > /results/qt-package-installed.sha256
      ldd /usr/bin/omarchy-phone /usr/bin/omarchy-phoned /usr/bin/omarchy-phone-app > /results/qt-package-libraries.log
      if grep -q "not found" /results/qt-package-libraries.log; then exit 1; fi
      ctest --test-dir /build -V > /results/qt-package-tests.log 2>&1
    '\''
  '
printf '{"passed":true,"scope":"package extraction into isolated Linux image","pacman_verified":false,"android_verified":false,"phone_accessed":false}\n' > "$results/qt-package-run.json"
