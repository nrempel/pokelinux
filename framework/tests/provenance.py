#!/usr/bin/env python3
from suite import adb,OUT
import json,subprocess,pathlib,hashlib
properties=['ro.build.fingerprint','ro.build.version.sdk','ro.build.version.release','ro.build.version.security_patch','ro.product.cpu.abi','ro.build.type','ro.debuggable','ro.boot.verifiedbootstate']
result={'properties':{p:adb('shell','getprop',p).strip() for p in properties},'selinux':adb('shell','getenforce').strip(),'physical_phone_accessed':False,'carrier_registration_verified':False,'apks':{}}
for name in ['bridge','trusted','untrusted','norequest','ims']:
 path=pathlib.Path('android-framework/apks')/(name+'.apk')
 cert=subprocess.check_output(['android-framework/sdk/build-tools/android-37.0/apksigner','verify','--print-certs',str(path)],text=True)
 digest=next(line.rsplit(': ',1)[1] for line in cert.splitlines() if 'certificate SHA-256 digest:' in line)
 result['apks'][name]={'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'signer_sha256':digest}
certs=result['apks'];assert certs['bridge']['signer_sha256']==certs['trusted']['signer_sha256']==certs['norequest']['signer_sha256']
assert certs['bridge']['signer_sha256']!=certs['untrusted']['signer_sha256']
assert result['properties']['ro.build.version.sdk']=='37'
assert result['selinux']=='Enforcing'
result['packages']=adb('shell','pm','list','packages','-U','org.omarchy')
result['features']=adb('shell','pm','list','features')
(OUT/'environment.json').write_text(json.dumps(result,indent=2)+'\n')
(OUT/'bridge-package.txt').write_text(adb('shell','dumpsys','package','org.omarchy.bridge'))
(OUT/'telecom-dump.txt').write_text(adb('shell','dumpsys','telecom'))
print(json.dumps({k:v for k,v in result.items() if k not in ['packages','features']},indent=2))
