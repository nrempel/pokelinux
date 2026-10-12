#!/usr/bin/env python3
"""Dedicated owned SDK emulator: synthetic frozen incoming/ACK plus original SQLite owner."""
import hashlib
import json
import subprocess
import sys
import time
import uuid
from pathlib import Path
PHASE = sys.argv[1]
assert PHASE in ('red', 'green', 'green-cancel')
ROOT = Path('artifacts/native-sms-receipt/binder')
OUT = ROOT / 'runtime' / PHASE
OUT.mkdir(exist_ok=True, parents=True)
ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
PACKAGE = 'org.pokelinux.smsreceiptpeer'
REMOTE = '/data/local/tmp/pokelinux-sms-receipt'
def run(*args, timeout=45):
    return subprocess.check_output(ADB + list(args), text=True, stderr=subprocess.STDOUT, timeout=timeout)
run('wait-for-device')
end = time.monotonic() + 180
while run('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'owned emulator boot timeout'
    time.sleep(2)
assert run('shell', 'getenforce').strip() == 'Enforcing'
(OUT / 'result.json').write_text(json.dumps({'passed': False, 'phase': PHASE, 'physical_phone_accessed': False}) + '\n')
run('shell', 'mkdir', '-p', REMOTE)
run('push', 'artifacts/ims-native-audit/oracle/oracle.jar', REMOTE + '/bootstrap.jar')
bootstrap = run('shell', 'env', 'CLASSPATH=' + REMOTE + '/bootstrap.jar', 'app_process', '/system/bin', 'org.omarchy.audit.FrameworkBootstrapOracle')
assert bootstrap.splitlines() == ['startInstrumentation\t51\tPASS', 'instrumentationStatus\t1\tPASS'], bootstrap
(OUT / 'framework-bootstrap.txt').write_text(bootstrap)
identity = run('shell', 'id') + run('shell', 'cat', '/proc/self/attr/current')
assert 'uid=2000(shell)' in identity and 'u:r:shell:s0' in identity
(OUT / 'client-identity.txt').write_text(identity)
(OUT / 'image.txt').write_text(run('shell', 'getprop', 'ro.build.fingerprint'))
(OUT / 'binder-libraries.sha256').write_text(run('shell', 'sha256sum', '/system/lib64/libbinder_ndk.so', '/system/lib64/libbinder.so'))
run('install', '--no-incremental', '-r', str(ROOT / 'peer/peer.apk'))
binary = ROOT / 'x86_64-unknown-linux-musl/release/sms-receipt-client'
assert binary.is_file()
(OUT / 'client').write_bytes(binary.read_bytes())
(OUT / 'peer.apk').write_bytes((ROOT / 'peer/peer.apk').read_bytes())
files = [Path('native-messaging/src/receipt.rs'), Path('native-sms-receipt-boundary/src/bin/sms-receipt-client.rs'), Path('framework/smsreceiptpeer/peer.cpp')]
(OUT / 'runtime-source-hashes.json').write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}, indent=2)+'\n')
run('shell', 'am', 'force-stop', PACKAGE)
target = REMOTE + '/client'
run('push', str(binary), target)
run('shell', 'chmod', '755', target)
try:
    try:
        output = run('shell', 'timeout', '35', target, REMOTE + '/state-' + uuid.uuid4().hex, timeout=45)
    except subprocess.CalledProcessError as error:
        (OUT / 'output.txt').write_text(error.output)
        print(error.output, flush=True)
        raise
    (OUT / 'output.txt').write_text(output)
    expected = ['frozen-bootstrap', 'raw-uncertainty-before-ipc-oneway', 'intermediate-not-final', 'exact-final-wrong-duplicate', 'equal-bytes-distinct-kinds', 'premature-arrival-quarantines-window', 'old-generation-and-stale-raw', 'cancel-held-final', 'permission-held-permit', 'unqualified-type-and-bounded-raw', 'death-held-final', 'restart-no-replay-or-dedup']
    assert output.splitlines() == ['sms-receipt-' + name + '\tPASS' for name in expected], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    dead = subprocess.run(ADB + ['shell', 'pidof', PACKAGE], text=True, capture_output=True, timeout=10)
    assert dead.returncode == 1 and not dead.stdout.strip(), 'owned peer must be dead and reaped'
    print(output, flush=True)
finally:
    run('shell', 'am', 'force-stop', PACKAGE)
(OUT / 'result.json').write_text(json.dumps({
    'passed': True, 'phase': PHASE,
    'scope': 'synthetic frozen standard messaging V3 incoming/ACK with original native SQLite receipt owner',
    'real_binder': True, 'selinux': 'Enforcing', 'raw_storage_before_ipc': True,
    'uncertainty_before_ipc': True, 'oneway_and_intermediate_not_final': True,
    'exact_test_final_only': True, 'wrong_serial_method_slot_type_fenced': True,
    'duplicate_and_old_generation_fenced': True, 'premature_arrival_quarantines_window': True,
    'cancel_permission_death_fenced': True, 'restart_without_replay': True,
    'equal_raw_arrivals_remain_distinct': True, 'backend_scope': 'test_backend',
    'last_incoming_modem_identity_qualified': False, 'modem_ack_success_qualified': False,
    'production_success_cause_policy_qualified': False, 'acknowledgement_replay_qualified': False,
    'physical_phone_accessed': False, 'vendor_execution': False, 'arm64_binder_verified': False,
}, indent=2)+'\n')
