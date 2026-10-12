#!/usr/bin/env python3
"""Owned SDK37 emulator: original generated public V3 NDK/Rust Binder and outbox."""
import json
import subprocess
import time
import uuid
from pathlib import Path
OUT = Path('artifacts/sms-binder')
ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
PACKAGE = 'org.pokelinux.smspeer'
REMOTE = '/data/local/tmp/pokelinux-sms-binder'
def run(*args, timeout=45):
    return subprocess.check_output(ADB + list(args), text=True, stderr=subprocess.STDOUT, timeout=timeout)
run('wait-for-device')
end = time.monotonic() + 180
while run('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'owned emulator boot timeout'
    time.sleep(2)
assert run('shell', 'getenforce').strip() == 'Enforcing'
OUT.mkdir(exist_ok=True, parents=True)
(OUT / 'result.json').write_text(json.dumps({'passed': False, 'phone_accessed': False}) + '\n')
run('shell', 'mkdir', '-p', REMOTE)
oracle = Path('artifacts/ims-native-audit/oracle/oracle.jar')
run('push', str(oracle), REMOTE + '/oracle.jar')
bootstrap = run('shell', 'env', 'CLASSPATH=' + REMOTE + '/oracle.jar', 'app_process', '/system/bin', 'org.omarchy.audit.FrameworkBootstrapOracle')
assert bootstrap.splitlines() == ['startInstrumentation\t51\tPASS', 'instrumentationStatus\t1\tPASS'], bootstrap
(OUT / 'framework-bootstrap.txt').write_text(bootstrap)
identity = run('shell', 'id') + run('shell', 'cat', '/proc/self/attr/current')
assert 'uid=2000(shell)' in identity and 'u:r:shell:s0' in identity, identity
(OUT / 'client-identity.txt').write_text(identity)
(OUT / 'image.txt').write_text(run('shell', 'getprop', 'ro.build.fingerprint'))
(OUT / 'binder-libraries.sha256').write_text(run('shell', 'sha256sum', '/system/lib64/libbinder_ndk.so', '/system/lib64/libbinder.so'))
run('install', '--no-incremental', '-r', str(OUT / 'peer/peer.apk'))
binary = OUT / 'x86_64-unknown-linux-musl/release/sms-binder-client'
assert binary.is_file(), 'native SMS client has not been built'
run('shell', 'am', 'force-stop', PACKAGE)
target = REMOTE + '/sms-binder-client'
run('push', str(binary), target)
run('shell', 'chmod', '755', target)
try:
    try:
        output = run('shell', 'timeout', '25', target, REMOTE + '/state-' + uuid.uuid4().hex, timeout=35)
    except subprocess.CalledProcessError as error:
        (OUT / 'output.txt').write_text(error.output)
        print(error.output, flush=True)
        raise
    (OUT / 'output.txt').write_text(output)
    expected = ['frozen-bootstrap', 'durable-before-ipc-oneway', 'intermediate-ack', 'malformed-ackpdu', 'final-matching-wrong-duplicate', 'cancel-consumed-late', 'replaced-generation-same-serial', 'error-no-negative-proof-retry', 'death-consumed-callback', 'restart-preserves-uncertainty']
    assert output.splitlines() == ['sms-binder-' + name + '\tPASS' for name in expected], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    dead = subprocess.run(ADB + ['shell', 'pidof', PACKAGE], text=True, capture_output=True, timeout=10)
    assert dead.returncode == 1 and not dead.stdout.strip(), 'owned peer must be dead and reaped'
    print(output, flush=True)
finally:
    run('shell', 'am', 'force-stop', PACKAGE)
(OUT / 'result.json').write_text(json.dumps({
    'passed': True, 'scope': 'synthetic frozen standard messaging V3 NDK/Rust kernel Binder with actual durable native outbox',
    'frozen_messaging_version': 3, 'frozen_messaging_hash': '30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb',
    'real_binder': True, 'selinux': 'Enforcing', 'uncertainty_before_ipc': True,
    'oneway_and_intermediate_ack_not_acceptance': True, 'matching_final_send_only': True, 'malformed_ackpdu_preserves_permit': True,
    'wrong_serial_method_slot_metadata_fenced': True, 'duplicate_and_old_generation_fenced': True,
    'cancel_and_death_consumed_callbacks_fenced': True, 'restart_persists_uncertainty': True,
    'modem_acceptance_qualified': False, 'carrier_delivery_verified': False,
    'receive_identity_qualified': False, 'acknowledgement_idempotence_qualified': False,
    'physical_phone_accessed': False, 'vendor_execution': False, 'arm64_binder_verified': False
}, indent=2) + '\n')
