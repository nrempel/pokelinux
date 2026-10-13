#!/usr/bin/env python3
"""Freeze exact current sources and build a private ARM tracer composition offline."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
IMAGE = 'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS', '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
FIXTURE = Path('tools/native-phone-tracer/fixture')
PRIVATE = Path('/dev/shm/pokelinux-native-phone-tracer-build-sol')


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        while data := f.read(65536):
            h.update(data)
    return h.hexdigest()


def bindings(directory):
    return {str(f.relative_to(directory)): {'bytes': f.stat().st_size, 'sha256': sha(f)} for f in sorted(directory.rglob('*')) if f.is_file() and not f.is_symlink()}


def closure(start):
    result = set()
    pending = [start]
    while pending:
        p = pending.pop().resolve()
        if p in result:
            continue
        assert p.is_relative_to(ROOT)
        result.add(p)
        manifest = tomllib.loads((p/'Cargo.toml').read_text())
        tables = [manifest]
        tables.extend(manifest.get('target', {}).values())
        for table in tables:
            for kind in ('dependencies', 'build-dependencies', 'dev-dependencies'):
                for dependency in table.get(kind, {}).values():
                    if isinstance(dependency, dict) and 'path' in dependency:
                        pending.append(p/dependency['path'])
    return sorted(result)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--label', required=True)
    p.add_argument('--phase', choices=['prepare', 'check-build'], required=True)
    args = p.parse_args()
    assert args.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.label)
    os.umask(0o077)
    PRIVATE.mkdir(mode=0o700, exist_ok=True)
    out = PRIVATE/args.label
    fixture_manifest = '/source/tools/native-phone-tracer/fixture/Cargo.toml'
    if args.phase == 'prepare':
        out.mkdir(mode=0o700)
        source = out/'source'
        source.mkdir()
        core_head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        crates = closure(ROOT/FIXTURE)
        tracked = [str(x.relative_to(ROOT)) for x in crates if x != ROOT/FIXTURE]
        assert not subprocess.check_output(['git', 'diff', '--name-only', 'HEAD', '--', *tracked], cwd=ROOT, text=True).strip(), 'selected tracked crate closure changed'
        for crate in crates:
            dest = source/crate.relative_to(ROOT)
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(crate, dest, ignore=shutil.ignore_patterns('target', '__pycache__'))
        for relative in [Path('tools/native-messages-incoming-binder/aidl')]:
            shutil.copytree(ROOT/relative, source/relative)
        idl = MAIN/'artifacts/sms-binder/frozen-aidl'
        shutil.copytree(idl, source/'artifacts/sms-binder/frozen-aidl')
        for crate in crates:
            for f in (source/crate.relative_to(ROOT)).rglob('*'):
                if f.is_file():
                    assert sha(f) == sha(ROOT/f.relative_to(source)), 'source changed during freeze'
        before = bindings(source)
        selection = dict(schema=1,core_head=core_head,root=str(ROOT),fixture_scope='Test-only existing per-app Factory/current lifecycle/frozen V3 incoming composition; no production modem selector',crates=[str(x.relative_to(ROOT)) for x in crates],sources_before_lock_resolution=before,original_frozen_idl={'source':str(idl),'files':bindings(idl)},image=IMAGE,resources={'cpus':8,'memory':'12g','pids_limit':512,'cargo_jobs':8},cargo_cache='read-only existing exact registry',target='aarch64-unknown-linux-gnu',no_device_access=True,no_production_source_change=True)
        (out/'build-helper.py').write_bytes(Path(__file__).read_bytes())
        (out/'selection-before-resolution.json').write_text(json.dumps(selection,indent=2)+'\n')
        script=f'''set -euo pipefail
rustc -Vv
cargo -V
rustfmt -V
aarch64-linux-gnu-gcc --version
cargo generate-lockfile --offline --manifest-path {fixture_manifest}
cargo metadata --locked --offline --manifest-path {fixture_manifest} --filter-platform aarch64-unknown-linux-gnu --format-version 1 > /proof/cargo-metadata.json
'''
        writable_fixture=True
        phase='prepare'
    else:
        assert out.is_dir()
        source=out/'source'
        selection=json.loads((out/'selection-before-resolution.json').read_text())
        before=selection['sources_before_lock_resolution']
        now=bindings(source)
        allowed='tools/native-phone-tracer/fixture/Cargo.lock'
        assert {k:v for k,v in now.items() if k!=allowed}=={k:v for k,v in before.items() if k!=allowed}, 'frozen source changed outside its newly resolved fixture lock'
        selection['sources_for_build']=now
        selection['fixture_lock_sha256']=sha(source/FIXTURE/'Cargo.lock')
        (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
        script=f'''set -euo pipefail
cargo fmt --manifest-path {fixture_manifest} -- --check
cargo clippy --locked --offline --manifest-path {fixture_manifest} --features isolated-fixture --bin messages-phone-tracer -- -D warnings
cargo clippy --locked --offline --manifest-path {fixture_manifest} -p pokelinux-lifecycle --bin lifecycle-launcher -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc -C link-arg=-fuse-ld=bfd'
cargo build --locked --offline --manifest-path {fixture_manifest} --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-phone-tracer --message-format=json > /proof/fixture-build-artifacts.jsonl
cargo build --locked --offline --manifest-path {fixture_manifest} -p pokelinux-lifecycle --target aarch64-unknown-linux-gnu --bin lifecycle-launcher --message-format=json > /proof/launcher-build-artifacts.jsonl
cargo build --locked --offline --manifest-path {fixture_manifest} -p pokelinux-messages --target aarch64-unknown-linux-gnu --bin messages-service --message-format=json > /proof/baseline-build-artifacts.jsonl
'''
        writable_fixture=False
        phase='check-build'
    target=PRIVATE/'mutable-target'
    target.mkdir(mode=0o700,exist_ok=True)
    command=DOCKER+['run','--rm','--pull=never','--name','pokelinux-native-phone-tracer-'+args.label+'-'+phase,'--network','none','--cpus','8','--memory','12g','--pids-limit','512','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',str(source)+':/source:ro','-v',str(target)+':/target:rw','-v',str(out)+':/proof:rw','-v',str(MAIN/'sim/rust-cargo')+':/cargo:ro','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/target','-e','CARGO_BUILD_JOBS=8','-e','CARGO_INCREMENTAL=0','-e','RUSTFLAGS=-C link-arg=-fuse-ld=bfd','-e','CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc','-e','AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar']
    if writable_fixture:
        command += ['-v',str(source/FIXTURE)+':/source/'+str(FIXTURE)+':rw']
    command+=['--entrypoint','bash',IMAGE,'-c',script]
    (out/(phase+'-command.json')).write_text(json.dumps(command,indent=2)+'\n')
    with (out/(phase+'.log')).open('wb') as log:
        result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
    (out/(phase+'-exit-code')).write_text(str(result.returncode)+'\n')
    print(json.dumps({'out':str(out),'phase':phase,'exit':result.returncode,'log':str(out/(phase+'.log'))}))
    if result.returncode:
        print((out/(phase+'.log')).read_text()[-7000:])
        raise SystemExit(result.returncode)
    if args.phase=='prepare':
        shutil.copyfile(source/FIXTURE/'Cargo.lock',ROOT/FIXTURE/'Cargo.lock')
    else:
        assert bindings(source)==selection['sources_for_build'], 'read-only frozen build source changed'
        bins=out/'bins';bins.mkdir()
        for name in ['messages-phone-tracer','lifecycle-launcher','messages-service']:
            dest=bins/name
            shutil.copyfile(target/'aarch64-unknown-linux-gnu/debug'/name,dest)
            dest.chmod(0o755)
        selection['bins']=bindings(bins)
        (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
        print(json.dumps({'bins':str(bins),'binding':selection['bins']}))


if __name__=='__main__':
    main()
