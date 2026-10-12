#!/usr/bin/env python3
"""Toggle call controls through visible UI in the owned emulator only."""
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

assert sys.argv[1:] in (['enable'], ['disable'])
wanted = 'true' if sys.argv[1] == 'enable' else 'false'
ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
def adb(*args):
    return subprocess.check_output(ADB + list(args), timeout=40)
def tree():
    adb('shell', 'uiautomator', 'dump', '/sdcard/window.xml')
    return ET.fromstring(adb('shell', 'cat', '/sdcard/window.xml'))
def node(label):
    found = [n for n in tree().iter('node') if n.get('text') == label and n.get('clickable') == 'true']
    assert len(found) == 1, label
    assert found[0].get('enabled') == 'true', label
    return found[0]
def tap(item):
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', item.get('bounds')))
    adb('shell', 'input', 'tap', str((x1+x2)//2), str((y1+y2)//2))
    time.sleep(.5)

adb('shell', 'am', 'start', '-n', 'org.omarchy.bridge/.DialActivity')
time.sleep(1)
tap(node('Bridge'))
control = node('Desktop call controls')
if control.get('checked') != wanted:
    tap(control)
assert node('Desktop call controls').get('checked') == wanted
print('Visible desktop call controls:', sys.argv[1], flush=True)
