#!/usr/bin/env python3
"""Refuse unpinned upstream schemas before either original or Rust generation."""
from pathlib import Path
import hashlib
import json
root = Path(__file__).resolve().parents[1]
frozen = root / 'artifacts/camera-boundary/frozen-aidl'
manifest = json.loads((root / 'framework/camerapeer/frozen-schema-manifest.json').read_text())
expected = set()
for item in manifest['imports']:
    path = frozen / item['path']
    expected.add(path)
    assert path.is_file(), 'Run tools/prepare-camera-boundary.py: ' + item['path']
    assert hashlib.sha256(path.read_bytes()).hexdigest() == item['sha256'], 'Unqualified camera schema: ' + item['path']
assert set(frozen.rglob('*.aidl')) == expected, 'Unpinned schema in isolated frozen-camera input tree'
