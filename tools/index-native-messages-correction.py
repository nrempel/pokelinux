#!/usr/bin/env python3
"""Freeze current correction separately; verify the complete historical checkpoint."""
import argparse, datetime, hashlib, json, shutil
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--desktop', required=True)
parser.add_argument('--process-suffix', required=True)
a = parser.parse_args()
root = Path('artifacts/native-messages/recovery-order-correction')
qualified = root/'qualified-release'
assert not qualified.exists(), 'preserve selected correction'

def digest(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for data in iter(lambda: stream.read(65536), b''): value.update(data)
    return value.hexdigest()

def read(path): return json.loads(Path(path).read_text())
def verify_map(mapping, prefix=Path('.')):
    for path, expected in mapping.items(): assert digest(prefix/path) == expected, str(prefix/path)
def hashes(paths): return {str(p):digest(p) for p in sorted(set(map(Path, paths)))}
def aggregate(mapping): return hashlib.sha256(json.dumps(mapping,sort_keys=True,separators=(',',':')).encode()).hexdigest()

old_path = Path('docs/evidence/native-messages-service-provenance.json')
old = read(old_path)
old_selection = Path(old['qualified_selection']['path'])
manifest = read(root/'frozen-baseline/manifest.json')
assert digest(old_path) == manifest['prior_provenance_sha256']
assert digest(old_selection) == manifest['prior_selection_sha256'] == 'ff8d206cc04b14706a0d40cf255ff0e63fd2279d9be8cb95aa485f9afff3772a'
verify_map(manifest['source_sha256'], root/'frozen-baseline/source')
verify_map(manifest['binaries_sha256'], root/'frozen-baseline/bins')
verify_map(old['artifact_sha256'])
for items in read(old_selection)['executables'].values():
    if isinstance(items, dict):
        verify_map(items)
        continue
    for item in items:
        assert digest(item['snapshot']) == item['sha256']
        assert digest(item['executable']) == item['sha256']
old_desktop = read('artifacts/native-messages/desktop/green-5/run-config.json')
verify_map(old_desktop['files'], Path('artifacts/native-messages/desktop/green-5'))
for phase in ['red-ordering','red-retained-fd']:
    path = root/phase
    verify_map(read(path/'source-sha256.json'), path/'source')
    assert read(path/'source-sha256.json')['native-lifecycle/src/service.rs'] == old['source_sha256']['native-lifecycle/src/service.rs']
    assert (path/'exit-code').read_text().strip() == '1'
    assert 'successful retained desktop launch left local Messages storage unavailable' in (path/'log').read_text()

worker_path = Path('docs/evidence/native-messages-ui-worker-provenance.json')
worker = read(worker_path)
worker_index_path = Path(worker['evidence_index']['path'])
worker_index = read(worker_index_path)
assert digest(worker_index_path) == worker['evidence_index']['sha256']
verify_map(worker_index['source_sha256']); verify_map(worker_index['proof_sha256'])
worker_green = read(Path(worker['green']['path'])/'result.json')
assert worker_green['passed'] and len(worker_green['cases']) == 7
for name, expected in worker_green['consumer_source_sha256'].items(): assert digest(Path('native-messages-ui')/name) == expected

sources = set(old['source_sha256']) - {'docs/native-lifecycle.md'}
sources.update(worker_index['source_sha256'])
sources.update(['tools/native-messages-recovery-order-fixture.py','tools/native-messages-retained-fd-app.py',
 'tools/native-messages-recovery-order.sh','tools/select-native-messages-correction.sh','tools/index-native-messages-correction.py'])
source_map = hashes(sources)
dependencies = dict(old['read_only_dependency_sha256'])
for name in ['native-lifecycle-fixture.py','native-lifecycle-storage-fixture.py','native-lifecycle-storage-cleanup-fixture.py']:
    path = Path('tools')/name; dependencies[str(path)] = digest(path)
verify_map(dependencies)
qualified.mkdir()
for name in sorted(set(source_map)|set(dependencies)):
    target = qualified/'source'/name; target.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(name,target)
executables = {}
for target in ['x86_64-unknown-linux-gnu','aarch64-unknown-linux-gnu']:
    selected = []
    for kind in ['bins','lifecycle','release','projection']:
        cargo_path = root/'cargo-json'/f'{target}-{kind}.jsonl'
        records = [json.loads(line) for line in cargo_path.read_text().splitlines()]
        assert records[-1] == {'reason':'build-finished','success':True}
        records = [r for r in records if r.get('reason') == 'compiler-artifact' and r.get('executable')
            and (kind != 'projection' or r['target']['name'] == 'projection')]
        assert len(records) == {'bins':4,'lifecycle':3,'release':3,'projection':1}[kind]
        for record in records:
            path = Path(record['executable']).relative_to('/target')
            actual = root/('components' if kind == 'projection' else 'build')/path
            snapshot = qualified/target/kind/actual.name; snapshot.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(actual,snapshot); snapshot.chmod(0o755)
            selected.append({'name':record['target']['name'],'class':kind,'cargo_json':str(cargo_path),
                'executable':str(actual),'snapshot':str(snapshot),'sha256':digest(actual),
                'fresh':record['fresh'],'features':record['features'],'test_cases':4 if kind=='projection' else None})
    executables[target] = selected
selection = {'source_sha256':source_map,'dependency_sha256':dependencies,'executables':executables}
(qualified/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')

desktop = Path('artifacts/native-messages/desktop')/a.desktop
config = read(desktop/'run-config.json'); verify_map(config['files'], desktop)
for path, expected in config['files'].items():
    if path.startswith('source/'):
        original = path.removeprefix('source/')
        assert digest(original) == expected, original
for name in ['messages-service','messages-launch','messages-stop','messages-seed-fixture','lifecycle-launcher','lifecycle-client']:
    selected = next(v for v in executables['aarch64-unknown-linux-gnu'] if v['name']==name and v['class'] in ('bins','lifecycle'))
    assert digest(desktop/'bins'/name) == selected['sha256']
result = read(desktop/'result.json'); assert result['passed'] and len(result['groups']) == 11
assert result['service_kernel']['passed'] and len(result['service_kernel']['groups']) == 10
assert result['service_foreign_fd']['passed'] and len(result['service_foreign_fd']['groups']) == 2
for phase in ['service-kernel','service-foreign-fd']:
    detail = read(desktop/phase/'retained-desktop-order.json')
    assert detail['old_worker'] != detail['new_worker'] and detail['old_worker_source'] != detail['new_worker_source']
    assert detail['old_app_actually_reaped'] and detail['old_worker_actually_reaped'] and detail['new_app_denied_before_current_authorization']
x86 = read(root/f'service-green-{a.process_suffix}'/'result.json'); assert x86['passed'] and len(x86['groups']) == 10
for phase in ['green-ordering','green-retained-fd','service-green']:
    pinned = read(root/f'{phase}-{a.process_suffix}'/'run-selection.json')
    verify_map(pinned['source_sha256']); verify_map(pinned['binary_sha256'])
for phase in ['green-ordering','green-retained-fd']:
    value = read(root/f'{phase}-{a.process_suffix}'/'result.json'); assert value['passed'] and len(value['groups']) == 2
regression_path = Path('artifacts/native-messages/lifecycle-regression-recovery-order/result.json')
regression = read(regression_path); assert regression['passed'] and regression['cases'] == 27
for name, expected in regression['binaries'].items():
    assert digest(regression_path.parent/'bins'/name) == expected
    assert next(v['sha256'] for v in executables['x86_64-unknown-linux-gnu'] if v['name']==name and v['class']=='lifecycle') == expected
for name, expected in regression['fixtures'].items(): assert dependencies['tools/'+name] == expected
component_log = (root/'component-green.log').read_text(); assert component_log.count('4 passed; 0 failed') == 2
production_qt = root/'qt-production-current/pokelinux-messages'
assert 'MESSAGES_ISOLATED_OBSERVER:BOOL=OFF' in (production_qt.parent/'CMakeCache.txt').read_text()
for name, expected in result['production_binary'].items(): assert digest(desktop/name) == expected

proofs = [old_path,old_selection,root/'frozen-baseline/manifest.json',root/'frozen-baseline/native-messages-service.md',
 root/'build-checks.log',root/'component-green.log',root/'lifecycle-regression.log',regression_path,
 worker_path,worker_index_path,Path('docs/native-messages-ui-worker.md'),qualified/'selection.json',
 production_qt,production_qt.parent/'build.log',production_qt.parent/'CMakeCache.txt']
for phase in ['red-ordering','red-retained-fd',f'green-ordering-{a.process_suffix}',f'green-retained-fd-{a.process_suffix}',f'service-green-{a.process_suffix}']:
    proofs.extend(p for p in (root/phase).iterdir() if p.is_file())
proofs.extend(p for p in (root/'cargo-json').iterdir() if p.is_file())
proofs.extend(p for p in desktop.rglob('*') if p.is_file() and 'source' not in p.relative_to(desktop).parts and 'bins' not in p.relative_to(desktop).parts)
artifact_map = hashes(proofs)
payload = {'schema_version':1,'written_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'host':'Harbour',
 'scope':'Installed local Messages history; source-bound post-retirement recovery and owned Qt worker correction',
 'source_sha256':source_map,'source_aggregate_sha256':aggregate(source_map),'read_only_dependency_sha256':dependencies,
 'qualified_selection':{'path':str(qualified/'selection.json'),'sha256':digest(qualified/'selection.json'),'cargo_json_complete':True,'snapshot':str(qualified)},
 'historical_checkpoint':{'provenance':str(old_path),'provenance_sha256':digest(old_path),'selection':str(old_selection),
  'selection_sha256':digest(old_selection),'source_aggregate_sha256':aggregate(old['source_sha256']),
  'source_snapshot':str(root/'frozen-baseline/source'),'source_snapshot_files':len(manifest['source_sha256']),
  'old_source_binary_proof_checks':'all archived source files and selected old build/snapshot binaries verified unchanged',
  'old_proof_files_verified':len(old['artifact_sha256']),'old_desktop_files_verified':len(old_desktop['files']),
  'desktop_groups':10,'full_service_groups':9,'status':'historical prior source; superseded for current service/UI behavior'},
 'runtime_reds':[{'class':'actual x86 Linux UID/PID/seqpacket held-read process ordering','path':str(root/phase),
  'source_snapshot_sha256':digest(root/phase/'source-sha256.json'),'exit_code':1,
  'failure':'successful retained desktop launch after actual old-app reap leaves local history unavailable',
  'foreign_fd_retained':phase=='red-retained-fd'}for phase in ['red-ordering','red-retained-fd']],
 'selected_green':{'x86_full_service':{'path':str(root/f'service-green-{a.process_suffix}'/'result.json'),'groups':x86['groups']},
  'x86_focused':{'ordering':str(root/f'green-ordering-{a.process_suffix}'/'result.json'),'retained_fd':str(root/f'green-retained-fd-{a.process_suffix}'/'result.json'),'groups_each':2},
  'arm64_kernel':{'desktop_result':str(desktop/'result.json'),'desktop_groups':result['groups'],
   'full_service_result':str(desktop/'service-kernel/result.json'),'full_service_groups':result['service_kernel']['groups'],
   'retained_fd_result':str(desktop/'service-foreign-fd/result.json'),'retained_fd_groups':result['service_foreign_fd']['groups'],'kernel':result['kernel'],
   'production_and_observer_elf_sha256':result['production_binary'],
   'human_inspected_synthetic_screenshot':str(desktop/'production-desktop.png'),'actual_Wayland_systemd':True,'actual_production_observer_OFF':True},
  'components':{'log':str(root/'component-green.log'),'x86_cases':4,'ARM64_user_QEMU_cases':4},
  'lifecycle_regression':{'path':str(regression_path),'cases':27,'scope':regression['scope']},
  'fmt_clippy_and_both_architecture_builds':str(root/'build-checks.log'),
  'x86_production_Qt':{'path':str(production_qt),'sha256':digest(production_qt),'observer_compiled_in':False},
  'independent_Qt_worker':{'doc':'docs/native-messages-ui-worker.md','provenance':str(worker_path),
   'index':str(worker_index_path),'index_sha256':digest(worker_index_path),'current_sources':6,'proofs':48,
   'red_path':worker['red']['path'],'genuine_behavior_reds':5,'green_path':worker['green']['path'],'groups':7,
   'scope':worker['scope'],'injections':worker['injections'],'queued_gui_scope':worker['production_policy']['queued_gui_scope']}},
 'recovery_policy':{'intent':'only successful fixed actual UID1000 launch; old source plus absolute2s BOOTTIME',
  'evaluation':'after all connection retirements; available-but-invalid old read retains intent',
  'restart':'consume before one attempt; require exact old child actually reaped and no occupied child',
  'read_granted':False,'old_query_replayed':False,'remote_effect_replayed':False,
  'fixture_app':'authored installed UID11010 Python process fixture; actual native Qt installed window independently qualified',
  'foreign_descriptor_scope':'one real unregistered child retains old app socket without requests; no general process-tree or uninterruptible-task guarantee',
  'real_poll_observation':'x86 pre/stopped271; ARM actual pre73 then stopped-1/do_signal_stop; actor handles EINTR in same connection pass'},
 'inherited_authority':old['authority'],'inherited_bounds':old['bounds'],'inherited_privacy':old['privacy'],
 'unselected_diagnostics':[str(root/'setup-red-ordering-unreaped-principal'),str(root/'qt-production'),
  'artifacts/native-messages/desktop/green-6:ARM poll observation setup',
  'artifacts/native-messages/desktop/green-7:ARM stopped syscall observation setup',
  'artifacts/native-messages/desktop/green-8:old-worker /proc sampling during reap transition'],
 'artifact_sha256':artifact_map,
 'remaining':['startup/reopen/pre-read projection only; no continuous raw ingress or frozen V3 ingress connection',
  'raw enumeration cap32; refuse partial history','production Omarchy lock/focus bridge and dynamic theme/session ACL renewal absent',
  'no modem receive/ACK/send/carrier delivery, physical Pixel/PowerVR, real suspend or kernel-uninterruptible cleanup qualification',
  'no new original Android/Binder parser execution; inherited33 DELIVER comparisons historical',
  'Qt exchange completion checked; queued GUI generation fence is not fresh authority or arbitrary delivery expiry']}
output = json.dumps(payload,indent=2)+'\n'
index = root/'final-evidence-index.json'; public = Path('docs/evidence/native-messages-correction-provenance.json')
assert not index.exists() and not public.exists(), 'preserve previous correction index'
index.write_text(output); public.write_text(output)
verify_map(source_map); verify_map(dependencies); verify_map(artifact_map)
print(json.dumps({'index_sha256':digest(index),'sources':len(source_map),'dependencies':len(dependencies),'proofs':len(artifact_map),
 'selected_executables':sum(map(len,executables.values())),'source_aggregate':aggregate(source_map)},indent=2))
