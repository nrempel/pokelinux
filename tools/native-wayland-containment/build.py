#!/usr/bin/env python3
"""Immutable offline build of the native Rust security-context driver."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];MAIN=Path('/home/ubuntu/Projects/pokelinux')
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser();p.add_argument('--label',required=True);p.add_argument('--cache-path',type=Path);a=p.parse_args();assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
    base=ROOT/'artifacts/native-wayland-containment';base.mkdir(parents=True,exist_ok=True)
    with (base/'build.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=base/a.label;assert not out.exists();out.mkdir();source=out/'source';source.mkdir()
        for n in ['native-display','native-kernel-identity']:shutil.copytree(ROOT/n,source/n,ignore=shutil.ignore_patterns('target','._*','__pycache__'))
        image='pixel-phone-rust:20261012-b472';cache=(a.cache_path or base/'mutable-build-target').resolve();cache.mkdir(exist_ok=True)
        selection={'target':'aarch64-unknown-linux-gnu','sources':{str(p.relative_to(source)):sha(p) for p in sorted(source.rglob('*')) if p.is_file()},'image_id':subprocess.check_output(DOCKER+['image','inspect',image,'--format','{{.Id}}'],text=True).strip(),'cpus':8,'memory':'12g','mutable_build_cache':str(cache)+'; selected sources and executables are separately frozen','chronological_red':'original-direct-app-red-1'};(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
        script='''set -euo pipefail
cargo fmt --manifest-path /source/native-display/Cargo.toml -- --check
cargo clippy --offline --locked --manifest-path /source/native-display/Cargo.toml --all-targets --features isolated-fixture -- -D warnings
RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc' cargo build --offline --locked --manifest-path /source/native-display/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bins
'''
        with (out/'build.log').open('w') as log:
            result=subprocess.run(DOCKER+['run','--rm','--pull=never','--name','pokelinux-display-build-'+a.label,'--network','none','--cpus','8','--memory','12g','--pids-limit','128','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(out)+':/results:rw','-v',str(cache)+':/cache:rw','-v',str(MAIN/'sim/rust-cargo')+':/cargo:rw','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/cache','--entrypoint','bash',image,'-c',script],stdout=log,stderr=subprocess.STDOUT)
        (out/'exit-code').write_text(str(result.returncode)+'\n')
        if result.returncode:print((out/'build.log').read_text()[-5000:]);raise SystemExit(result.returncode)
        bins=out/'bins';bins.mkdir();selection['executables']={}
        for name in ['display-context-fixture','display-supervisor-fixture']:
            exe=bins/name;shutil.copyfile(cache/'aarch64-unknown-linux-gnu/debug'/name,exe);exe.chmod(0o755)
            selection['executables'][name]=sha(exe);info=subprocess.check_output(DOCKER+['run','--rm','--pull=never','--network','none','--cpus','1','--memory','256m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','--read-only','-v',str(exe)+':/input:ro',image,'aarch64-linux-gnu-readelf','-h','-l','-d','/input'],text=True);assert 'AArch64' in info and 'INTERP' not in info and '(NEEDED)' not in info;(out/(name+'-elf.txt')).write_text(info)
        selection['supervisor_chronological_red']='original-supervisor-control-red-1; actual held current protocol helper before supervisor implementation'
        (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n');print(json.dumps(selection['executables']))
if __name__=='__main__':main()
