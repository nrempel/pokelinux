#!/usr/bin/env python3
"""Freeze and run bounded real host storage controls in disposable Docker."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

HERE = Path(__file__).resolve().parent
D = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
     '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
IMAGE = 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args()
    assert not a.output.exists()
    build = json.loads((a.build / 'selection.json').read_text())
    assert build['phase'] == 'green' and build['build_returncode'] == 0
    for name, expected in build['sources'].items():
        assert sha(a.build / 'source' / name) == expected
    for name, expected in build['bins'].items():
        assert sha(a.build / name) == expected
    a.output.mkdir(mode=0o700)
    fixture = a.output / 'fixture.py'
    shutil.copyfile(HERE / 'restart-host.py', fixture)
    fixture.chmod(0o444)
    command = D + ['run', '--rm', '--pull=never', '--name', 'pokelinux-lock-' + a.output.name,
        '--network', 'none', '--cpus', '2', '--memory', '512m', '--pids-limit', '64', '--cap-drop', 'ALL']
    for cap in ['SETUID', 'SETGID', 'CHOWN', 'KILL', 'DAC_OVERRIDE']:
        command += ['--cap-add', cap]
    command += ['--security-opt', 'no-new-privileges', '--read-only', '-e', 'NATIVE_LIFECYCLE_ISOLATED=1']
    for mount in ['/opt:rw,exec,nosuid,nodev', '/run:rw,nosuid,nodev', '/var/lib:rw,nosuid,nodev',
                  '/tmp:rw,nosuid,nodev', '/usr/libexec:rw,exec,nosuid,nodev,mode=755']:
        command += ['--tmpfs', mount]
    command += ['-v', str((a.build / 'host-bins').resolve()) + ':/bins:ro',
        '-v', str(fixture.resolve()) + ':/fixture.py:ro', '-v', str(a.output.resolve()) + ':/evidence:rw',
        IMAGE, 'python3', '/fixture.py']
    selection = {'build_selection_sha256': sha(a.build / 'selection.json'), 'fixture_sha256': sha(fixture),
        'host_bins': {name: expected for name, expected in build['bins'].items() if name.startswith('host-bins/')},
        'network': 'none', 'image': IMAGE, 'UID1000_native_client': True, 'phone_accessed': False}
    (a.output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
    (a.output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    execution = {'selection_sha256': sha(a.output / 'selection.json'), 'command_sha256': sha(a.output / 'command.json'),
                 'started_monotonic_ns': time.monotonic_ns(), 'runner_sha256': sha(__file__)}
    with (a.output / 'run.log').open('x') as log:
        child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        execution['actual_container_cli_pid'] = child.pid
        execution['returncode'] = child.wait()
    execution['completed_monotonic_ns'] = time.monotonic_ns()
    execution['direct_container_cli_wait'] = True
    execution['fixture_unchanged'] = sha(fixture) == selection['fixture_sha256']
    execution['bins_unchanged'] = all(sha(a.build / name) == expected for name, expected in selection['host_bins'].items())
    execution['result_sha256'] = sha(a.output / 'result.json') if (a.output / 'result.json').exists() else None
    (a.output / 'execution.json').write_text(json.dumps(execution, indent=2) + '\n')
    print(json.dumps(execution, indent=2))
    return execution['returncode']


if __name__ == '__main__':
    raise SystemExit(main())
