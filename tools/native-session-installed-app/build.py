#!/usr/bin/env python3
"""Snapshot all selected production inputs before building the isolated ARM entry."""
import argparse, fcntl, hashlib, json, os, shutil, subprocess
from pathlib import Path
REPO=Path(__file__).resolve().parents[2];BASE=REPO/'artifacts/native-session-installed-app'
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock'];IMAGE='pixel-phone-rust:20261012-b472'
CRATES=['native-lifecycle','native-messages','native-session','native-kernel-identity','native-inbox','native-messaging']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--label',required=True);p.add_argument('--from-build');a=p.parse_args();assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 BASE.mkdir(parents=True,exist_ok=True)
 with (BASE/'build.lock').open('w') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=BASE/a.label;assert not out.exists(),'preserve previous build';out.mkdir();source=out/'source';source.mkdir()
  core=REPO
  if a.from_build:
   previous=BASE/a.from_build;old=json.loads((previous/'selection.json').read_text())
   for n,d in old['sources'].items():assert sha(previous/'source'/n)==d
   core=previous/'source'
  for n in CRATES:shutil.copytree(core/n,source/n,ignore=shutil.ignore_patterns('target','__pycache__','._*'))
  shutil.copytree(Path(__file__).parent,source/'tools/native-session-installed-app',ignore=shutil.ignore_patterns('target','__pycache__'))
  selection={'target':'aarch64-unknown-linux-gnu','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip(),'source_selection':'current shared core snapshot; no overlays' if not a.from_build else 'explicit unchanged shared core from '+a.from_build,'selected_core_build':a.from_build,'chronological_red':'red-before-composition-3','sources':{str(q.relative_to(source)):sha(q) for q in sorted(source.rglob('*')) if q.is_file()},'image_id':subprocess.check_output(DOCKER+['image','inspect',IMAGE,'--format','{{.Id}}'],text=True).strip(),'rustflags':'-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc','container_cpus':8,'container_memory':'12g'}
  (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
  script='''set -euo pipefail
cargo fmt --manifest-path /source/tools/native-session-installed-app/fixture/Cargo.toml -- --check
cargo clippy --offline --locked --manifest-path /source/tools/native-session-installed-app/fixture/Cargo.toml --features isolated-fixture -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --offline --locked --manifest-path /source/tools/native-session-installed-app/fixture/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-session-installed-fixture
cargo build --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --target aarch64-unknown-linux-gnu --bin lifecycle-launcher
cargo build --offline --locked --manifest-path /source/native-messages/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-seed-fixture
'''
  command=DOCKER+['run','--rm','--pull=never','--name','pokelinux-installed-app-build-'+a.label,'--network','none','--cpus','8','--memory','12g','--pids-limit','128','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(out)+':/results:rw','-v',str(REPO/'sim/rust-cargo')+':/cargo:rw','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/results/target','--entrypoint','bash',IMAGE,'-c',script]
  with (out/'build.log').open('w') as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
  (out/'exit-code').write_text(str(r.returncode)+'\n')
  if r.returncode:print((out/'build.log').read_text()[-5000:]);raise SystemExit(r.returncode)
  bins=out/'bins';bins.mkdir()
  for n in ['messages-session-installed-fixture','lifecycle-launcher','messages-seed-fixture']:
   shutil.copyfile(out/'target/aarch64-unknown-linux-gnu/debug'/n,bins/n);(bins/n).chmod(0o755)
   info=subprocess.check_output(DOCKER+['run','--rm','--pull=never','--network','none','--cpus','1','--memory','128m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','--read-only','--user',f'{os.getuid()}:{os.getgid()}','-v',str(bins/n)+':/probe:ro','--entrypoint','aarch64-linux-gnu-readelf',IMAGE,'-h','-l','-d','/probe'],text=True);assert 'AArch64' in info and 'INTERP' not in info and '(NEEDED)' not in info;(out/(n+'-elf.txt')).write_text(info)
  selection['bins']={q.name:sha(q) for q in bins.iterdir()};(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n');print(json.dumps({'build':a.label,'bins':selection['bins']},indent=2))
if __name__=='__main__':main()
