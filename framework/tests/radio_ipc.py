#!/usr/bin/env python3
"""Owned SDK37 emulator only: framework bootstrap and native radio IPC."""
from pathlib import Path
import subprocess
import time

ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
def run(*args):
    try:
        return subprocess.check_output(ADB + list(args), text=True, stderr=subprocess.STDOUT, timeout=45)
    except subprocess.CalledProcessError as error:
        print(error.output, flush=True)
        raise

run('wait-for-device')
end = time.monotonic() + 180
while run('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'boot timeout'
    time.sleep(2)
assert run('shell', 'getenforce').strip() == 'Enforcing'
run('pull', '/system/framework/framework.jar', 'artifacts/native-radio/framework.jar')
root = '/data/local/tmp/pokelinux-radio-ipc'
run('shell', 'mkdir', '-p', root)
run('push', 'artifacts/ims-native-audit/oracle/oracle.jar', root + '/oracle.jar')
run('shell', 'chmod', '444', root + '/oracle.jar')
output = run('shell', 'env', 'CLASSPATH=' + root + '/oracle.jar', 'app_process', '/system/bin', 'org.omarchy.audit.FrameworkBootstrapOracle')
Path('artifacts/native-radio/bootstrap-abi.txt').write_text(output)
rows = output.splitlines()
assert len(rows) == 2 and rows[0].startswith('startInstrumentation\t') and rows[0].endswith('\tPASS'), output
assert rows[1] == 'instrumentationStatus\t1\tPASS', output
print(output, flush=True)
binary = Path('artifacts/native-radio/x86_64-unknown-linux-musl/release/radio-ipc-test')
assert binary.is_file(), 'native radio IPC test binary is not implemented'

assert rows[0] == 'startInstrumentation\t51\tPASS', 'unqualified framework ABI'
run('install', '--no-incremental', '-r', 'artifacts/native-radio/peer/peer.apk')
identity = run('shell', 'id') + run('shell', 'cat', '/proc/self/attr/current')
assert 'uid=2000(shell)' in identity and 'u:r:shell:s0' in identity, identity
Path('artifacts/native-radio/ipc-client-identity.txt').write_text(identity)
Path('artifacts/native-radio/ipc-image.txt').write_text(run('shell', 'getprop', 'ro.build.fingerprint'))
run('push', str(binary), root + '/radio-ipc-test')
run('shell', 'chmod', '755', root + '/radio-ipc-test')
try:
    output = run('shell', root + '/radio-ipc-test', '51', 'expire-during-registration')
    Path('artifacts/native-radio/registration-expire-output.txt').write_text(output)
    assert output.splitlines() == ['radio-bootstrap\tPASS', 'radio-registration-held\tPASS', 'radio-registration-held-expire\tPASS', 'radio-registration-late-success-rejected\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    print(output, flush=True)
    run('shell', 'am', 'force-stop', 'org.pokelinux.radiopeer')
    output = run('shell', root + '/radio-ipc-test', '51', 'cancel-during-registration')
    Path('artifacts/native-radio/registration-cancel-output.txt').write_text(output)
    assert output.splitlines() == ['radio-bootstrap\tPASS', 'radio-registration-held\tPASS', 'radio-registration-held-cancel\tPASS', 'radio-registration-late-success-rejected\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    print(output, flush=True)
    run('shell', 'am', 'force-stop', 'org.pokelinux.radiopeer')
    output = run('shell', root + '/radio-ipc-test', '51', 'session-owner')
    Path('artifacts/native-radio/owner-output.txt').write_text(output)
    assert output.splitlines() == ['radio-bootstrap\tPASS', 'radio-owner-activation\tPASS', 'radio-owner-consumed-cancel\tPASS', 'radio-owner-consumed-replacement\tPASS', 'radio-owner-late-endpoint\tPASS', 'radio-owner-stop\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    print(output, flush=True)
    run('shell', 'am', 'force-stop', 'org.pokelinux.radiopeer')
    output = run('shell', root + '/radio-ipc-test', '51', 'death-during-registration')
    Path('artifacts/native-radio/registration-death-output.txt').write_text(output)
    assert output.splitlines() == ['radio-bootstrap\tPASS', 'radio-registration-death\tDeadObject\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    print(output, flush=True)
    run('shell', 'am', 'force-stop', 'org.pokelinux.radiopeer')
    # A fresh client process and peer exercise ordinary setup after the death case.
    output = run('shell', root + '/radio-ipc-test', '51')
    Path('artifacts/native-radio/ipc-output.txt').write_text(output)
    lines = output.splitlines()
    expected = ['radio-bootstrap\tPASS', 'radio-registration\tPASS', 'radio-indications\tPASS', 'radio-assembly\tPASS']
    expected += ['radio-roundtrip\t'+str(n)+'\tPASS' for n in [0, 2048, 2049, 4096]]
    expected += ['radio-callback-overflow\tPASS']
    assert lines[:9] == expected and len(lines) == 16, output
    assert lines[9].startswith('radio-peer\t') and lines[10] == 'radio-clear-observed\tPASS', output
    assert lines[11].startswith('radio-late-callback\tandroid.os.') and lines[11].endswith('\tPASS'), output
    assert lines[12:] == ['radio-malformed-wakeup\tPASS', 'radio-owner-cancel\tPASS', 'radio-service-death\tPASS', 'radio-already-dead\tPASS'], output
    assert run('shell', 'getenforce').strip() == 'Enforcing'
    print(output, flush=True)
finally:
    run('shell', 'am', 'force-stop', 'org.pokelinux.radiopeer')
