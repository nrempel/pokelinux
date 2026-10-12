#!/usr/bin/env python3
"""Fail closed when framework evidence is incomplete; do not infer hardware success."""
from pathlib import Path
import json
root=Path('android-framework/results')
def load(name):return json.loads((root/name).read_text())
permissions=load('permissions.json');telecom=load('telecom.json');ims=load('ims-fixture.json');negative=load('ims-negative.json');env=load('environment.json')
expected={
'authorized_client':'OK','different_signature':'SecurityException','same_signer_without_request':'SecurityException',
'forwarded_binder':'RELAY','unknown_version':'IllegalArgumentException','wrong_interface':'SecurityException','unknown_operation':'IllegalArgumentException',
'incall_direct_bind':'SecurityException','phone_permission_denied':'SecurityException','framework_observer_denied':'SecurityException',
'phone_permission_granted':'OK','framework_observer_granted':'OK','phone_permission_revoked':'SecurityException','framework_observer_revoked':'SecurityException',
'rebind_after_force_stop':'OK','disabled_package':'IllegalStateException','reenabled_package':'OK'}
assert set(permissions)==set(expected),permissions.keys()
assert all(permissions[k]['outcome']==v for k,v in expected.items())
assert all(permissions[k]['outcome']=='SecurityException' for k in ['different_signature','same_signer_without_request','phone_permission_denied','framework_observer_denied','phone_permission_revoked','framework_observer_revoked','incall_direct_bind'])
assert permissions['forwarded_binder']['value']=='SecurityException'
assert len(telecom)==6 and any(c['state']==2 for c in telecom['ringing_callback']['calls'])
assert any(c['state']==4 for c in telecom['active_callback']['calls'])
assert not telecom['removed_callback']['calls'] and not telecom['no_callback_without_role']['calls']
assert ims['passed'] and negative['rejected_without_permission_bypass']
assert env['properties']['ro.build.version.sdk']=='37' and env['selinux']=='Enforcing'
assert env['physical_phone_accessed'] is False and env['carrier_registration_verified'] is False
network=load('network.json')[0];container=load('container.json')[0]
assert network['Internal'] and not container['HostConfig']['PortBindings']
assert not container['HostConfig']['Privileged'] and container['HostConfig']['CapDrop']==['ALL']
assert [d['PathOnHost'] for d in container['HostConfig']['Devices']]==['/dev/kvm']
stock=load('stock-activation.json')
assert 'INSTALL_FAILED_SHARED_USER_INCOMPATIBLE' in stock['com.android.telephony.imsmedia']['install_output']
assert 'getSimStateForSlotIndex' in (root/'shannon-logcat.txt').read_text()
result={'status':'PASS','permission_lifecycle_checks':len(permissions),'telecom_checks':len(telecom),'ims_reference_activation':True,'ims_wrong_permission_rejected':True,'stock_shannon_bound_but_unusable':True,'stock_pixel_ims_media_shared_uid_rejected':True,'physical_phone_accessed':False,'carrier_registration_verified':False,'voice_media_verified':False,'powervr_verified':False,'native_arch_android_emulator_transport_verified':load('transport-verification.json')['emulator_transport_verified'],'phone_transport_deployment_verified':False}
(root/'verification.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
