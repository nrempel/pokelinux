#!/usr/bin/env python3
"""Boot only this fixture's cloned ARM guest, snapshot writes, no NIC/host devices."""
import fcntl, json, os, subprocess, sys
from pathlib import Path
root=Path('/workspace'); phase=sys.argv[1]
assert phase in ['driver-red','driver-green','held-red','green','ndk-red','ndk-green']
base=root/'artifacts/native-arm64-binder'; label=sys.argv[2] if len(sys.argv)>2 else phase
assert label.replace('-','').replace('_','').isalnum()
out=base/'runtime'/label
out.mkdir(parents=True, exist_ok=False)
with (base/'vm.lock').open('w') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    cmd=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-accel','tcg,thread=multi',
         '-smp','3','-m','2048','-kernel',str(root/'sim/boot/Image'),'-initrd',str(root/'sim/boot/initramfs-linux.img'),
         '-append','console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service '
         'systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service pixel.binder-phase='+phase,
         '-drive',f'file={base}/vm/root.img,if=none,id=root,format=raw,snapshot=on',
         '-device','virtio-blk-pci,drive=root','-device','virtio-rng-pci','-nic','none',
         '-fsdev',f'local,id=project,path={root},security_model=none,readonly=on',
         '-device','virtio-9p-pci,fsdev=project,mount_tag=project',
         '-fsdev',f'local,id=results,path={out},security_model=mapped-xattr',
         '-device','virtio-9p-pci,fsdev=results,mount_tag=results','-nographic','-monitor','none','-no-reboot']
    (out/'command.json').write_text(json.dumps(cmd,indent=2)+'\n')
    with (out/'serial.log').open('w') as log:
        subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=240,check=True)
    result=json.loads((out/'result.json').read_text()); print(json.dumps(result,indent=2))
    assert result['architecture']=='aarch64' and result['qemu_mode']=='system'
    assert result['passed'] == (phase not in ['driver-red','held-red','ndk-red'])
