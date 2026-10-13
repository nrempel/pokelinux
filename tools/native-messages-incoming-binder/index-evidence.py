#!/usr/bin/env python3
"""Publish clean exact-source/checksum metadata, never retained runtime bytes/logs."""
import hashlib,json,sys,tomllib
from pathlib import Path
root=Path(__file__).resolve().parents[2]
base=Path('artifacts/native-messages-incoming-binder')
label=sys.argv[1]
assert label.replace('-','').replace('_','').isalnum()
def sha(path):return hashlib.sha256((root/path).read_bytes()).hexdigest()
def read(path):return json.loads((root/path).read_text())
def record(path):return {'path':str(path),'sha256':sha(path)}
selection_path=base/'selections'/label/'selection.json';selection=read(selection_path)
runtime_path=base/'runtime'/label/'result.json';runtime=read(runtime_path)
assert runtime['passed'] and runtime['architecture']=='aarch64' and runtime['qemu_mode']=='system'
assert runtime['storage_qualified'] and runtime['installed_authority_qualified']
assert len(runtime['checks'])==18 and all(row['passed'] for row in runtime['checks'].values())
assert runtime['kernel_identity_boundary']['passed'] and runtime['kernel_identity_boundary']['cases']==3
for path,digest in selection['sources'].items():assert sha(Path(path))==digest,path
for path,digest in selection['binaries'].items():assert sha(base/path)==digest,path
for row in runtime['checks'].values():
    fixture=row['fixture'];assert fixture['peer_reaped'] and fixture['manager_reaped'] and fixture['explicit_fixture_reset_after_actual_reap']
    assert fixture['genuine_android_library_mapping'] and not fixture['ndk_stub_runtime']
assert runtime['checks']['closed_window_raw_and_current_authority']['projected_before_any_read']==2
assert runtime['checks']['held_read_failure_and_history_only_child_whole_restart']['stale_callback_cannot_admit']
stop=runtime['checks']['actual-systemd-pending-completion']
assert stop['main_exit']['EXIT_CODE']=='exited' and stop['main_exit']['EXIT_STATUS']=='0'
assert stop['healthy_policy_seal_exact_epoch_and_bytes'] and stop['policy_validation_and_history_restart_ready']
red=[]
for name,signature in [('raw-only-red-v1','raw_committed_background_projection_missing'),
    ('stop-completion-red-v1','stop_ack_returned_before_actual_service_exit'),
    ('systemd-stop-red-v3','systemd_signalled_before_actual_service_completion')]:
    path=base/'runtime'/name/'result.json';value=read(path)
    assert not value['passed'] and value.get('expected_red') and value.get('red_signature')==signature
    if name=='raw-only-red-v1':
        assert value['checks']['closed_window_raw_and_current_authority']['actual_current_authority_read']
        assert value['checks']['closed_window_raw_and_current_authority']['projected_before_any_read']==0
    if name=='systemd-stop-red-v3':
        outcome=value['checks']['actual-systemd-pending-completion']['main_exit'];assert outcome['EXIT_CODE']=='killed' and outcome['EXIT_STATUS']=='TERM'
    row={'runtime':record(path),'signature':signature,'selection':record(base/'selections'/name/'selection.json')}
    if name!='raw-only-red-v1':
        before=read(base/'selections'/name/'selection.json')
        row['qualified_pre_fix_scope']='exact old messages-stop source and ELF; other incoming source is not newly qualified by this focused stop proof'
        row['helper_source_sha256']=before['sources']['native-messages/src/bin/messages-stop.rs']
        row['helper_elf_sha256']=before['binaries']['target/aarch64-unknown-linux-gnu/debug/messages-stop']
    red.append(row)
service_path=base/'x86-service-final-v1/result.json';service=read(service_path);assert service['passed'] and len(service['groups'])==10
regression_path=Path('artifacts/native-messages/lifecycle-regression-incoming-v1/result.json');regression=read(regression_path);assert regression['passed'] and regression['cases']==27
packages={}
for name in ['tools/native-messages-incoming-binder/fixture','native-messages','native-lifecycle','native-inbox','native-messaging','native-kernel-identity']:
    lock=Path(name)/'Cargo.lock'
    for package in tomllib.loads((root/lock).read_text())['package']:
        if 'checksum' in package:
            key=package['name']+'@'+package['version'];assert packages.get(key,package['checksum'])==package['checksum'];packages[key]=package['checksum']
inputs=read(base/'verified-inputs.json');api=read(Path('native-kernel-identity/public-api-inputs.json'))
for row in api['files']:assert sha(Path(row['private_cache']))==row['sha256']
prior_path=Path('docs/evidence/native-messages-correction-provenance.json')
proofs={}
for folder in [base/'runtime'/label,base/'runtime/raw-only-red-v1',base/'runtime/stop-completion-red-v1',base/'runtime/systemd-stop-red-v3',base/'x86-service-final-v1',Path('artifacts/native-messages/lifecycle-regression-incoming-v1')]:
    for path in sorted((root/folder).rglob('*')):
        if path.is_file():proofs[str(path.relative_to(root))]=sha(path.relative_to(root))
for path in ['build-final-v2.log','normal-core-checks-final-v2.log','kernel-boundary-build-v1.log']:
    proofs[str(base/path)]=sha(base/path)
public={
    'scope':'authored receive-only frozen messaging V3 synthetic composition through genuine ARM kernel Binder, sole supervised raw/inbox child and current-authority read; normal installed Messages remains history-only',
    'prior_installed_freeze':{'commit':'ac57dd6e038c36e9278e07ba1b90b0084a4a5c24','provenance':record(prior_path),'scope':'historical installed desktop/UI/recovery correction; its exact sources, ELFs and proofs remain preserved'},
    'current_selection':record(selection_path),'current_runtime':record(runtime_path),
    'source_checksums':selection['sources'],'elf_checksums':{str(base/path):value for path,value in selection['binaries'].items()},
    'generated_checksums':selection['generated'],'registry_dependency_checksums':dict(sorted(packages.items())),
    'proof_checksums':proofs,
    'qualified_results':{'actual_arm_kernel_binder_storage_authority':{'passed':True,'groups':18,'checks':list(runtime['checks']),'architecture':runtime['architecture'],'kernel':runtime['kernel'],'actual_kernel':True,'qemu_mode':'system'},
        'kernel_identity':{'passed':True,'actual_unix_kernel_cases':2,'injected_reply_shape_with_actual_owned_descriptor_cleanup':1,'arm_kernel_and_x86':True},
        'x86_component':{'passed':True,'cases':4,'scope':'existing projection component cases; distinct from Binder composition'},
        'x86_installed_messages':{'passed':True,'groups':10,'result':record(service_path)},
        'lifecycle_regression':{'passed':True,'cases':27,'result':record(regression_path),'scope':regression['scope']}},
    'executed_reds':red,
    'genuine_runtime':{'frozen_public_schema_sha256':inputs['frozen_manifest_sha256'],'aidl_compiler_sha256':inputs['aidl_compiler_sha256'],'library_metadata':inputs['runtime'],'runtime_substitution':False,'runtime_bytes_published':False},
    'public_kernel_api':record(Path('native-kernel-identity/public-api-inputs.json')),
    'bounds':{'callback_retained_bytes':512,'queue':8,'active_occurrences':1,'original_ingress_deadline_ns':1000000000,'source_attempt_markers':8,'raw_snapshot':32,'stop_original_deadline_ns':3000000000},
    'fences':{'normal_service_backend_selector':False,'ack_or_send_operation':False,'read_authority_changed':False,'durable_one_attempt_before_binder':True,'history_restart_reattaches_backend':False,'fresh_successor_requires_independent_peer_and_explicit_context_reset':True,'owned_children_peer_manager_actually_reaped':True,'local_pidfd_exit_is_healthy_seal_or_vendor_cleanup':False},
    'limits':['synthetic producer; no vendor or carrier receive','generated Binder decoder allocation precedes callback retained-payload bound','held-read app close explicitly fails source; history-only recovery','raw33 commits then terminal refusal before partial latest32 import','complete enumeration and retention unqualified','no production notifications, wake or trusted session bridge','no uninterruptible kernel-task cleanup qualification','no physical phone, vendor execution, host Binder/adb/USB or VM NIC'],
    'retained_unselected_diagnostics':[
        {'path':str(base/'runtime/scaffold-v1-smoke'),'reason':'lookup failed before registration'},
        {'path':str(base/'runtime/incoming-green-v1'),'reason':'test observation used SQLite during an active zero-busy-timeout write'},
        {'path':str(base/'runtime/incoming-green-v2'),'reason':'15 groups passed; isolated systemd fixture omitted RuntimeDirectory lifecycle and used an ephemeral profile'},
        {'path':str(base/'runtime/systemd-stop-red-v1'),'reason':'post-stop reset status cannot witness Main signal'},
        {'path':str(base/'runtime/systemd-stop-red-v2'),'reason':'observer result destination lacked dropped-capability write access'},
        {'path':str(base/'x86-service-v1'),'reason':'selected builder image had no Python interpreter'}],
    'current_source_rerun':'Final selection follows explicit callback sequence poison failure latch; previous incoming-green-final-v1 remains preserved with its passed18+3 results'
}
(root/'docs/evidence/native-messages-incoming-provenance.json').write_text(json.dumps(public,indent=2)+'\n')
print(json.dumps({'passed':True,'sources':len(selection['sources']),'binaries':len(selection['binaries']),'generated':len(selection['generated']),'dependencies':len(packages),'proofs':len(proofs),'integration_groups':18,'kernel_cases':3},indent=2))
