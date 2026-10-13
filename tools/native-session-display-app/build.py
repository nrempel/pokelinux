#!/usr/bin/env python3
"""Freeze current shared code and one isolated display/session fixture."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];MAIN=Path('/home/ubuntu/Projects/pokelinux')
BASE=ROOT/'artifacts/native-session-display-app'
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
CRATES=['native-lifecycle','native-messages','native-session','native-display','native-kernel-identity','native-inbox','native-messaging']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser();p.add_argument('--label',required=True);p.add_argument('--cache-path',type=Path,required=True);a=p.parse_args()
    assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
    BASE.mkdir(parents=True,exist_ok=True)
    with (BASE/'build.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=BASE/a.label;assert not out.exists();out.mkdir();source=out/'source';source.mkdir()
        for crate in CRATES:shutil.copytree(ROOT/crate,source/crate,ignore=shutil.ignore_patterns('target','__pycache__','._*'))
        shutil.copytree(Path(__file__).parent/'fixture',source/'tools/native-session-display-app/fixture',ignore=shutil.ignore_patterns('target'))
        image='pixel-phone-rust:20261012-b472';cache=a.cache_path.resolve();assert cache.is_dir()
        selection={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'sources':{str(q.relative_to(source)):sha(q) for q in sorted(source.rglob('*')) if q.is_file()},'image_id':subprocess.check_output(DOCKER+['image','inspect',image,'--format','{{.Id}}'],text=True).strip(),'cpus':8,'memory':'12g','network':'none','mutable_cache':str(cache),'frozen_old_targets_rebuilt':False,'source_lifecycle':'required source stop/occupied; actor admission fencing and actual reaping under same original two-second cleanup deadline','chronological_resource_red':'original-composite-held-policy-stop-resource-red-1'}
        (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
        script='''set -euo pipefail
cargo fmt --manifest-path /source/tools/native-session-display-app/fixture/Cargo.toml -- --check
cargo clippy --offline --locked --manifest-path /source/tools/native-session-display-app/fixture/Cargo.toml --features isolated-fixture -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --offline --locked --manifest-path /source/tools/native-session-display-app/fixture/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-session-display-fixture
cargo build --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --target aarch64-unknown-linux-gnu --bin lifecycle-launcher
cargo build --offline --locked --manifest-path /source/native-messages/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-seed-fixture
'''
        with (out/'build.log').open('w') as log:
            result=subprocess.run(DOCKER+['run','--rm','--pull=never','--name','pokelinux-display-session-build-'+a.label,'--network','none','--cpus','8','--memory','12g','--pids-limit','256','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(cache)+':/cache:rw','-v',str(MAIN/'sim/rust-cargo')+':/cargo:ro','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/cache','-e','CARGO_BUILD_JOBS=8','--entrypoint','bash',image,'-c',script],stdout=log,stderr=subprocess.STDOUT)
        (out/'exit-code').write_text(str(result.returncode)+'\n')
        if result.returncode:print((out/'build.log').read_text()[-5000:]);raise SystemExit(result.returncode)
        bins=out/'bins';bins.mkdir();selection['bins']={}
        for name in ['messages-session-display-fixture','lifecycle-launcher','messages-seed-fixture']:
            exe=bins/name;shutil.copyfile(cache/'aarch64-unknown-linux-gnu/debug'/name,exe);exe.chmod(0o755);selection['bins'][name]=sha(exe)
            info=subprocess.check_output(DOCKER+['run','--rm','--pull=never','--network','none','--cpus','1','--memory','256m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','-v',str(exe)+':/input:ro',image,'aarch64-linux-gnu-readelf','-h','-l','-d','/input'],text=True);assert 'AArch64' in info and 'INTERP' not in info and '(NEEDED)' not in info;(out/(name+'-elf.txt')).write_text(info)
        (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n');print(json.dumps(selection['bins']))
if __name__=='__main__':main()
