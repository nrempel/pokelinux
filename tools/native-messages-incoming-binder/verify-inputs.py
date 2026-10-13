#!/usr/bin/env python3
"""Verify cached public schemas/compiler plus genuine runtime, with no fetch/mutation."""
import hashlib,json
from pathlib import Path
root=Path(__file__).resolve().parents[2]
base=root/'artifacts/native-messages-incoming-binder'
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
schema=root/'artifacts/sms-binder/frozen-schema-manifest.json'
assert sha(schema)=='3688bca6338ee23b27d5c8c470f97aa7fe2754daa3df746452633610d73793a6'
for row in json.loads(schema.read_text())['sources']:
    if 'path' in row: assert sha(root/'artifacts/sms-binder'/row['path'])==row['sha256'],row['path']
for manifest in ['ndk-link-inputs.json','public-inputs.json']:
    path=root/'tools/native-arm64-binder'/manifest
    data=json.loads(path.read_text())
    if manifest=='ndk-link-inputs.json':
        for name,expected in data['sha256'].items(): assert sha(root/name)==expected,name
    else:
        for name,row in data.items(): assert sha(root/'artifacts/native-arm64-binder/public'/name)==row['sha256'],name
headers=root/'artifacts/camera-boundary/public-headers.json'
assert sha(headers)=='b21e537dab35b30292e1aadec296167a068bf4eca8b5d9f4630c0b40111433d6'
for name,row in json.loads(headers.read_text()).items():
    assert sha(root/'artifacts/camera-boundary/public-headers/android'/name)==row['sha256'],name
runtime=json.loads((root/'tools/native-arm64-binder/android-runtime-inputs.json').read_text())
for name,row in runtime['files'].items():
    path=root/row['path'];raw=path.read_bytes()
    assert raw[:4]==b'\x7fELF' and int.from_bytes(raw[18:20],'little')==183,name
    assert sha(path)==row['sha256'],name
base.mkdir(parents=True,exist_ok=True)
(base/'verified-inputs.json').write_text(json.dumps({'frozen_manifest_sha256':sha(schema),'runtime':runtime,
    'aidl_compiler_sha256':sha(root/'android-framework/sdk/build-tools/android-37.0/aidl'),
    'runtime_substitution':False},indent=2)+'\n')
print('Verified pinned frozen schema and retained public/compiler/genuine runtime inputs')
