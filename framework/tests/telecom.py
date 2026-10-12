#!/usr/bin/env python3
"""Real Telecom callbacks from the emulator radio, never a physical or carrier call."""
from suite import adb,probe,OUT
import json,time
results={}
def status():
 p=probe();assert p.get('outcome')=='OK',p
 return json.loads(p['value'])
def wait_for(name,predicate):
 end=time.monotonic()+30
 while time.monotonic()<end:
  value=status()
  if predicate(value):
   results[name]=value
   (OUT/'telecom.json').write_text(json.dumps(results,indent=2)+'\n')
   return value
  time.sleep(0.5)
 raise AssertionError((name,value))
def command(*args):
 result=adb('emu','gsm',*args);assert 'OK' in result,result
# Not a routable destination: command enters the emulator's local radio simulator.
NUMBER='5550100'
try:
 command('cancel',NUMBER)
except AssertionError:pass
original=adb('shell','cmd','role','get-role-holders','--user','0','android.app.role.DIALER').strip()
try:
 adb('shell','cmd','role','add-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge')
 wait_for('role_active',lambda s:s['dialer_role'])
 command('call',NUMBER)
 wait_for('ringing_callback',lambda s:any(c['state']==2 for c in s['calls']))
 command('accept',NUMBER)
 wait_for('active_callback',lambda s:any(c['state']==4 for c in s['calls']))
 command('cancel',NUMBER)
 wait_for('removed_callback',lambda s:not s['calls'])
 adb('shell','cmd','role','remove-role-holder','--user','0','android.app.role.DIALER','org.omarchy.bridge')
 wait_for('role_removed',lambda s:not s['dialer_role'])
 command('call',NUMBER)
 time.sleep(3)
 value=status();assert not value['calls'],value
 results['no_callback_without_role']=value
 (OUT/'telecom.json').write_text(json.dumps(results,indent=2)+'\n')
 print(json.dumps({'passed':len(results),'checks':list(results)},indent=2))
finally:
 command('cancel',NUMBER)
 if original and original!='org.omarchy.bridge':
  adb('shell','cmd','role','add-role-holder','--user','0','android.app.role.DIALER',original)
