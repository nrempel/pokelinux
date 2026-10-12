#!/usr/bin/env python3
"""Diagnostic: repeated stock Android calls, with no bridge APK installed.

Run only inside the owned disposable emulator container. This experiment
records each real result; it does not retry an Answer or force ACTIVE.
"""
import json
from pathlib import Path
import re
import subprocess
import time

ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
OUT = Path('android-framework/results/radio-baseline')
OUT.mkdir(parents=True, exist_ok=True)

def adb(*args):
    return subprocess.check_output(ADB + list(args), timeout=40).decode()

def wait_for(predicate, timeout=10):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = predicate()
        if value:
            return value
        time.sleep(.2)
    return None

adb('wait-for-device')
assert wait_for(lambda: adb('shell', 'getprop', 'sys.boot_completed').strip() == '1', 180)
assert 'package:org.omarchy.bridge' not in adb('shell', 'pm', 'list', 'packages', 'org.omarchy.bridge')
role = adb('shell', 'cmd', 'role', 'get-role-holders', 'android.app.role.DIALER').strip()
assert role and role != 'org.omarchy.bridge', role
records = []
radio_file = (OUT / 'radio.log').open('w')
radio = subprocess.Popen(ADB + ['logcat', '-b', 'radio', '-v', 'threadtime'],
                         stdout=radio_file, stderr=subprocess.STDOUT)
try:
    for index in range(10):
        number = '5550100'
        adb('emu', 'gsm', 'call', number)
        assert wait_for(lambda: re.search(r'^\s*\[Call id=.*state=RINGING',
                                         adb('shell', 'dumpsys', 'telecom'), re.M))
        before = adb('emu', 'gsm', 'list')
        if index == 0:
            (OUT / 'stock-incoming.png').write_bytes(
                subprocess.check_output(ADB + ['exec-out', 'screencap', '-p'], timeout=20))
        adb('shell', 'input', 'keyevent', 'KEYCODE_CALL')
        active = bool(wait_for(lambda: re.search(r'^\s*\[Call id=.*state=ACTIVE',
                                                 adb('shell', 'dumpsys', 'telecom'), re.M)))
        after = adb('emu', 'gsm', 'list')
        if index == 0:
            (OUT / 'stock-after-answer.png').write_bytes(
                subprocess.check_output(ADB + ['exec-out', 'screencap', '-p'], timeout=20))
        records.append({'iteration': index, 'active': active,
                        'radio_before': before, 'radio_after': after})
        (OUT / f'telecom-{index}.txt').write_text(adb('shell', 'dumpsys', 'telecom'))
        adb('emu', 'gsm', 'cancel', number)
        assert wait_for(lambda: not re.search(r'^\s*\[Call id=.*state=(RINGING|ACTIVE)',
                                              adb('shell', 'dumpsys', 'telecom'), re.M))
        time.sleep(.5)
finally:
    radio.terminate()
    radio.wait(timeout=5)
    radio_file.close()
    (OUT / 'result.json').write_text(json.dumps({
        'passed': len(records) == 10 and all(r['active'] for r in records),
        'dialer': role, 'bridge_installed': False, 'phone_accessed': False,
        'input': 'Android KEYCODE_CALL', 'records': records,
    }, indent=2) + '\n')
    adb('emu', 'gsm', 'cancel', '5550100')
assert len(records) == 10 and all(r['active'] for r in records), records
print('Stock Android radio baseline: 10 calls passed', flush=True)
