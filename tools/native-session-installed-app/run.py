#!/usr/bin/env python3
"""Immutable, owned network-free original compositor / installed Qt selection."""
import argparse, fcntl, hashlib, json, os, shutil, subprocess
from pathlib import Path
REPO=Path(__file__).resolve().parents[2]
BASE=REPO/'artifacts/native-session-installed-app'
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
IMAGE='pixel-graphics-sim:20261011-b472'
def sha(p):
 h=hashlib.sha256()
 with Path(p).open('rb') as f:
  while chunk:=f.read(1024*1024):h.update(chunk)
 return h.hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--phase',required=True,choices=['red','green']);p.add_argument('--label',required=True);p.add_argument('--build');p.add_argument('--loss',default='unload',choices=['unload','stopped','death']);p.add_argument('--production',action='store_true');p.add_argument('--vm-key',default='main');p.add_argument('--prepare-lock',action='store_true');p.add_argument('--skip-lock',action='store_true');a=p.parse_args()
 assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 assert a.vm_key and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.vm_key)
 BASE.mkdir(parents=True,exist_ok=True)
 with (BASE/('vm-'+a.vm_key+'.lock')).open('w') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
  out=BASE/a.label;assert not out.exists(),'preserve every selected run';out.mkdir();source=out/'source';bins=out/'bins';source.mkdir();bins.mkdir()
  for n in ['run.py','boot.sh','guest.py']:shutil.copyfile(Path(__file__).parent/n,source/n)
  compositor=REPO/'artifacts/native-session-compositor/green-owned-source-final'
  comp=json.loads((compositor/'result.json').read_text());assert comp['passed'];assert sha(compositor/'bins/adapter.so')==comp['adapter_elf_sha256']
  shutil.copyfile(compositor/'source/adapter.cpp',source/'compositor-adapter.cpp')
  shutil.copyfile(compositor/'selection.json',source/'compositor-build-selection.json')
  for n,q in [('adapter.so',compositor/'bins/adapter.so'),('probe.so',compositor/'bins/probe.so'),('lock-client',compositor/'lock-client')]:shutil.copyfile(q,bins/n)
  qt=REPO/'artifacts/native-messages/desktop/green-10';proof=json.loads((qt/'result.json').read_text());assert proof['passed']
  for n in ['pokelinux-messages','pokelinux-messages-observer']:assert sha(qt/n)==proof['production_binary'][n];shutil.copyfile(qt/n,bins/n)
  for n in ['lifecycle-launcher','messages-seed-fixture','messages-service']:shutil.copyfile(qt/'bins'/n,bins/n)
  shutil.copytree(qt/'source/native-app',source/'native-app');shutil.copytree(qt/'source/native-messages-ui',source/'native-messages-ui')
  for n in ['native-lifecycle','native-messages']:shutil.copytree(qt/'source'/n,source/('baseline-'+n))
  selection={'phase':a.phase,'baseline':'actual staged normal installed service; no session-source selector','qt_stage':str(qt.relative_to(REPO)),'compositor_stage':str(compositor.relative_to(REPO)),'expected_hyprland_elf_sha256':comp['hyprland_elf_sha256'],'sample_budget_ns':100000000,'source_revival_permitted':False,'source_loss_case':a.loss,'normal_production_qt':a.production,'owned_vm_key':a.vm_key,'prepare_lock_before_first_source':a.prepare_lock,'skip_lock_group':a.skip_lock,'qemu_smp':4,'container_cpus':6,'container_memory':'8g'}
  if a.phase=='green':
   assert a.build
   build=BASE/a.build;bs=json.loads((build/'selection.json').read_text());assert bs['target']=='aarch64-unknown-linux-gnu'
   for n,d in bs['sources'].items():assert sha(build/'source'/n)==d
   for n,d in bs['bins'].items():assert sha(build/'bins'/n)==d;shutil.copyfile(build/'bins'/n,bins/n)
   shutil.copytree(build/'source',source/'rust-build');selection['rust_build']=a.build;selection['rust_selection']=bs
  for f in bins.iterdir():f.chmod(0o755)
  selection['files']={str(q.relative_to(out)):sha(q) for d in [source,bins] for q in sorted(d.rglob('*')) if q.is_file()}
  selection['kernel_sha256']=sha(REPO/'sim/boot/Image');selection['initrd_sha256']=sha(REPO/'sim/boot/initramfs-linux.img');selection['image_id']=subprocess.check_output(DOCKER+['image','inspect',IMAGE,'--format','{{.Id}}'],text=True).strip()
  vm=BASE/('vm-'+a.vm_key);vm.mkdir(exist_ok=True);root=vm/'root.img'
  if not root.exists():
   subprocess.run(['cp','--reflink=auto','--sparse=always',str(REPO/'sim/rootfs.img'),str(root)],check=True)
   entry=vm/'entry';entry.write_text('#!/bin/bash\nset -euo pipefail\nmkdir -p /workspace /results\nmount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace\nmount -t 9p -o trans=virtio,version=9p2000.L results /results\nexec python3 /results/source/guest.py\n')
   with (vm/'clone.log').open('w') as log:
    for command in ['rm /usr/local/bin/pixel-sim-boot','write '+str(entry)+' /usr/local/bin/pixel-sim-boot','set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755']:subprocess.run(['debugfs','-w','-R',command,str(root)],stdout=log,stderr=subprocess.STDOUT,check=True)
  selection['owned_vm_root_sha256']=sha(root)
  name='pokelinux-session-installed-app-'+a.label
  cid=subprocess.check_output(DOCKER+['create','--name',name,'--pull=never','--network','none','--cpus','6','--memory','8g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(REPO)+':/workspace:ro','-v',str(vm)+':/vm:ro','-v',str(out)+':/results:rw',IMAGE,'bash','/results/source/boot.sh'],text=True).strip()
  selection['container_id']=cid;(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
  try:
   with (out/'serial.log').open('w') as log:completed=subprocess.run(DOCKER+['start','-a',cid],stdout=log,stderr=subprocess.STDOUT)
   (out/'exit-code').write_text(str(completed.returncode)+'\n')
  finally:subprocess.run(DOCKER+['rm','-f',cid],stdout=subprocess.DEVNULL,check=True)
  if not (out/'result.json').exists():print((out/'serial.log').read_text()[-5000:]);raise AssertionError('guest produced no result')
  result=json.loads((out/'result.json').read_text());print(json.dumps({k:result.get(k) for k in ['passed','red_detected','setup_complete','groups','error','all_tracked_children_reaped','source_events']},indent=2))
  assert completed.returncode==0 and result['all_tracked_children_reaped'] and result['setup_complete']
  if a.phase=='red':assert result['red_detected'] and not result['passed'];print('Qualified pre-composition installed Qt behavioral RED')
  else:assert result['passed']
if __name__=='__main__':main()
