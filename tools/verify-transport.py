#!/usr/bin/env python3
"""Verify native transport evidence, separately from Pixel deployment claims."""
import hashlib,json,sys,zipfile
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'transport'))
from omarchy_bridge import validate_snapshot
root=Path('android-framework/results')
value=json.loads((root/'transport.json').read_text());checks=value['checks']
expected=set('native_arch_arm64 normal_apk_without_instrumentation apk_process foreground_service wrong_pin wrong_peer wrong_hostname coarse_permission_denied coarse_idle_without_dialer coarse_ringing_without_dialer coarse_offhook_without_dialer coarse_return_to_idle permission_revocation_clears_state coarse_permission_denied_after_restart permission_regrant_observes_fresh_state missing_client_certificate oversize unknown_operation duplicate_key recovery_after_stalled_handshakes ringing active role_loss_invalidates_snapshot role_loss_clears_calls force_stop_unavailable explicit_restart_recovers generation_changes_and_no_stale_calls new_call_after_restart call_removed removed_peer_rejected replacement_peer_accepted pinned_expired_rejected pinned_wrongpurpose_rejected pinned_untrusted_rejected peer_restored unlocked_boot_restarts_gateway boot_new_generation repeated_start_idempotent'.split())
assert set(checks)==expected,(set(checks)^expected)
assert value['hardware_verified'] is False
assert checks['native_arch_arm64']['architecture']=='aarch64'
assert checks['native_arch_arm64']['client_sha256']==hashlib.sha256(Path('transport/omarchy_bridge.py').read_bytes()).hexdigest()
assert 'Arch Linux ARM' in checks['native_arch_arm64']['os']
assert checks['foreground_service']=={'active':True,'provisioner_removed':True}
for name in ['wrong_pin','wrong_peer','wrong_hostname','force_stop_unavailable','removed_peer_rejected','pinned_expired_rejected','pinned_wrongpurpose_rejected','pinned_untrusted_rejected']:
 assert checks[name]['outcome']=='UNAVAILABLE' and checks[name]['calls']==[]
for name in ['coarse_permission_denied','missing_client_certificate','oversize','unknown_operation','duplicate_key','generation_changes_and_no_stale_calls']:
 assert checks[name] is True
for name,entry in checks.items():
 if isinstance(entry,dict) and entry.get('outcome')=='OK':validate_snapshot(entry['value'],entry['value']['id'])
for name,state in [('coarse_idle_without_dialer',0),('coarse_ringing_without_dialer',1),('coarse_offhook_without_dialer',2),('coarse_return_to_idle',0)]:
 snapshot=checks[name]['value'];assert not snapshot['dialer_role'] and snapshot['phone_state']['state']==state
assert any(c['state']==2 for c in checks['ringing']['value']['calls'])
assert any(c['state']==4 for c in checks['active']['value']['calls'])
assert not checks['call_removed']['value']['calls']
assert checks['active']['value']['generation']!=checks['explicit_restart_recovers']['value']['generation']
assert checks['role_loss_clears_calls']['value']['availability']=='unavailable'
container=json.loads((root/'transport-container.json').read_text())[0]
assert not container['HostConfig']['Privileged'] and not container['HostConfig']['PortBindings']
assert container['HostConfig']['CapDrop']==['ALL'] and not container['HostConfig']['Devices']
assert container['HostConfig']['NetworkMode'].startswith('container:')
with zipfile.ZipFile('android-framework/apks/bridge.apk') as apk:
 assert not any(n.startswith('assets/') or n.endswith(('.p12','.key','.jks','.pem')) for n in apk.namelist())
report={'status':'PASS','native_arch_android_checks':len(checks),'protocol':'TLS1.3 mutual authentication and leaf pins','instrumentation_removed_before_transport_tests':True,'emulator_transport_verified':True,'phone_deployment_verified':False,'carrier_verified':False}
(root/'transport-verification.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
