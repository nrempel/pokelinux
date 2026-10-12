#!/usr/bin/env python3
"""Fetch only pinned public libperfmgr reference files into ignored artifacts."""
import base64
import hashlib
import json
import pathlib
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = json.loads((ROOT / 'tools/power-reference/fetch-manifest.json').read_text())
DESTINATION = ROOT / 'artifacts/native-power/reference/upstream'
for item in MANIFEST['files']:
    path = DESTINATION / item['path']
    if path.exists():
        data = path.read_bytes()
    else:
        with urllib.request.urlopen(item['url'], timeout=45) as response:
            encoded = response.read(2 * 1024 * 1024)
        data = base64.b64decode(encoded, validate=True)
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + '.partial')
        with temporary.open('xb') as output:
            output.write(data)
        if len(data) != item['bytes'] or hashlib.sha256(data).hexdigest() != item['sha256']:
            temporary.unlink()
            raise RuntimeError(f"Reference identity mismatch: {item['path']}")
        temporary.rename(path)
    if len(data) != item['bytes'] or hashlib.sha256(data).hexdigest() != item['sha256']:
        raise RuntimeError(f"Reference identity mismatch: {item['path']}")
print(json.dumps({'verified_files': len(MANIFEST['files']), 'commit': MANIFEST['commit'],
                  'destination': str(DESTINATION)}))
