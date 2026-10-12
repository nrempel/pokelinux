#!/usr/bin/env python3
"""Verify bounded selected public inputs; upstream code stays ignored."""
import hashlib
import json
from pathlib import Path
import urllib.request
root=Path(__file__).resolve().parent.parent
manifest=json.loads((root/'tools/native-lifecycle-reference.json').read_text())
assert manifest['commit'] == 'e8f897f4afef0031fe618a8e94127a0934896aba'
assert len(manifest['files']) == 4
for entry in manifest['files']:
    path=Path(entry['path'])
    assert not path.is_absolute() and '..' not in path.parts
    expected=f"https://raw.githubusercontent.com/torvalds/linux/{manifest['commit']}/{path}"
    assert entry['url'] == expected
    target=root/'artifacts/native-lifecycle/reference'/path
    if target.exists(): data=target.read_bytes()
    else:
        with urllib.request.urlopen(expected,timeout=20) as response: data=response.read(1_048_577)
    assert len(data) == entry['bytes'] <= 1_048_576
    assert hashlib.sha256(data).hexdigest() == entry['sha256']
    target.parent.mkdir(parents=True,exist_ok=True)
    if not target.exists(): target.write_bytes(data)
print(json.dumps({'verified_inputs':4,'upstream_published':False}))
