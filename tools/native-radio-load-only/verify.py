#!/usr/bin/env python3
"""Read-only verification and safe authored summary of public-only evidence."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'artifacts/native-radio-load-only'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
classes=[
 ('public-containment-red-1','freeze','GENUINE_MISSING_CONTAINMENT_RED',1),
 ('public-containment-green-1','freeze-green-2','BOOTSTRAP_FAILURE',1),
 ('public-containment-green-2','freeze-green-3','HARNESS_CLOSURE_FAILURE',1),
 ('public-containment-green-3','freeze-green-4','ACTUAL_PUBLIC_CONTAINMENT_GREEN',0),
]
bindings={};epochs=[]
for label,freeze,classification,expected_exit in classes:
    selected=BASE/freeze;out=BASE/'runtime'/label;selection=json.loads((selected/'selection.json').read_text());result=json.loads((out/'result.json').read_text())
    for name,want in selection['files'].items():
        path=selected/name;assert digest(path)==want,f'frozen binding changed: {path}';bindings[str(path.relative_to(ROOT))]=want
    before=json.loads((out/'input-before.json').read_text());after={Path(line.split(None,1)[1]).name:line.split(None,1)[0] for line in (out/'input-after.sha256').read_text().splitlines()}
    assert before==after=={name:row['sha256'] for name,row in selection['inputs'].items()}
    assert (out/'canonical-lock-before.txt').read_bytes()==(out/'canonical-lock-after.txt').read_bytes()
    assert 'FLOCK  ADVISORY  READ' in (out/'canonical-lock-after.txt').read_text() and 'ino:\t28974090' in (out/'canonical-lock-after.txt').read_text()
    assert (out/'selection-before.sha256').read_bytes()==(out/'selection-after.sha256').read_bytes()
    assert (out/'selection-before.sha256').read_text().split()[0]==digest(selected/'selection.json')
    waited=json.loads((out/'actual-qemu-wait.json').read_text());assert waited['actual_wait'] and waited['exit']==0
    assert int((out/'docker-exit-code.txt').read_text())==expected_exit
    argv=(out/'docker-command.argv0').read_bytes().decode().split('\0')[:-1]
    assert argv[argv.index('--network')+1]=='none' and '--read-only' in argv
    assert selection['runtime_image'] in argv and selection['original_runtime_admitted'] is False and selection['registration_admitted'] is False
    assert result['architecture']=='aarch64' and result['qemu_mode']=='system' and result['vendor_execution'] is False and result['physical_phone_accessed'] is False
    if classification=='GENUINE_MISSING_CONTAINMENT_RED':assert result['expected_red'] and result['cleanup_passed'] and result['actual_reaped'] and result['durable_uncertainty_retained']
    if classification=='BOOTSTRAP_FAILURE':assert not result['expected_red'] and 'bootstrap_or_cleanup_error' in result
    if classification=='HARNESS_CLOSURE_FAILURE':assert result['passed'] and result['cleanup_passed'] and expected_exit==1
    if classification=='ACTUAL_PUBLIC_CONTAINMENT_GREEN':assert result['passed'] and result['cleanup_passed'] and 'bootstrap_or_cleanup_error' not in result
    for path in sorted(out.iterdir()):
        if path.is_file():bindings[str(path.relative_to(ROOT))]=digest(path)
    bindings[str((selected/'selection.json').relative_to(ROOT))]=digest(selected/'selection.json')
    epochs.append({'label':label,'classification':classification,'selection_sha256':digest(selected/'selection.json'),'result_sha256':digest(out/'result.json'),'result_mtime_ns':(out/'result.json').stat().st_mtime_ns,'docker_actual_wait_exit':expected_exit,'qemu_actual_wait_exit':0,'input_before_after_equal':True,'canonical_readonly_lock_before_after_equal':True,'vendor_execution':False,'registration_admission':False})
assert [r['result_mtime_ns'] for r in epochs]==sorted(r['result_mtime_ns'] for r in epochs)
green=json.loads((BASE/'runtime/public-containment-green-3/result.json').read_text());assert set(green['checks'])=={'held','probes','second-exec','task-limit','queued-late','parent-death'}
checks={}
for name,row in green['checks'].items():
    assert row['passed'];brief={'passed':True,'recorded_status_samples':len(row['control_ns']),'maximum_recorded_status_ns':max(row['control_ns'])}
    if 'report' in row:
        report=row['report'];assert report['actual_root_wait'] and report['all_traced_tasks_waited'] and report['cgroup_pids_current']=='0'
        assert report['status']['occupied'] is False and report['registration_admission'] is False and report['vendor_cleanup'] is False
        brief.update({'actual_root_wait':True,'all_traced_tasks_waited':True,'cgroup_pids_current':0,'stop_ns':row['stop_ns'],'terminal_reason':report['status']['fenced'],'attempted_exec_syscall_entries':report['status']['execs']})
        if name=='probes':brief['public_checkpoints']=[{k:v for k,v in packet.items() if k!='nonce'} for packet in report['status']['checkpoints']]
        if name=='queued-late':
            packets=[e for e in report['events'] if e['event']=='authenticated-checkpoint'];assert packets and all(not p['admitted'] and p['boottime_ns']>=report['status']['until_boottime_ns'] for p in packets)
            brief['late_checkpoint_rejected_after_original_until']=True
    else:
        assert row['orphan_actual_wait']['actual_direct_wait'] and row['orphan_actual_wait']['status']==9
        brief.update({'parent_killed_through_retained_pidfd':True,'final_credentials_before_parent_death':True,'orphan_actual_direct_wait':True,'orphan_terminal_signal':9})
    checks[name]=brief
before=json.loads((BASE/'freeze-green-3/selection.json').read_text());after=json.loads((BASE/'freeze-green-4/selection.json').read_text());changed=[name for name,h in before['files'].items() if after['files'].get(name)!=h];assert changed==['source/tools/native-radio-load-only/vm.py']
summary={'status':'PUBLIC_HARMLESS_CHILD_CONTAINMENT_PRECURSOR_QUALIFIED','production_commit':'2134c62a06fe46a2a4ecbe4526653a9824822924','scope':'Authored static public Rust controls under actual Arch ARM64 kernel7.1.6 in system-QEMU. Isolated experimental owner; no production radio implementation changed.','chronology':epochs,'checks':checks,'maximum_recorded_control_ns':green['maximum_observed_control_ns'],'observed_control_budget_ns':250000000,'absolute_clock':'CLOCK_BOOTTIME','single_startup_deadline_ns':5000000000,'single_first_terminal_cleanup_deadline_ns':2000000000,'final_wrapper_delta':changed,'vendor_execution':False,'original_linker_bionic_vendor_load':False,'physical_phone_accessed':False,'registration_admission':False,'vendor_semantic_cleanup':False,'original_runtime_admission':'PROHIBITED_PENDING_SEPARATE_REVIEW_AND_NAMESPACE_BINDING','binding_count':len(bindings),'bindings':bindings}
encoded=json.dumps(summary,indent=2)+'\n'
if len(sys.argv)>1:
    with Path(sys.argv[1]).open('x') as stream:stream.write(encoded)
print(json.dumps({'status':summary['status'],'binding_count':len(bindings),'epochs':len(epochs),'checks':len(checks),'maximum_recorded_control_ns':summary['maximum_recorded_control_ns']},indent=2))
