#!/usr/bin/env python3
"""Owned SDK37 emulator only. Original generated NDK code over real Binder."""
from pathlib import Path
import json
import subprocess
import time

OUT = Path('artifacts/sensors-boundary')
ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
PACKAGE = 'org.pokelinux.sensorpeer'
REMOTE = '/data/local/tmp/pokelinux-sensors-boundary'

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
(OUT / 'runtime-result.json').write_text(json.dumps({'passed': False, 'phone_accessed': False}) + '\n')
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
binary = OUT / 'x86_64-unknown-linux-musl/release/sensor-fmq-client'
assert binary.is_file(), 'sensor FMQ client has not been built'
identity = run('shell', 'cmd', 'package', 'list', 'packages', '-U', PACKAGE).strip()
assert identity.startswith('package:' + PACKAGE + ' uid:'), identity
fixture_uid = int(identity.split(' uid:')[1])
assert fixture_uid >= 10000
(OUT / 'fixture-identity.txt').write_text(identity + '\n')
run('shell', 'am', 'force-stop', PACKAGE)
target = REMOTE + '/sensor-fmq-client'
run('push', str(binary), target)
run('shell', 'chmod', '755', target)
try:
    try:
        output = run('shell', 'timeout', '25', target, str(fixture_uid), timeout=35)
    except subprocess.CalledProcessError as error:
        (OUT / 'runtime-output.txt').write_text(error.output)
        raise
    (OUT / 'runtime-output.txt').write_text(output)
    assert output.splitlines() == [
        'sensor-frozen-ndk-abi-bootstrap\tPASS',
        'sensor-authenticated-per-client\tPASS',
        'sensor-original-one-second-wait\tPASS',
        'sensor-admitted-before-ack\tPASS',
        'sensor-exact-split-ack-accounting\tPASS',
        'sensor-bounded-queue-backpressure\tPASS',
        'sensor-revocation-publication-fence\tPASS',
        'sensor-stale-generation-fd-lifetime\tPASS',
        'sensor-malformed-event-quarantine\tPASS',
        'sensor-bounded-death-quarantine\tPASS',
        'sensor-original-eventflag-timeout\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    process = subprocess.run(ADB + ['shell', 'pidof', PACKAGE], text=True, capture_output=True, timeout=10)
    assert process.returncode == 1 and not process.stdout.strip(), 'owned peer process must be dead'
    print(output, flush=True)
finally:
    run('shell', 'am', 'force-stop', PACKAGE)
(OUT / 'runtime-result.json').write_text(json.dumps({
    'passed': True, 'scope': 'device-neutral Rust owner driven by exact public SensorsV3 Binder plus original libfmq/EventFlag and synthetic fixed Event payloads',
    'frozen_sensors_version': 3, 'event_quantum': 96,
    'real_binder': True, 'real_shared_memory_fmq': True, 'original_public_fmq_executed': True,
    'selinux': 'Enforcing', 'authenticated_binder_caller': True,
    'admission_before_queue_read': True, 'processed_before_ack': True,
    'revocation_publication_fenced': True, 'stale_generation_fenced': True,
    'death_retains_quarantine': True, 'bounded_fd_cycles': 32,
    'physical_sensor_accessed': False, 'vendor_execution': False,
    'calibration_verified': False, 'physical_location_verified': False,
    'actual_suspend_verified': False, 'actual_wakelock_authority_integrated': False,
    'arm64_sensor_verified': False
}, indent=2) + '\n')
