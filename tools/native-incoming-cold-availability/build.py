#!/usr/bin/env python3
"""Freeze one unchanged shared core and only the explicit synthetic composition."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'artifacts/native-incoming-cold-availability';MAIN=Path('/home/ubuntu/Projects/pokelinux');FROZEN=ROOT/'artifacts/native-incoming-session-app/incoming-session-composed-green-arm-v2/source/rust-build'
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock'];IMAGE='sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6'
CRATES=['native-lifecycle','native-messages','native-session','native-kernel-identity','native-inbox','native-messaging']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--label',required=True);p.add_argument('--cache-path',type=Path,required=True);a=p.parse_args();assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 scratch=a.cache_path.expanduser().resolve()
 if not scratch.is_dir():p.error('--cache-path must name an existing mutable Cargo target directory')
 BASE.mkdir(parents=True,exist_ok=True)
 with (BASE/'build.lock').open('w') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=BASE/a.label;out.mkdir();source=out/'source';source.mkdir()
  for crate in CRATES:shutil.copytree(FROZEN/crate,source/crate,ignore=shutil.ignore_patterns('target','__pycache__'))
  shutil.copytree(FROZEN/'tools/native-messages-incoming-binder',source/'tools/native-messages-incoming-binder',ignore=shutil.ignore_patterns('target','__pycache__'))
  shutil.copytree(ROOT/'tools/native-incoming-cold-availability',source/'tools/native-incoming-cold-availability',ignore=shutil.ignore_patterns('target','__pycache__'))
  (source/'artifacts/sms-binder/frozen-aidl').mkdir(parents=True)
  manifest='/source/tools/native-incoming-cold-availability/fixture/Cargo.toml'
  name='messages-session-cold-timing-fixture'
  s={'phase':'diagnostic','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'frozen_core_head':'518954b112dbd344584f29bcb6d9e503dc1800ac','frozen_failed_epoch':'incoming-session-composed-green-arm-v2','instrumentation_only':True,'sample_budget_ns':100000000,'source_revival_permitted':False,'target':'aarch64-unknown-linux-gnu','sources':{str(f.relative_to(source)):sha(f) for f in sorted(source.rglob('*')) if f.is_file()},'cache_path':str(scratch),'scratch_target_policy':'explicit mutable Cargo target; retired disk and immutable proof targets must not be written','image_id':IMAGE,'cpus':8,'memory':'12g','cargo_cache':'readonly','rustflags':'-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'}
  (out/'selection.json').write_text(json.dumps(s,indent=2)+'\n')
  script=f'''set -euo pipefail
cargo fmt --manifest-path {manifest} -- --check
cargo clippy --locked --offline --manifest-path {manifest} --bin {name} -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --locked --offline --manifest-path {manifest} --target aarch64-unknown-linux-gnu --bin {name}
cargo build --locked --offline --manifest-path /source/native-lifecycle/Cargo.toml --target aarch64-unknown-linux-gnu --bin lifecycle-launcher
'''
  command=DOCKER+['run','--rm','--pull=never','--name','pokelinux-incoming-cold-sol-build','--network','none','--cpus','8','--memory','12g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(scratch)+':/target:rw','-v',str(MAIN/'sim/rust-cargo')+':/cargo:ro','-v',str(MAIN/'artifacts/sms-binder/frozen-aidl')+':/source/artifacts/sms-binder/frozen-aidl:ro','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/target','-e','CARGO_BUILD_JOBS=8','-e','CARGO_INCREMENTAL=0','--entrypoint','bash',IMAGE,'-c',script]
  (out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
  with (out/'build.log').open('w') as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
  (out/'exit-code').write_text(str(r.returncode)+'\n')
  if r.returncode:print((out/'build.log').read_text()[-5000:]);raise SystemExit(r.returncode)
  bins=out/'bins';bins.mkdir()
  for n in [name,'lifecycle-launcher']:
   f=scratch/'aarch64-unknown-linux-gnu/debug'/n;shutil.copyfile(f,bins/n);(bins/n).chmod(0o755)
  s['bins']={f.name:sha(f) for f in bins.iterdir()};(out/'selection.json').write_text(json.dumps(s,indent=2)+'\n');print(json.dumps({'label':a.label,'bins':s['bins']},indent=2))
if __name__=='__main__':main()
