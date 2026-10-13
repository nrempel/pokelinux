#!/usr/bin/env python3
"""Fetch the pinned public archive into ignored storage; publish only its manifest."""
import hashlib,json,tarfile,urllib.request
from pathlib import Path
project=Path(__file__).resolve().parents[1]
manifest=json.loads((project/'native-theme/public-manifest.json').read_text())
out=project/'artifacts/native-theme';out.mkdir(parents=True,exist_ok=True)
archive=out/'omarchy-public.tar.gz'
if not archive.exists():
    temporary=out/'omarchy-download.partial'
    with urllib.request.urlopen(manifest['archive_url'],timeout=90) as remote, temporary.open('wb') as local:
        while chunk:=remote.read(1024*1024):local.write(chunk)
    if hashlib.sha256(temporary.read_bytes()).hexdigest()!=manifest['archive_sha256']:
        temporary.unlink();raise SystemExit('Public archive hash mismatch')
    temporary.replace(archive)
assert hashlib.sha256(archive.read_bytes()).hexdigest()==manifest['archive_sha256']
with tarfile.open(archive) as source:
    prefix='omarchy-'+manifest['commit']+'/'
    for name,digest in manifest['files'].items():
        member=source.getmember(prefix+name)
        assert member.isfile() and hashlib.sha256(source.extractfile(member).read()).hexdigest()==digest,name
print('Pinned original public archive verified:',manifest['commit'])
