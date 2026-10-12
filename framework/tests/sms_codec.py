#!/usr/bin/env python3
"""Execute AOSP's real SMS encoder on the owned SDK37 emulator, without sending."""
import hashlib
from pathlib import Path
import subprocess
import time

ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
def adb(*args):
    return subprocess.check_output(ADB + list(args), timeout=45, text=True, stderr=subprocess.STDOUT)
adb('wait-for-device')
end = time.monotonic() + 180
while adb('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'owned emulator boot timeout'
    time.sleep(2)
assert adb('shell', 'getenforce').strip() == 'Enforcing'
root = '/data/local/tmp/pokelinux-sms-codec'
adb('shell', 'mkdir', '-p', root)
jar = Path('artifacts/ims-native-audit/oracle/oracle.jar')
adb('push', str(jar), root + '/oracle.jar')
adb('shell', 'chmod', '444', root + '/oracle.jar')
output = adb('shell', 'env', 'CLASSPATH=' + root + '/oracle.jar', 'app_process', '/system/bin', 'org.omarchy.audit.SmsCodecOracle')
out = Path('artifacts/native-messaging'); out.mkdir(exist_ok=True)
(out / 'aosp-submit-output.tsv').write_text(output)
rows = [line for line in output.splitlines() if line.startswith('sms-submit\t')]
assert len(rows) == 27 and all(len(row.split('\t')) == 8 for row in rows), output
assert len({row.split('\t')[1] for row in rows}) == 27, 'fixture IDs must be unique'
framework = out / 'telephony-common.jar'
adb('pull', '/system/framework/telephony-common.jar', str(framework))
(out / 'framework-sha256.txt').write_text(hashlib.sha256(framework.read_bytes()).hexdigest() + '\n')
(out / 'sms-image.txt').write_text(adb('shell', 'getprop', 'ro.build.fingerprint'))
(out / 'oracle-sha256.txt').write_text(hashlib.sha256(jar.read_bytes()).hexdigest() + '\n')
assert adb('shell', 'getenforce').strip() == 'Enforcing'
print('Actual AOSP SMS-SUBMIT encoder executed: 27 fixtures; no SMS sent', flush=True)

fixtures = root + '/aosp-submit.tsv'
adb('push', 'native-messaging/tests/fixtures/aosp-submit.tsv', fixtures)
received = adb('shell', 'env', 'CLASSPATH=' + root + '/oracle.jar', 'app_process', '/system/bin', 'org.omarchy.audit.SmsReceiveOracle', fixtures)
(out / 'aosp-deliver-output.tsv').write_text(received)
rows = [line for line in received.splitlines() if line.startswith('sms-deliver\t')]
assert len(rows) == 33 and len({row.split('\t')[1] for row in rows}) == 33, received
assert all(len(row.split('\t')) == 9 for row in rows), received
print('Actual AOSP SMS-DELIVER decoder executed: 33 synthetic fixtures; no SMS received from a modem', flush=True)
