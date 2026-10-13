#!/usr/bin/env python3
"""Fetch only pinned public AOSP fixture inputs; genuine runtime is retained separately."""
import base64
import hashlib
import json
from pathlib import Path
import urllib.request
root = Path(__file__).resolve().parents[2]
inputs = json.loads((root/'tools/native-arm64-binder/public-inputs.json').read_text())
out = root/'artifacts/native-arm64-binder/public'
for relative, entry in inputs.items():
    path = out/relative
    if not path.exists():
        assert entry['url'].startswith('https://android.googlesource.com/platform/')
        raw = base64.b64decode(urllib.request.urlopen(entry['url'], timeout=30).read(), validate=True)
        assert hashlib.sha256(raw).hexdigest() == entry['sha256'], relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
    assert hashlib.sha256(path.read_bytes()).hexdigest() == entry['sha256'], relative
print('Pinned public AOSP fixture inputs verified:', len(inputs))
