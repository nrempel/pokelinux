#!/usr/bin/env python3
"""Seal an explicit composed source snapshot, ARM binaries and readonly inputs."""
import datetime,hashlib,json,shutil,sys
from pathlib import Path
ROOT=Path('/workspace');SNAP=Path('/snapshot');base=ROOT/sys.argv[1];freeze=base/'freeze'
assert not freeze.exists();freeze.mkdir(parents=True)
def digest(path):
    value=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()
files={}
def retain(source,relative):
    target=freeze/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target);target.chmod(0o555 if relative.startswith('bins/') else 0o444);files[relative]=digest(target)
for directory in ['native-camera','native-camera-service','native-lifecycle','native-kernel-identity','tools/native-camera-service-boundary']:
    for source in (SNAP/directory).rglob('*'):
        if source.is_file() and 'target' not in source.parts and '__pycache__' not in source.parts:
            relative=str(source.relative_to(SNAP));assert digest(source)==digest(ROOT/relative),'snapshot/live divergence '+relative;retain(source,'sources/'+relative)
for relative in ['framework/camerapeer/frozen-schema-manifest.json','tools/native-arm64-binder/boot.sh']:
    retain(ROOT/relative,'sources/'+relative)
manifest=json.loads((ROOT/'framework/camerapeer/frozen-schema-manifest.json').read_text())
schemas=Path('/inputs/frozen-aidl')
assert {str(p.relative_to(schemas)) for p in schemas.rglob('*.aidl')}=={row['path'] for row in manifest['imports']}
for row in manifest['imports']:assert digest(schemas/row['path'])==row['sha256']
for name in ['camera-composed-service','lifecycle-launcher','lifecycle-client','camera-red-manager','camera-red-peer','camera-red-probe']:
    retain(Path('/target/aarch64-unknown-linux-gnu/debug')/name,'bins/'+name)
generated=list(Path('/target/aarch64-unknown-linux-gnu/debug/build').glob('native-camera-service-boundary-fixture-*/out/camera.rs'))
assert generated and len({digest(p) for p in generated})==1
for source in generated:retain(source,'generated/'+source.parent.parent.name+'/camera.rs')
inputs={name:{'sha256':digest(Path('/inputs')/name)} for name in ['Image','initramfs-linux.img','root.img']}
assert inputs['root.img']['sha256']=='973a19f9661e915b2d9eb88753ef4d59585cc6ebd4565e43a03b43e74b0f7579'
assert inputs['Image']['sha256']=='2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b'
assert inputs['initramfs-linux.img']['sha256']=='a9bb29d29abda94748c9948f903c3c1704f829eea477652b9a4201a61f36a647'
selection={'scope':sys.argv[2],'created_at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'base_commit':'2134c62a06fe46a2a4ecbe4526653a9824822924','files':files,'inputs':inputs,'rust_image':'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6','simulator_image':'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6','fixture_runtime':'sha256:07b2b220c62ebe15a90af7e10d703297b511508891492a292ae986f0521c448a','guest':sys.argv[3],'expected':sys.argv[4],'network':'none','ui':False,'app_uid':11020,'backend_uid':11030,'Source_claim':False,'original_ndk_claim':False,'vendor_execution':False,'physical_phone_accessed':False,'backing':'read-only raw input; bounded temporary qcow2 overlay; no clone','canonical_lock':'shared existing native-arm64-binder/vm.lock through actual Docker wait and posthash; historical VM reader exclusive; historical prepare refuses present backing separately'}
(freeze/'selection.json').write_text(json.dumps(selection,indent=2)+'\n');(freeze/'selection.json').chmod(0o444)
print(json.dumps({'selection_sha256':digest(freeze/'selection.json'),'files':len(files),'inputs':inputs},indent=2))
