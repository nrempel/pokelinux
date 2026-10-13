#!/usr/bin/env python3
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'artifacts/native-incoming-session-app';MAIN=Path('/home/ubuntu/Projects/pokelinux');VM=Path('/tmp/pokelinux-receive-isolation-sol/artifacts/native-messages-receive-isolation/vm')
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock'];IMAGE='sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while c:=f.read(1048576):h.update(c)
 return h.hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--phase',choices=['red','green'],required=True);p.add_argument('--label',required=True);p.add_argument('--build',required=True);p.add_argument('--case',choices=['full-matrix','source-loss'],default='full-matrix');a=p.parse_args();assert a.phase=='green' or a.case=='full-matrix';assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 with (BASE/'vm-output.lock').open('w') as output_lock,(VM.parent/'vm-image.lock').open('a') as input_lock:
  fcntl.flock(output_lock,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(input_lock,fcntl.LOCK_SH|fcntl.LOCK_NB)
  out=BASE/a.label;out.mkdir();bins=out/'bins';bins.mkdir();source=out/'source';source.mkdir();project=out/'project';(project/'tools/native-messages-receive-isolation').mkdir(parents=True)
  for name in ['boot.sh','guest.py','run.py']:shutil.copyfile(Path(__file__).parent/name,source/name)
  # Existing immutable receiver boot inode invokes this facade; no backing inode changes.
  shutil.copyfile(Path(__file__).parent/'guest.py',project/'tools/native-messages-receive-isolation/guest.py')
  build=BASE/a.build;bs=json.loads((build/'selection.json').read_text());assert bs['phase']==a.phase
  for name,value in bs['sources'].items():assert sha(build/'source'/name)==value
  for name,value in bs['bins'].items():assert sha(build/'bins'/name)==value;shutil.copyfile(build/'bins'/name,bins/name)
  shutil.copytree(build/'source',source/'rust-build')
  compositor=MAIN/'artifacts/native-session-compositor/green-owned-source-final';cs=json.loads((compositor/'result.json').read_text());assert cs['passed']
  for name in ['adapter.so','probe.so']:shutil.copyfile(compositor/'bins'/name,bins/name)
  assert sha(bins/'adapter.so')==cs['adapter_elf_sha256']
  qt=MAIN/'artifacts/native-messages/desktop/green-10';qs=json.loads((qt/'result.json').read_text());assert qs['passed'];assert sha(qt/'pokelinux-messages')==qs['production_binary']['pokelinux-messages'];shutil.copyfile(qt/'pokelinux-messages',bins/'pokelinux-messages');shutil.copytree(qt/'source/native-app',source/'native-app')
  peer=MAIN/'artifacts/native-messages-incoming-binder/selections/incoming-green-final-v2/binaries'
  pins={'incoming-peer':'d3ace7cd833d10aa81217bae20d7a8765d06c3160167dbf69d848a9ce72560f4','property-info':'7c81ebd6b1b2bde547675aed2f6ee4cdc0e0b2a4892160c5af08b92b0d0f4f85','property-init':'27ba656934837f802ee05a7817ab83dd25fc1bbac57bd491a269dc5d338067a2','incoming-manager':'40395c1363de8057735ae0a883f6d9498a73e7a4f2a119bf1157332a5eec6dbd','incoming-probe':'270195bacdabd702a564bb2eb8a9f011bafd045e086afc7c1f5a16074dab5992'}
  for name,value in pins.items():assert sha(peer/name)==value;shutil.copyfile(peer/name,bins/name)
  for path in bins.iterdir():path.chmod(0o755)
  s={'phase':a.phase,'case':a.case,'build':a.build,'core_head':bs['head'],'build_selection_sha256':sha(build/'selection.json'),'sample_budget_ns':100000000,'source_revival_permitted':False,'normal_production_qt':True,'expected_hyprland_elf_sha256':cs['hyprland_elf_sha256'],'peer_inputs':pins,'vm_backing':'existing immutable receiver vm/root.img','vm_backing_sha256':sha(VM/'root.img'),'kernel_sha256':sha(MAIN/'sim/boot/Image'),'initrd_sha256':sha(MAIN/'sim/boot/initramfs-linux.img'),'cpus':6,'memory':'8g','guest_vcpus':4,'network':'none','snapshot':True,'input_lock':'shared; preparers must acquire exclusive','files':{str(f.relative_to(out)):sha(f) for parent in [bins,source,project] for f in sorted(parent.rglob('*')) if f.is_file()}}
  (out/'selection.json').write_text(json.dumps(s,indent=2)+'\n')
  command=DOCKER+['run','--rm','--pull=never','--name','pokelinux-incoming-session-sol-vm','--network','none','--cpus','6','--memory','8g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(out)+':/results:rw','-v',str(project)+':/project:ro','-v',str(VM)+':/vm:ro','-v',str(MAIN/'sim/boot')+':/kernel:ro','-v',str(MAIN/'artifacts/runtime')+':/android-runtime:ro',IMAGE,'bash','/results/source/boot.sh']
  (out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
  with (out/'serial.log').open('w') as log:run=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
  (out/'exit-code').write_text(str(run.returncode)+'\n')
  r=json.loads((out/'result.json').read_text());print(json.dumps(r,indent=2));assert run.returncode==0 and r['all_tracked_children_reaped']
  if a.phase=='red':assert r['expected_red'] and not r['passed'];raise SystemExit(1)
  assert r['passed']
if __name__=='__main__':main()
