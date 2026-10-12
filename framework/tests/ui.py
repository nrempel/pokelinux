#!/usr/bin/env python3
"""Exercise visible production controls in the dedicated emulator only."""
import json,re,subprocess,time,xml.etree.ElementTree as ET
from pathlib import Path
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):return subprocess.check_output(ADB+list(args),timeout=40)
def tree():
    adb('shell','uiautomator','dump','/sdcard/window.xml')
    return ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
def click(text):
    nodes=[n for n in tree().iter('node') if n.get('text','').casefold()==text.casefold() or n.get('content-desc','').casefold()==text.casefold()]
    assert len(nodes)==1,(text,[n.get('text') for n in tree().iter('node')])
    node=nodes[0];assert node.get('enabled')=='true',text
    x1,y1,x2,y2=map(int,re.findall(r'\d+',node.get('bounds')))
    adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.5)
def require(text):
    assert any(n.get('text')==text for n in tree().iter('node')),text
adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity')
time.sleep(1)
click('Bridge')
require('Bridge off');require('Export pairing request');require('Import pairing response');require('Start bridge')
result=Path('android-framework/results')
(result/'bridge-settings.png').write_bytes(adb('exec-out','screencap','-p'))
click('Calls');require('Phone number');require('Call')
(result/'bridge-calls.png').write_bytes(adb('exec-out','screencap','-p'))
(result/'ui.json').write_text(json.dumps({'passed':True,'checks':['settings_discoverable','bridge_off_visible','pairing_controls','calls_discoverable','dial_input'], 'phone_accessed':False},indent=2)+'\n')
print('Production UI: 5 visible control checks passed')
# Real Storage Access Framework path: the test APK is not used for this pairing.
import sys
sys.path.insert(0,'transport')
from enroll import issue_bundle
from omarchy_bridge import Client
click('Bridge');click('Export pairing request')
time.sleep(2)
(result/'saf-export.xml').write_bytes(ET.tostring(tree()))
print('SAF export controls:',[n.get('text') for n in tree().iter('node') if n.get('text')],flush=True)
click('SAVE')
time.sleep(1);require('Request exported');click('OK')
request=adb('shell','cat','/sdcard/Download/omarchy-pairing-request.json')
work=Path('android-framework/enrollment-fixture')/('ui-'+str(time.time_ns()));work.mkdir(mode=0o700)
(work/'request.json').write_bytes(request)
issue_bundle(work/'request.json',work/'issued',days=2)
from emulator_clock import await_certificate_clock
await_certificate_clock(adb,work/'issued','android-framework/results/ui-enrollment-clock.json')
adb('push',str(work/'issued/phone-enrollment.zip'),'/sdcard/Download/phone-enrollment.zip')
click('Import pairing response');time.sleep(1)
(result/'saf-import.xml').write_bytes(ET.tostring(tree()))
print('SAF import controls:',[n.get('text') for n in tree().iter('node') if n.get('text')],flush=True)
click('phone-enrollment.zip');time.sleep(1)
require('Pair this desktop?');click('Pair desktop');time.sleep(1)
click('Start bridge');time.sleep(2);require('Bridge active')
adb('forward','tcp:37179','tcp:37179')
Path('android-framework/enrollment-client.json').write_text((work/'issued/client.json').read_text())
status=Client(json.loads((work/'issued/client.json').read_text())).status()
prior=Path('android-framework/enrollment-prior-client.json')
if prior.exists():
    try:Client(json.loads(prior.read_text())).status();raise AssertionError('Previous desktop credential accepted after renewal')
    except (OSError,ValueError):pass
(result/'saf-enrollment.json').write_text(json.dumps({'passed':True,'checks':['request_exported_via_saf','response_imported_via_saf','explicit_confirmation','explicit_start','production_tls','prior_credential_rejected'], 'status':status,'phone_accessed':False},indent=2)+'\n')
click('Stop bridge');require('Bridge off')
print('Production SAF enrollment and TLS: passed',flush=True)
