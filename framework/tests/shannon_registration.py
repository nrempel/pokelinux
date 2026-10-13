#!/usr/bin/env python3
"""Pure original parser probes on one explicitly owned SDK emulator only."""
import hashlib,json,subprocess,time,sys
from pathlib import Path
adb_prefix=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):
 return subprocess.check_output(adb_prefix+list(args),timeout=60,text=True,stderr=subprocess.STDOUT)
adb('wait-for-device');end=time.monotonic()+180
while adb('shell','getprop','sys.boot_completed').strip()!='1':
 assert time.monotonic()<end,'owned emulator boot timeout';time.sleep(2)
assert adb('shell','getenforce').strip()=='Enforcing'
root=Path('artifacts/ims-registration-rea');apk=Path('artifacts/telephony-audit/input/system_ext/priv-app/ShannonIms/ShannonIms.apk')
assert hashlib.sha256(apk.read_bytes()).hexdigest()=='8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879'
remote='/data/local/tmp/pokelinux-registration-indication-oracle';adb('shell','mkdir','-p',remote)
for source,name in [(apk,'input.apk'),(root/'oracle/oracle.jar','oracle.jar')]:
 adb('push',str(source),remote+'/'+name);adb('shell','chmod','444',remote+'/'+name)
mode=sys.argv[1] if len(sys.argv)>1 else 'metadata'
assert mode in ('metadata','parser')
args=['metadata'] if mode=='metadata' else []
record=root/'runtime'/sys.argv[2] if len(sys.argv)>2 else root
try:
 output=adb('shell','env','CLASSPATH='+remote+'/oracle.jar:'+remote+'/input.apk','app_process','/system/bin','org.omarchy.audit.RegistrationIndicationOracle',*args)
except subprocess.CalledProcessError as failure:
 (record/'original-parser-failed-output.txt').write_text(failure.output)
 (record/'original-parser-failed-result.json').write_text(json.dumps({'exit_code':failure.returncode,'original_completed':False},indent=2)+'\n')
 for name,command in [('android-memory.txt',('shell','dumpsys','meminfo')),('android-kill-log.txt',('logcat','-d','-b','system','-b','main','-t','300'))]:
  try:(record/name).write_text(adb(*command))
  except Exception as error:(record/(name+'.error')).write_text(type(error).__name__)
 raise
(record/('original-'+mode+'-output.txt')).write_text(output)
rows=[json.loads(line) for line in output.splitlines() if line.startswith('{')]
assert rows,'original produced no records'
if mode=='parser':
 assert len(rows)==122 and rows[0]['record']=='constants'
 assert len({r['id'] for r in rows[1:]})==121
 assert all(r['exception'] is None for r in rows[1:])
(record/('original-'+mode+'.json')).write_text(json.dumps(rows,indent=2)+'\n')
sources=['framework/shannon-registration-oracle/RegistrationIndicationOracle.java','framework/tests/shannon_registration.py','tools/build-shannon-registration-oracle.sh','tools/native-ims-registration-suite.sh']
proof={'passed':True,'mode':mode,'records':len(rows),'original_apk_sha256':hashlib.sha256(apk.read_bytes()).hexdigest(),'oracle_jar_sha256':hashlib.sha256((root/'oracle/oracle.jar').read_bytes()).hexdigest(),'source_sha256':{s:hashlib.sha256(Path(s).read_bytes()).hexdigest() for s in sources},'selinux':'Enforcing','native_ims_state_published':False,'vendor_services_or_modem_called':False,'physical_phone_accessed':False}
(record/'result.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps({'mode':mode,'records':len(rows),'selinux':'Enforcing','original_executed':True,'parser_constructed':mode=='parser','services_or_modem_called':False,'physical_phone_accessed':False}),flush=True)
