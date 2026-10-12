#!/usr/bin/env python3
"""Run exact native ARM64 radio and messaging test artifacts as unprivileged alarm."""
import hashlib
import json
from pathlib import Path
import platform
import resource
import subprocess
import traceback

out = Path('/results')
result = {'passed': False, 'architecture': platform.machine(), 'qemu_mode': 'system',
          'hardware_verified': False, 'phone_accessed': False, 'kernel': platform.release(),
          'os': Path('/etc/os-release').read_text(), 'checks': {}, 'binary_sha256': {}}

def execute(crate, manifest, name, process_tests):
    rows = [json.loads(line) for line in Path('/workspace/artifacts', crate, manifest).read_text().splitlines()]
    paths = [row['executable'] for row in rows if row.get('reason') == 'compiler-artifact'
             and row.get('target', {}).get('name') == name
             and row.get('profile', {}).get('test') and row.get('executable')]
    assert len(paths) == 1, paths
    binary = Path('/workspace/artifacts', crate) / Path(paths[0]).relative_to('/target')
    result['binary_sha256'][name] = hashlib.sha256(binary.read_bytes()).hexdigest()
    prefix = ['runuser', '-u', 'alarm', '--', str(binary)]
    listing = subprocess.check_output(prefix + ['--list'], text=True, timeout=20)
    (out / (name + '-test-list.txt')).write_text(listing)
    for test in process_tests:
        assert test + ': test' in listing, 'ARM64 native process test missing: ' + test
    execution = subprocess.run(prefix, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=60)
    (out / (name + '-components.log')).write_text(execution.stdout)
    assert execution.returncode == 0, execution.stdout
    result['checks'][name + '_components_under_arm64_kernel'] = True
    for test in process_tests:
        execution = subprocess.run(prefix + ['--exact', test, '--include-ignored', '--nocapture'],
                                   text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)
        (out / (test.split('::')[-1] + '.log')).write_text(execution.stdout)
        assert execution.returncode == 0 and '1 passed' in execution.stdout, execution.stdout
        result['checks'][test.split('::')[-1]] = True

try:
    assert platform.machine() == 'aarch64'
    assert 'pixel.mode=native-process' in Path('/proc/cmdline').read_text()
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    execute('native-radio', 'owner-test-build-arm64.json', 'pokelinux_radio', [
        'worker::tests::blocked_worker_drop_aborts_process_instead_of_detaching_or_joining',
        'supervisor::tests::spawn_failure_leaves_durable_admission_and_restart_refuses_launch'])
    execute('native-messaging', 'outbox-test-build-arm64.json', 'outbox', [])
    execute('native-messaging', 'outbox_process-test-build-arm64.json', 'outbox_process', [
        'sigkill_releases_owner_but_does_not_release_uncertain_part'])
    result['checks']['unprivileged_execution'] = True
    result['passed'] = True
except Exception as error:
    result['error'] = str(error)
    (out / 'failure.txt').write_text(traceback.format_exc())
finally:
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    subprocess.run(['systemctl', 'poweroff'], check=False)
