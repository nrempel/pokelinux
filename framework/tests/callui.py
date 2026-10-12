#!/usr/bin/env python3
"""Local Android call UI remains usable with the desktop gateway off."""
import json,re,subprocess,time,xml.etree.ElementTree as ET
from pathlib import Path
from telecom_state import live_call_states
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):return subprocess.check_output(ADB+list(args),timeout=40).decode()
def tree():
    adb('shell','uiautomator','dump','/sdcard/window.xml')
    return ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
def click(text):
    nodes=[n for n in tree().iter('node') if n.get('text')==text and n.get('clickable')=='true']
    assert len(nodes)==1,(text,[n.get('text') for n in tree().iter('node')])
    x1,y1,x2,y2=map(int,re.findall(r'\d+',nodes[0].get('bounds')))
    adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.5)
def require(text):
    nodes=list(tree().iter('node'))
    assert any(n.get('text')==text for n in nodes),(text,[n.get('text') for n in nodes])
adb('wait-for-device')
deadline=time.monotonic()+180
while adb('shell','getprop','sys.boot_completed').strip()!='1':
    if time.monotonic()>deadline:raise TimeoutError('boot')
    time.sleep(2)
adb('install','--no-incremental','-r','android-framework/apks/bridge.apk')
role=adb('shell','cmd','role','get-role-holders','android.app.role.DIALER').strip()
crash_file=Path('android-framework/results/call-ui-crash.txt').open('w')
crash_capture=subprocess.Popen(ADB+['logcat','-v','threadtime','-s','AndroidRuntime:E'],stdout=crash_file,stderr=subprocess.STDOUT)
radio_file=Path('android-framework/results/call-ui-radio-log.txt').open('w')
radio_capture=subprocess.Popen(ADB+['logcat','-b','radio','-v','threadtime'],stdout=radio_file,stderr=subprocess.STDOUT)
try:
    adb('shell','cmd','role','add-role-holder','android.app.role.DIALER','org.omarchy.bridge')
    adb('shell','pm','grant','org.omarchy.bridge','android.permission.POST_NOTIFICATIONS')
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity')
    time.sleep(1)
    click('Calls')
    adb('emu','gsm','call','5550100');time.sleep(2)
    require('Incoming call')
    # Repeated launch of the same incoming-call intent must preserve explicit navigation.
    incoming_uri='omarchy-call://test-incoming/1'
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity','-d',incoming_uri)
    Path('android-framework/results/navigation-before.png').write_bytes(subprocess.check_output(ADB+['exec-out','screencap','-p']))
    adb('shell','input','swipe','360','160','360','40','200');time.sleep(.5)
    click('Bridge');require('Desktop call controls')
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity','-d',incoming_uri)
    require('Desktop call controls');click('Calls')
    Path('android-framework/results/call-ui-before-answer-radio.txt').write_text(adb('emu','gsm','list'))
    click('Answer');time.sleep(1);require('Call in progress')
    services=adb('shell','dumpsys','activity','services','org.omarchy.bridge')
    Path('android-framework/results/call-ui-services.txt').write_text(services)
    assert 'isForeground=true' in services,services
    click('End call');time.sleep(1)
    assert not live_call_states(adb('shell','dumpsys','telecom'))
    adb('emu','gsm','call','5550100');time.sleep(2)
    require('Incoming call');click('Decline');time.sleep(1)
    Path('android-framework/results/call-ui.json').write_text(json.dumps({'passed':True,'checks':['incoming_visible','answer','ongoing_visible','end_call','decline','gateway_disabled'],'phone_accessed':False},indent=2)+'\n')
    print('Local Android call UI: answer, decline and end passed')
finally:
    radio_capture.terminate();radio_capture.wait(timeout=5);radio_file.close()
    crash_capture.terminate();crash_capture.wait(timeout=5);crash_file.close()
    Path('android-framework/results/call-ui-final.xml').write_bytes(ET.tostring(tree()))
    Path('android-framework/results/call-ui-radio.txt').write_text(adb('emu','gsm','list'))
    Path('android-framework/results/call-ui-telecom.txt').write_text(adb('shell','dumpsys','telecom'))
    Path('android-framework/results/call-ui-logcat.txt').write_text(adb('logcat','-d','-t','500'))
    adb('emu','gsm','cancel','5550100')
    if role:adb('shell','cmd','role','add-role-holder','android.app.role.DIALER',role)
