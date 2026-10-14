#!/bin/bash
set -euo pipefail
qemu-img create -f qcow2 -F raw -b /vm/root.img /tmp/auth-scope-overlay.qcow2
exec timeout --signal=TERM --kill-after=5s 110s qemu-system-aarch64 \
  -machine virt,gic-version=3 -cpu max -accel tcg,thread=multi -smp 2 -m 1024 \
  -kernel /boot-inputs/Image -initrd /boot-inputs/initramfs-linux.img \
  -append 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service systemd.mask=display-manager.service systemd.mask=user@1000.service' \
  -drive file=/tmp/auth-scope-overlay.qcow2,if=none,id=root,format=qcow2 \
  -device virtio-blk-pci,drive=root -device virtio-rng-pci -nic none \
  -fsdev local,id=project,path=/workspace,security_model=none,readonly=on \
  -device virtio-9p-pci,fsdev=project,mount_tag=project \
  -fsdev local,id=results,path=/results,security_model=mapped-xattr \
  -device virtio-9p-pci,fsdev=results,mount_tag=results \
  -nographic -monitor none -no-reboot
