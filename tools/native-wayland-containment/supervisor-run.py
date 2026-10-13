#!/usr/bin/env python3
"""Freeze a separate original-kernel supervisor epoch; never reuse labels."""
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
    parser=argparse.ArgumentParser();parser.add_argument('--phase',required=True,choices=['red','green']);parser.add_argument('--label',required=True);parser.add_argument('--build',required=True);parser.add_argument('--case',default='held-control',choices=['held-control','initialization-deadline','source-death','parent-death','normal','queued-late-ready']);a=parser.parse_args()
    assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
    base=REPO/'artifacts/native-wayland-containment';base.mkdir(parents=True,exist_ok=True)
    with (base/(a.label+'.lock')).open('a') as label_lock, (MAIN/'artifacts/native-session-installed-app/vm-production.lock').open('a') as lock:
        fcntl.flock(label_lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        # Every VM input is read-only; QEMU snapshot writes are per-container.
        # Clone preparation or mutators must retain LOCK_EX on this same key.
        fcntl.flock(lock,fcntl.LOCK_SH|fcntl.LOCK_NB)
        out=base/a.label;assert not out.exists();out.mkdir();(out/'source').mkdir();(out/'bins').mkdir()
        shutil.copyfile(Path(__file__).with_name('supervisor-guest.py'),out/'source/guest.py')
        shutil.copyfile(Path(__file__),out/'source/supervisor-run.py')
        shutil.copyfile(Path(__file__).with_name('registry-probe.py'),out/'source/registry-probe.py')
        notes=Path(__file__).with_name('supervisor-test-notes.md')
        if notes.exists():shutil.copyfile(notes,out/'source/supervisor-test-notes.md')
        shutil.copyfile(MAIN/'tools/native-session-installed-app/boot.sh',out/'source/boot.sh')
        selected_source=REPO/'native-display' if a.phase=='red' else base/a.build/'source/native-display'
        assert selected_source.is_dir()
        shutil.copytree(selected_source,out/'source/native-display',ignore=shutil.ignore_patterns('target'))
        shutil.copyfile(base/a.build/'selection.json',out/'build-selection.json')
        binary='display-context-fixture' if a.phase=='red' else 'display-supervisor-fixture'
        shutil.copyfile(base/a.build/'bins'/binary,out/'bins'/binary);(out/'bins'/binary).chmod(0o755)
        current=json.loads((MAIN/'artifacts/native-session-compositor/green-owned-source-final/result.json').read_text())
        vm=MAIN/'artifacts/native-session-installed-app/vm-production';image='pixel-graphics-sim:20261011-b472'
        supervisor_before=(REPO/'native-display/src/bin/display-supervisor-fixture.rs').exists()
        if a.phase=='red':assert not supervisor_before,'Chronological baseline requires missing native supervisor implementation'
        selection={'phase':a.phase,'case':a.case,'build':a.build,'binary':binary,'supervisor_binary_source_existed':supervisor_before,'response_budget_ns':250000000,'original_initialization_budget_ns':5000000000,'worker_memory_bytes':134217728,'worker_fd_limit':64,'original_root_image_reused_readonly':True,'input_vm_lock_mode':'shared-readonly','per_label_lock_exclusive':True,'root_image_sha256':sha(vm/'root.img'),'kernel_sha256':sha(MAIN/'sim/boot/Image'),'initrd_sha256':sha(MAIN/'sim/boot/initramfs-linux.img'),'hyprland_sha256':current['hyprland_elf_sha256'],'source_selection':'Actual held current SDK protocol child before native supervisor implementation' if a.phase=='red' else 'Actual original compositor with selected native Root supervisor','container_image_id':subprocess.check_output(DOCKER+['image','inspect',image,'--format','{{.Id}}'],text=True).strip(),'cpus':6,'memory':'8g','guest_cpus':4,'no_nic':True,'files':{str(q.relative_to(out)):sha(q) for q in sorted(out.rglob('*')) if q.is_file()}}
        name='pokelinux-display-supervisor-'+a.label
        cid=subprocess.check_output(DOCKER+['create','--name',name,'--pull=never','--network','none','--cpus','6','--memory','8g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(MAIN)+':/workspace:ro','-v',str(vm)+':/vm:ro','-v',str(out)+':/results:rw',image,'bash','/results/source/boot.sh'],text=True).strip();selection['container_id']=cid;(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
        try:
            with (out/'serial.log').open('w') as log:completed=subprocess.run(DOCKER+['start','-a',cid],stdout=log,stderr=subprocess.STDOUT)
        finally:subprocess.run(DOCKER+['rm','-f',cid],check=True,stdout=subprocess.DEVNULL)
        (out/'exit-code').write_text(str(completed.returncode)+'\n')
        assert (out/'result.json').exists(),(out/'serial.log').read_text()[-5000:]
        result=json.loads((out/'result.json').read_text());print(json.dumps(result,indent=2)[:8000]);assert completed.returncode==0 and result['setup_complete'] and result['all_children_reaped']
        if a.phase=='red':assert result['red_detected'] and not result['passed'];print('Actual SIGSTOP SDK control responsiveness RED before native supervisor implementation')
        else:assert result['passed']
if __name__=='__main__':main()
