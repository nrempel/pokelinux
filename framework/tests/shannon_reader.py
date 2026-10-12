#!/usr/bin/env python3
"""Only pure reader bytecode, synthetic bytes and an explicitly owned SDK emulator."""
import hashlib,json,subprocess,time
from pathlib import Path
adb_prefix=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):
    return subprocess.check_output(adb_prefix+list(args),timeout=45,text=True,stderr=subprocess.STDOUT)
adb('wait-for-device');end=time.monotonic()+180
while adb('shell','getprop','sys.boot_completed').strip()!='1':
    assert time.monotonic()<end,'owned emulator boot timeout';time.sleep(2)
assert adb('shell','getenforce').strip()=='Enforcing'
root=Path('artifacts/ims-response-rea');apk=Path('artifacts/telephony-audit/input/system_ext/priv-app/ShannonIms/ShannonIms.apk')
assert hashlib.sha256(apk.read_bytes()).hexdigest()=='8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879'
remote='/data/local/tmp/pokelinux-reader-oracle';adb('shell','mkdir','-p',remote)
for source,name in [(apk,'input.apk'),(root/'oracle/oracle.jar','oracle.jar')]:
    adb('push',str(source),remote+'/'+name);adb('shell','chmod','444',remote+'/'+name)
output=adb('shell','env','CLASSPATH='+remote+'/oracle.jar:'+remote+'/input.apk','app_process','/system/bin','org.omarchy.audit.ReaderOracle')
(root/'original-reader-output.txt').write_text(output)
rows=[json.loads(line) for line in output.splitlines() if line.startswith('{')]
assert len(rows)>200 and len({v['id'] for v in rows})==len(rows)
(root/'original-reader.json').write_text(json.dumps(rows,indent=2)+'\n')
(root/'oracle-result.json').write_text(json.dumps({'passed':True,'cases':len(rows),'original_apk_executed':True,'pure_reader_only':True,'services_or_modem_called':False,'hardware_verified':False,'selinux':'Enforcing'},indent=2)+'\n')
print('Original IMS reader synthetic cases:',len(rows),flush=True)
