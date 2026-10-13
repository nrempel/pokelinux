#!/usr/bin/env python3
"""Freeze and build one immutable lock-admission source selection offline."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
IMAGE = 'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6'
D = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
     '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
CRATES = ['native-display', 'native-lifecycle', 'native-session', 'native-messages',
          'native-kernel-identity', 'native-inbox', 'native-messaging', 'native-camera']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--phase', choices=['red', 'green'], required=True)
    a = p.parse_args()
    assert not a.output.exists()
    a.output.mkdir(mode=0o700)
    source = a.output / 'source'
    source.mkdir()
    for name in CRATES:
        shutil.copytree(ROOT / name, source / name, ignore=shutil.ignore_patterns('target', '__pycache__', '._*'))
    fixture = Path('tools/native-display-app-generations/app-context-fixture')
    shutil.copytree(ROOT / fixture, source / fixture, ignore=shutil.ignore_patterns('target', '__pycache__'))
    selected = {'phase': a.phase, 'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                'image': IMAGE, 'network': 'none', 'source_budget_ns': 100_000_000,
                'sources': {str(q.relative_to(source)): sha(q) for q in sorted(source.rglob('*')) if q.is_file()}}
    assert selected['sources']['native-session/src/lib.rs'] == '610aa41e9c290ee1184ef994b74039485d4b1d485e553124e829312b12c5a220'
    script = '''set -euo pipefail
export CARGO_HOME=/cargo CARGO_TARGET_DIR=/cache CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0
cargo fmt --manifest-path /source/native-lifecycle/Cargo.toml -- --check
cargo clippy --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --all-targets -- -D warnings
cargo build --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --bins
mkdir -p /output/host-bins /output/arm-bins
cp /cache/debug/lifecycle-* /output/host-bins/
export RUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc'
cargo build --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --target aarch64-unknown-linux-gnu --bins
cp /cache/aarch64-unknown-linux-gnu/debug/lifecycle-* /output/arm-bins/
cargo build --offline --locked --manifest-path /source/tools/native-display-app-generations/app-context-fixture/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-app-context-fixture
cargo build --offline --locked --manifest-path /source/native-messages/Cargo.toml --features isolated-fixture --target aarch64-unknown-linux-gnu --bin messages-seed-fixture
cp /cache/aarch64-unknown-linux-gnu/debug/messages-app-context-fixture /cache/aarch64-unknown-linux-gnu/debug/messages-seed-fixture /output/arm-bins/
'''
    (a.output / 'build.sh').write_text(script)
    command = D + ['run', '--rm', '--pull=never', '--name', 'pokelinux-lock-build-' + a.output.name,
        '--network', 'none', '--cpus', '8', '--memory', '12g', '--pids-limit', '256', '--cap-drop', 'ALL',
        '--security-opt', 'no-new-privileges', '--user', f'{os.getuid()}:{os.getgid()}',
        '--tmpfs', '/cache:rw,exec,nosuid,nodev,size=4g,mode=1777',
        '-v', str(source.resolve()) + ':/source:ro', '-v', str(MAIN / 'sim/rust-cargo') + ':/cargo:ro',
        '-v', str(a.output.resolve()) + ':/output:rw', '--entrypoint', 'bash', IMAGE, '/output/build.sh']
    (a.output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    (a.output / 'selection.json').write_text(json.dumps(selected, indent=2) + '\n')
    with (a.output / 'build.log').open('x') as log:
        child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        code = child.wait()
    selected['build_returncode'] = code
    selected['build_cli_actually_waited'] = True
    selected['build_log_sha256'] = sha(a.output / 'build.log')
    selected['bins'] = {str(q.relative_to(a.output)): sha(q) for tree in ['host-bins', 'arm-bins']
                        for q in sorted((a.output / tree).glob('*')) if q.is_file() and q.suffix != '.d'}
    (a.output / 'selection.json').write_text(json.dumps(selected, indent=2) + '\n')
    print(json.dumps({'returncode': code, 'selection_sha256': sha(a.output / 'selection.json'), 'bins': selected['bins']}, indent=2))
    return code


if __name__ == '__main__':
    raise SystemExit(main())
