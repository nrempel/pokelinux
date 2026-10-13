#!/usr/bin/env python3
"""Verified own pre-correction source snapshot for runtime red only."""
from pathlib import Path
import hashlib,json,shutil,subprocess,tempfile
root=Path(__file__).resolve().parents[1]
m=json.loads((root/'native-theme/worker-tests/baseline-manifest.json').read_text())
out=root/'artifacts/native-theme-worker/baseline'
def check(path):
 for name,digest in m['files'].items():assert hashlib.sha256((path/name).read_bytes()).hexdigest()==digest,name
if out.exists():check(out)
else:
 out.parent.mkdir(parents=True,exist_ok=True)
 with tempfile.TemporaryDirectory(dir=out.parent,prefix='reader-baseline-') as temporary:
  stage=Path(temporary)
  for name,digest in m['post_correction_sources'].items():
   data=(root/'native-app'/name).read_bytes();assert hashlib.sha256(data).hexdigest()==digest,name
   (stage/name).write_bytes(data)
  subprocess.run(['patch','--batch','-p1','-i',str(root/'native-theme/worker-tests/pre-correction.patch')],cwd=stage,check=True)
  check(stage)
  (stage/'manifest.json').write_text(json.dumps({'files':m['files'],'scope':m['scope']},indent=2)+'\n')
  stage.rename(out)
print('Verified pre-correction consumer snapshot; test execution still uses actual Qt and fc-match.')
