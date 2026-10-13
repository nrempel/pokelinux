#!/usr/bin/env python3
"""Freeze one unchanged shared core and only the explicit synthetic composition."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'artifacts/native-incoming-cold-availability';MAIN=Path('/home/ubuntu/Projects/pokelinux');LEGACY_FROZEN=ROOT/'artifacts/native-incoming-session-app/incoming-session-composed-green-arm-v2/source/rust-build'
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock'];IMAGE='sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6'
CRATES=['native-lifecycle','native-messages','native-session','native-kernel-identity','native-inbox','native-messaging']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--base-root',type=Path,required=True);p.add_argument('--base-head',required=True);p.add_argument('--patch-file',type=Path,required=True);p.add_argument('--patch-sha256',required=True);p.add_argument('--expected-observer-sha256',required=True);p.add_argument('--label',required=True);p.add_argument('--cache-path',type=Path,required=True);a=p.parse_args();assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
 selected=a.base_root.expanduser().resolve()
 head=subprocess.check_output(['git','rev-parse',a.base_head],cwd=selected,text=True).strip()
 assert head==a.base_head=='10ce2f19fde19c5a9eedbfccb01de45a1941dca7','explicit immutable base mismatch'
 patch_file=a.patch_file.expanduser().resolve();patch_bytes=patch_file.read_bytes()
 assert hashlib.sha256(patch_bytes).hexdigest()==a.patch_sha256
 assert patch_bytes.count(b'diff --git ')==1 and patch_bytes.startswith(b'diff --git a/native-session/src/lib.rs b/native-session/src/lib.rs\n')
 assert b'--- a/native-session/src/lib.rs\n+++ b/native-session/src/lib.rs\n' in patch_bytes
 assert all(word not in patch_bytes for word in [b'GIT binary patch',b'new file mode',b'deleted file mode',b'rename from',b'rename to'])
 scratch=a.cache_path.expanduser().resolve()
 if not scratch.is_dir():p.error('--cache-path must name an existing mutable Cargo target directory')
 BASE.mkdir(parents=True,exist_ok=True)
 with (BASE/'build.lock').open('w') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=BASE/a.label;out.mkdir();source=out/'source';source.mkdir()
  # Materialize immutable Git object bytes, not a mutable worktree overlay.
  base_hashes={}
  records=subprocess.check_output(['git','ls-tree','-r','-z',head,'--',*CRATES,'tools/native-messages-incoming-binder'],cwd=selected).split(b'\0')
  for record in records:
   if not record:continue
   attributes,name=record.split(b'\t',1);mode,kind,oid=attributes.split();relative=name.decode()
   assert kind==b'blob' and mode in [b'100644',b'100755'] and not Path(relative).is_absolute() and '..' not in Path(relative).parts
   data=subprocess.check_output(['git','cat-file','blob',oid.decode()],cwd=selected)
   f=source/relative;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes(data);f.chmod(0o755 if mode==b'100755' else 0o644)
   base_hashes[relative]=hashlib.sha256(data).hexdigest();assert sha(f)==base_hashes[relative]
  shutil.copytree(ROOT/'tools/native-incoming-cold-availability',source/'tools/native-incoming-cold-availability',ignore=shutil.ignore_patterns('target','__pycache__'))
  immutable_patch=out/'reviewed-source.patch';immutable_patch.write_bytes(patch_bytes);immutable_patch.chmod(0o444)
  assert sha(source/'native-session/src/lib.rs')=='3f2e9d07b3163dcc057b85bef6d1f353f10b9f207558d5d41386532970da7bb2'
  apply_command=['patch','--batch','--fuzz=0','-p1','-i',str(immutable_patch)]
  dry=subprocess.run([*apply_command,'--dry-run'],cwd=source,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (out/'patch-check.log').write_text(dry.stdout);assert dry.returncode==0
  applied=subprocess.run(apply_command,cwd=source,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (out/'patch-apply.log').write_text(applied.stdout);assert applied.returncode==0
  assert sha(source/'native-session/src/lib.rs')==a.expected_observer_sha256=='610aa41e9c290ee1184ef994b74039485d4b1d485e553124e829312b12c5a220'
  for relative,value in base_hashes.items():
   if relative!='native-session/src/lib.rs':assert sha(source/relative)==value,relative
  assert len([n for n in base_hashes if n.split('/')[0] in CRATES])==66
  (source/'artifacts/sms-binder/frozen-aidl').mkdir(parents=True)
  manifest='/source/tools/native-incoming-cold-availability/current-fixture/Cargo.toml'
  name='messages-session-cold-current-fixture'
  s={'phase':'diagnostic','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'frozen_core_head':head,'selected_core_root':str(selected),'explicit_core_head':a.base_head,'immutable_patch_file':'reviewed-source.patch','immutable_patch_sha256':a.patch_sha256,'core_base_file_hashes':{n:h for n,h in base_hashes.items() if n.split('/')[0] in CRATES},'patched_observer_sha256':a.expected_observer_sha256,'other_current_core_files_unchanged':65,'fixture_scope':'base10ce plus explicit reviewed Observer patch; actual Provider stop/occupied and same bounded nonblocking timing; independent 34 IPC tests are not included in this diagnostic selection','instrumentation_only':False,'production_patch_scope':'native-session/src/lib.rs only','sample_budget_ns':100000000,'source_revival_permitted':False,'target':'aarch64-unknown-linux-gnu','sources':{str(f.relative_to(source)):sha(f) for f in sorted(source.rglob('*')) if f.is_file()},'cache_path':str(scratch),'scratch_target_policy':'explicit mutable Cargo target; retired disk and immutable proof targets must not be written','image_id':IMAGE,'cpus':8,'memory':'12g','cargo_cache':'readonly','rustflags':'-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'}
  (out/'selection.json').write_text(json.dumps(s,indent=2)+'\n')
  script=f'''set -euo pipefail
cargo fmt --manifest-path {manifest} -- --check
cargo clippy --locked --offline --manifest-path {manifest} --bin {name} -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --locked --offline --manifest-path {manifest} --target aarch64-unknown-linux-gnu --bin {name}
cargo build --locked --offline --manifest-path /source/native-lifecycle/Cargo.toml --target aarch64-unknown-linux-gnu --bin lifecycle-launcher
'''
  command=DOCKER+['run','--rm','--pull=never','--name','pokelinux-incoming-cold-patched-build','--network','none','--cpus','8','--memory','12g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(scratch)+':/target:rw','-v',str(MAIN/'sim/rust-cargo')+':/cargo:ro','-v',str(MAIN/'artifacts/sms-binder/frozen-aidl')+':/source/artifacts/sms-binder/frozen-aidl:ro','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/target','-e','CARGO_BUILD_JOBS=8','-e','CARGO_INCREMENTAL=0','--entrypoint','bash',IMAGE,'-c',script]
  (out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
  with (out/'build.log').open('w') as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
  (out/'exit-code').write_text(str(r.returncode)+'\n')
  if r.returncode:print((out/'build.log').read_text()[-5000:]);raise SystemExit(r.returncode)
  bins=out/'bins';bins.mkdir()
  for n in [name,'lifecycle-launcher']:
   f=scratch/'aarch64-unknown-linux-gnu/debug'/n;shutil.copyfile(f,bins/n);(bins/n).chmod(0o755)
  s['bins']={f.name:sha(f) for f in bins.iterdir()};(out/'selection.json').write_text(json.dumps(s,indent=2)+'\n');print(json.dumps({'label':a.label,'bins':s['bins']},indent=2))
if __name__=='__main__':main()
