#!/usr/bin/env python3
"""Verify genuine retained Android runtime; metadata/hashes only, no execution."""
import hashlib
import json
from pathlib import Path
root = Path(__file__).resolve().parents[2]
manifest = json.loads((root/'tools/native-arm64-binder/android-runtime-inputs.json').read_text())
for name, entry in manifest['files'].items():
    path = root/entry['path']
    raw = path.read_bytes()
    assert raw[:4] == b'\x7fELF' and int.from_bytes(raw[18:20], 'little') == 183, name
    assert hashlib.sha256(raw).hexdigest() == entry['sha256'], name
out = root/'artifacts/native-arm64-binder/android-runtime-closure.json'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(manifest, indent=2)+'\n')
print('Genuine retained ARM64 Android runtime verified:', len(manifest['files']), 'files')
