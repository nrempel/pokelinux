#!/bin/bash
set -euo pipefail
if grep -q 'pixel.mode=provision' /proc/cmdline; then
    exec > >(tee /results/provision.log) 2>&1
    pacman-key --init
    pacman-key --populate archlinuxarm
    # Retain the verified boot kernel and its matching modules.
    pacman -Syu --needed --noconfirm --ignore linux-aarch64 \
        hyprland mesa libglvnd grim foot seatd wayland-utils \
        python python-evdev python-pillow gcc pkgconf wayland wayland-protocols \
        gtk3 python-gobject python-cairo libnotify quickshell qt6-tools inotify-tools jq
    quickshell --version > /results/quickshell-version.txt
    pacman -Q quickshell qt6-base qt6-declarative qt6-wayland > /results/qml-packages.txt
    pacman -Q > /results/packages.txt
    systemctl disable sshd.service || true
    usermod -a -G video,input alarm
    printf '{"provisioned":true}\n' > /results/provision.json
elif grep -q 'pixel.mode=radio-process' /proc/cmdline; then
  exec python3 /workspace/sim/guest/radio_process.py
elif grep -q 'pixel.mode=service-package' /proc/cmdline; then
  python3 /workspace/sim/guest/service_package.py
elif grep -q 'pixel.mode=omarchy-shell' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py
elif grep -q 'pixel.mode=phone-recovery' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py --phone-plugin --phone-recovery
elif grep -q 'pixel.mode=phone-upgrade' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py --phone-plugin --phone-upgrade
elif grep -q 'pixel.mode=phone-broken' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py --phone-plugin --phone-broken
elif grep -q 'pixel.mode=phone-lifecycle' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py --phone-plugin --phone-lifecycle
elif grep -q 'pixel.mode=phone-plugin-android' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py --phone-plugin --android
elif grep -q 'pixel.mode=phone-plugin' /proc/cmdline; then
    exec python3 /workspace/sim/guest/omarchy_shell.py --phone-plugin
elif grep -q 'pixel.mode=qml-card' /proc/cmdline; then
  python3 /workspace/sim/guest/qml_card.py
elif grep -q 'pixel.mode=qml-service' /proc/cmdline; then
  python3 /workspace/sim/guest/qml_service.py
elif grep -q 'pixel.mode=qml-process' /proc/cmdline; then
  python3 /workspace/sim/guest/qml_process.py
elif grep -q 'pixel.mode=qml-check' /proc/cmdline; then
    exec python3 /workspace/sim/guest/qml_check.py
elif grep -q 'pixel.mode=phone' /proc/cmdline; then
    exec > >(tee /results/guest.log) 2>&1
    exec python3 /workspace/sim/guest/phone_guest.py
elif grep -q 'pixel.mode=transport' /proc/cmdline; then
    exec > >(tee /results/guest.log) 2>&1
    exec python3 /workspace/sim/guest/transport.py
else
    exec > >(tee /results/guest.log) 2>&1
    exec python3 /workspace/sim/guest/test.py
fi
