#!/usr/bin/env python3
"""Execute the actual ARM64 Rust test binary under the guest kernel as alarm."""
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
          'os': Path('/etc/os-release').read_text(), 'checks': {}}
try:
    assert platform.machine() == 'aarch64'
    assert 'pixel.mode=radio-process' in Path('/proc/cmdline').read_text()
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    rows = [json.loads(line) for line in Path('/workspace/artifacts/native-radio/owner-test-build-arm64.json').read_text().splitlines()]
    paths = [row['executable'] for row in rows if row.get('reason') == 'compiler-artifact'
             and row.get('target', {}).get('name') == 'pokelinux_radio'
             and row.get('profile', {}).get('test') and row.get('executable')]
    assert len(paths) == 1, paths
    binary = Path('/workspace/artifacts/native-radio') / Path(paths[0]).relative_to('/target')
    result['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    prefix = ['runuser', '-u', 'alarm', '--', str(binary)]
    listing = subprocess.check_output(prefix + ['--list'], text=True, timeout=20)
    (out / 'test-list.txt').write_text(listing)
    tests = ['worker::tests::blocked_worker_drop_aborts_process_instead_of_detaching_or_joining',
             'supervisor::tests::spawn_failure_leaves_durable_admission_and_restart_refuses_launch']
    for test in tests:
        assert test + ': test' in listing, 'ARM64 native process test missing: ' + test
    execution = subprocess.run(prefix, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=60)
    (out / 'components.log').write_text(execution.stdout)
    assert execution.returncode == 0, execution.stdout
    result['checks']['components_under_arm64_kernel'] = True
    for test in tests:
        execution = subprocess.run(prefix + ['--exact', test, '--include-ignored', '--nocapture'],
                                   text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)
        (out / (test.split('::')[-1] + '.log')).write_text(execution.stdout)
        assert execution.returncode == 0 and '1 passed' in execution.stdout, execution.stdout
        result['checks'][test.split('::')[-1]] = True
    result['checks']['unprivileged_execution'] = True
    result['passed'] = True
except Exception as error:
    result['error'] = str(error)
    (out / 'failure.txt').write_text(traceback.format_exc())
finally:
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    subprocess.run(['systemctl', 'poweroff'], check=False)
