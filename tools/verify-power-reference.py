#!/usr/bin/env python3
"""Validate and publish only outcomes from our synthetic reference scenarios."""
import hashlib
import json
import pathlib

root = pathlib.Path(__file__).resolve().parent.parent
source = root / 'artifacts/native-power/reference/reference.tsv'
expected = {'renewal-extended': '1', 'renewal-never-shortens': '1', 'expired-active': '0',
            'forever-active': '1', 'owner-cancel': '1', 'cancelled-active': '0',
            'initial-default': 'idle', 'low-value': 'low', 'high-value': 'high',
            'cancel-high': 'low', 'cancel-last': 'idle', 'expired-high': 'idle',
            'invalid-value-accepted': '0', 'cancel-owner-all-values': 'idle',
            'failed-retry-ms': '500', 'failed-file-absent': '1',
            'retry-current-value': 'high', 'retry-default': 'idle'}
reference_only = {'held-descriptor': '1', 'released-descriptor': '0'}
seen = {}
for line in source.read_text().splitlines():
    scope, name, value = line.split('\t')
    table = expected if scope == 'shared' else reference_only if scope == 'reference-only' else {}
    if name in seen or table.get(name) != value:
        raise RuntimeError(f'Unexpected original-source outcome: {scope}/{name}/{value}')
    seen[name] = value
if seen != expected | reference_only:
    raise RuntimeError('Original-source reference scenarios incomplete')
fixture = root / 'native-power/tests/fixtures/public-node-reference.tsv'
fixture.write_bytes(source.read_bytes())
print(json.dumps({'shared_outcomes': len(expected), 'reference_only_fd_outcomes': len(reference_only),
                  'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(),
                  'execution': 'unmodified pinned public RequestGroup/Node/FileNode source with our POSIX shims'}))
