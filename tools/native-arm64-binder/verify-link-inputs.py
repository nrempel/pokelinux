#!/usr/bin/env python3
"""Check exact cached NDK compiler, public headers and link-time stubs."""
import hashlib
import json
from pathlib import Path
root = Path(__file__).resolve().parents[2]
inputs = json.loads((root/'tools/native-arm64-binder/ndk-link-inputs.json').read_text())
for relative, expected in inputs['sha256'].items():
    assert hashlib.sha256((root/relative).read_bytes()).hexdigest() == expected, relative
print('Cached NDK compiler/header/link inputs verified:', len(inputs['sha256']))
