#!/usr/bin/env python3
"""Run a reviewed frozen selection while holding the shared read-only input lock."""
import argparse
import fcntl
import hashlib
import json
import subprocess
import time
from pathlib import Path

DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH',
          'docker', '--host=unix:///var/run/docker.sock']
MAIN = Path('/home/ubuntu/Projects/pokelinux')


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--selection-dir', required=True, type=Path)
    p.add_argument('--selection-sha256', required=True)
    p.add_argument('--command-sha256', required=True)
    a = p.parse_args()
    root = a.selection_dir.resolve()
    assert sha(root / 'selection.json') == a.selection_sha256
    assert sha(root / 'command.json') == a.command_sha256
    selection = json.loads((root / 'selection.json').read_text())
    command = json.loads((root / 'command.json').read_text())
    assert command[:len(DOCKER)] == DOCKER
    assert selection['native_pixels'] == {'width': 1080, 'height': 2424}
    assert not selection['physical_phone_accessed'] and selection['network'] == 'none'
    assert not selection['frozen_vm_or_kernel_modified']
    assert all(not (root / name).exists() for name in ('execution.json', 'console.log', 'result.json'))
    for name, expected in selection['sources'].items():
        assert Path(name).name == name and sha(root / 'source' / name) == expected, name
    image = subprocess.check_output(DOCKER + ['image', 'inspect', '--format', '{{.Id}}', selection['runtime_image']], text=True).strip()
    assert image == selection['runtime_image']
    record = {'selection_sha256': a.selection_sha256, 'command_sha256': a.command_sha256,
              'runner_sha256': sha(__file__), 'runtime_image': image,
              'shared_readonly_lock_held_across_runtime': False,
              'direct_container_cli_wait': False, 'hardware_verified': False}
    lockpath = Path(selection['shared_input_lock'])
    assert lockpath == MAIN / 'artifacts/native-session-installed-app/vm-production.lock'
    with lockpath.open('rb') as lock:
        fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        inputs = {'input_root_sha256': MAIN / 'artifacts/native-session-installed-app/vm-production/root.img',
                  'kernel_sha256': MAIN / 'sim/boot/Image',
                  'initrd_sha256': MAIN / 'sim/boot/initramfs-linux.img'}
        for key, path in inputs.items():
            assert sha(path) == selection[key], key
        record['shared_readonly_lock_held_across_runtime'] = True
        record['runtime_started_monotonic_ns'] = time.monotonic_ns()
        with (root / 'console.log').open('x') as log:
            child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
            record['direct_cli_pid'] = child.pid
            record['returncode'] = child.wait()
        record['runtime_completed_monotonic_ns'] = time.monotonic_ns()
        record['direct_container_cli_wait'] = True
        # Check immutable inputs again under the same lock, after the snapshot exits.
        record['inputs_unchanged_after_runtime'] = all(sha(path) == selection[key] for key, path in inputs.items())
    if (root / 'result.json').exists():
        record['result_sha256'] = sha(root / 'result.json')
        record['guest_result'] = json.loads((root / 'result.json').read_text())
    else:
        record['guest_result'] = None
    with (root / 'execution.json').open('x') as out:
        json.dump(record, out, indent=2)
        out.write('\n')
    print(json.dumps({'execution': str(root / 'execution.json'), 'returncode': record['returncode'],
                      'guest_passed': bool(record['guest_result'] and record['guest_result'].get('passed'))}, indent=2))
    return 0 if record['returncode'] == 0 and record['inputs_unchanged_after_runtime'] and record['guest_result'] and record['guest_result'].get('passed') else 1


if __name__ == '__main__':
    raise SystemExit(main())
