#!/usr/bin/env python3
import os,subprocess
from pathlib import Path
p=Path('/workspace');out=p/'android-framework/transport-vm';out.mkdir(exist_ok=True)
command=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-accel','tcg,thread=multi','-smp','2','-m','2048','-kernel',str(p/'sim/boot/Image'),'-initrd',str(p/'sim/boot/initramfs-linux.img'),'-append','console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service pixel.mode=transport','-drive',f'file={p}/sim/rootfs.img,if=none,id=root,format=raw,snapshot=on','-device','virtio-blk-pci,drive=root','-device','virtio-rng-pci','-global','virtio-net-pci.romfile=','-nic','user,model=virtio-net-pci','-fsdev',f'local,id=project,path={p},security_model=none,readonly=on','-device','virtio-9p-pci,fsdev=project,mount_tag=project','-fsdev',f'local,id=results,path={out},security_model=mapped-xattr','-device','virtio-9p-pci,fsdev=results,mount_tag=results','-nographic','-monitor','none','-no-reboot']
with (out/'serial.log').open('w') as log:
 subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=1000)
