#!/usr/bin/env python3
"""Verify the SAF-enrolled AndroidKeyStore identity from actual ARM64 Arch."""
import hashlib,json,re,subprocess,time,uuid,xml.etree.ElementTree as ET
from pathlib import Path
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
ROOT=Path('android-framework/transport-vm')
def adb(*args):return subprocess.check_output(ADB+list(args),timeout=40)
def click(text):
    adb('shell','uiautomator','dump','/sdcard/window.xml')
    tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
    found=[n for n in tree.iter('node') if n.get('text')==text]
    assert len(found)==1,(text,[n.get('text') for n in tree.iter('node')])
    coords=list(map(int,re.findall(r'\d+',found[0].get('bounds'))))
    adb('shell','input','tap',str((coords[0]+coords[2])//2),str((coords[1]+coords[3])//2));time.sleep(.5)
def request(op,**args):
    token=uuid.uuid4().hex;temp=ROOT/'request.tmp';temp.write_text(json.dumps({'id':token,'op':op,**args}));temp.replace(ROOT/'request.json')
    deadline=time.monotonic()+(90 if op in ('enrolled_prepare_service','enrolled_qml_service') else 30)
    while time.monotonic()<deadline:
        try:
            response=json.loads((ROOT/'response.json').read_text())
            if response['id']==token:return response
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.2)
    raise TimeoutError(op)
def controls_when(predicate):
    deadline=time.monotonic()+30
    while time.monotonic()<deadline:
        result=request('enrolled_controls')
        if result['outcome']=='OK' and predicate(result['value']):return result['value']
        time.sleep(.5)
    raise AssertionError(result)
def command(snapshot,action,**changes):
    call=snapshot['calls'][0]
    values=dict(generation=snapshot['generation'],call_id=call['id'],expected_revision=snapshot['revision'],expected_state=call['state'],action=action,operation_id=uuid.uuid4().hex)
    values.update(changes);return values
def run_command(values,outcome,reason):
    result=request('enrolled_control',command=values)
    assert result['outcome']=='OK' and result['value']['outcome']==outcome and result['value']['reason']==reason,result
    return result
original_role=adb('shell','cmd','role','get-role-holders','android.app.role.DIALER').decode().strip()
checks={}
complete=False
radio_file=Path('android-framework/results/native-radio-log.txt').open('w')
radio_capture=subprocess.Popen(ADB+['logcat','-b','radio','-v','threadtime'],stdout=radio_file,stderr=subprocess.STDOUT)
try:
    deadline=time.monotonic()+240
    while not (ROOT/'ready.json').exists():
        if time.monotonic()>deadline:raise TimeoutError('Arch boot')
        time.sleep(1)
    ready=json.loads((ROOT/'ready.json').read_text());assert ready['architecture']=='aarch64' and 'Arch Linux' in ready['os']
    assert ready['client_sha256']==hashlib.sha256(Path('transport/omarchy_bridge.py').read_bytes()).hexdigest()
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1)
    click('Bridge');click('Start bridge');time.sleep(2)
    adb('forward','tcp:37179','tcp:37179')
    status=request('enrolled');assert status['outcome']=='OK',status
    controls=request('enrolled_controls');assert controls['outcome']=='OK',controls
    initial=controls['value'];assert not initial['controls_allowed']
    checks['disabled']=run_command(dict(generation=initial['generation'],call_id=1,expected_revision=initial['revision'],expected_state=2,action='answer',operation_id=uuid.uuid4().hex),'rejected','disabled')
    adb('shell','cmd','role','add-role-holder','android.app.role.DIALER','org.omarchy.bridge')
    adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1)
    click('Bridge');click('Desktop call controls')
    adb('emu','gsm','call','5550100')
    ringing=controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2 and value['controls_allowed'])
    service=request('enrolled_service')
    assert service['outcome']=='OK' and service['value']['passed'],service
    checks['background_service']=service
    assert service['value'].get('answer_after_local_disconnect') is True,service
    assert service['value'].get('duplicate_not_redispatched') is True,service
    assert service['value'].get('active_observed') is True,service
    assert service['value'].get('disconnect_observed_idle') is True,service
    controls_when(lambda value:not value['calls'])
    adb('emu','gsm','call','5550100')
    controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2)
    decline_service=request('enrolled_service', action='reject')
    assert decline_service['outcome']=='OK' and decline_service['value']['decline_observed_idle'],decline_service
    checks['background_service_decline']=decline_service
    controls_when(lambda value:not value['calls'])
    prepared=request('enrolled_prepare_service')
    assert prepared['outcome']=='OK' and prepared['value']['ready'],prepared
    checks['packaged_service_prepared']=prepared
    adb('emu','gsm','call','5550100')
    controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2)
    installed=request('enrolled_installed_service')
    assert installed['outcome']=='OK' and installed['value']['passed'],installed
    checks['packaged_user_service']=installed
    controls_when(lambda value:not value['calls'])
    adb('emu','gsm','call','5550100')
    controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2)
    qml=request('enrolled_qml_service')
    assert qml['outcome']=='OK' and qml['value']['passed'],qml
    checks['qml_user_service']=qml
    controls_when(lambda value:not value['calls'])
    adb('emu','gsm','call','5550100')
    ringing=controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2 and value['controls_allowed'])
    answer=command(ringing,'answer')
    checks['answer']=run_command(answer,'submitted','accepted')
    active=controls_when(lambda value:value['calls'] and value['calls'][0]['state']==4)
    telecom=adb('shell','dumpsys','telecom').decode()
    assert re.search(r'^\s*\[Call id=.*state=ACTIVE',telecom,re.M),telecom[:3000]
    checks['duplicate']=run_command(answer,'submitted','accepted')
    checks['conflicting_replay']=run_command(dict(answer,action='reject'),'rejected','replay_conflict')
    checks['stale_revision']=run_command(command(active,'disconnect',expected_revision=ringing['revision']),'rejected','stale_revision')
    active=controls_when(lambda value:value['calls'] and value['calls'][0]['state']==4)
    checks['hang_up']=run_command(command(active,'disconnect'),'submitted','accepted')
    controls_when(lambda value:not value['calls'])
    adb('emu','gsm','call','5550100')
    ringing=controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2)
    checks['decline']=run_command(command(ringing,'reject'),'submitted','accepted')
    controls_when(lambda value:not value['calls'])
    adb('emu','gsm','call','5550100')
    ringing=controls_when(lambda value:value['calls'] and value['calls'][0]['state']==2)
    old_command=command(ringing,'answer')
    time.sleep(2)
    # Dismiss the Android heads-up banner that covers the app navigation.
    adb('shell','input','swipe','360','160','360','40','200');time.sleep(.5)
    click('Bridge');click('Desktop call controls')
    checks['revocation']=run_command(old_command,'rejected','disabled')
    click('Desktop call controls')
    checks['new_authorization_generation']=run_command(old_command,'rejected','stale_generation')
    adb('emu','gsm','cancel','5550100')
    wrong=request('enrolled_wrong_pin');assert wrong['outcome']=='UNAVAILABLE',wrong
    click('Stop bridge');time.sleep(1)
    stopped=request('enrolled');assert stopped['outcome']=='UNAVAILABLE',stopped
    Path('android-framework/results/native-enrollment.json').write_text(json.dumps({'passed':True,'runtime':ready,'status':status,'controls':checks,'wrong_pin':wrong,'stopped':stopped,'phone_accessed':False},indent=2)+'\n')
    complete=True
    print('Native ARM64 Arch -> SAF-enrolled AndroidKeyStore TLS: passed',flush=True)
finally:
    radio_capture.terminate();radio_capture.wait(timeout=5);radio_file.close()
    Path('android-framework/results/native-radio.txt').write_bytes(adb('emu','gsm','list'))
    Path('android-framework/results/native-telecom.txt').write_bytes(adb('shell','dumpsys','telecom'))
    Path('android-framework/results/native-logcat.txt').write_bytes(adb('logcat','-d','-t','1000'))
    Path('android-framework/results/native-control-progress.json').write_text(json.dumps({'passed':complete,'checks':checks,'phone_accessed':False},indent=2)+'\n')
    adb('emu','gsm','cancel','5550100')
    if original_role:adb('shell','cmd','role','add-role-holder','android.app.role.DIALER',original_role)
    token=uuid.uuid4().hex;temp=ROOT/'request.tmp';temp.write_text(json.dumps({'id':token,'op':'stop'}));temp.replace(ROOT/'request.json')
