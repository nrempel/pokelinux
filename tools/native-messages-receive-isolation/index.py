#!/usr/bin/env python3
"""Freeze only authored sources/binaries and publish clean hashes of scoped proof."""
import hashlib,json,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'artifacts/native-messages-receive-isolation'
LABELS=['frozen-held-close-green-arm','frozen-sqlite-permit-green-arm','frozen-reader-errors-green-arm','frozen-enumeration-components-green-arm']
REDS=['held-close-desired-red-arm','held-sqlite-lock-red-arm-v2']
SELECT=BASE/'selections/receive-isolation-final-v1'

def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
 return h.hexdigest()

def relative(path):return str(path.relative_to(ROOT))

def pin(path):return {'path':relative(path),'sha256':digest(path)}

def copy_once(source,target):
 target.parent.mkdir(parents=True,exist_ok=True)
 if target.exists():assert digest(target)==digest(source),'frozen selection cannot be relabeled'
 else:shutil.copyfile(source,target)

tracked=subprocess.check_output(['git','ls-files','-z'],cwd=ROOT).decode().split('\0')
prefixes=['native-messages/','native-inbox/','native-messaging/','native-lifecycle/','native-kernel-identity/','tools/native-messages-incoming-binder/']
paths={ROOT/p for p in tracked if p and any(p.startswith(prefix) for prefix in prefixes)}
paths.update(p for p in (ROOT/'tools/native-messages-receive-isolation').iterdir() if p.is_file())
paths.update(ROOT/p for p in ['README.md','AGENTS.md','docs/native-messages-receive-isolation.md','docs/native-messages-service.md','docs/native-messages-incoming.md','docs/native-messages-enumeration.md','docs/native-lifecycle.md'])
SELECT.mkdir(parents=True,exist_ok=True)
sources={}
for path in sorted(paths):
 copy_once(path,SELECT/'sources'/path.relative_to(ROOT));sources[relative(path)]=digest(path)

binaries={}
for name in ['messages-incoming-fixture-service','messages-service','lifecycle-client','lifecycle-launcher','messages-stop']:
 path=BASE/'target/aarch64-unknown-linux-gnu/debug'/name
 copy_once(path,SELECT/'binaries'/name);binaries[relative(path)]=digest(path)
for manifest in ['projection-arm-tests.jsonl','receipt-arm-tests.jsonl','inbox-arm-tests.jsonl']:
 for line in (BASE/'target'/manifest).read_text().splitlines():
  row=json.loads(line)
  if row.get('reason')=='compiler-artifact' and row.get('executable') and row['profile']['test']:
   path=BASE/'target'/Path(row['executable']).relative_to('/target')
   copy_once(path,SELECT/'binaries'/path.name);binaries[relative(path)]=digest(path)

proof={};groups=0
for label in LABELS+REDS:
 directory=BASE/'runtime'/label
 result=json.loads((directory/'result.json').read_text())
 assert result['architecture']=='aarch64' and result['kernel']=='7.1.6-1-aarch64-ARCH' and result['network']=='none' and result['qemu_mode']=='system'
 assert not result['phone_accessed'] and not result['vendor_execution'] and not result['session_provider']
 if label in LABELS:
  assert result['passed'];groups+=len(result['checks'])
 else:assert result.get('expected_red') and not result['passed']
 for path in sorted(directory.iterdir()):
  if path.is_file():proof[relative(path)]=digest(path)
for name in ['final-component-check-v3.log','frozen-format-clippy.log','frozen-arm-build.log','input/input-pins.json','baseline/selection.json','split-draft/selection.json','sqlite-lock-red/selection.json']:
 path=BASE/name;proof[relative(path)]=digest(path)
for stage in ['baseline','split-draft','sqlite-lock-red']:
 selection=json.loads((BASE/stage/'selection.json').read_text())
 for path,sha in selection['source_sha256'].items():assert digest(BASE/stage/'sources'/path)==sha
 for name,sha in selection['binary_sha256'].items():assert digest(BASE/stage/'binaries'/name)==sha

# Real-kernel process cases must actually run; all Inbox executables must remain logged.
arm=BASE/'runtime'/LABELS[-1]
expected={'projection-arm-tests.jsonl.log':[9],'receipt-arm-tests.jsonl.log':[22],'inbox-arm-tests.jsonl.log':[4,15,1]}
for name,counts in expected.items():
 value=(arm/name).read_text()
 for count in counts:assert f'test result: ok. {count} passed; 0 failed; 0 ignored;' in value
assert groups==17
inputs={}
for name,sha in json.loads((BASE/'input/input-pins.json').read_text()).items():
 assert digest(BASE/'input'/name)==sha;inputs[name]=sha
runtime=Path('/home/ubuntu/Projects/pokelinux/artifacts/runtime')
originals={}
for name in ['system/lib64/libbinder_ndk.so','apex/com.android.runtime/bin/linker64','apex/com.android.runtime/lib64/bionic/libc.so']:
 originals[name]=digest(runtime/name)
kernel=Path('/home/ubuntu/Projects/pokelinux/sim/boot')
originals.update({f'kernel/{name}':digest(kernel/name) for name in ['Image','initramfs-linux.img']})
aidl=Path('/home/ubuntu/Projects/pokelinux/artifacts/sms-binder/frozen-aidl')
aidl_hashes={str(path.relative_to(aidl)):digest(path) for path in sorted(aidl.rglob('*.aidl'))}
selection={'frozen_public_aidl_sha256':aidl_hashes,'base_head':'84ac3f856d9b71aeb7bccbfd33eb5a8a828b079f','source_sha256':sources,'binary_sha256':binaries,'proof_sha256':proof,'fixture_inputs_sha256':inputs,'readonly_runtime_kernel_sha256':originals}
(SELECT/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
public={
 'scope':'independent supervised readonly child and exact-occurrence projection permission preserving same live synthetic receiver; no session Provider or physical-device claim',
 'base_head':selection['base_head'],'final_selection':pin(SELECT/'selection.json'),
 'frozen_public_aidl':aidl_hashes,'sources':sources,'binaries':binaries,'proof':proof,'fixture_inputs':inputs,'readonly_runtime_kernel_hashes':originals,
 'images':{'builder':'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6','simulator':'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'},
 'executed_reds':[pin(BASE/'runtime'/label/'result.json') for label in REDS],
 'executed_greens':[pin(BASE/'runtime'/label/'result.json') for label in LABELS],
 'counts':{'arm_runtime_groups_including_three_component_groups':groups,'arm_integration_groups':14,'projection_per_architecture':9,'receipt_per_architecture':22,'inbox_per_architecture':20,'focused_cases_per_architecture':51},
 'evidence_classification':{'original_runtime_libraries':'actual retained Bionic/libbinder_ndk, readonly; authored generated NDK peer and PDUs','original_decoder':'existing33 recorded AOSP outputs; no new parser execution','kernel':'actual ARM64 Arch7.1.6 Binder/SQLite/process, system-QEMU, no NIC','x86':'actual authored SQLite/process tests inside isolated offline builder','phone':False,'vendor_execution':False,'session_provider':False},
 'diagnostics':'All earlier labels/logs remain unselected; first ARM enumeration omitted2 ignored process tests and overwrote earlier Inbox executable logs, corrected final run includes all51 cases and retains every output.',
 'limits':['normal production entrypoint remains history-only','no production incoming/producer/ACK/reset admission','uninterruptible or unobserved cleanup does not grant capacity; original arrival deadline can terminate source','raw append-only4096/inbox2048text/4096key/512conversation and cold backlog gates remain','no joint current session/compositor/Qt qualification from this isolated lane','no phone boot/modem/carrier/notifications/wake/retention proof'],
}
public_path=ROOT/'docs/evidence/native-messages-receive-isolation-provenance.json'
public_path.write_text(json.dumps(public,indent=2)+'\n')
print(json.dumps({'selection':pin(SELECT/'selection.json'),'public_provenance':pin(public_path),'groups':groups,'focused_cases_per_architecture':51},indent=2))
