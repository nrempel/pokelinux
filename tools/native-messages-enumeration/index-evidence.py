#!/usr/bin/env python3
"""Clean complete-enumeration evidence; publishes hashes/booleans only."""
import hashlib,json,shutil,sys,tomllib
from pathlib import Path
root=Path(__file__).resolve().parents[2];base=Path('artifacts/native-messages-enumeration')
def sha(p):return hashlib.sha256((root/p).read_bytes()).hexdigest()
def read(p):return json.loads((root/p).read_text())
def ref(p):return {'path':str(p),'sha256':sha(p)}
runtime=base/'runtime/enum-green-v1/result.json';value=read(runtime)
assert value['passed'] and value['architecture']=='aarch64' and len(value['checks'])==6
assert all(r['passed'] and r['fixture']['peer_reaped'] and r['fixture']['manager_reaped'] and r['fixture']['explicit_fixture_reset_after_actual_reap'] for r in value['checks'].values())
prior=base/'selections/enum-green-v1/selection.json';selection=read(prior)
# Later additions are only documentation and indexing, not executed source edits.
for name,digest in selection['sources'].items():assert sha(Path(name))==digest,name
for name,digest in selection['binaries'].items():assert sha(base/name)==digest,name
out=base/'selections/enum-final-v1';out.mkdir(parents=True,exist_ok=False)
sources={}
for directory in ['native-messaging','native-messages','native-kernel-identity','tools/native-messages-enumeration']:
    for p in sorted((root/directory).rglob('*')):
        if p.is_file() and '__pycache__' not in p.parts and p.suffix in ['.rs','.py','.sh','.toml','.lock','.md']:
            name=p.relative_to(root);target=root/out/'sources'/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,target);sources[str(name)]=sha(name)
(root/out/'binaries').mkdir()
for name,digest in selection['binaries'].items():shutil.copyfile(root/base/name,root/out/'binaries'/Path(name).name)
final={'sources':sources,'binaries':selection['binaries'],'executed_selection':ref(prior),'runtime':ref(runtime),'later_sources':'only new reproduction README/indexing script; executed source checksums unchanged','frozen_lifecycle_actor':selection['frozen_lifecycle_source_selector']}
(root/out/'selection.json').write_text(json.dumps(final,indent=2)+'\n')
red_component=base/'runtime/component-red-v1/result.json';red_arm=base/'runtime/arm-live33-red-v1/result.json';component=read(red_component);arm=read(red_arm)
assert not component['passed'] and component['expected_red'] and component['actual_sqlite_raw_occurrences']==33
assert not arm['passed'] and arm['expected_red'] and arm['checks']['live_binder33_desired_complete_background_projection']['actual_live33rd_generated_binder_callback']
green_component=base/'runtime/component-green-v1/result.json';assert read(green_component)['passed'] and read(green_component)['bounded_turns']==5
proofs={}
for folder in ['runtime/component-red-v1','runtime/arm-live33-red-v1','runtime/component-green-v1','runtime/enum-green-v1']:
    for p in sorted((root/base/folder).rglob('*')):
        if p.is_file():proofs[str(p.relative_to(root))]=sha(p.relative_to(root))
for name in ['core-checks-v1.log','core-checks-v2.log','green-build-v2.log','component-red-build.log']:
    proofs[str(base/name)]=sha(base/name)
deps={}
for folder in ['native-messaging','native-messages','native-inbox','native-kernel-identity','tools/native-messages-enumeration/fixture']:
    for p in tomllib.loads((root/folder/'Cargo.lock').read_text())['package']:
        if 'checksum' in p:deps[p['name']+'@'+p['version']]=p['checksum']
public={'scope':'bounded complete raw receipt enumeration and incremental projection with immutable prior actor/peer; later session composition and production vendor are unqualified',
'prior_incoming_commit':'f733d3b','prior_actor_peer_selector':ref(Path(selection['frozen_lifecycle_source_selector'])),
'final_selection':ref(out/'selection.json'),'executed_selection':ref(prior),'runtime':ref(runtime),
'sources':sources,'elf_checksums':{str(base/name):digest for name,digest in selection['binaries'].items()},'proof_checksums':proofs,'registry_dependencies':dict(sorted(deps.items())),
'reds':{'actual_sqlite33':ref(red_component),'actual_arm_live33':ref(red_arm),'pre_implementation_selection':ref(base/'selections/enum-red-v1/selection.json'),'scope':'fresh desired complete33 assertions executed before implementation; historical snapshot33 refusal is not relabelled'},
'greens':{'actual_arm_kernel_binder_storage_authority':{'passed':True,'groups':6,'checks':list(value['checks']),'kernel':value['kernel'],'independent_actually_reaped_peer_contexts':4},'actual_sqlite33':ref(green_component),'receipt_cases':{'x86':22,'arm_kernel':22},'projection_cases':{'x86':6,'arm_kernel':6},'strict_clippy':True},
'contracts':{'fixed_upper_sequence':True,'owner_bound_cursor_and_checkpoint':True,'page_max':32,'projection_turn_import_max':8,'scalar_inventory_seq_max':4097,'raw_receipt_cap':4096,'inbox_key_cap':4096,'inbox_text_cap':2048,'inbox_conversation_cap':512,'schema_version':1,'raw_deletion_or_retention_supported':False,'cursor_data_is_read_or_ack_authority':False,'callback_recreation':False,'implicit_source_reattachment':False,'ack_send_enabled':False,'normal_production_entrypoint':'history-only'},
'limits':['append-only contiguous raw sequence; external raw deletion/reordering refused','metadata inventory scans up to4097; not O(1)','eight imports do not bound filesystem syscall completion','original startup/read/arrival deadlines remain unchanged; large cold backlog may time out unavailable','capacity limits remain explicit; no eviction/retention/production throughput qualification','frozen f733 actor used; later Root session composition not qualified here','prior decoder allocation/callback mutex wait/held-read close caveats remain','no phone/vendor/network/host Binder/adb/USB execution']}
(root/'docs/evidence/native-messages-enumeration-provenance.json').write_text(json.dumps(public,indent=2)+'\n')
print(json.dumps({'passed':True,'sources':len(sources),'elfs':len(selection['binaries']),'proofs':len(proofs),'dependencies':len(deps),'arm_groups':6,'receipt_cases':22,'projection_cases':6},indent=2))
