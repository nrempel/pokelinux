#!/usr/bin/env python3
"""Fetch only public Android17 frozen messaging3 and its radio3 imports."""
import base64, hashlib, json, urllib.request
from pathlib import Path
ROOT = Path('artifacts/sms-binder')
SCHEMA = ROOT / 'frozen-aidl'
SCHEMA.mkdir(parents=True, exist_ok=True)
BASE = 'https://android.googlesource.com/platform/hardware/interfaces/+/android-17.0.0_r1/'
rows = []
def fetch(path, binary=False):
    url = BASE + path + ('?format=TEXT' if binary else '?format=JSON')
    data = urllib.request.urlopen(url, timeout=30).read()
    return base64.b64decode(data) if binary else json.loads(data[4:])
for module, package in [('android.hardware.radio.messaging', 'android/hardware/radio/messaging'), ('android.hardware.radio', 'android/hardware/radio')]:
    prefix = 'radio/aidl/aidl_api/' + module + '/3/'
    inventory = fetch(prefix + package + '/')
    for entry in inventory['entries']:
        if entry['type'] != 'blob' or not entry['name'].endswith('.aidl'):
            continue
        path = prefix + package + '/' + entry['name']
        data = fetch(path, True)
        output = SCHEMA / package / entry['name']
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)
        rows.append({'path': str(output.relative_to(ROOT)), 'url': BASE + path, 'sha256': hashlib.sha256(data).hexdigest()})
    digest = fetch(prefix + '.hash', True).decode().strip()
    if module.endswith('messaging'):
        assert digest == '30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb'
    rows.append({'module': module, 'version': 3, 'interface_hash': digest, 'tree_id': inventory['id']})
(ROOT / 'frozen-schema-manifest.json').write_text(json.dumps({'release_tag': 'android-17.0.0_r1', 'sources': rows}, indent=2) + '\n')
print('Public frozen schemas fetched:', sum('path' in r for r in rows))
