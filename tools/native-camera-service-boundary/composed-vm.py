#!/usr/bin/env python3
"""Known readonly ARM backing, bounded overlay, actual QEMU wait and no NIC."""
import hashlib,json,os,subprocess,sys
from pathlib import Path
root=Path('/workspace');base=root/sys.argv[1];label=sys.argv[2];assert label.replace('-','').replace('_','').isalnum();out=base/'runtime'/label
assert out.is_dir() and not (out/'serial.log').exists()
selection=json.loads((base/'freeze/selection.json').read_text())
def digest(path):
    value=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()
for name,value in selection['files'].items():assert digest(base/'freeze'/name)==value,'frozen selection changed '+name
for name,row in selection['inputs'].items():assert digest(Path('/inputs')/name)==row['sha256'],'input changed '+name
(out/'input-before.json').write_text(json.dumps({name:row['sha256'] for name,row in selection['inputs'].items()},indent=2)+'\n')
overlay='/tmp/camera-root-overlay.qcow2';command=['qemu-img','create','-f','qcow2','-F','raw','-b','/inputs/root.img',overlay]
(out/'overlay-command.json').write_text(json.dumps(command,indent=2)+'\n');created=subprocess.run(command,capture_output=True,text=True,timeout=10);(out/'overlay-create.log').write_text(created.stdout+created.stderr);assert created.returncode==0
info=subprocess.run(['qemu-img','info','--output=json','--backing-chain',overlay],capture_output=True,text=True,timeout=10);assert info.returncode==0;(out/'overlay-input.json').write_text(info.stdout);chain=json.loads(info.stdout);assert len(chain)==2 and chain[0]['format']=='qcow2' and chain[1]['format']=='raw' and chain[1]['filename']=='/inputs/root.img'
command=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-accel','tcg,thread=multi','-smp','3','-m','2048','-kernel','/inputs/Image','-initrd','/inputs/initramfs-linux.img','-append','console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service','-drive',f'file={overlay},if=none,id=root,format=qcow2','-device','virtio-blk-pci,drive=root','-device','virtio-rng-pci','-nic','none','-fsdev','local,id=project,path=/workspace,security_model=none,readonly=on','-device','virtio-9p-pci,fsdev=project,mount_tag=project','-fsdev',f'local,id=results,path={out},security_model=mapped-xattr','-device','virtio-9p-pci,fsdev=results,mount_tag=results','-nographic','-monitor','none','-no-reboot']
(out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
with (out/'serial.log').open('w') as log:run=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,timeout=300,env=dict(os.environ,TMPDIR='/tmp'))
(out/'qemu-exit-code.txt').write_text(str(run.returncode)+'\n');assert run.returncode==0
result=json.loads((out/'result.json').read_text());print(json.dumps(result,indent=2));assert result['architecture']=='aarch64' and result['fixture_cleanup_passed']
if selection['expected']=='red':
    assert result['expected_red'] and result['red_signature']=='actual_supervisor_eagain_revoke_retains_ready_worker_expected_reap';sys.exit(1)
assert result['passed']
