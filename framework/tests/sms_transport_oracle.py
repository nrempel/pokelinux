#!/usr/bin/env python3
"""Execute isolated original Shannon SMS codecs/local tracker state; no radio dispatch."""
import hashlib
import subprocess
import time
from pathlib import Path

ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
OUT = Path('artifacts/sms-transport-audit')
OUT.mkdir(exist_ok=True)
def adb(*args):
    return subprocess.check_output(ADB + list(args), timeout=45, text=True,
                                   stderr=subprocess.STDOUT)
adb('wait-for-device')
end = time.monotonic() + 180
while adb('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'owned emulator boot timeout'
    time.sleep(2)
assert adb('shell', 'getenforce').strip() == 'Enforcing'
apk = Path('artifacts/telephony-audit/input/system_ext/priv-app/ShannonIms/ShannonIms.apk')
assert hashlib.sha256(apk.read_bytes()).hexdigest() == '8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879'
jar = Path('artifacts/ims-native-audit/oracle/oracle.jar')
root = '/data/local/tmp/pokelinux-sms-transport-oracle'
adb('shell', 'mkdir', '-p', root)
for source, name in [(apk, 'shannon.apk'), (jar, 'oracle.jar')]:
    adb('push', str(source), root + '/' + name)
    adb('shell', 'chmod', '444', root + '/' + name)
try:
    output = adb('shell', 'env', 'CLASSPATH=' + root + '/oracle.jar:' + root + '/shannon.apk',
                 'app_process', '/system/bin', 'org.omarchy.audit.SmsTransportOracle')
except subprocess.CalledProcessError as error:
    (OUT / 'original-runtime-failure.txt').write_text(error.output)
    (OUT / 'original-runtime-failure-logcat.txt').write_text(adb('logcat', '-d', '-s', 'AndroidRuntime:E'))
    raise
(OUT / 'original-output.tsv').write_text(output)
rows = [line for line in output.splitlines() if line.startswith('sms-transport\t')]
assert rows and rows[-1] == 'sms-transport\tcomplete\tPASS', 'missing completion receipt'
(OUT / 'oracle-sha256.txt').write_text(hashlib.sha256(jar.read_bytes()).hexdigest() + '\n')
(OUT / 'image.txt').write_text(adb('shell', 'getprop', 'ro.build.fingerprint'))
assert adb('shell', 'getenforce').strip() == 'Enforcing'
print('Original SMS codecs and controlled local tracker state executed; no service startup or radio dispatch', flush=True)
