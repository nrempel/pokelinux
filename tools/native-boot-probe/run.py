#!/usr/bin/env python3
"""Boot an authored RAM-only initramfs on the retained non-Pixel ARM kernel.

No root filesystem, original vendor userspace, phone or network is attached.
"""
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from diagnostics import validate

ROOT = Path(__file__).resolve().parents[2]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
KERNEL = MAIN/'sim/boot/Image'
KERNEL_SHA = '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b'
IMAGE = '561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
DOCKER = ['docker','--host=unix:///var/run/docker.sock']

def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()

def cpio(entries):
    """Fixed authored Linux newc archive; no caller-controlled traversal."""
    data = bytearray()
    for inode, (name, mode, payload, major, minor) in enumerate(entries+[('TRAILER!!!',0,b'',0,0)],1):
        encoded = name.encode()+b'\0'
        fields = (inode,mode,0,0,2 if mode & 0o170000 == 0o040000 else 1,0,len(payload),0,0,major,minor,len(encoded),0)
        data.extend(b'070701'+b''.join(f'{value:08x}'.encode() for value in fields))
        data.extend(encoded)
        data.extend(b'\0'*(-len(data)%4))
        data.extend(payload)
        data.extend(b'\0'*(-len(data)%4))
    data.extend(b'\0'*(-len(data)%512))
    return gzip.compress(bytes(data),compresslevel=1,mtime=0)

if __name__ == '__main__':
    if len(sys.argv)!=3 or sys.argv[1] not in ('red','green','missing-child'):
        raise SystemExit('usage: run.py red|green|missing-child UNIQUE_LABEL')
    mode,label=sys.argv[1:]
    if not label or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_' for c in label):
        raise SystemExit('invalid immutable output label')
    out=ROOT/'artifacts/native-boot-probe'/label
    out.mkdir(parents=True,mode=0o700)
    assert digest(KERNEL)==KERNEL_SHA
    entries=[('dev',0o040755,b'',0,0),('proc',0o040755,b'',0,0),('sys',0o040755,b'',0,0),
             ('dev/console',0o020600,b'',5,1),('dev/null',0o020666,b'',1,3),
             ('dev/kmsg',0o020600,b'',1,11)]
    selected={'kernel':KERNEL_SHA,'runner':digest(Path(__file__)),
              'diagnostics':digest(Path(__file__).with_name('diagnostics.py'))}
    if mode!='red':
        target=Path(os.environ['NATIVE_BOOT_PROBE_TARGET'])/'aarch64-unknown-linux-gnu/release'
        for name,archive in [('pokelinux-boot-probe','init'),('pokelinux-boot-child','probe-child')]:
            if mode=='missing-child' and archive=='probe-child': continue
            path=target/name
            payload=path.read_bytes()
            assert 0<len(payload)<=16*1024*1024 and payload[:4]==b'\x7fELF'
            entries.append((archive,0o100755,payload,0,0));selected[archive]=digest(path)
    initramfs=out/'probe.cpio.gz'
    initramfs.write_bytes(cpio(entries));selected['initramfs']=digest(initramfs)
    (out/'selection.json').write_text(json.dumps(selected,indent=2)+'\n')
    env=os.environ.copy()
    for key in ('DOCKER_HOST','DOCKER_CONTEXT','DOCKER_TLS','DOCKER_TLS_VERIFY','DOCKER_CERT_PATH'):env.pop(key,None)
    name='pokelinux-native-boot-probe-'+label
    qemu=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-smp','2','-m','256',
          '-nic','none','-nographic','-monitor','none','-no-reboot','-kernel','/inputs/Image','-initrd','/inputs/probe.cpio.gz',
          '-append','console=ttyAMA0 earlycon=pl011,0x9000000 rdinit=/init panic=1']
    command=DOCKER+['run','--rm','--name',name,'--network','none','--read-only','--cap-drop','ALL',
                   '--cpus','2','--memory','512m','--pids-limit','64',
                   '-v',f'{KERNEL}:/inputs/Image:ro','-v',f'{initramfs}:/inputs/probe.cpio.gz:ro',IMAGE]+qemu
    (out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
    started=time.monotonic();timed_out=False
    with (out/'console.log').open('wb') as log:
        process=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,env=env)
        try: code=process.wait(timeout=45)
        except subprocess.TimeoutExpired:
            timed_out=True
            subprocess.run(DOCKER+['kill',name],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=True,timeout=5)
            code=process.wait(timeout=5)
    console=(out/'console.log').read_text(errors='replace')
    assert digest(KERNEL)==KERNEL_SHA and digest(initramfs)==selected['initramfs']
    assert digest(Path(__file__))==selected['runner']
    assert digest(Path(__file__).with_name('diagnostics.py'))==selected['diagnostics']
    diagnostic_result=None
    if mode=='red':
        passed=False
        expected_red=(not timed_out and code==0 and 'check access for rdinit=/init failed: -2' in console
                      and 'VFS: Unable to mount root fs on unknown-block(0,0)' in console
                      and 'POKELINUX_NATIVE_PID1' not in console)
    else:
        expected_red=False
        diagnostic_result=validate(console, mode)
        passed=not timed_out and code==0 and diagnostic_result['passed']
    result={'evidence':'authored RAM initramfs on actual Arch ARM guest kernel; not Pixel boot',
            'kernel_sha256':KERNEL_SHA,'qemu_mode':'system','mode':mode,'passed':passed,'expected_red':expected_red,
            'docker_actual_wait':True,'docker_exit':code,'timed_out':timed_out,
            'elapsed_seconds':round(time.monotonic()-started,3),'phone_accessed':False,'vendor_userspace_executed':False,
            'root_disk_attached':False,'console_sha256':digest(out/'console.log'),
            'diagnostic_acceptance':diagnostic_result}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result),flush=True)
    raise SystemExit(0 if passed else 1)
