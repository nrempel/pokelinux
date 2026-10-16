#!/usr/bin/env python3
"""Select exact Cargo artifacts; never discover guest executables by glob."""
import hashlib
import json
from pathlib import Path
root=Path(__file__).resolve().parent.parent
base=root/'artifacts/native-lifecycle'
items=[]
for filename,kind in [('arm64-bins-cargo.jsonl','binary'),('arm64-lib-cargo.jsonl','library_test')]:
    raw=(base/filename).read_bytes()
    records=[json.loads(line) for line in raw.decode().splitlines()]
    selected=[r for r in records if r.get('reason') == 'compiler-artifact' and r.get('executable') and bool(r.get('profile',{}).get('test')) == (kind == 'library_test')]
    names={'lifecycle-daemon','lifecycle-launcher','lifecycle-client','lifecycle-lock','sounds-launch'} if kind == 'binary' else {'pokelinux_lifecycle'}
    assert {r['target']['name'] for r in selected} == names
    assert len(selected) == len(names)
    for record in selected:
        cargo_path=Path(record['executable'])
        relative=cargo_path.relative_to('/target')
        assert relative.parts[0] == 'aarch64-unknown-linux-gnu'
        path=base/'target'/relative
        items.append({'kind':kind,'name':record['target']['name'],'cargo_path':str(cargo_path),'project_path':str(path.relative_to(root)),'guest_path':'/workspace/'+str(path.relative_to(root)),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'bytes':path.stat().st_size,'cargo_manifest':str((base/filename).relative_to(root)),'cargo_manifest_sha256':hashlib.sha256(raw).hexdigest()})
manifest={'target':'aarch64-unknown-linux-gnu','runtime_qualified':False,'artifacts':items,'fixture':'tools/native-lifecycle-fixture.py','fixture_sha256':hashlib.sha256((root/'tools/native-lifecycle-fixture.py').read_bytes()).hexdigest(),'fixture_environment':{'NATIVE_LIFECYCLE_ISOLATED':'1'},'component_environment':{'NATIVE_LIFECYCLE_TEST_ROOT':'/run'},'required_runtime':'actual Linux kernel, root in disposable guest, Python3, GNU libc; no network/phone/device transport','fixture_binary_root':'/workspace/artifacts/native-lifecycle/target/aarch64-unknown-linux-gnu/debug'}
(base/'arm64-artifacts.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'artifacts':len(items),'runtime_qualified':False}))
