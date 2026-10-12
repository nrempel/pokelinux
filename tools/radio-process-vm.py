#!/usr/bin/env python3
"""Owned ARM64 Linux kernel VM only; no device or user-mode QEMU substitution."""
import datetime
import json
import os
from pathlib import Path
import subprocess

root = Path('/workspace')
out = root / 'sim/results' / ('radio-process-' + datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ'))
out.mkdir()
print('ARM64 process results:', out, flush=True)
command = ['qemu-system-aarch64', '-machine', 'virt,gic-version=3', '-cpu', 'max',
           '-accel', 'tcg,thread=multi', '-smp', '3', '-m', '3072',
           '-kernel', str(root / 'sim/boot/Image'), '-initrd', str(root / 'sim/boot/initramfs-linux.img'),
           '-append', 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service '
                      'systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service pixel.mode=radio-process',
           '-drive', f'file={root}/sim/rootfs.img,if=none,id=root,format=raw,snapshot=on',
           '-device', 'virtio-blk-pci,drive=root', '-device', 'virtio-rng-pci', '-nic', 'none',
           '-fsdev', f'local,id=project,path={root},security_model=none,readonly=on',
           '-device', 'virtio-9p-pci,fsdev=project,mount_tag=project',
           '-fsdev', f'local,id=results,path={out},security_model=mapped-xattr',
           '-device', 'virtio-9p-pci,fsdev=results,mount_tag=results',
           '-nographic', '-monitor', 'none', '-no-reboot']
(out / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
with (out / 'serial.log').open('w') as log:
    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=300,
                   check=True, env=os.environ.copy())
result = json.loads((out / 'result.json').read_text())
print(json.dumps(result, indent=2), flush=True)
assert result['passed'] and result['architecture'] == 'aarch64'
assert result['qemu_mode'] == 'system' and result['hardware_verified'] is False
(root / 'artifacts/native-radio/arm64-process-vm-result.json').write_text(json.dumps({
    'results': str(out.relative_to(root)), 'result': result}, indent=2) + '\n')
