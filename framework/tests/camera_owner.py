#!/usr/bin/env python3
"""Owned SDK37 emulator only. Original generated NDK code over real Binder."""
from pathlib import Path
import json
import subprocess
import time

OUT = Path('artifacts/camera-boundary')
ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
PACKAGE = 'org.pokelinux.camerapeer'
REMOTE = '/data/local/tmp/pokelinux-camera-boundary'

def run(*args, timeout=45):
    try:
        return subprocess.check_output(ADB + list(args), text=True, stderr=subprocess.STDOUT, timeout=timeout)
    except subprocess.CalledProcessError as error:
        print(error.output, flush=True)
        raise

run('wait-for-device')
end = time.monotonic() + 180
while run('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'boot timeout'
    time.sleep(2)
assert run('shell', 'getenforce').strip() == 'Enforcing'
OUT.mkdir(exist_ok=True, parents=True)
(OUT / 'owner-result.json').write_text(json.dumps({'passed': False, 'phone_accessed': False}) + '\n')
run('shell', 'mkdir', '-p', REMOTE)
# Original installed framework reflection qualifies disposable AM bootstrap only.
oracle = Path('artifacts/ims-native-audit/oracle/oracle.jar')
assert oracle.is_file(), 'build-shannon-oracle.sh must prepare FrameworkBootstrapOracle'
run('push', str(oracle), REMOTE + '/oracle.jar')
bootstrap = run('shell', 'env', 'CLASSPATH=' + REMOTE + '/oracle.jar', 'app_process', '/system/bin', 'org.omarchy.audit.FrameworkBootstrapOracle')
assert bootstrap.splitlines() == ['startInstrumentation\t51\tPASS', 'instrumentationStatus\t1\tPASS'], bootstrap
(OUT / 'framework-bootstrap.txt').write_text(bootstrap)
identity = run('shell', 'id') + run('shell', 'cat', '/proc/self/attr/current')
assert 'uid=2000(shell)' in identity and 'u:r:shell:s0' in identity, identity
(OUT / 'client-identity.txt').write_text(identity)
(OUT / 'image.txt').write_text(run('shell', 'getprop', 'ro.build.fingerprint'))
(OUT / 'binder-libraries.sha256').write_text(run('shell', 'sha256sum', '/system/lib64/libbinder_ndk.so', '/system/lib64/libbinder.so'))
run('install', '--no-incremental', '-r', str(OUT / 'peer.apk'))
binary = OUT / 'x86_64-unknown-linux-musl/release/camera-owner-client'
assert binary.is_file(), 'camera ownership client has not been built'
run('shell', 'am', 'force-stop', PACKAGE)
target = REMOTE + '/camera-owner-client'
run('push', str(binary), target)
run('shell', 'chmod', '755', target)
try:
    try:
        output = run('shell', 'timeout', '25', target, timeout=35)
    except subprocess.CalledProcessError as error:
        (OUT / 'owner-output.txt').write_text(error.output)
        raise
    (OUT / 'owner-output.txt').write_text(output)
    assert output.splitlines() == [
        'camera-owner-frozen-bootstrap\tPASS',
        'camera-owner-return-complete-fence\tPASS',
        'camera-owner-cached-error-duplicate\tPASS',
        'camera-owner-cache-removal\tPASS',
        'camera-owner-error-invalid-count\tPASS',
        'camera-owner-single-shot-receipts\tPASS',
        'camera-owner-late-metadata-after-reuse\tPASS',
        'camera-owner-held-revoke-generation\tPASS',
        'camera-owner-death-quarantine\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    process = subprocess.run(ADB + ['shell', 'pidof', PACKAGE], text=True, capture_output=True, timeout=10)
    assert process.returncode == 1 and not process.stdout.strip(), 'owned peer process must be dead'
    print(output, flush=True)
finally:
    run('shell', 'am', 'force-stop', PACKAGE)
(OUT / 'owner-result.json').write_text(json.dumps({
    'passed': True, 'scope': 'device-neutral Rust owner driven by exact public generated cameraV4 Binder callbacks and synthetic pipe FDs',
    'frozen_camera_device_version': 4, 'frozen_common_version': 2,
    'real_binder': True, 'selinux': 'Enforcing', 'cached_handles': True,
    'returned_complete_fence_distinct': True, 'invalid_count_quarantined': True,
    'revocation_while_submit_held': True, 'configuration_and_session_fenced': True,
    'death_retains_quarantine': True, 'unknown_duplicate_stale_returns_fenced': True,
    'sync_file_verified': False, 'dma_cleanup_verified': False,
    'physical_phone_accessed': False, 'vendor_execution': False,
    'camera_capture_verified': False, 'arm64_camera_verified': False
}, indent=2) + '\n')
