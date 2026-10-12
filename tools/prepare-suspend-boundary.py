#!/usr/bin/env python3
"""Fetch only pinned public suspend closure into ignored artifacts."""
import argparse,base64,hashlib,json
from pathlib import Path
import urllib.request
root=Path(__file__).resolve().parents[1]
manifest=json.loads((root/'framework/suspendpeer/fetch-manifest.json').read_text())
parser=argparse.ArgumentParser();parser.add_argument('--verify-only',action='store_true');args=parser.parse_args()
for item in manifest['files']:
 path=root/'artifacts/suspend-boundary'/item['path']
 if not path.is_file() and not args.verify_only:
  with urllib.request.urlopen(item['url'],timeout=30) as response:data=base64.b64decode(response.read(2*1024*1024),validate=True)
  assert len(data)==item['bytes'] and hashlib.sha256(data).hexdigest()==item['sha256']
  path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
 data=path.read_bytes();assert len(data)==item['bytes'] and hashlib.sha256(data).hexdigest()==item['sha256'],path
for item in manifest['reused_public_files']:
 path=root/item['path'];data=path.read_bytes();assert len(data)==item['bytes'] and hashlib.sha256(data).hexdigest()==item['sha256'],path
print(json.dumps({'verified_public_inputs':len(manifest['files']),'frozen_version':manifest['version'],'frozen_hash':manifest['hash']}))
