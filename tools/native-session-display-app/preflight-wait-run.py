#!/usr/bin/env python3
"""Actual ARM preflight cleanup signal order; no shared/joint fixture mutations."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];MAIN=Path('/home/ubuntu/Projects/pokelinux');BASE=ROOT/'artifacts/native-session-display-app'
D=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock'];IMAGE='pixel-graphics-sim:20261011-b472'
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while c:=f.read(1048576):h.update(c)
 return h.hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--label',required=True);p.add_argument('--build',required=True);p.add_argument('--phase',choices=['green'],required=True);a=p.parse_args();assert a.label.startswith('preflight-') and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 vm=MAIN/'artifacts/native-session-installed-app/vm-production'
 with (BASE/(a.label+'.lock')).open('a') as own,(MAIN/'artifacts/native-session-installed-app/vm-production.lock').open('a') as shared:
  fcntl.flock(own,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(shared,fcntl.LOCK_SH|fcntl.LOCK_NB)
  out=BASE/a.label;out.mkdir();source=out/'source';source.mkdir();bins=out/'bins';bins.mkdir()
  for n in ['preflight-wait-run.py','preflight-wait-guest.py','preflight-boot.sh']:shutil.copyfile(Path(__file__).with_name(n),source/n)
  # Existing immutable boot inode executes this private output copy.
  shutil.copyfile(source/'preflight-wait-guest.py',source/'guest.py')
  build=BASE/a.build;bs=json.loads((build/'selection.json').read_text())
  for name,value in bs['sources'].items():assert sha(build/'source'/name)==value
  shutil.copytree(build/'source',source/'rust-build');shutil.copyfile(build/'selection.json',source/'rust-build-selection.json')
  for name in ['messages-session-display-fixture','lifecycle-launcher']:
   assert sha(build/'bins'/name)==bs['bins'][name];shutil.copyfile(build/'bins'/name,bins/name);(bins/name).chmod(0o755)
  if a.phase=='red':assert bs['bins']['messages-session-display-fixture']=='c76d4d96395d1c537838d08210583c5970c41859817b75c4b6ddedc298a29321'
  s={'phase':a.phase,'rust_build':a.build,'rust_selection_sha256':sha(build/'selection.json'),'service_elf_sha256':bs['bins']['messages-session-display-fixture'],'scope':'actual initializing-SDK actor preflight fence order plus real terminal service-parent wait4 for both children before service exit; not Ready or exhausted cleanup budget','network':'none','container_cpus':6,'container_memory':'8g','guest_vcpus':4,'backing_readonly_snapshot':True,'input_lock':'shared immutable VM; preparers require exclusive','vm_sha256':sha(vm/'root.img'),'kernel_sha256':sha(MAIN/'sim/boot/Image'),'initrd_sha256':sha(MAIN/'sim/boot/initramfs-linux.img'),'image_id':subprocess.check_output(D+['image','inspect',IMAGE,'--format','{{.Id}}'],text=True).strip(),'files':{str(f.relative_to(out)):sha(f) for d in [source,bins] for f in sorted(d.rglob('*')) if f.is_file()}}
  (out/'selection.json').write_text(json.dumps(s,indent=2)+'\n')
  command=D+['run','--rm','--pull=never','--name','pokelinux-display-preflight-sol-'+a.label,'--network','none','--cpus','6','--memory','8g','--pids-limit','256','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(MAIN)+':/workspace:ro','-v',str(vm)+':/vm:ro','-v',str(out)+':/results:rw',s['image_id'],'bash','/results/source/preflight-boot.sh']
  (out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
  with (out/'serial.log').open('w') as log:run=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
  (out/'exit-code').write_text(str(run.returncode)+'\n');r=json.loads((out/'result.json').read_text());print(json.dumps(r,indent=2));assert run.returncode==0 and r['all_tracked_children_reaped'] and r['actual_preflight_EADDRINUSE'] and r['service_parent_wait4_for_both_children']
  if a.phase=='red':assert r['expected_red'] and not r['passed'];raise SystemExit(1)
  assert r['passed']
if __name__=='__main__':main()
