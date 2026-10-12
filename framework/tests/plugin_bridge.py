#!/usr/bin/env python3
"""Full Omarchy/Hyprland touch -> installed Arch service -> enrolled Android."""
import hashlib,json,re,subprocess,time,xml.etree.ElementTree as ET
from pathlib import Path
from telecom_state import live_call_states

ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
ROOT=Path('android-framework/phone-vm')
OUT=Path('android-framework/results')
def adb(*args):return subprocess.check_output(ADB+list(args),timeout=40).decode()
def click(text):
    adb('shell','uiautomator','dump','/sdcard/window.xml')
    tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
    nodes=[n for n in tree.iter('node') if n.get('text')==text and n.get('clickable')=='true']
    assert len(nodes)==1,(text,[n.get('text') for n in tree.iter('node')])
    x1,y1,x2,y2=map(int,re.findall(r'\d+',nodes[0].get('bounds')))
    adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.5)
def wait_file(name,timeout=180):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        try:return json.loads((ROOT/name).read_text())
        except (FileNotFoundError,json.JSONDecodeError):pass
        if name!='result.json' and (ROOT/'result.json').exists():
            raise AssertionError((ROOT/'result.json').read_text())
        time.sleep(.2)
    raise TimeoutError(name)

role=adb('shell','cmd','role','get-role-holders','android.app.role.DIALER').strip()
checks={};passed=False
radio_file=(OUT/'plugin-radio.log').open('w')
radio=subprocess.Popen(ADB+['logcat','-b','radio','-v','threadtime'],stdout=radio_file,stderr=subprocess.STDOUT)
try:
    adb('shell','cmd','role','add-role-holder','android.app.role.DIALER','org.omarchy.bridge')
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1)
    click('Bridge');click('Start bridge');click('Desktop call controls');click('Desktop outgoing calls')
    adb('forward','tcp:37179','tcp:37179')
    outgoing=wait_file('outgoing.json',300)
    telecom=adb('shell','dumpsys','telecom')
    (OUT/'plugin-outgoing-telecom.txt').write_text(telecom)
    states=live_call_states(telecom)
    assert len(states)==1 and states[0] in ('DIALING','CONNECTING','ACTIVE'),states
    checks['native_outgoing']={'cli':outgoing,'android_states':states}
    (ROOT/'outgoing-verified.json').write_text('{}\n')
    ready=wait_file('ready.json',300)
    assert ready['architecture']=='aarch64' and ready['mode']=='native-omarchy-android',ready
    for name,digest in ready['installed_files'].items():
        assert hashlib.sha256((Path('transport')/name).read_bytes()).hexdigest()==digest,name
    assert not live_call_states(adb('shell','dumpsys','telecom'))
    adb('emu','gsm','call','5550100')
    checks['active_panel']=wait_file('active.json',90)
    assert checks['active_panel']['callState']==4 and checks['active_panel']['title']=='Call in progress'
    telecom=adb('shell','dumpsys','telecom')
    (OUT/'plugin-active-telecom.txt').write_text(telecom)
    assert live_call_states(telecom)==['ACTIVE'],live_call_states(telecom)
    checks['android_active']=True
    (ROOT/'active-verified.json').write_text('{}\n')
    guest=wait_file('result.json',90)
    assert guest['passed'],guest
    assert not live_call_states(adb('shell','dumpsys','telecom'))
    checks['android_idle']=True
    checks['guest']=guest
    passed=True
    print('Native Omarchy touchscreen -> installed Arch service -> Android Telecom: passed',flush=True)
finally:
    radio.terminate();radio.wait(timeout=5);radio_file.close()
    (OUT/'plugin-final-telecom.txt').write_text(adb('shell','dumpsys','telecom'))
    (OUT/'plugin-bridge.json').write_text(json.dumps({'passed':passed,'checks':checks,
        'phone_accessed':False,'hardware_verified':False},indent=2)+'\n')
    adb('emu','gsm','cancel','5550100')
    if role:adb('shell','cmd','role','add-role-holder','android.app.role.DIALER',role)
