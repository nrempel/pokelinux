#!/usr/bin/env python3
"""Owned SDK37 emulator only. Original generated NDK code over real Binder."""
from pathlib import Path
import json
import os
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
(OUT / 'result.json').write_text(json.dumps({'passed': False, 'phone_accessed': False}) + '\n')
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
clients = [('ndk', OUT / 'camera-ndk-client', [
    'camera-ndk-bootstrap\tPASS', 'camera-ndk-fd-independent\tPASS',
    'camera-ndk-error-output-untouched\tPASS', 'camera-ndk-error-uncertain\tPASS',
    'camera-ndk-fd-budget\t64\tPASS', 'camera-ndk-service-death\tPASS'])]
if os.environ.get('CAMERA_RUST_CLIENT') == '1':
    clients.append(('rust', OUT / 'x86_64-unknown-linux-musl/release/camera-binder-client', [
        'camera-rust-bootstrap\tPASS', 'camera-rust-fd-independent\tPASS',
        'camera-rust-error-uncertain\tPASS', 'camera-rust-fd-budget\t64\tPASS',
        'camera-rust-service-death\tPASS']))
try:
    for kind, binary, expected in clients:
        assert binary.is_file(), 'native client has not been built'
        run('shell', 'am', 'force-stop', PACKAGE)
        target = REMOTE + '/client-' + kind
        run('push', str(binary), target)
        run('shell', 'chmod', '755', target)
        try:
            output = run('shell', 'timeout', '25', target, timeout=35)
        except subprocess.CalledProcessError as error:
            (OUT / (kind + '-output.txt')).write_text(error.output)
            raise
        (OUT / (kind + '-output.txt')).write_text(output)
        assert output.splitlines() == expected, output
        assert run('shell', 'getenforce').strip() == 'Enforcing'
        process = subprocess.run(ADB + ['shell', 'pidof', PACKAGE], text=True, capture_output=True, timeout=10)
        assert process.returncode == 1 and not process.stdout.strip(), 'peer death must reap owned test process'
        print(output, flush=True)
finally:
    run('shell', 'am', 'force-stop', PACKAGE)
(OUT / 'result.json').write_text(json.dumps({
    'passed': True, 'phone_accessed': False,
    'scope': 'synthetic original-generated NDK kernel Binder integration',
    'ndk_proxy_dispatcher_executed': True,
    'rust_interoperability': len(clients) == 2,
    'service_error_omits_count': True,
    'receiver_and_returned_fd_independent_lifetimes': True,
    'repeated_fd_budget': 64, 'bounded_service_death': True,
    'selinux': 'Enforcing', 'vendor_execution': False,
    'camera_capture_verified': False, 'dma_safety_verified': False
}, indent=2) + '\n')
