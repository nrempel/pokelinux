#!/usr/bin/env python3
"""Start the enrolled bridge through visible controls in the owned emulator."""
import re
import subprocess
import time
import xml.etree.ElementTree as ET

ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
def adb(*args):
    return subprocess.check_output(ADB + list(args), timeout=40)
def tree():
    adb('shell', 'uiautomator', 'dump', '/sdcard/window.xml')
    return ET.fromstring(adb('shell', 'cat', '/sdcard/window.xml'))
def click(label):
    matches = [n for n in tree().iter('node') if n.get('text') == label and n.get('clickable') == 'true']
    assert len(matches) == 1, label
    assert matches[0].get('enabled') == 'true'
    left, top, right, bottom = map(int, re.findall(r'\d+', matches[0].get('bounds')))
    adb('shell', 'input', 'tap', str((left + right)//2), str((top + bottom)//2))
    time.sleep(.5)

assert adb('shell', 'getenforce').strip() == b'Enforcing'
# The watch API requires the Android dialer role even when there are no calls.
# Assign it only in this disposable emulator. Desktop command switches remain off.
adb('shell', 'cmd', 'role', 'add-role-holder', '--user', '0',
    'android.app.role.DIALER', 'org.omarchy.bridge')
assert adb('shell', 'cmd', 'role', 'get-role-holders', '--user', '0',
           'android.app.role.DIALER').strip() == b'org.omarchy.bridge'
adb('shell', 'am', 'start', '-n', 'org.omarchy.bridge/.DialActivity')
time.sleep(1)
click('Bridge')
click('Start bridge')
end = time.monotonic() + 10
while time.monotonic() < end:
    if any(n.get('text') == 'Bridge active' for n in tree().iter('node')):
        print('Visible Start bridge activated enrolled endpoint', flush=True)
        break
    time.sleep(.5)
else:
    raise AssertionError('Bridge did not become active')

# Stage disposable test credentials in the production Rust layout. This is test
# preparation only; the production loader accepts no prior configuration format.
import json
import os
import tempfile
from pathlib import Path
config = json.loads(Path('android-framework/enrollment-client.json').read_text())
directory = Path(tempfile.mkdtemp(prefix='rust-enrollment-', dir='android-framework'))
uid, gid = int(os.environ['RUST_TEST_UID']), int(os.environ['RUST_TEST_GID'])
for name, source in [('ca.pem', 'ca'), ('client.pem', 'certificate'), ('client.key', 'private_key')]:
    target = directory / name
    with target.open('xb') as out:
        os.chmod(target, 0o600)
        out.write(Path(config[source]).read_bytes())
    os.chown(target, uid, gid)
endpoint = directory / 'endpoint.json'
with endpoint.open('x') as out:
    os.chmod(endpoint, 0o600)
    json.dump({'endpoint': f"{config['host']}:{config['port']}",
               'server_name': config['server_name'], 'server_sha256': config['server_sha256']}, out)
os.chown(endpoint, uid, gid)
os.chown(directory, uid, gid)
Path('android-framework/rust-enrollment-path.txt').write_text(str(directory.resolve()))
