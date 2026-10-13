#!/usr/bin/env python3
"""Boot-time entry for the owned network-free ARM authority guest."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import traceback

ROOT = Path('/results')
selection = json.loads((ROOT / 'selection.json').read_text())
result = {'passed': False, 'qemu_mode': 'system', 'phone_hardware_qualified': False}
try:
    assert os.getuid() == 0 and os.uname().machine == 'aarch64'
    assert os.uname().release == '7.1.6-1-aarch64-ARCH'
    assert sorted(path.name for path in Path('/sys/class/net').iterdir()) == ['lo']
    for name, expected in selection['sources'].items():
        path = ROOT / 'source' / name
        assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, name
    for name, expected in selection['elf_sha256'].items():
        path = ROOT / 'bins' / name
        assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, name
    args = ['python3', str(ROOT / 'source/tools/native-session-authority/runner.py'),
            '--inside', '--bins', str(ROOT / 'bins'), '--output', str(ROOT / 'execution')]
    for case in selection.get('selected_cases', []):
        args += ['--case', case]
    env = dict(os.environ, NATIVE_MESSAGES_ISOLATED='1')
    with (ROOT / 'execution.log').open('w') as log:
        completed = subprocess.run(args, stdout=log, stderr=subprocess.STDOUT, env=env, timeout=180)
    result.update(json.loads((ROOT / 'execution/result.json').read_text()))
    result['runner_exit_code'] = completed.returncode
    result['qemu_mode'] = 'system'
    assert result['kernel']['machine'] == 'aarch64' and result['pidfs_identity_qualified']
except BaseException as error:
    result['passed'] = False
    result['error'] = str(error)
    result['traceback'] = traceback.format_exc()
finally:
    (ROOT / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    subprocess.run(['sync'], timeout=10)
    subprocess.run(['poweroff'], timeout=10)
