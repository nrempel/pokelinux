#!/usr/bin/env python3
"""Exact corrected ARM64 receipt tests under an actual isolated Linux kernel."""
import hashlib
import json
import platform
import resource
import subprocess
import traceback
from pathlib import Path

out = Path('/results')
result = {'passed': False, 'architecture': platform.machine(), 'kernel': platform.release(),
          'qemu_mode': 'system', 'hardware_verified': False, 'phone_accessed': False,
          'network': 'none', 'checks': {}, 'binary_sha256': {}}
try:
    assert platform.machine() == 'aarch64'
    assert 'pixel.mode=receipt-process' in Path('/proc/cmdline').read_text()
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    root = Path('/workspace')
    selection_path = root / 'artifacts/native-sms-receipt/window-fixed/arm64-selection.json'
    selection = json.loads(selection_path.read_text())
    result['selection_sha256'] = hashlib.sha256(selection_path.read_bytes()).hexdigest()
    for key, path in [('receipt_source_sha256', 'native-messaging/src/receipt.rs'),
                      ('receipt_tests_sha256', 'native-messaging/tests/receipt.rs'),
                      ('module_export_sha256', 'native-messaging/src/lib.rs')]:
        assert hashlib.sha256((root / path).read_bytes()).hexdigest() == selection[key], path
    cargo_path = root / selection['cargo_json']
    rows = [json.loads(line) for line in cargo_path.read_text().splitlines()]
    selected = [row['executable'] for row in rows if row.get('reason') == 'compiler-artifact'
                and row.get('target', {}).get('name') == 'receipt'
                and row.get('profile', {}).get('test') and row.get('executable')]
    assert len(selected) == 1
    expected = Path('artifacts/native-sms-receipt/window-fixed') / Path(selected[0]).relative_to('/target')
    assert str(expected) == selection['executable']
    binary = root / selection['executable']
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == selection['sha256']
    result['cargo_manifest_sha256'] = hashlib.sha256(cargo_path.read_bytes()).hexdigest()
    result['binary_sha256']['receipt'] = selection['sha256']
    prefix = ['runuser', '-u', 'alarm', '--', str(binary)]
    listing = subprocess.check_output(prefix + ['--list'], text=True, timeout=30)
    (out / 'receipt-list.txt').write_text(listing)
    for name in [selection['parent'], selection['helper']]:
        assert name + ': test' in listing, name
    execution = subprocess.run(prefix + ['--include-ignored', '--nocapture'], text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=90)
    (out / 'receipt-components.log').write_text(execution.stdout)
    assert execution.returncode == 0 and '19 passed; 0 failed; 0 ignored' in execution.stdout, execution.stdout
    result['checks'] = {'unprivileged_execution': True, 'native_receipt_tests': 19,
                        'actual_owned_child_sigkill_reap_restart': True,
                        'no_uncertain_ack_replay': True, 'distinct_equal_redelivery': True}
    result['passed'] = True
except BaseException as error:
    result['error'] = str(error)
    (out / 'failure.txt').write_text(traceback.format_exc())
finally:
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    subprocess.run(['systemctl', 'poweroff'], check=False)
