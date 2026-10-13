#!/usr/bin/env python3
"""Bounded serial bootstrap into service-free PID1; retained actual QEMU wait."""
import hashlib,json,os,selectors,subprocess,sys,time
from pathlib import Path
BASE=Path('/workspace/artifacts/native-radio-load-only');label=sys.argv[1];case=sys.argv[2];freeze=sys.argv[3] if len(sys.argv)>3 else 'freeze';assert freeze.replace('-','').isalnum()
assert label.replace('-','').isalnum() and case in ['red','green']
OUT=BASE/'runtime'/label;assert OUT.is_dir() and not (OUT/'serial.log').exists()
selection=json.loads((BASE/freeze/'selection.json').read_text())
def digest(p):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for chunk in iter(lambda:f.read(1048576),b''):h.update(chunk)
    return h.hexdigest()
before={name:digest(Path('/inputs')/name) for name in selection['inputs']}
assert all(before[name]==row['sha256'] for name,row in selection['inputs'].items())
(OUT/'input-before.json').write_text(json.dumps(before,indent=2)+'\n')
overlay='/tmp/radio-public-overlay.qcow2'
subprocess.run(['qemu-img','create','-f','qcow2','-F','raw','-b','/inputs/root.img',overlay],check=True,timeout=10)
info=subprocess.run(['qemu-img','info','--output=json','--backing-chain',overlay],capture_output=True,text=True,check=True,timeout=10)
(OUT/'overlay-input.json').write_text(info.stdout)
chain=json.loads(info.stdout);assert len(chain)==2 and chain[1]['filename']=='/inputs/root.img'
cmd=['qemu-system-aarch64','-machine','virt,gic-version=3','-cpu','max','-accel','tcg,thread=multi','-smp','4','-m','4096','-kernel','/inputs/Image','-initrd','/inputs/initramfs-linux.img','-append','console=ttyAMA0 root=/dev/vda rw rootwait panic=10 init=/usr/bin/bash -- --noprofile --norc','-drive',f'file={overlay},if=none,id=root,format=qcow2','-device','virtio-blk-pci,drive=root','-nic','none','-fsdev','local,id=project,path=/workspace,security_model=none,readonly=on','-device','virtio-9p-pci,fsdev=project,mount_tag=project','-fsdev',f'local,id=results,path={OUT},security_model=mapped-xattr','-device','virtio-9p-pci,fsdev=results,mount_tag=results','-nographic','-monitor','none','-no-reboot']
(OUT/'command.json').write_text(json.dumps(cmd,indent=2)+'\n')
bootstrap=f'mkdir -p /workspace /results; mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace; mount -t 9p -o trans=virtio,version=9p2000.L results /results; exec python3 /workspace/artifacts/native-radio-load-only/{freeze}/source/tools/native-radio-load-only/guest.py {case} {freeze}\n'
(OUT/'serial-bootstrap.txt').write_text(bootstrap)
qemu=subprocess.Popen(cmd,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=dict(os.environ,TMPDIR='/tmp'))
serial=bytearray();released=False;deadline=time.monotonic()+180
try:
    with selectors.DefaultSelector() as sel, (OUT/'serial.log').open('wb') as log:
        sel.register(qemu.stdout,selectors.EVENT_READ)
        while time.monotonic()<deadline:
            if sel.select(.1):
                chunk=os.read(qemu.stdout.fileno(),65536)
                if not chunk:break
                serial.extend(chunk);log.write(chunk);log.flush();assert len(serial)<=16*1024*1024
                if not released and b'bash-' in serial and b'# ' in serial[-4096:]:
                    qemu.stdin.write(bootstrap.encode());qemu.stdin.flush();released=True
            if qemu.poll() is not None:break
        assert released,'service-free shell did not become ready'
        # EOF can precede visibility of the process exit status. Observe the
        # actual process with one bounded wait; a zero/expired budget still fails.
        if qemu.poll() is None:qemu.wait(timeout=min(5,max(0,deadline-time.monotonic())))
finally:
    if qemu.poll() is None:qemu.kill()
    exit_code=qemu.wait(timeout=5)
    (OUT/'actual-qemu-wait.json').write_text(json.dumps({'pid':qemu.pid,'actual_wait':True,'exit':exit_code,'service_free_release':released},indent=2)+'\n')
assert exit_code==0
result=json.loads((OUT/'result.json').read_text());print(json.dumps(result,indent=2))
if case=='red':assert result['expected_red'] and result['cleanup_passed'] and 'bootstrap_or_cleanup_error' not in result;sys.exit(1)
assert result['passed'] and result['cleanup_passed'] and 'bootstrap_or_cleanup_error' not in result
