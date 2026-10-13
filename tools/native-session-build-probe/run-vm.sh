#!/bin/bash
set -euo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 SDL_VIDEODRIVER=x11 XDG_RUNTIME_DIR=/tmp
set +e
timeout --signal=TERM --kill-after=10s 900s \
 xvfb-run -a -s '-screen 0 800x900x24 -nolisten tcp' \
 qemu-system-aarch64 -machine virt,gic-version=3 -cpu max \
 -accel tcg,thread=multi -smp 3 -m 3072 \
 -kernel /workspace/sim/boot/Image -initrd /workspace/sim/boot/initramfs-linux.img \
 -append 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service' \
 -drive file=/vm/root.img,if=none,id=root,format=raw,snapshot=on \
 -device virtio-blk-pci,drive=root -device virtio-gpu-gl-pci,xres=800,yres=900 \
 -device virtio-keyboard-pci -device virtio-rng-pci -nic none \
 -fsdev local,id=project,path=/workspace,security_model=none,readonly=on \
 -device virtio-9p-pci,fsdev=project,mount_tag=project \
 -fsdev local,id=results,path=/results,security_model=mapped-xattr \
 -device virtio-9p-pci,fsdev=results,mount_tag=results \
 -display sdl,gl=on -serial stdio -monitor none -no-reboot > /results/serial.log 2>&1
status=$?
set -e
python3 - "$status" <<'REPORT'
import json,sys
from pathlib import Path
p=Path('/results/result.json')
if not p.exists(): raise SystemExit(int(sys.argv[1]) or 100)
r=json.loads(p.read_text())
print(json.dumps({'passed':r['passed'],'groups':r['groups'],'error':r.get('error')},indent=2))
raise SystemExit(int(sys.argv[1]) or (0 if r['passed'] else 101))
REPORT
