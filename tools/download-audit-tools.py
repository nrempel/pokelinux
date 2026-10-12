#!/usr/bin/env python3
"""Fetch pinned Google resource tools; runs only during audit preparation."""
import hashlib,io,json,urllib.request,zipfile
from pathlib import Path
url='https://dl.google.com/android/repository/build-tools_r37_linux.zip'
expected='01af179347cbcd9c208b7f8171f7b21f6dd1d2f85bcd15e88caa51d5d7b86060'
root=Path('audit');root.mkdir(exist_ok=True);archive=root/'build-tools_r37_linux.zip'
if not archive.exists():
 with urllib.request.urlopen(url,timeout=60) as response: data=response.read()
 if hashlib.sha256(data).hexdigest()!=expected:raise SystemExit('SDK archive hash mismatch')
 archive.write_bytes(data)
data=archive.read_bytes()
if hashlib.sha256(data).hexdigest()!=expected:raise SystemExit('SDK archive hash mismatch')
with zipfile.ZipFile(io.BytesIO(data)) as z:
 for name in z.namelist():
  if name.endswith(('/aapt2','/dexdump')):
   out=root/'sdk-tools'/Path(name).name;out.parent.mkdir(exist_ok=True);out.write_bytes(z.read(name));out.chmod(0o755)
(root/'sdk-tools/provenance.json').write_text(json.dumps({'url':url,'sha256':expected,'repository_sha1':hashlib.sha1(data).hexdigest()},indent=2)+'\n')
print('Verified Android build-tools 37')
