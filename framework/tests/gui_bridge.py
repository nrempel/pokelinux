#!/usr/bin/env python3
"""Actual pacman-installed ARM64 GTK/Hyprland touch -> mTLS -> Android Telecom."""
import hashlib,json,re,subprocess,time,uuid,xml.etree.ElementTree as ET
from pathlib import Path
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554'];ROOT=Path('android-framework/phone-vm')
def adb(*args):return subprocess.check_output(ADB+list(args),timeout=40).decode()
def click(text):
    adb('shell','uiautomator','dump','/sdcard/window.xml');tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
    nodes=[n for n in tree.iter('node') if n.get('text')==text and n.get('clickable')=='true'];assert len(nodes)==1,(text,[n.get('text') for n in tree.iter('node')])
    x1,y1,x2,y2=map(int,re.findall(r'\d+',nodes[0].get('bounds')));adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.5)
def request(op,**args):
    token=uuid.uuid4().hex;temp=ROOT/'request.tmp';temp.write_text(json.dumps({'id':token,'op':op,**args}));temp.replace(ROOT/'request.json')
    if op=='stop':return
    end=time.monotonic()+25
    while time.monotonic()<end:
        try:
            response=json.loads((ROOT/'response.json').read_text())
            if response['id']==token:return response
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.2)
    raise TimeoutError(op)
def observed(predicate):
    end=time.monotonic()+45;value=None
    while time.monotonic()<end:
        value=request('inspect')
        if predicate(value):return value
        time.sleep(.5)
    raise AssertionError(value)
def state(value,wanted):return value.get('snapshot') and value['snapshot']['calls'] and value['snapshot']['calls'][0]['state']==wanted
def button(value,label):return any(b['label']==label and b['enabled'] for b in value['buttons'])
original=adb('shell','cmd','role','get-role-holders','android.app.role.DIALER').strip();checks={};passed=False
try:
    adb('shell','cmd','role','add-role-holder','android.app.role.DIALER','org.omarchy.bridge')
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1)
    click('Bridge');click('Start bridge');click('Desktop call controls');click('Desktop outgoing calls')
    adb('forward','tcp:37179','tcp:37179')
    end=time.monotonic()+420
    while not (ROOT/'ready.json').exists():
        if (ROOT/'result.json').exists():raise AssertionError((ROOT/'result.json').read_text())
        if time.monotonic()>end:raise TimeoutError('Graphical Arch boot')
        time.sleep(1)
    ready=json.loads((ROOT/'ready.json').read_text());assert ready['architecture']=='aarch64'
    for name,digest in ready['installed_files'].items():assert digest==hashlib.sha256((Path('transport')/name).read_bytes()).hexdigest(),name
    observed(lambda v:'Phone connected' in v['labels'])
    launched=request('launch');assert launched['launched']
    checks['launcher']=observed(lambda v:v.get('app_id')=='org.omarchy.Phone' and v.get('windows')==1 and v.get('activations',0)>=2)
    request('theme')
    checks['theme']=observed(lambda v:'#33cc99' in v.get('theme',''))
    request('touch',button='Keypad')
    request('screenshot',name='keypad-open.png')
    for digit in '5550100':request('touch',button=digit)
    request('screenshot',name='keypad-entered.png')
    observed(lambda v:v.get('number')=='5550100' and button(v,'Call'))
    request('screenshot',name='dial-ready.png')
    request('touch',button='Keypad')
    checks['outgoing']=[]
    for iteration in range(2):
        observed(lambda v:button(v,'Call'))
        request('touch',button='Call')
        dialing=observed(lambda v:state(v,4) and button(v,'End call') and not button(v,'Call'))
        request('screenshot',name='dialing.png');request('touch',button='End call')
        ended=observed(lambda v:v.get('snapshot') and not v['snapshot']['calls'] and button(v,'Call'))
        checks['outgoing'].append(dict(dialing=dialing,ended=ended))
    adb('emu','gsm','call','5550100')
    checks['incoming']=observed(lambda v:state(v,2) and button(v,'Answer'))
    request('screenshot',name='incoming.png');request('touch',button='Answer')
    checks['answered']=observed(lambda v:state(v,4) and button(v,'End call') and v['outcome'] is None)
    telecom=adb('shell','dumpsys','telecom');assert re.search(r'^\s*\[Call id=.*state=ACTIVE',telecom,re.M)
    request('screenshot',name='active.png');request('touch',button='End call')
    checks['ended']=observed(lambda v:v.get('snapshot') and not v['snapshot']['calls'])
    adb('emu','gsm','call','5550100')
    observed(lambda v:state(v,2) and button(v,'Decline'));request('touch',button='Decline')
    checks['declined']=observed(lambda v:v.get('snapshot') and not v['snapshot']['calls'])
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1);click('Bridge');click('Stop bridge')
    checks['unavailable']=observed(lambda v:'Phone unavailable' in v['labels'] and not any(b['enabled'] for b in v['buttons'] if b['label'] in ('Answer','Decline','End call')))
    request('screenshot',name='unavailable.png');passed=True
    print('Installed Arch GTK/Hyprland guest touch -> Android call controls: passed',flush=True)
finally:
    Path('android-framework/results/gui-bridge.json').write_text(json.dumps({'passed':passed,'checks':checks,'phone_accessed':False,'graphics_hardware_verified':False},indent=2)+'\n')
    adb('emu','gsm','cancel','5550100')
    if original:adb('shell','cmd','role','add-role-holder','android.app.role.DIALER',original)
    request('stop')
