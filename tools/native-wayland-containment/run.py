#!/usr/bin/env python3
"""Freeze one actual-kernel containment epoch using an existing immutable VM."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
REPO=Path(__file__).resolve().parents[2]
MAIN=Path('/home/ubuntu/Projects/pokelinux')
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
def sha(p):
    h=hashlib.sha256()
    with Path(p).open('rb') as f:
        while data:=f.read(1024*1024):h.update(data)
    return h.hexdigest()
def main():
    p=argparse.ArgumentParser();p.add_argument('--phase',required=True,choices=['red','green']);p.add_argument('--label',required=True);p.add_argument('--build');a=p.parse_args()
    assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
    base=REPO/'artifacts/native-wayland-containment';base.mkdir(parents=True,exist_ok=True)
    with (MAIN/'artifacts/native-session-installed-app/vm-production.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        out=base/a.label;assert not out.exists();out.mkdir();(out/'source').mkdir()
        for name in ['guest.py','registry-probe.py','run.py']:shutil.copyfile(Path(__file__).parent/name,out/'source'/name)
        shutil.copyfile(MAIN/'tools/native-session-installed-app/boot.sh',out/'source/boot.sh')
        if a.build:
            (out/'bins').mkdir();shutil.copyfile(base/a.build/'bins/display-context-fixture',out/'bins/display-context-fixture');(out/'bins/display-context-fixture').chmod(0o755)
        current=json.loads((MAIN/'artifacts/native-session-compositor/green-owned-source-final/result.json').read_text())
        vm=MAIN/'artifacts/native-session-installed-app/vm-production'
        image='pixel-graphics-sim:20261011-b472'
        selection={'phase':a.phase,'build':a.build,'original_root_image_reused_readonly':True,'root_image_sha256':sha(vm/'root.img'),'kernel_sha256':sha(MAIN/'sim/boot/Image'),'initrd_sha256':sha(MAIN/'sim/boot/initramfs-linux.img'),'hyprland_sha256':current['hyprland_elf_sha256'],'source_selection':'current authored fixture before native backend implementation' if a.phase=='red' else 'current authored fixture and frozen native backend','container_image_id':subprocess.check_output(DOCKER+['image','inspect',image,'--format','{{.Id}}'],text=True).strip(),'cpus':6,'memory':'8g','guest_cpus':4,'no_nic':True,'files':{str(q.relative_to(out)):sha(q) for q in sorted(out.rglob('*')) if q.is_file()}}
        name='pokelinux-wayland-containment-'+a.label
        cid=subprocess.check_output(DOCKER+['create','--name',name,'--pull=never','--network','none','--cpus','6','--memory','8g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(MAIN)+':/workspace:ro','-v',str(vm)+':/vm:ro','-v',str(out)+':/results:rw',image,'bash','/results/source/boot.sh'],text=True).strip();selection['container_id']=cid;(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
        try:
            with (out/'serial.log').open('w') as log:completed=subprocess.run(DOCKER+['start','-a',cid],stdout=log,stderr=subprocess.STDOUT)
        finally:subprocess.run(DOCKER+['rm','-f',cid],check=True,stdout=subprocess.DEVNULL)
        (out/'exit-code').write_text(str(completed.returncode)+'\n')
        assert (out/'result.json').exists(),(out/'serial.log').read_text()[-5000:]
        r=json.loads((out/'result.json').read_text());print(json.dumps(r,indent=2)[:6500]);assert completed.returncode==0 and r['setup_complete'] and r['all_children_reaped']
        if a.phase=='red':assert r['red_detected'] and not r['passed'];print('Actual original compositor app privilege boundary RED before native Rust backend')
        else:assert r['passed']
if __name__=='__main__':main()
