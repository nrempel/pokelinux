#!/usr/bin/env python3
"""Attempt unchanged factory APK installation/activation on the disposable API37 image.
This records compatibility barriers; failed activation is not a successful IMS test.
"""
from suite import adb,OUT
import pathlib,json,hashlib,time
base=pathlib.Path('artifacts/telephony-audit/input')
manifest=json.loads(pathlib.Path('artifacts/telephony-audit/input-manifest.json').read_text())
hashes={r['path']:r['sha256'] for r in manifest}
packages=[('ShannonIms','com.shannon.imsservice'),('PixelImsMediaService','com.android.telephony.imsmedia'),('OemRilService','com.samsung.slsi.telephony.oemril')]
results={};installed=[]
try:
 for folder,pkg in packages:
  paths=list((base/'system_ext/priv-app'/folder).glob('*.apk'));assert len(paths)==1,paths
  p=paths[0];relative='/'+str(p.relative_to(base));digest=hashlib.sha256(p.read_bytes()).hexdigest()
  assert digest==hashes[relative],relative
  existing=adb('shell','pm','path',pkg,check=False).strip()
  assert not existing,('must not replace existing package',pkg,existing)
  output=adb('install','--no-incremental',str(p),check=False)
  results[pkg]={'factory_path':relative,'sha256':digest,'install_output':output,'usable_ims_verified':False}
  if 'Success' in output:installed.append(pkg)
 if 'com.shannon.imsservice' in installed:
  adb('logcat','-c')
  result=results['com.shannon.imsservice']
  result['select_override']=adb('shell','cmd','phone','ims','set-ims-service','-s','0','-c','-f','1','com.shannon.imsservice',check=False)
  time.sleep(8)
  result['selected_package']=adb('shell','cmd','phone','ims','get-ims-service','-s','0','-c','-f','1',check=False)
  (OUT/'shannon-services.txt').write_text(adb('shell','dumpsys','activity','services','com.shannon.imsservice',check=False))
  (OUT/'shannon-logcat.txt').write_text(adb('logcat','-d','-v','threadtime',check=False))
  (OUT/'shannon-package.txt').write_text(adb('shell','dumpsys','package','com.shannon.imsservice',check=False))
  services=(OUT/'shannon-services.txt').read_text()
  log=(OUT/'shannon-logcat.txt').read_text()
  result['framework_binding_observed']='com.android.phone/1001' in services and 'hasBound=true' in services
  result['feature_creation_called']='createMmTelFeatureForSubscription' in log
  result['hidden_api_rejection']='getSimStateForSlotIndex' in log and 'using linking: denied' in log
  result['invalid_feature_container']='createFeatureContainer: invalid state' in log
  result['activation_verdict']='Framework binding and feature creation are partial activation only. Data-app hidden-API rejection and invalid feature container prevent a usable IMS claim.'
finally:
 adb('shell','cmd','phone','ims','clear-ims-service-override','-s','0',check=False)
 for pkg in installed:adb('uninstall',pkg,check=False)
 (OUT/'stock-activation.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
