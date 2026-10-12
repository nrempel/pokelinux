#!/usr/bin/env python3
"""IMS lifecycle contract only: no network registration, call media, or carrier simulation."""
from suite import adb,probe,OUT
import json,time
pkg='org.omarchy.imsfixture';result={}
try:
 adb('logcat','-c')
 result['override']=adb('shell','cmd','phone','ims','set-ims-service','-s','0','-c','-f','1',pkg,check=False)
 result['selected']=adb('shell','cmd','phone','ims','get-ims-service','-s','0','-c','-f','1',check=False).strip()
 assert result['selected']==pkg,result
 end=time.monotonic()+30
 while time.monotonic()<end:
  log=adb('logcat','-d','-s','OmarchyIms:I','*:S')
  if 'FEATURE_READY_CALLBACK' in log:break
  time.sleep(1)
 result['lifecycle_log']=log
 assert 'CREATE_MMTEL' in log and 'FEATURE_READY_CALLBACK' in log,log
 result['direct_bind']=probe(component=pkg+'/org.omarchy.imsfixture.TestImsService')
 assert result['direct_bind'].get('outcome')=='SecurityException',result['direct_bind']
 result['framework_services']=adb('shell','dumpsys','activity','services',pkg)
 assert 'com.android.phone' in result['framework_services'],result['framework_services']
 result['carrier_registration_verified']=False
 result['passed']=True
finally:
 result['clear_override']=adb('shell','cmd','phone','ims','clear-ims-service-override','-s','0',check=False)
 (OUT/'ims-fixture.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
