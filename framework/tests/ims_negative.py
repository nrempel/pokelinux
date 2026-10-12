#!/usr/bin/env python3
from suite import adb,OUT
import json,time
adb('logcat','-c')
result={}
try:
 result['install']=adb('install','--no-incremental','-r','android-framework/apks/imsbad.apk')
 for _ in range(30):
  if adb('shell','service','check','phone',check=False).strip()=='Service phone: found':break
  time.sleep(1)
 result['override']=adb('shell','cmd','phone','ims','set-ims-service','-s','0','-c','-f','1','org.omarchy.imsbad')
 time.sleep(3)
 log=adb('logcat','-d','-v','brief')
 relevant=[line for line in log.splitlines() if 'imsbad' in line or 'OmarchyIms' in line]
 result['resolver_log']='\n'.join(relevant)
 assert any('BIND_IMS_SERVICE' in line and 'imsbad' in line for line in relevant),result
 assert not any('CREATE_MMTEL' in line for line in relevant),result
 result['rejected_without_permission_bypass']=True
finally:
 result['clear']=adb('shell','cmd','phone','ims','clear-ims-service-override','-s','0',check=False)
 adb('uninstall','org.omarchy.imsbad',check=False)
 (OUT/'ims-negative.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
