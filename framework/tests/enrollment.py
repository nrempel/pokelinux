#!/usr/bin/env python3
"""Production AndroidKeyStore enrollment and TLS adapter tests; emulator only."""
import base64,hashlib,json,os,subprocess,sys,time,zipfile,io,re,xml.etree.ElementTree as ET
from pathlib import Path
from emulator_clock import await_certificate_clock
sys.path.insert(0,'transport')
from enroll import issue_bundle
from omarchy_bridge import Client
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
ROOT=Path('android-framework/enrollment-fixture')
RESULT=Path('android-framework/results/enrollment.json')
checks={}
def adb(*args):return subprocess.check_output(ADB+list(args),text=True,stderr=subprocess.STDOUT,timeout=45)
def probe(op,**args):
    cmd=['shell','am','instrument','-w','-e','op',op]
    for k,v in args.items():cmd+=['-e',k,v]
    return adb(*cmd,'org.omarchy.enrollmentprobe/.Probe')
def start():
    output=adb('shell','am','start','-W','-n','org.omarchy.bridge/.DialActivity')
    assert 'Error:' not in output,output
    deadline=time.monotonic()+12
    while True:
        adb('shell','uiautomator','dump','/sdcard/window.xml')
        view=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
        if any(n.get('text')=='Bridge' and n.get('clickable')=='true' for n in view.iter('node')):break
        if time.monotonic()>=deadline:raise AssertionError(('DialActivity did not become visible',output,[n.get('text') for n in view.iter('node')]))
        time.sleep(.2)
    adb('forward','tcp:37179','tcp:37179')
def click(text):
    adb('shell','uiautomator','dump','/sdcard/window.xml')
    tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
    nodes=[n for n in tree.iter('node') if n.get('text')==text and n.get('clickable')=='true']
    assert len(nodes)==1,(text,[n.get('text') for n in tree.iter('node')])
    x1,y1,x2,y2=map(int,re.findall(r'\d+',nodes[0].get('bounds')))
    adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.5)
def observe(client):
    deadline=time.monotonic()+20
    while True:
        try:return client.status()
        except (OSError,ValueError):
            if time.monotonic()>deadline:raise
            time.sleep(.5)
try:
    adb('wait-for-device')
    deadline=time.monotonic()+180
    while adb('shell','getprop','sys.boot_completed').strip()!='1':
        if time.monotonic()>deadline:raise RuntimeError('boot timeout')
        time.sleep(2)
    for name in ['bridge','enrollmentprobe']:adb('install','--no-incremental','-r',f'android-framework/apks/{name}.apk')
    # Upgrade residue cannot select the old test file identity in the release APK.
    adb('install','--no-incremental','android-framework/apks/provisioner.apk')
    legacy=adb('shell','am','instrument','-w','org.omarchy.provision/org.omarchy.provision.Provision');assert 'PROVISIONED' in legacy
    adb('uninstall','org.omarchy.provision');start()
    old_client=Client(json.loads(Path('android-framework/transport-fixture/client.json').read_text()))
    try:old_client.status();raise AssertionError('Release activated legacy credentials')
    except OSError:checks['legacy_upgrade_identity_rejected']=True
    probe('disable')
    output=probe('request');assert 'non_exportable=true' in output,output
    request=next(line.split('request=',1)[1] for line in output.splitlines() if line.startswith('INSTRUMENTATION_RESULT: request='))
    ROOT.mkdir(mode=0o700,exist_ok=True)
    stamp=str(time.time_ns());work=ROOT/stamp;work.mkdir(mode=0o700)
    (work/'request.json').write_bytes(base64.b64decode(request))
    checks['android_key_non_exportable']=True
    pins=issue_bundle(work/'request.json',work/'issued',days=2)
    await_certificate_clock(adb,work/'issued','android-framework/results/enrollment-clock.json')
    bundle=(work/'issued/phone-enrollment.zip').read_bytes()
    altered=io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(bundle)) as source,zipfile.ZipFile(altered,'w') as dest:
        for name in source.namelist():dest.writestr(name,b'b'*32 if name=='request-id' else source.read(name))
    output=probe('import',bundle=base64.b64encode(altered.getvalue()).decode());assert 'GeneralSecurityException' in output,output
    checks['wrong_request_rejected']=True
    output=probe('import',bundle=base64.b64encode(bundle).decode());assert 'ENROLLED' in output,output
    checks['public_bundle_imported']=True
    start();click('Bridge');click('Start bridge')
    client=Client(json.loads((work/'issued/client.json').read_text()))
    status=observe(client);checks['android_keystore_tls13']=status
    output=probe('import',bundle=base64.b64encode(bundle).decode());assert 'IllegalStateException' in output,output
    checks['enabled_replacement_rejected']=True
    start();observe(client);checks['rejection_preserves_active_pairing']=True
    click('Bridge');click('Stop bridge')
    start()
    try:client.status();raise AssertionError('disabled bridge accepted connection')
    except OSError:checks['disabled_activity_does_not_restart']=True
    Path('android-framework/enrollment-prior-client.json').write_text((work/'issued/client.json').read_text());Path('android-framework/enrollment-prior-client.json').chmod(0o600)
    checks['phone_accessed']=False
    RESULT.write_text(json.dumps({'passed':True,'checks':checks},indent=2)+'\n')
    print(json.dumps({'enrollment':'passed','checks':list(checks)},indent=2))
except Exception as error:
    RESULT.write_text(json.dumps({'passed':False,'checks':checks,'error':str(error)},indent=2)+'\n')
    raise
