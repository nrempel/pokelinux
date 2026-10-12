#!/usr/bin/env python3
"""Same-kernel direct loopback integration; ADB is only the test process launcher."""
import hashlib,json,re,shlex,subprocess,time,uuid,xml.etree.ElementTree as ET
from pathlib import Path
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554'];BASE='/data/local/tmp/omarchy-direct-test'
def adb(*args):return subprocess.check_output(ADB+list(args),text=True,stderr=subprocess.STDOUT,timeout=90)
def click(text):
 adb('shell','uiautomator','dump','/sdcard/window.xml');tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
 nodes=[n for n in tree.iter('node') if n.get('text')==text and n.get('clickable')=='true'];assert len(nodes)==1,(text,[n.get('text') for n in tree.iter('node')])
 x1,y1,x2,y2=map(int,re.findall(r'\d+',nodes[0].get('bounds')));adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.5)
def request(op,**values):
 command=['env','PYTHONHOME='+BASE+'/arch-runtime/usr','PYTHONUTF8=1',BASE+'/arch-runtime/usr/lib/ld-linux-x86-64.so.2','--library-path',BASE+'/arch-runtime/usr/lib',BASE+'/arch-runtime/usr/bin/python3',BASE+'/direct_client.py',BASE+'/client.json',json.dumps({'op':op,**values},separators=(',',':'))]
 return json.loads(adb('shell',shlex.join(command)))
def observe(predicate):
 end=time.monotonic()+30
 while time.monotonic()<end:
  result=request('controls')
  if predicate(result['value']):return result['value']
  time.sleep(.5)
 raise AssertionError(result)
def command(snapshot,action):
 call=snapshot['calls'][0];return dict(generation=snapshot['generation'],call_id=call['id'],expected_revision=snapshot['revision'],expected_state=call['state'],action=action)
original=adb('shell','cmd','role','get-role-holders','android.app.role.DIALER').strip();checks={};passed=False
try:
 adb('forward','--remove-all');assert not adb('forward','--list').strip()
 assert adb('shell','getenforce').strip()=='Enforcing'
 adb('shell','mkdir','-p',BASE);adb('shell','chmod','700',BASE)
 adb('push','android-framework/arch-runtime.tar.gz',BASE+'/runtime.tar.gz')
 adb('shell','tar','-xzf',BASE+'/runtime.tar.gz','-C',BASE)
 config=json.loads(Path('android-framework/enrollment-client.json').read_text());config['host']='127.0.0.1'
 for field in ('ca','certificate','private_key'):
  source=Path(config[field]);destination=BASE+'/'+field+'.pem';adb('push',str(source),destination);config[field]=destination
 config_file=Path('android-framework/direct-client.json');config_file.write_text(json.dumps(config));config_file.chmod(0o600)
 adb('push',str(config_file),BASE+'/client.json');adb('shell','chmod','600',BASE+'/private_key.pem',BASE+'/client.json')
 for source in ('transport/omarchy_bridge.py','framework/tests/direct_client.py'):adb('push',source,BASE+'/'+Path(source).name)
 adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1);click('Bridge');click('Start bridge')
 runtime=request('status');assert runtime['libc'][0]=='glibc' and runtime['uid']==2000 and runtime['architecture']=='x86_64',runtime
 assert runtime['net_namespace']==adb('shell','readlink','/proc/self/ns/net').strip()
 checks['direct_mtls']=runtime
 adb('shell','cmd','role','add-role-holder','android.app.role.DIALER','org.omarchy.bridge')
 adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(.5);click('Bridge');click('Desktop call controls')
 # Outgoing requests traverse actual Arch glibc -> mTLS -> public Telecom.
 assert request('dial_status')['value']['reason']=='disabled'
 click('Desktop outgoing calls')
 # Default dialer role normally grants the required call permissions. If Android
 # presents its permission dialog, this test must explicitly exercise it.
 dial=request('dial_status')['value']
 assert dial['can_dial'],dial
 checks['outgoing']=[]
 for iteration in range(2):
  dial=request('dial_status')['value'];assert dial['can_dial'],dial
  values=dict(generation=dial['generation'],offer=dial['offer'],number='5550100',operation_id=uuid.uuid4().hex)
  result=request('dial',command=values)['value'];assert result['outcome']=='invocation_attempted',result
  call=observe(lambda v:v['calls'] and 'disconnect' in v['calls'][0]['actions'])
  assert request('dial_status')['value']['pending']
  assert request('dial',command=values)['value']==result
  conflict=request('dial',command=dict(values,number='5550101'))['value'];assert conflict['reason']=='replay_conflict',conflict
  call=observe(lambda v:v['calls'] and 'disconnect' in v['calls'][0]['actions'])
  ended=request('control',command=command(call,'disconnect'))['value'];assert ended['outcome']=='submitted',ended
  observe(lambda v:not v['calls'])
  next_offer=request('dial_status')['value'];assert next_offer['can_dial'] and not next_offer['pending'],next_offer
  checks['outgoing'].append(dict(invocation=result,observed_call=call,duplicate_preserved=True,changed_number_rejected=True,terminal_cleared=next_offer))
 adb('emu','gsm','call','5550100');ringing=observe(lambda v:v['calls'] and v['calls'][0]['state']==2)
 answer=request('control',command=command(ringing,'answer'));assert answer['value']['outcome']=='submitted';checks['answer']=answer
 active=observe(lambda v:v['calls'] and v['calls'][0]['state']==4)
 assert re.search(r'^\s*\[Call id=.*state=ACTIVE',adb('shell','dumpsys','telecom'),re.M)
 end=request('control',command=command(active,'disconnect'));assert end['value']['outcome']=='submitted';checks['end']=end
 observe(lambda v:not v['calls'])
 adb('emu','gsm','call','5550100');ringing=observe(lambda v:v['calls'] and v['calls'][0]['state']==2)
 rejected=request('control',command=command(ringing,'reject'));assert rejected['value']['outcome']=='submitted';checks['decline']=rejected
 observe(lambda v:not v['calls']);assert not adb('forward','--list').strip()
 adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(.5);click('Bridge');click('Stop bridge')
 try:request('status');raise AssertionError('Stopped endpoint accepted request')
 except subprocess.CalledProcessError:checks['stop_unavailable']=True
 passed=True;print('Arch glibc -> direct Android loopback mTLS, no ADB forwarding: passed',flush=True)
finally:
 adb('emu','gsm','cancel','5550100')
 if original:adb('shell','cmd','role','add-role-holder','android.app.role.DIALER',original)
 Path('android-framework/results/direct-transport.json').write_text(json.dumps({'passed':passed,'checks':checks,'adb_data_forwarding':False,'selinux':'Enforcing','phone_accessed':False,'runtime_sha256':hashlib.sha256(Path('android-framework/arch-runtime.tar.gz').read_bytes()).hexdigest()},indent=2)+'\n')
