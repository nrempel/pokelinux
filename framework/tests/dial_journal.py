#!/usr/bin/env python3
"""Real Android storage survives bridge process death; no call is placed."""
import json,subprocess,time
from pathlib import Path
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):return subprocess.check_output(ADB+list(args),text=True,stderr=subprocess.STDOUT,timeout=45)
def probe(op):return adb('shell','am','instrument','-w','-e','op',op,'org.omarchy.enrollmentprobe/.Probe')
adb('wait-for-device');deadline=time.monotonic()+180
while adb('shell','getprop','sys.boot_completed').strip()!='1':
    if time.monotonic()>deadline:raise TimeoutError('Emulator boot')
    time.sleep(2)
for name in ('bridge','enrollmentprobe'):adb('install','--no-incremental','-r',f'android-framework/apks/{name}.apk')
assert 'Enforcing' in adb('shell','getenforce')
checks={}
for op,expected in (('journal_begin','RESERVED'),('journal_read','PENDING'),('journal_unrelated','PENDING'),('journal_end','RESOLVED'),('journal_read','EMPTY')):
    output=probe(op);assert 'outcome='+expected in output,(op,output)
    checks[op+'_'+expected]=True
    adb('shell','am','force-stop','org.omarchy.bridge')
result={'passed':True,'checks':checks,'selinux':'Enforcing','phone_accessed':False,'calls_placed':False}
Path('android-framework/results/dial-journal.json').write_text(json.dumps(result,indent=2)+'\n')
print('Android outgoing journal survives process death and exact completion clears it',flush=True)
