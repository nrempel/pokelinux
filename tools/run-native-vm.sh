#!/bin/bash
# Run inside pixel-phone-qemu-native with read-only /workspace and /vm,
# and a fresh /results on the container's Linux filesystem. Copy results to
# the host afterwards: macOS bind mounts do not support mapped-xattr metadata.
# No network or hardware passthrough.
set -euo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 SDL_VIDEODRIVER=x11 XDG_RUNTIME_DIR=/tmp
timeout --signal=TERM --kill-after=10s 900s \
  xvfb-run -a -s '-screen 0 800x600x24 -nolisten tcp' \
  qemu-system-aarch64 -machine virt,gic-version=3 -cpu max \
  -accel tcg,thread=multi -smp 4 -m 4096 \
  -kernel /vm/Image -initrd /vm/initramfs-linux.img \
  -append 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service' \
  -drive file=/vm/rootfs.img,if=none,id=root,format=raw,snapshot=on \
  -device virtio-blk-pci,drive=root \
  -device virtio-gpu-gl-pci,xres=800,yres=600 \
  -device virtio-keyboard-pci -device virtio-rng-pci -nic none \
  -fsdev local,id=project,path=/workspace,security_model=none,readonly=on \
  -device virtio-9p-pci,fsdev=project,mount_tag=project \
  -fsdev local,id=results,path=/results,security_model=mapped-xattr \
  -device virtio-9p-pci,fsdev=results,mount_tag=results \
  -display sdl,gl=on -serial stdio -monitor none -no-reboot
python - <<'PY'
import json
from pathlib import Path
result = json.loads(Path('/results/result.json').read_text())
if result.get('passed') is not True or result.get('hardware_verified') is not False:
    raise SystemExit('Native VM validation failed; inspect result.json and guest logs')
print('Native virtual-GPU validation passed; Pixel hardware remains unverified.')
PY
