#!/usr/bin/env python3
import fcntl,json,subprocess,sys
from pathlib import Path
ROOT=Path('/workspace'); BASE=ROOT/'artifacts/native-messages-receive-isolation'
phase,label=sys.argv[1:];assert phase in ['red','green','lock-red','lock-green','errors-green','enumeration-green'] and label.replace('-','').replace('_','').isalnum()
out=BASE/'runtime'/label;out.mkdir(parents=True,exist_ok=False)
with (BASE/'vm.lock').open('w') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
 cmd=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-accel','tcg,thread=multi','-smp','4','-m','4096','-kernel','/kernel/Image','-initrd','/kernel/initramfs-linux.img','-append','console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service pixel.receive-isolation='+phase,'-drive',f'file={BASE}/vm/root.img,if=none,id=root,format=raw,snapshot=on','-device','virtio-blk-pci,drive=root','-device','virtio-rng-pci','-nic','none','-fsdev',f'local,id=project,path={ROOT},security_model=none,readonly=on','-device','virtio-9p-pci,fsdev=project,mount_tag=project','-fsdev',f'local,id=runtime,path=/android-runtime,security_model=none,readonly=on','-device','virtio-9p-pci,fsdev=runtime,mount_tag=android-runtime','-fsdev',f'local,id=results,path={out},security_model=mapped-xattr','-device','virtio-9p-pci,fsdev=results,mount_tag=results','-nographic','-monitor','none','-no-reboot']
 (out/'command.json').write_text(json.dumps(cmd,indent=2)+'\n')
 with (out/'serial.log').open('w') as log: subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=420,check=True)
 result=json.loads((out/'result.json').read_text());print(json.dumps(result,indent=2))
 assert result['architecture']=='aarch64' and result['qemu_mode']=='system'
 if phase=='lock-red':
  assert result.get('expected_red') and result.get('signature')=='held_sqlite_reader_stops_background_projection';sys.exit(1)
 if phase=='red':
  assert result.get('expected_red') and result.get('signature')=='held_read_close_loses_same_live_background_source';sys.exit(1)
 assert result['passed']
