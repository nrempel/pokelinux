#!/bin/sh
# Offline pacman installation and native Qt validation in disposable Arch ARM.
set -eu
if [ "$#" -ne 5 ]; then
  echo 'usage: arch-package-suite.sh ABS_PACKAGE ABS_ENROLLMENT ABS_SEED ABS_TLS_PEER ABS_ACTIVATOR' >&2
  exit 2
fi
for input in "$@"; do
  case "$input" in /*) ;; *) echo 'absolute fixture paths required' >&2; exit 2 ;; esac
done
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
results="$root/artifacts/rust-native-local"
printf '{"passed":false,"scope":"Arch ARM package and native Qt integration"}\n' > "$results/arch-package-run.json"
docker run --rm --network none --cpus 3 --memory 3g --pids-limit 256 \
  --cap-drop ALL --cap-add SETUID --cap-add SETGID --cap-add CHOWN --security-opt no-new-privileges \
  -e FIXTURE_UID="$(id -u)" -e FIXTURE_GID="$(id -g)" \
  -e PHONE_TEST_BINARIES=/usr/bin -e PHONE_TEST_APP=/usr/bin/omarchy-phone-app \
  -e PHONE_TEST_SERVICES=/usr/share/dbus-1/services \
  -e PHONE_TEST_ENROLLMENT=/fixture/enrollment -e PHONE_TEST_SEED=/fixture/seed_pending \
  -e PHONE_TEST_PEER=/fixture/qt_android_peer -e PHONE_TEST_ACTIVATOR=/fixture/activate_phone \
  -v "$root:/workspace:ro" -v "$1:/package.tar:ro" \
  -v "$2:/fixture/enrollment:ro" -v "$3:/fixture/seed_pending:ro" \
  -v "$4:/fixture/qt_android_peer:ro" -v "$5:/fixture/activate_phone:ro" \
  -v "$results:/results" pixel-phone-arch-native:20261012 /bin/bash -ec '
    if ! getent group "$FIXTURE_GID" > /dev/null; then groupadd --gid "$FIXTURE_GID" phone-validation; fi
    if ! getent passwd "$FIXTURE_UID" > /dev/null; then useradd --uid "$FIXTURE_UID" --gid "$FIXTURE_GID" --no-create-home --home-dir /tmp/phone-arch-home --shell /usr/bin/nologin phone-validation; fi
    if ! pacman -U --disable-sandbox-filesystem --noconfirm /package.tar > /tmp/pacman-install.log 2>&1; then cat /tmp/pacman-install.log; exit 1; fi
    if ! pacman -Qkk omarchy-phone > /tmp/pacman-check.log 2>&1; then cat /tmp/pacman-check.log; exit 1; fi
    exec setpriv --reuid="$FIXTURE_UID" --regid="$FIXTURE_GID" --clear-groups /bin/bash -ec '\''
      export HOME=/tmp/phone-arch-home XDG_CACHE_HOME=/tmp/phone-arch-cache
      mkdir -p "$HOME" "$XDG_CACHE_HOME"
      cp /tmp/pacman-install.log /results/arch-package-install.log
      cp /tmp/pacman-check.log /results/arch-package-check.log
      pacman -Q > /results/arch-package-versions.txt
      sha256sum /usr/bin/omarchy-phone /usr/bin/omarchy-phoned /usr/bin/omarchy-phone-app \
        /usr/share/dbus-1/services/org.omarchy.Phone.service > /results/arch-package-installed.sha256
      ldd /usr/bin/omarchy-phone /usr/bin/omarchy-phoned /usr/bin/omarchy-phone-app > /results/arch-package-libraries.log
      if grep -q "not found" /results/arch-package-libraries.log; then exit 1; fi
      cmake -G Ninja -S /workspace/native-app -B /tmp/build -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr > /results/arch-package-build.log 2>&1
      cmake --build /tmp/build -j3 >> /results/arch-package-build.log 2>&1
      ctest --test-dir /tmp/build -V > /results/arch-package-tests.log 2>&1
    '\''
  '
printf '{"passed":true,"scope":"Arch ARM package and native Qt integration","pacman_verified":true,"omarchy_session_verified":false,"android_verified":false,"phone_accessed":false}\n' > "$results/arch-package-run.json"
