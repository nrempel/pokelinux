#!/usr/bin/env python3
"""Boot only this fixture's cloned ARM guest, snapshot writes, no NIC/host devices."""
import fcntl, json, os, subprocess, sys
from pathlib import Path
root=Path('/workspace'); phase=sys.argv[1]
assert phase in ['red','green']
base=root/'artifacts/native-messages-enumeration'; label=sys.argv[2] if len(sys.argv)>2 else phase
assert label.replace('-','').replace('_','').isalnum()
out=base/'runtime'/label
out.mkdir(parents=True, exist_ok=False)
with (base/'vm.lock').open('w') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    cmd=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-accel','tcg,thread=multi',
         '-smp','3','-m','2048','-kernel',str(root/'sim/boot/Image'),'-initrd',str(root/'sim/boot/initramfs-linux.img'),
         '-append','console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service '
         'systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service pixel.messages-enumeration-phase='+phase,
         '-drive',f'file={base}/vm/root.img,if=none,id=root,format=raw,snapshot=on',
         '-device','virtio-blk-pci,drive=root','-device','virtio-rng-pci','-nic','none',
         '-fsdev',f'local,id=project,path={root},security_model=none,readonly=on',
         '-device','virtio-9p-pci,fsdev=project,mount_tag=project',
         '-fsdev',f'local,id=results,path={out},security_model=mapped-xattr',
         '-device','virtio-9p-pci,fsdev=results,mount_tag=results','-nographic','-monitor','none','-no-reboot']
    (out/'command.json').write_text(json.dumps(cmd,indent=2)+'\n')
    with (out/'serial.log').open('w') as log:
        subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=360,check=True)
    result=json.loads((out/'result.json').read_text()); print(json.dumps({key:result[key] for key in ["passed","architecture","kernel","scope","checks","error"] if key in result},indent=2))
    assert result['architecture']=='aarch64' and result['qemu_mode']=='system'
    if phase=='red':
        assert result.get('expected_red') and result.get('signature')=='live_binder33_complete_projection_unavailable'
        sys.exit(1)
    assert result['passed'] is True
