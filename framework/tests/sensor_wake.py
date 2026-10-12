#!/usr/bin/env python3
"""Owned SDK37 emulator only. Original generated NDK code over real Binder."""
from pathlib import Path
import json
import sys
import subprocess
import time

PHASE=sys.argv[1]
OUT=Path('artifacts/sensor-wake/runtime')/PHASE
ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
PACKAGE = 'org.pokelinux.sensorpeer'
REMOTE = '/data/local/tmp/pokelinux-sensor-wake'

def run(*args, timeout=45):
    try:
        return subprocess.check_output(ADB + list(args), text=True, stderr=subprocess.STDOUT, timeout=timeout)
    except subprocess.CalledProcessError as error:
        print(error.output, flush=True)
        raise

run('wait-for-device')
end = time.clock_gettime(time.CLOCK_BOOTTIME) + 180
while run('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.clock_gettime(time.CLOCK_BOOTTIME) < end, 'boot timeout'
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
run('install', '--no-incremental', '-r', 'artifacts/sensors-boundary/peer.apk')
run('install','--no-incremental','-r','artifacts/suspend-boundary/peer.apk')
run('push','artifacts/suspend-boundary/peer.apk',REMOTE+'/suspend-bootstrap.apk')
flags=run('shell','env','CLASSPATH='+REMOTE+'/suspend-bootstrap.apk','app_process','/system/bin','org.pokelinux.suspendpeer.FlagOracle')
assert flags.strip()=='no-restart\t8',flags
(OUT/'framework-flags.txt').write_text(flags)
run('push','artifacts/suspend-boundary/suspend-ndk-client-locked',REMOTE+'/suspend-ndk-client')
run('shell','chmod','755',REMOTE+'/suspend-ndk-client')
binary = Path('artifacts/sensor-wake/x86_64-unknown-linux-musl/release/sensor-wake-client')
assert binary.is_file(), 'sensor FMQ client has not been built'
identity = run('shell', 'cmd', 'package', 'list', 'packages', '-U', PACKAGE).strip()
assert identity.startswith('package:' + PACKAGE + ' uid:'), identity
fixture_uid = int(identity.split(' uid:')[1])
assert fixture_uid >= 10000
(OUT / 'fixture-identity.txt').write_text(identity + '\n')
run('shell','am','force-stop','org.pokelinux.suspendpeer')
run('shell', 'am', 'force-stop', PACKAGE)
target=REMOTE+'/sensor-wake-client'
run('push', str(binary), target)
run('shell', 'chmod', '755', target)
try:
    try:
        output=run('shell','timeout','40',target,REMOTE+'/files-'+PHASE,REMOTE+'/suspend-ndk-client',timeout=50)
    except subprocess.CalledProcessError as error:
        (OUT / 'runtime-output.txt').write_text(error.output)
        raise
    (OUT / 'runtime-output.txt').write_text(output)
    assert output.splitlines() == [
        'sensor-wake-frozen-v1-shared-coordinator\tPASS',
        'sensor-wake-acquire-before-original-read\tPASS',
        'sensor-wake-processing-backpressure-handoff\tPASS',
        'sensor-wake-retained-v1-reference\tPASS',
        'sensor-wake-responsive-acquire-revocation\tPASS',
        'sensor-wake-real-fd-acquire-failures\tPASS',
        'sensor-wake-written-signal-failure-quarantine\tPASS',
        'sensor-wake-malformed-generation-retention\tPASS',
        'sensor-wake-real-binder-death-retention\tPASS',
        'sensor-wake-owner-drop-fences-before-release\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    process = subprocess.run(ADB + ['shell', 'pidof', PACKAGE], text=True, capture_output=True, timeout=10)
    assert process.returncode == 1 and not process.stdout.strip(), 'owned peer process must be dead'
    print(output, flush=True)
finally:
    collected=subprocess.run(ADB+['pull',REMOTE+'/files-'+PHASE,str(OUT/'backend-files')],capture_output=True,text=True,timeout=20)
    (OUT/'backend-pull.txt').write_text(collected.stdout+collected.stderr)
    run('shell', 'am', 'force-stop', PACKAGE)
    run('shell','am','force-stop','org.pokelinux.suspendpeer')
(OUT/'runtime-result.json').write_text(json.dumps({'passed':True,'phase':PHASE,'real_binder':True,'original_fmq_executed':True,'frozen_v1_ndk_shared_coordinator':True,'actual_native_wake_leases':True,'kernel_power_backend':'injected private regular files only','actual_hardware_suspend':False,'physical_sensor':False,'vendor_execution':False},indent=2)+'\n')
