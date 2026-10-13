#!/usr/bin/env python3
"""Freeze one actual current actor/display/app composition epoch."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
REPO=Path(__file__).resolve().parents[2];MAIN=Path('/home/ubuntu/Projects/pokelinux');BASE=REPO/'artifacts/native-session-display-app'
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock'];IMAGE='pixel-graphics-sim:20261011-b472'
def sha(p):
 h=hashlib.sha256()
 with Path(p).open('rb') as f:
  while data:=f.read(1024*1024):h.update(data)
 return h.hexdigest()
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--label',required=True);parser.add_argument('--test-case',default='direct-realm-composition-red',choices=['direct-realm-composition-red','composite-stop-resource-red','composite-held-policy-stop-green','composite-held-sdk-loss-green']);parser.add_argument('--build');a=parser.parse_args();assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 BASE.mkdir(exist_ok=True)
 with (BASE/(a.label+'.lock')).open('a') as label_lock,(MAIN/'artifacts/native-session-installed-app/vm-production.lock').open('a') as lock:
  fcntl.flock(label_lock,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(lock,fcntl.LOCK_SH|fcntl.LOCK_NB)
  out=BASE/a.label;assert not out.exists();out.mkdir();source=out/'source';bins=out/'bins';source.mkdir();bins.mkdir()
  for name in ['run.py','guest.py','test-lane-notes.md']:shutil.copyfile(Path(__file__).with_name(name),source/name)
  shutil.copyfile(MAIN/'tools/native-session-installed-app/boot.sh',source/'boot.sh');shutil.copyfile(REPO/'tools/native-wayland-containment/registry-probe.py',source/'registry-probe.py')
  if a.test_case=='direct-realm-composition-red':
   assert a.build is None;build=MAIN/'artifacts/native-session-installed-app/combined-read-selector-green-build';fixture_binary='messages-session-installed-fixture'
  else:
   assert a.build;build=BASE/a.build;fixture_binary='messages-session-display-fixture'
  bs=json.loads((build/'selection.json').read_text())
  if a.test_case=='direct-realm-composition-red':assert bs['bins'][fixture_binary]=='4de010e554c5d0261a0a593d97ccb2b8e7c5241035385dceeaa43f70cbe3ebfb'
  for name,digest in bs['sources'].items():assert sha(build/'source'/name)==digest
  shutil.copytree(build/'source',source/'rust-build');shutil.copyfile(build/'selection.json',source/'rust-build-selection.json')
  compositor=MAIN/'artifacts/native-session-compositor/green-owned-source-final';comp=json.loads((compositor/'result.json').read_text());assert comp['passed']
  for name,path in [('adapter.so',compositor/'bins/adapter.so'),('probe.so',compositor/'bins/probe.so'),('lock-client',compositor/'lock-client')]:shutil.copyfile(path,bins/name)
  shutil.copyfile(compositor/'selection.json',source/'compositor-build-selection.json')
  qt=MAIN/'artifacts/native-messages/desktop/green-10';proof=json.loads((qt/'result.json').read_text());assert proof['passed']
  for name in ['pokelinux-messages','pokelinux-messages-observer']:assert sha(qt/name)==proof['production_binary'][name];shutil.copyfile(qt/name,bins/name)
  assert sha(qt/'pokelinux-messages')=='4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8'
  for name in ['messages-service','lifecycle-launcher','messages-seed-fixture']:shutil.copyfile(qt/'bins'/name,bins/name)
  for name,digest in bs['bins'].items():assert sha(build/'bins'/name)==digest;shutil.copyfile(build/'bins'/name,bins/name)
  shutil.copytree(qt/'source/native-app',source/'native-app')
  for path in bins.iterdir():path.chmod(0o755)
  vm=MAIN/'artifacts/native-session-installed-app/vm-production';assert (vm/'root.img').exists()
  selection={'phase':'green','test_phase':'green' if a.test_case.endswith('-green') else 'red','test_case':a.test_case,'published_main_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=MAIN,text=True).strip(),'rust_build':a.build or 'combined-read-selector-green-build','fixture_binary':fixture_binary,'rust_selection':bs,'normal_production_qt':True,'expected_hyprland_elf_sha256':comp['hyprland_elf_sha256'],'sample_budget_ns':100000000,'source_revival_permitted':False,'input_vm_lock_mode':'shared-readonly','per_label_lock_exclusive':True,'qemu_smp':4,'container_cpus':6,'container_memory':'8g','owned_vm_root_sha256':sha(vm/'root.img'),'kernel_sha256':sha(MAIN/'sim/boot/Image'),'initrd_sha256':sha(MAIN/'sim/boot/initramfs-linux.img'),'image_id':subprocess.check_output(DOCKER+['image','inspect',IMAGE,'--format','{{.Id}}'],text=True).strip(),'files':{str(p.relative_to(out)):sha(p) for d in [source,bins] for p in sorted(d.rglob('*')) if p.is_file()}}
  name='pokelinux-session-display-app-'+a.label;cid=subprocess.check_output(DOCKER+['create','--name',name,'--pull=never','--network','none','--cpus','6','--memory','8g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(MAIN)+':/workspace:ro','-v',str(vm)+':/vm:ro','-v',str(out)+':/results:rw',IMAGE,'bash','/results/source/boot.sh'],text=True).strip();selection['container_id']=cid;(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
  try:
   with (out/'serial.log').open('w') as output:completed=subprocess.run(DOCKER+['start','-a',cid],stdout=output,stderr=subprocess.STDOUT)
  finally:subprocess.run(DOCKER+['rm','-f',cid],check=True,stdout=subprocess.DEVNULL)
  assert (out/'result.json').exists(),(out/'serial.log').read_text()[-5000:];result=json.loads((out/'result.json').read_text());print(json.dumps({k:result.get(k) for k in ['passed','red_detected','groups','error','availability_unqualified','all_tracked_children_reaped']},indent=2)[:6500])
  assert completed.returncode==0 and result['all_tracked_children_reaped'] and result['setup_complete']
  assert result['actual_focus_and_grant_admission_qualified']
  if a.test_case.endswith('-green'):assert result['passed'] and not result['red_detected'];print('Actual selected actor/display/app GREEN; normal captures still require visual inspection')
  else:assert result['red_detected'] and not result['passed'];print('Actual selected composition desired RED; normal conversation still requires visual inspection')
if __name__=='__main__':main()
