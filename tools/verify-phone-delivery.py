#!/usr/bin/env python3
"""Validate saved end-to-end evidence against the exact delivered bridge sources."""
import hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def load(name):return json.loads((ROOT/name).read_text())
def digest(name):return hashlib.sha256((ROOT/name).read_bytes()).hexdigest()
payload=json.loads(subprocess.check_output([sys.executable,str(ROOT/'tools/verify-release-payload.py'),str(ROOT/'artifacts/android-controls/bridge-unsigned.apk'),str(ROOT/'artifacts/android-controls/bridge-test-signed.apk')],text=True))
assert payload==load('artifacts/android-controls/release-payload.json')
native=load('artifacts/android-controls/native-enrollment.json')
assert native['passed'] and native['phone_accessed'] is False
assert native['runtime']['architecture']=='aarch64'
assert native['runtime']['client_sha256']==digest('transport/omarchy_bridge.py')
expected={'disabled','answer','duplicate','conflicting_replay','stale_revision','hang_up','decline','revocation','new_authorization_generation'}
assert set(native['controls'])==expected
for key in ('answer','duplicate','hang_up','decline'):assert native['controls'][key]['value']['outcome']=='submitted'
for key in expected-{'answer','duplicate','hang_up','decline'}:assert native['controls'][key]['value']['outcome']=='rejected'
assert native['wrong_pin']['outcome']==native['stopped']['outcome']=='UNAVAILABLE'
gui=load('artifacts/phone-ui/gui-bridge.json');guest=load('artifacts/phone-ui/result.json')
assert gui['passed'] and guest['passed'] and not gui['phone_accessed'] and not guest['hardware_verified']
assert guest['architecture']=='aarch64' and guest['package_sha256']==digest('artifacts/client/omarchy-phone-0.1.0-1-any.pkg.tar')
for name,value in guest['installed_files'].items():assert value==digest('transport/'+name),name
assert gui['checks']['launcher']['app_id']=='org.omarchy.Phone'
assert gui['checks']['launcher']['windows']==1 and gui['checks']['launcher']['activations']>=2
assert '#33cc99' in gui['checks']['theme']['theme']
assert gui['checks']['incoming']['snapshot']['calls'][0]['state']==2
assert gui['checks']['answered']['snapshot']['calls'][0]['state']==4
assert gui['checks']['answered']['outcome'] is None
assert not gui['checks']['ended']['snapshot']['calls'] and not gui['checks']['declined']['snapshot']['calls']
assert gui['checks']['unavailable']['snapshot'] is None
direct=load('artifacts/android-controls/direct-transport.json');life=load('artifacts/android-controls/lifecycle.json')
assert direct['passed'] and not direct['adb_data_forwarding'] and direct['selinux']=='Enforcing'
assert direct['checks']['direct_mtls']['libc'][0]=='glibc' and direct['checks']['direct_mtls']['uid']==2000
assert direct['checks']['stop_unavailable'] and not direct['phone_accessed']
assert life['passed'] and not life['phone_accessed'] and all(life['checks'].values())
for mode in ('gui','direct'):
 proof=load('artifacts/android-controls/'+mode+'-provenance.json')
 assert proof['apk_sha256']==digest('artifacts/android-controls/bridge-test-signed.apk')
 assert proof['selinux']=='Enforcing' and not proof['phone_accessed']
 expected_sources={str(path.relative_to(ROOT)) for path in [*(ROOT/'framework/bridge').glob('*'),*(ROOT/'transport').glob('*.py')] if path.is_file()}
 ignored={'transport/phone.py','transport/phone_model.py','transport/phone_theme.py'} if mode=='direct' else set()
 assert set(proof['sources'])-ignored==expected_sources-ignored,(mode,'source inventory changed; rerun integration')
 for name,value in proof['sources'].items():
  # Direct transport executes the CLI, not the GTK application. GUI provenance
  # independently verifies the installed presentation files and package.
  if mode=='direct' and name in ('transport/phone.py','transport/phone_model.py','transport/phone_theme.py'):continue
  assert digest(name)==value,(mode,name)
report={'offline_bridge':'PASS','unsigned_release_payload_matches_tested_apk':True,'native_arm64_call_control':True,'installed_gtk_hyprland_touch':True,'same_kernel_arch_glibc_direct_loopback':True,'production_enrollment_and_boot':True,'pixel_hardware_verified':False,'carrier_ims_verified':False,'voice_audio_verified':False,'bootable_pixel_os':False,'apk_sha256':digest('artifacts/android-controls/bridge-test-signed.apk'),'client_package_sha256':digest('artifacts/client/omarchy-phone-0.1.0-1-any.pkg.tar')}
(ROOT/'artifacts/android-controls/delivery-verification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
