#!/usr/bin/env python3
"""Runs only inside the isolated emulator container; every adb call pins the serial."""
import json, pathlib, subprocess, time, sys
ADB='android-framework/sdk/platform-tools/adb'
OUT=pathlib.Path('android-framework/results'); OUT.mkdir(exist_ok=True)

def adb(*args, check=True):
 p=subprocess.run([ADB,'-s','emulator-5554',*args],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=45)
 if check and p.returncode: raise RuntimeError(p.stdout)
 return p.stdout

def probe(client='trusted', **args):
 cmd=['shell','am','instrument','-w','-r']
 for k,v in args.items():cmd+=['-e',k,str(v)]
 cmd+=[f'org.omarchy.{client}/org.omarchy.probe.Probe']
 raw=adb(*cmd)
 fields={}
 for line in raw.splitlines():
  if line.startswith('INSTRUMENTATION_RESULT: '):
   k,_,v=line[24:].partition('=');fields[k]=v
 fields['raw']=raw
 return fields

if __name__=='__main__':
 for _ in range(120):
  if adb('shell','getprop','sys.boot_completed',check=False).strip()=='1':break
  time.sleep(2)
 else:raise SystemExit('BOOT_TIMEOUT')
 results={}
 def expect(name,result,outcome,value=None):
  results[name]=result
  (OUT/'permissions.json').write_text(json.dumps(results,indent=2)+'\n')
  assert result.get('outcome')==outcome,(name,result)
  if value is not None:assert result.get('value')==value,(name,result)
 for name in ['bridge','trusted','untrusted','norequest']:
  print(adb('install','--no-incremental','-r',f'android-framework/apks/{name}.apk'),flush=True)
 expect('authorized_client',probe(),'OK')
 status=json.loads(results['authorized_client']['value'])
 assert str(status['service_uid'])!=results['authorized_client']['client_uid']
 expect('different_signature',probe('untrusted'),'SecurityException')
 expect('same_signer_without_request',probe('norequest'),'SecurityException')
 expect('forwarded_binder',probe(relay='true'),'RELAY','SecurityException')
 expect('unknown_version',probe(version=2),'IllegalArgumentException')
 expect('wrong_interface',probe(token='wrong.interface'),'SecurityException')
 expect('unknown_operation',probe(operation=999),'IllegalArgumentException')
 expect('incall_direct_bind',probe(component='org.omarchy.bridge/org.omarchy.bridge.CallService'),'SecurityException')
 adb('shell','cmd','role','remove-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge',check=False)
 adb('shell','pm','revoke','org.omarchy.bridge','android.permission.READ_PHONE_STATE',check=False)
 expect('phone_permission_denied',probe(operation=2),'SecurityException')
 expect('framework_observer_denied',probe(operation=3),'SecurityException')
 adb('shell','pm','grant','org.omarchy.bridge','android.permission.READ_PHONE_STATE')
 expect('phone_permission_granted',probe(operation=2),'OK')
 expect('framework_observer_granted',probe(operation=3),'OK')
 adb('shell','pm','revoke','org.omarchy.bridge','android.permission.READ_PHONE_STATE')
 expect('phone_permission_revoked',probe(operation=2),'SecurityException')
 expect('framework_observer_revoked',probe(operation=3),'SecurityException')
 adb('shell','am','force-stop','org.omarchy.bridge')
 expect('rebind_after_force_stop',probe(),'OK')
 adb('shell','pm','disable-user','--user','0','org.omarchy.bridge')
 try:expect('disabled_package',probe(),'IllegalStateException')
 finally:adb('shell','pm','enable','org.omarchy.bridge')
 expect('reenabled_package',probe(),'OK')
 print(json.dumps({'passed':len(results),'checks':list(results)},indent=2))
