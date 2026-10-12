#!/usr/bin/env python3
"""Negative controls against saved evidence: incomplete or false reports must fail."""
import copy,json,shutil,subprocess,sys,tempfile
from pathlib import Path
project=Path(__file__).resolve().parents[1]
source=project/'artifacts/android-transport'
original=json.loads((source/'results/transport.json').read_text())
results={}
with tempfile.TemporaryDirectory(prefix='bridge-evidence-') as temp:
 root=Path(temp);evidence=root/'android-framework/results';evidence.mkdir(parents=True)
 (root/'android-framework/apks').mkdir();(root/'transport').mkdir()
 shutil.copy2(source/'bridge.apk',root/'android-framework/apks/bridge.apk')
 shutil.copy2(source/'results/transport-container.json',evidence/'transport-container.json')
 shutil.copy2(project/'transport/omarchy_bridge.py',root/'transport/omarchy_bridge.py')
 for case in ['valid','missing_check','unauthorized_accepted','false_hardware','wrong_source_hash','false_idle']:
  value=copy.deepcopy(original)
  if case=='missing_check':del value['checks']['ringing']
  if case=='unauthorized_accepted':value['checks']['wrong_peer']['outcome']='OK'
  if case=='false_hardware':value['hardware_verified']=True
  if case=='wrong_source_hash':value['checks']['native_arch_arm64']['client_sha256']='0'*64
  if case=='false_idle':value['checks']['coarse_ringing_without_dialer']['value']['phone_state']['state']=0
  (evidence/'transport.json').write_text(json.dumps(value))
  completed=subprocess.run([sys.executable,str(project/'tools/verify-transport.py')],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  expected=completed.returncode==0 if case=='valid' else completed.returncode!=0
  assert expected,(case,completed.stdout)
  results[case]={'passed':True,'verifier_exit_code':completed.returncode}
(source/'evidence-controls.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
