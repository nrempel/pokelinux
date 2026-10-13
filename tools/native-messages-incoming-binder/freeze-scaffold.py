#!/usr/bin/env python3
"""Freeze exact authored scaffold/generated artifacts; genuine runtime bytes stay private in place."""
import hashlib,json,shutil,sys
from pathlib import Path
root=Path(__file__).resolve().parents[2];base=root/'artifacts/native-messages-incoming-binder'
label=sys.argv[1];assert label.replace('-','').replace('_','').isalnum()
out=base/'selections'/label;out.mkdir(parents=True,exist_ok=False)
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
selected={}
inputs=[]
for directory in ['tools/native-messages-incoming-binder','native-messages','native-lifecycle','native-inbox','native-messaging','native-kernel-identity']:
    inputs.extend((root/directory).rglob('*'))
for path in sorted(inputs):
    if path.is_file() and '__pycache__' not in path.parts and (path.suffix in ['.rs','.toml','.lock','.sh','.py','.cpp','.aidl','.md'] or path.name=='public-api-inputs.json'):
        relative=path.relative_to(root);target=out/'sources'/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,target);selected[str(relative)]=digest(path)
binaries={}
for relative in ['incoming-peer','property-info','property-init',*['target/aarch64-unknown-linux-gnu/debug/'+name for name in ['incoming-manager','incoming-probe','incoming-smoke','messages-incoming-fixture-service','messages-service','messages-stop','lifecycle-launcher','lifecycle-client','lifecycle-daemon']]]:
    path=base/relative;assert path.is_file();target=out/'binaries'/Path(relative).name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,target);binaries[relative]=digest(path)
for row in json.loads((base/'kernel-test-artifacts.json').read_text()):
    path=base/row['path'];assert digest(path)==row['sha256'];target=out/'binaries'/path.name;shutil.copyfile(path,target);binaries[row['path']]=digest(path)
shutil.copyfile(base/'kernel-test-artifacts.json',out/'kernel-test-artifacts.json')
generated={}
for folder in [base/'ndk/headers',base/'ndk/generated',base/'target/aarch64-unknown-linux-gnu/debug/build']:
    for path in sorted(folder.rglob('*')):
        if path.is_file() and path.suffix in ['.h','.cpp','.rs']:
            generated[str(path.relative_to(root))]=digest(path)
shutil.copyfile(base/'verified-inputs.json',out/'verified-inputs.json')
(out/'selection.json').write_text(json.dumps({'scope':'explicit immutable source/generated/ELF selection; runtime qualification is recorded separately',
    'sources':selected,'binaries':binaries,'generated':generated},indent=2)+'\n')
print(out.relative_to(root))
