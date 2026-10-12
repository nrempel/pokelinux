#!/usr/bin/env python3
"""Actual native ARM64 Arch -> mTLS -> Android Telecom integration."""
from suite import adb,OUT
import hashlib,json,time,uuid,socket,ssl,struct,sys
from pathlib import Path
sys.path.insert(0,'/workspace/transport')
from omarchy_bridge import Client,receive,send
vm=Path('android-framework/transport-vm');checks={}
def record(name,value):
 checks[name]=value
 (OUT/'transport.json').write_text(json.dumps({'checks':checks,'hardware_verified':False},indent=2)+'\n')
 print('PASS',name,flush=True)
def request(op='status'):
 identity=uuid.uuid4().hex
 temp=vm/'request.tmp';temp.write_text(json.dumps({'id':identity,'op':op}));temp.replace(vm/'request.json')
 until=time.monotonic()+20
 while time.monotonic()<until:
  try:
   value=json.loads((vm/'response.json').read_text())
   if value['id']==identity:return value
  except (FileNotFoundError,json.JSONDecodeError):pass
  time.sleep(.1)
 raise TimeoutError('Arch client response')
def wait(name,predicate):
 end=time.monotonic()+40
 while time.monotonic()<end:
  value=request()
  if predicate(value):record(name,value);return value
  time.sleep(.5)
 raise AssertionError((name,value))
def ok(value):return value['outcome']=='OK'
def launch():
 adb('shell','am','start','-W','-n','org.omarchy.bridge/.DialActivity')
 adb('shell','input','keyevent','KEYCODE_HOME')

def gsm(action):assert 'OK' in adb('emu','gsm',action,'5550100')
original=adb('shell','cmd','role','get-role-holders','--user','0','android.app.role.DIALER').strip()
try:
 adb('install','--no-incremental','android-framework/apks/provisioner.apk')
 result=adb('shell','am','instrument','-w','org.omarchy.provision/org.omarchy.provision.Provision')
 assert 'PROVISIONED' in result,result
 # Provisioner is now gone. No instrumentation runs during network tests.
 adb('uninstall','org.omarchy.provision')
 launch()
 adb('forward','tcp:37179','tcp:37179')
 end=time.monotonic()+240
 while not (vm/'ready.json').exists() and time.monotonic()<end:time.sleep(1)
 ready=json.loads((vm/'ready.json').read_text());assert ready['architecture']=='aarch64' and 'Arch Linux ARM' in ready['os'],ready
 record('native_arch_arm64',ready)
 first=wait('normal_apk_without_instrumentation',ok)
 record('apk_process',{'instrumentation_running':adb('shell','dumpsys','activity','instrumentation').strip()})
 services=adb('shell','dumpsys','activity','services','org.omarchy.bridge')
 assert 'isForeground=true' in services,services
 assert 'org.omarchy.provision' not in adb('shell','pm','list','packages'), 'Provisioner still installed'
 record('foreground_service',{'active':True,'provisioner_removed':True})
 for op in ('wrong_pin','wrong_peer','wrong_hostname'):
  value=request(op);assert value['outcome']=='UNAVAILABLE',value;record(op,value)
 # Basic phone state does not take over the retained Android dialer.
 assert not first['value']['dialer_role']
 assert first['value']['phone_state']['availability']=='unavailable'
 record('coarse_permission_denied',True)
 adb('shell','pm','grant','org.omarchy.bridge','android.permission.READ_PHONE_STATE')
 wait('coarse_idle_without_dialer',lambda r:ok(r) and not r['value']['dialer_role'] and r['value']['phone_state']['state']==0)
 gsm('call');wait('coarse_ringing_without_dialer',lambda r:ok(r) and r['value']['phone_state']['state']==1)
 gsm('accept');wait('coarse_offhook_without_dialer',lambda r:ok(r) and r['value']['phone_state']['state']==2)
 gsm('cancel');wait('coarse_return_to_idle',lambda r:ok(r) and r['value']['phone_state']['state']==0)
 adb('shell','pm','revoke','org.omarchy.bridge','android.permission.READ_PHONE_STATE')
 wait('permission_revocation_clears_state',lambda r:r['outcome']=='UNAVAILABLE' or r['value']['phone_state']['availability']=='unavailable')
 launch()
 wait('coarse_permission_denied_after_restart',lambda r:ok(r) and r['value']['phone_state']['availability']=='unavailable' and r['value']['phone_state']['state'] is None)
 adb('shell','pm','grant','org.omarchy.bridge','android.permission.READ_PHONE_STATE')
 wait('permission_regrant_observes_fresh_state',lambda r:ok(r) and r['value']['phone_state']['state']==0)
 # Same CA, different leaf must fail; missing certificate must fail too.
 config=json.loads(Path('android-framework/transport-fixture/client.json').read_text())
 context=ssl.create_default_context(cafile=config['ca']);context.minimum_version=ssl.TLSVersion.TLSv1_3
 try:
  with socket.create_connection(('127.0.0.1',37179),timeout=7) as raw:
   with context.wrap_socket(raw,server_hostname=config['server_name']) as sock:
    send(sock,{'v':1,'id':uuid.uuid4().hex,'op':'status'});receive(sock)
  raise AssertionError('Missing client identity accepted')
 except (OSError,ValueError):record('missing_client_certificate',True)
 client=Client(config)
 for name,body in [('oversize',struct.pack('!I',4097)),('unknown_operation',b'{"v":1,"id":"'+b'0'*32+b'","op":"dial"}'),('duplicate_key',b'{"v":1,"v":1,"id":"'+b'0'*32+b'","op":"status"}')]:
  with socket.create_connection(('127.0.0.1',37179),timeout=7) as raw:
   with client.context.wrap_socket(raw,server_hostname=config['server_name']) as sock:
    sock.sendall(body if name=='oversize' else struct.pack('!I',len(body))+body)
    assert sock.recv(1)==b'',name
  record(name,True)
 # Queue saturation uses sockets that never start TLS. Hard deadlines must release them.
 stalled=[socket.create_connection(('127.0.0.1',37179),timeout=2) for _ in range(8)]
 time.sleep(6)
 for sock in stalled:sock.close()
 wait('recovery_after_stalled_handshakes',ok)
 adb('shell','cmd','role','add-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge')
 gsm('call')
 wait('ringing',lambda r:ok(r) and any(c['state']==2 for c in r['value']['calls']))
 gsm('accept')
 active=wait('active',lambda r:ok(r) and any(c['state']==4 for c in r['value']['calls']))
 adb('shell','cmd','role','remove-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge')
 wait('role_loss_invalidates_snapshot',lambda r:r['outcome']=='UNAVAILABLE' or (ok(r) and not r['value']['dialer_role'] and not r['value']['calls']))
 launch()
 wait('role_loss_clears_calls',lambda r:ok(r) and not r['value']['dialer_role'] and not r['value']['calls'] and r['value']['availability']=='unavailable')
 gsm('cancel')
 adb('shell','am','force-stop','org.omarchy.bridge')
 wait('force_stop_unavailable',lambda r:r['outcome']=='UNAVAILABLE' and r['calls']==[])
 time.sleep(2)
 assert request()['outcome']=='UNAVAILABLE','App restarted despite user force-stop'
 launch()
 restart=wait('explicit_restart_recovers',ok)
 assert restart['value']['generation']!=active['value']['generation']
 assert not restart['value']['calls']
 record('generation_changes_and_no_stale_calls',True)
 adb('shell','cmd','role','add-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge')
 gsm('call');wait('new_call_after_restart',lambda r:ok(r) and any(c['state']==2 for c in r['value']['calls']))
 gsm('cancel');wait('call_removed',lambda r:ok(r) and not r['value']['calls'])
 # Peer replacement takes effect only through a stopped service and complete provisioning.
 def provision_peer(name):
  pin=hashlib.sha256(Path('android-framework/transport-fixture/'+name+'.der').read_bytes()).hexdigest()
  adb('install','--no-incremental','android-framework/apks/provisioner.apk')
  value=adb('shell','am','instrument','-w','-e','peer_pin',pin,'org.omarchy.provision/org.omarchy.provision.Provision')
  assert 'PROVISIONED' in value,value
  adb('uninstall','org.omarchy.provision');launch()
 provision_peer('other')
 value=request();assert value['outcome']=='UNAVAILABLE',value;record('removed_peer_rejected',value)
 value=request('wrong_peer');assert ok(value),value;record('replacement_peer_accepted',value)
 for peer in ('expired','wrongpurpose','untrusted'):
  provision_peer(peer)
  value=request(peer);assert value['outcome']=='UNAVAILABLE',value;record('pinned_'+peer+'_rejected',value)
 if original:adb('shell','cmd','role','add-role-holder','--user','0','android.app.role.DIALER',original)
 else:adb('shell','cmd','role','remove-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge')
 provision_peer('client');wait('peer_restored',ok)
 before=request()['value']['generation']
 # This is a full emulator reboot, not a synthetic broadcast or instrumentation restart.
 adb('reboot')
 time.sleep(3)
 end=time.monotonic()+180
 while time.monotonic()<end:
  try:
   if adb('shell','getprop','sys.boot_completed').strip()=='1':break
  except Exception:pass
  time.sleep(2)
 else:raise TimeoutError('Emulator reboot')
 adb('forward','tcp:37179','tcp:37179')
 boot=wait('unlocked_boot_restarts_gateway',lambda r:ok(r) and not r['value']['dialer_role'])
 assert boot['value']['generation']!=before
 record('boot_new_generation',True)
 # Repeated visible launches must not create competing listeners.
 launch();launch();wait('repeated_start_idempotent',ok)
 print(json.dumps({'passed':len(checks)}),flush=True)
finally:
 try:gsm('cancel')
 except Exception:pass
 if original:adb('shell','cmd','role','add-role-holder','--user','0','android.app.role.DIALER',original)
 temp=vm/'request.tmp';temp.write_text(json.dumps({'id':uuid.uuid4().hex,'op':'stop'}));temp.replace(vm/'request.json')
