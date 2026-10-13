#!/usr/bin/env python3
"""Freeze actual process tests and helpers without executing the selected tests."""
import argparse
import fcntl
import json
import os
import shutil
import subprocess
from pathlib import Path
from common import BASE, CRATES, DOCKER, MAIN, ROOT, RUST_IMAGE, hashes, image_id, label, sha, write_json


def select_artifacts(path):
    selected = {}
    for line in path.read_text().splitlines():
        event = json.loads(line)
        if event.get('reason') != 'compiler-artifact' or not event.get('executable'):
            continue
        target = event['target']
        name = target['name']
        if name == 'process' and target['kind'] == ['test'] and event['profile']['test']:
            selected['process'] = event['executable']
        elif name in ('session-peer-fixture', 'session-observer-fixture') and not event['profile']['test']:
            selected[name] = event['executable']
    assert set(selected) == {'process', 'session-peer-fixture', 'session-observer-fixture'}, selected
    return selected


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--label', required=True, type=label)
    parser.add_argument('--phase', choices=['red', 'green'], required=True)
    parser.add_argument('--cache-path', required=True, type=Path)
    args = parser.parse_args()
    cache = args.cache_path.resolve()
    assert cache.is_dir() and cache.is_relative_to(Path('/dev/shm')), 'Use an existing, owned mutable RAM cache'
    BASE.mkdir(parents=True, exist_ok=True)
    with (BASE / 'build.lock').open('a') as lock, (cache / 'freshness-build.lock').open('a') as cache_lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fcntl.flock(cache_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        out = BASE / args.label
        out.mkdir()
        source = out / 'source'
        source.mkdir()
        for crate in CRATES:
            shutil.copytree(ROOT / crate, source / crate,
                            ignore=shutil.ignore_patterns('target', '__pycache__', '._*'))
        shutil.copytree(Path(__file__).parent, source / 'tools/native-session-freshness-chain',
                        ignore=shutil.ignore_patterns('__pycache__', '*.pyc'))
        selection = {
            'phase': args.phase, 'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'source_checksums': hashes(source), 'image_id': image_id(RUST_IMAGE),
            'mutable_cache': str(cache), 'cpus': 8, 'memory': '12g', 'network': 'none',
            'scope': 'native-session actual process tests and authored peer/observer helpers; no phone or compositor claim',
            'old_targets_written': False, 'tests_executed_during_build': False,
        }
        write_json(out / 'selection.json', selection)
        script = '''set -euo pipefail
cargo fmt --manifest-path /source/native-session/Cargo.toml -- --check
cargo clippy --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --all-targets -- -D warnings
export RUSTFLAGS='-C target-feature=+crt-static'
cargo build --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --target x86_64-unknown-linux-gnu --bins --message-format=json > /output/host-cargo.jsonl
cargo test --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --target x86_64-unknown-linux-gnu --test process --no-run --message-format=json >> /output/host-cargo.jsonl
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bins --message-format=json > /output/arm-cargo.jsonl
cargo test --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --test process --no-run --message-format=json >> /output/arm-cargo.jsonl
'''
        (out / 'build.sh').write_text(script)
        command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-freshness-build-' + args.label,
            '--network', 'none', '--cpus', '8', '--memory', '12g', '--pids-limit', '256', '--cap-drop', 'ALL',
            '--security-opt', 'no-new-privileges', '--user', f'{os.getuid()}:{os.getgid()}',
            '-v', str(source) + ':/source:ro', '-v', str(cache) + ':/cache:rw',
            '-v', str(MAIN / 'sim/rust-cargo') + ':/cargo:ro', '-v', str(out) + ':/output:rw',
            '-e', 'CARGO_HOME=/cargo', '-e', 'CARGO_TARGET_DIR=/cache', '-e', 'CARGO_BUILD_JOBS=8',
            '--entrypoint', 'bash', selection['image_id'], '/output/build.sh']
        write_json(out / 'command.json', command)
        with (out / 'build.log').open('w') as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        (out / 'exit-code').write_text(str(result.returncode) + '\n')
        if result.returncode:
            print((out / 'build.log').read_text()[-6000:])
            raise SystemExit(result.returncode)
        selection['artifacts'] = {}
        for arch in ('host', 'arm'):
            compiled = select_artifacts(out / (arch + '-cargo.jsonl'))
            selection['artifacts'][arch] = {}
            for role, container_path in compiled.items():
                relative = Path(container_path).relative_to('/cache')
                frozen = out / 'cache-tree' / arch / relative
                frozen.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(cache / relative, frozen)
                frozen.chmod(0o755)
                selection['artifacts'][arch][role] = {
                    'compiled_path': container_path, 'frozen_path': str(frozen.relative_to(out)), 'sha256': sha(frozen)}
                tool = 'aarch64-linux-gnu-readelf' if arch == 'arm' else 'readelf'
                info = subprocess.check_output(DOCKER + ['run', '--rm', '--pull=never', '--network', 'none',
                    '--cpus', '1', '--memory', '256m', '--pids-limit', '32', '--cap-drop', 'ALL',
                    '--security-opt', 'no-new-privileges', '-v', str(frozen) + ':/input:ro',
                    selection['image_id'], tool, '-h', '-l', '-d', '/input'], text=True)
                assert ('AArch64' if arch == 'arm' else 'X86-64') in info and 'INTERP' not in info and '(NEEDED)' not in info
                (out / f'{arch}-{role}-elf.txt').write_text(info)
        write_json(out / 'selection.json', selection)
        print(json.dumps({'label': args.label, 'selection_sha256': sha(out / 'selection.json'), 'artifacts': selection['artifacts']}, indent=2))


if __name__ == '__main__':
    main()
