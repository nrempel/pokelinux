#!/usr/bin/env python3
"""Execute one frozen native-before-original lock-entry label under its RO lock."""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import subprocess
import time

DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
MAIN = Path('/home/ubuntu/Projects/pokelinux')


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
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
    assert selection['phase'] in ['original-lock-restart-red', 'original-lock-restart-green']
    assert selection['native_pixels'] == {'width': 1080, 'height': 2424}
    assert selection['source_budget_ns'] == 100_000_000
    assert not selection['Source_revival_or_reset_permitted']
    assert command[:len(DOCKER)] == DOCKER and selection['network'] == 'none'
    assert not selection['physical_phone_accessed'] and not selection['vendor_runtime_selected']
    assert all(not (root / name).exists() for name in ['execution.json', 'console.log', 'result.json'])
    for name, expected in selection['files'].items():
        assert not Path(name).is_absolute() and '..' not in Path(name).parts
        assert sha(root / name) == expected, name
    image = subprocess.check_output(DOCKER + ['image', 'inspect', '--format', '{{.Id}}', selection['runtime_image']], text=True).strip()
    assert image == selection['runtime_image']
    lockpath = Path(selection['shared_input_lock'])
    assert lockpath == MAIN / 'artifacts/native-session-installed-app/vm-production.lock'
    record = {'selection_sha256': a.selection_sha256, 'command_sha256': a.command_sha256,
              'runner_sha256': sha(__file__), 'runtime_image': image,
              'shared_readonly_lock_held_across_runtime': False, 'direct_container_cli_wait': False}
    with (root / 'output.lock').open('x') as output_lock, lockpath.open('rb') as lock:
        fcntl.flock(output_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        inputs = {'input_root_sha256': MAIN / 'artifacts/native-session-installed-app/vm-production/root.img',
                  'kernel_sha256': MAIN / 'sim/boot/Image', 'initrd_sha256': MAIN / 'sim/boot/initramfs-linux.img'}
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
        record['inputs_unchanged_after_runtime'] = all(sha(path) == selection[key] for key, path in inputs.items())
    result = json.loads((root / 'result.json').read_text()) if (root / 'result.json').exists() else None
    record['result_sha256'] = sha(root / 'result.json') if result is not None else None
    record['green_qualified'] = bool(result and result.get('passed') and result.get('native_lock_admission_qualified')
        and all(g['passed'] for g in result['groups']) and result.get('all_tracked_processes_waited_or_finally_absent'))
    record['expected_red_qualified'] = bool(result and selection['phase'].endswith('-red') and result.get('expected_red_qualified') and result.get('desired_boundary_red') and result.get('all_tracked_processes_waited_or_finally_absent'))
    with (root / 'execution.json').open('x') as output:
        json.dump(record, output, indent=2)
        output.write('\n')
    print(json.dumps(record, indent=2))
    return 0 if record['returncode'] == 0 and record['inputs_unchanged_after_runtime'] and (record['expected_red_qualified'] if selection['phase'].endswith('-red') else record['green_qualified']) else 1


if __name__ == '__main__':
    raise SystemExit(main())
