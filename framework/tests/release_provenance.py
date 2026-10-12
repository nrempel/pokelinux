#!/usr/bin/env python3
"""Match the installed emulator APK and release client sources to a named test run."""
import hashlib,json,subprocess,sys,tempfile
from pathlib import Path
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554'];mode=sys.argv[1]
assert mode in ('enrollment','gui','plugin','direct')
def adb(*args):return subprocess.check_output(ADB+list(args),text=True,timeout=60).strip()
def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
remote=adb('shell','pm','path','org.omarchy.bridge').removeprefix('package:');assert remote.startswith('/data/app/') and '\n' not in remote
with tempfile.TemporaryDirectory() as work:
 installed=Path(work)/'installed.apk';adb('pull',remote,str(installed));assert digest(installed)==digest('android-framework/apks/bridge.apk')
 subprocess.run(['python3','tools/verify-release-apk.py',str(installed)],check=True)
result={'mode':mode,'apk_sha256':digest('android-framework/apks/bridge.apk'),'signing':'disposable emulator test signer; not a distribution release','selinux':adb('shell','getenforce'),'android_fingerprint':adb('shell','getprop','ro.build.fingerprint'),'phone_accessed':False,'sources':{}}
for path in sorted([*Path('framework/bridge').glob('*'),*Path('transport').glob('*.py')]):
 if path.is_file():result['sources'][str(path)]=digest(path)
version='0.6.0' if mode=='plugin' else '0.1.0'
result['client_package_sha256']=digest(f'artifacts/client/omarchy-phone-{version}-1-any.pkg.tar')
Path('android-framework/results/'+mode+'-provenance.json').write_text(json.dumps(result,indent=2)+'\n')
print('Installed release-code APK matched for '+mode,flush=True)
