#!/usr/bin/env python3
"""Actual host-kernel fixture argument ownership check; never joint app evidence."""
import argparse, fcntl, hashlib, json, os, resource, shutil, socket, subprocess, sys
from pathlib import Path
REPO=Path('/') if '--inside' in sys.argv else Path(__file__).resolve().parents[2];BASE=REPO/'artifacts/native-session-installed-app'
D=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
IMAGE='pixel-phone-rust:20261012-b472';RUNTIME='pixel-graphics-build:20261011-b472'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def inside(exe,output):
 assert os.getuid()==0
 resource.setrlimit(resource.RLIMIT_CORE,(0,0))
 with socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET)[0] as receive:
  raw=receive.fileno();args=[exe,'/run/check','/run/check-policy','/run/check-profile','/run/check-launcher','/run/check-raw','/run/check-inbox',str(raw),'0'+str(raw),str(os.getpid()),'0','0123456789abcdef0123456789abcdef']
  child=subprocess.Popen(args,pass_fds=(raw,),env=dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  text=child.communicate(timeout=5)[0].decode(errors='replace')
  result={'same_numeric_descriptor':True,'first_spelling':str(raw),'alias_spelling':'0'+str(raw),'kernel':os.uname().release,'target':'x86_64-unknown-linux-gnu','exit_code':child.returncode,'clean_duplicate_refusal':child.returncode==1 and 'PermissionDenied' in text,'child_reaped':child.poll() is not None,'output':text}
 print(json.dumps(result));return 0
def main():
 if '--inside' in sys.argv:return inside(*sys.argv[2:])
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--label',required=True);p.add_argument('--from-build',default='current-joint-build-2');p.add_argument('--phase',choices=['red','green'],required=True);a=p.parse_args();assert all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 BASE.mkdir(exist_ok=True)
 with (BASE/'fd-ownership.lock').open('w') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=BASE/a.label;assert not out.exists();out.mkdir();source=out/'source';selected=BASE/a.from_build;bs=json.loads((selected/'selection.json').read_text())
  for n,d in bs['sources'].items():assert sha(selected/'source'/n)==d
  shutil.copytree(selected/'source',source)
  # Green overlays only the now-corrected authored fixture entry, no shared core.
  if a.phase=='green':shutil.copyfile(Path(__file__).parent/'fixture/src/bin/service.rs',source/'tools/native-session-installed-app/fixture/src/bin/service.rs')
  shutil.copyfile(__file__,out/'fd-ownership-test.py')
  meta={'phase':a.phase,'selected_core_build':a.from_build,'overlay':['tools/native-session-installed-app/fixture/src/bin/service.rs'] if a.phase=='green' else [],'sources':{str(q.relative_to(source)):sha(q) for q in source.rglob('*') if q.is_file()},'test_driver_sha256':sha(out/'fd-ownership-test.py')}
  (out/'selection.json').write_text(json.dumps(meta,indent=2)+'\n')
  cmd=D+['run','--rm','--pull=never','--name','pokelinux-fd-ownership-build-'+a.label,'--network','none','--cpus','8','--memory','12g','--pids-limit','128','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(out)+':/results:rw','-v',str(REPO/'sim/rust-cargo')+':/cargo:rw','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/results/target','-e','RUSTFLAGS=-C target-feature=+crt-static',IMAGE,'bash','-c','cargo build --offline --locked --manifest-path /source/tools/native-session-installed-app/fixture/Cargo.toml --features isolated-fixture --target x86_64-unknown-linux-gnu --bin messages-session-installed-fixture']
  with (out/'build.log').open('w') as log:subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True)
  exe=out/'target/x86_64-unknown-linux-gnu/debug/messages-session-installed-fixture';meta['executable_sha256']=sha(exe);(out/'selection.json').write_text(json.dumps(meta,indent=2)+'\n')
  execution=subprocess.run(D+['run','--rm','--pull=never','--name','pokelinux-fd-ownership-'+a.label,'--network','none','--cpus','1','--memory','256m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','--read-only','--tmpfs','/run:rw,nosuid,nodev','-e','NATIVE_MESSAGES_ISOLATED=1','-v',str(out)+':/results:ro','-v',str(exe)+':/fixture:ro',RUNTIME,'python3','/results/fd-ownership-test.py','--inside','/fixture','-'],check=True,capture_output=True,text=True)
  result=json.loads(execution.stdout);(out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(execution.stdout,end='');assert result['child_reaped'];assert result['clean_duplicate_refusal']==(a.phase=='green');print('Qualified fixture FD ownership '+a.phase.upper())
if __name__=='__main__':raise SystemExit(main())
