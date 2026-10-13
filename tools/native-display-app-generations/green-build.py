#!/usr/bin/env python3
"""Freeze future production app-slot code and actual fixture; never execute tests."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];MAIN=Path('/home/ubuntu/Projects/pokelinux');BASE=ROOT/'artifacts/native-display-app-generations'
D=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
IMAGE='pixel-phone-rust:20261012-b472';SOURCE='610aa41e9c290ee1184ef994b74039485d4b1d485e553124e829312b12c5a220'
IMAGE_ID='sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6'
CRATES=['native-display','native-lifecycle','native-session','native-messages','native-kernel-identity','native-inbox','native-messaging']
def sha(p):
 h=hashlib.sha256()
 with Path(p).open('rb') as f:
  while b:=f.read(1048576):h.update(b)
 return h.hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--label',required=True);p.add_argument('--core-path',required=True,type=Path);p.add_argument('--cache-path',required=True,type=Path);a=p.parse_args()
 assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 core=a.core_path.resolve();cache=a.cache_path.resolve();assert cache.is_dir() and cache.is_relative_to('/dev/shm')
 assert sha(core/'native-session/src/lib.rs')==SOURCE,'Explicit corrected Source610aa is required; never relabel the old runtime'
 assert (core/'native-lifecycle/src/display.rs').is_file() and (core/'native-display/src/listener.rs').is_file()
 original_sources={str(q.relative_to(core)):sha(q) for n in CRATES for q in sorted((core/n).rglob('*')) if q.is_file() and not any(part in ['target','__pycache__'] or part.startswith('._') for part in q.relative_to(core).parts)}
 image_id=subprocess.check_output(D+['image','inspect',IMAGE,'--format','{{.Id}}'],text=True).strip();assert image_id==IMAGE_ID,(image_id,IMAGE_ID)
 BASE.mkdir(exist_ok=True)
 with (BASE/'green-build.lock').open('a') as l,(cache/'app-generation-build.lock').open('a') as c:
  fcntl.flock(l,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(c,fcntl.LOCK_EX|fcntl.LOCK_NB);out=BASE/a.label;out.mkdir();source=out/'source';source.mkdir()
  for n in CRATES:shutil.copytree(core/n,source/n,ignore=shutil.ignore_patterns('target','__pycache__','._*'))
  after_sources={str(q.relative_to(core)):sha(q) for n in CRATES for q in sorted((core/n).rglob('*')) if q.is_file() and not any(part in ['target','__pycache__'] or part.startswith('._') for part in q.relative_to(core).parts)}
  assert after_sources==original_sources,'Production core changed during freeze; this label is unselected'
  assert all(sha(source/n)==value for n,value in original_sources.items()),'Frozen source differs from selected core'
  fixture=source/'tools/native-display-app-generations/app-context-fixture';shutil.copytree(Path(__file__).parent/'app-context-fixture',fixture,ignore=shutil.ignore_patterns('target','__pycache__'))
  for q in sorted(Path(__file__).parent.glob('green-*')):
   if q.is_file():dst=source/'tools/native-display-app-generations'/q.name;shutil.copyfile(q,dst)
  selected={'core_checkout':str(core),'core_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=core,text=True).strip(),'core_source_checksums':original_sources,'core_snapshot_verified_unchanged':True,'source_library_sha256':SOURCE,'scope':'new production per-app AppSlot+Context ownership, independent plain Observer and actual normalQt fixture; no runtime result at build','sources':{str(q.relative_to(source)):sha(q) for q in sorted(source.rglob('*')) if q.is_file()},'image_id':image_id,'cpus':8,'memory':'12g','network':'none','mutable_RAM_cache':str(cache),'CARGO_INCREMENTAL':'0','old_RED_rebuilt_or_relabelled':False}
  (out/'selection.json').write_text(json.dumps(selected,indent=2)+'\n')
  script='''set -euo pipefail
cargo fmt --manifest-path /source/tools/native-display-app-generations/app-context-fixture/Cargo.toml -- --check
cargo clippy --offline --locked --manifest-path /source/tools/native-display-app-generations/app-context-fixture/Cargo.toml --features isolated-fixture -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --offline --locked --manifest-path /source/tools/native-display-app-generations/app-context-fixture/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-app-context-fixture
cargo build --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --target aarch64-unknown-linux-gnu --bin lifecycle-launcher
cargo build --offline --locked --manifest-path /source/native-messages/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-seed-fixture
'''
  (out/'build.sh').write_text(script)
  command=D+['run','--rm','--pull=never','--name','pokelinux-app-generation-build-'+a.label,'--network','none','--cpus','8','--memory','12g','--pids-limit','256','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(cache)+':/cache:rw','-v',str(MAIN/'sim/rust-cargo')+':/cargo:ro','-v',str(out)+':/output:rw','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/cache','-e','CARGO_BUILD_JOBS=8','-e','CARGO_INCREMENTAL=0','--entrypoint','bash',selected['image_id'],'/output/build.sh']
  (out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
  with (out/'build.log').open('w') as log:done=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
  (out/'exit-code').write_text(str(done.returncode)+'\n')
  if done.returncode:print((out/'build.log').read_text()[-6000:]);raise SystemExit(done.returncode)
  bins=out/'bins';bins.mkdir();selected['bins']={}
  for n in ['messages-app-context-fixture','lifecycle-launcher','messages-seed-fixture']:
   q=bins/n;shutil.copyfile(cache/'aarch64-unknown-linux-gnu/debug'/n,q);q.chmod(0o755);selected['bins'][n]=sha(q)
   info=subprocess.check_output(D+['run','--rm','--pull=never','--network','none','--cpus','1','--memory','256m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','-v',str(q)+':/input:ro',selected['image_id'],'aarch64-linux-gnu-readelf','-h','-l','-d','/input'],text=True);assert 'AArch64' in info and 'INTERP' not in info and '(NEEDED)' not in info;(out/(n+'-elf.txt')).write_text(info)
  (out/'selection.json').write_text(json.dumps(selected,indent=2)+'\n');print(json.dumps(selected['bins'],indent=2))
if __name__=='__main__':main()
