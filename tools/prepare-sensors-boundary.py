#!/usr/bin/env python3
"""Only pinned public inputs; no vendor/physical target execution."""
from pathlib import Path
import base64
import hashlib
import json
import urllib.request
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/sensors-boundary/public'
manifest=json.loads((ROOT/'framework/sensorpeer/public-manifest.json').read_text())
for item in manifest['files']:
    path=OUT/item['path'];path.parent.mkdir(parents=True,exist_ok=True)
    if not path.is_file():
        with urllib.request.urlopen(item['url'],timeout=45) as response:
            path.write_bytes(base64.b64decode(response.read()))
    assert hashlib.sha256(path.read_bytes()).hexdigest()==item['sha256'], 'Unqualified public FMQ input: '+item['path']
expected={OUT/item['path'] for item in manifest['files'] if item['path'].endswith('.aidl')}
assert set((OUT/'frozen-aidl').rglob('*.aidl'))==expected, 'Unmanifested or missing frozen schema'
for item in manifest['existing_qualified_build_inputs']:
    path=ROOT/item['path']
    assert path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest()==item['sha256'], 'Unqualified compiler/helper input: '+item['path']
print('Pinned public SensorsV3/FMQ source closure ready',flush=True)
