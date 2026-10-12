#!/usr/bin/env python3
"""Run original serializer bytecode in the owned emulator, without starting IMS."""
import hashlib
from pathlib import Path
import subprocess
import time
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):
 try:
  return subprocess.check_output(ADB+list(args),timeout=45,text=True,stderr=subprocess.STDOUT)
 except subprocess.CalledProcessError as error:
  print(error.output,flush=True)
  raise
adb('wait-for-device')
end=time.monotonic()+180
while adb('shell','getprop','sys.boot_completed').strip()!='1':
 assert time.monotonic()<end,'boot timeout'
 time.sleep(2)
assert adb('shell','getenforce').strip()=='Enforcing'
root='/data/local/tmp/omarchy-shannon-oracle'
adb('shell','mkdir','-p',root)
apk=Path('artifacts/telephony-audit/input/system_ext/priv-app/ShannonIms/ShannonIms.apk')
assert hashlib.sha256(apk.read_bytes()).hexdigest()=='8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879'
for source,name in [(apk,'shannon.apk'),(Path('artifacts/ims-native-audit/oracle/oracle.jar'),'oracle.jar')]:
 adb('push',str(source),root+'/'+name)
 adb('shell','chmod','444',root+'/'+name)
output=adb('shell','env','CLASSPATH='+root+'/oracle.jar:'+root+'/shannon.apk','app_process','/system/bin','org.omarchy.audit.CodecOracle')
Path('artifacts/ims-native-audit/original-codec-output.txt').write_text(output)
rows=[line.split('\t') for line in output.splitlines() if line.startswith(('writeByte\t','writeShort\t','writeInt\t'))]
assert len(rows)==45,(len(rows),output)
print('Original Shannon DataWriter executed: 45 reference cases',flush=True)

output=adb('shell','env','CLASSPATH='+root+'/oracle.jar:'+root+'/shannon.apk','app_process','/system/bin','org.omarchy.audit.MessageOracle')
Path('artifacts/ims-native-audit/original-message-output.txt').write_text(output)
rows=[line.split('\t') for line in output.splitlines() if line.startswith(('RilReqStackStartReq\t','RilReqSetImsRegistrationState\t'))]
assert len(rows)==162,(len(rows),output)
print('Original Shannon message encoders executed: 162 reference cases',flush=True)

output=adb('shell','env','CLASSPATH='+root+'/oracle.jar:'+root+'/shannon.apk','app_process','/system/bin','org.omarchy.audit.BinderOracle')
Path('artifacts/ims-native-audit/original-binder-output.txt').write_text(output)
rows=[line for line in output.splitlines() if line.startswith('binderRaw\t')]
assert rows==['binderRaw\t'+str(i)+'\tPASS' for i in range(5)],output
assert 'binderRejected\tPASS' in output,output
rows=[line for line in output.splitlines() if line.startswith('binderSegment\t')]
assert rows==['binderSegment\t'+str(i)+'\tPASS' for i in range(5)],output
assert 'binderSegmentRejected\tPASS' in output,output
print('Original Shannon Binder proxies executed: 10 payload cases and 2 rejected transactions',flush=True)

output=adb('shell','env','CLASSPATH='+root+'/oracle.jar:'+root+'/shannon.apk','app_process','/system/bin','org.omarchy.audit.SegmentationOracle')
Path('artifacts/ims-native-audit/original-segmentation-output.txt').write_text(output)
rows=[line for line in output.splitlines() if line.startswith('segments\t')]
assert rows==[
 'segments\t2049\t2048,1\t2049',
 'segments\t4095\t2048,2047\t4095',
 'segments\t4096\t2048,0\t2048',
 'segments\t4097\t2048,2048,1\t4097',
 'segments\t6144\t2048,2048,0\t4096',
],output
print('Original Shannon segmentation executed: 5 boundaries; exact multiples lose final chunk',flush=True)

output=adb('shell','env','CLASSPATH='+root+'/oracle.jar:'+root+'/shannon.apk','app_process','/system/bin','org.omarchy.audit.ResponseInfoOracle')
Path('artifacts/ims-native-audit/original-response-info-output.txt').write_text(output)
rows=[line.split('\t') for line in output.splitlines() if line.startswith('responseInfo\t')]
assert len(rows)==7 and all(len(row)==7 and len(row[-1])==56 for row in rows),output
print('Original vendor response Parcelable executed: 7 boundary fixtures',flush=True)

output=adb('shell','env','CLASSPATH='+root+'/oracle.jar:'+root+'/shannon.apk','app_process','/system/bin','org.omarchy.audit.ResponseDispatchOracle')
Path('artifacts/ims-native-audit/original-response-dispatch-output.txt').write_text(output)
rows=[line for line in output.splitlines() if line.startswith('responseDispatch\t')]
assert rows==['responseDispatch\t'+str(i)+'\tPASS' for i in range(4)],output
print('Original response receiver executed: match, unknown serial, ignored segment, trailing-data rejection',flush=True)
