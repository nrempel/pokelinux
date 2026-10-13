#!/usr/bin/env python3
"""Create a verified readonly source snapshot for the genuine pre-font runtime red."""
from pathlib import Path
import hashlib,json,shutil,subprocess,tempfile
root=Path(__file__).resolve().parents[1]
manifest=json.loads((root/'native-theme/baseline-manifest.json').read_text())
target=root/'artifacts/native-theme/baseline'
def verify(path):
    for name,digest in manifest['files'].items():
        assert hashlib.sha256((path/name).read_bytes()).hexdigest()==digest,name
if target.exists():
    verify(target)
    assert json.loads((target/'manifest.json').read_text())==manifest
else:
    target.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(dir=target.parent,prefix='baseline-build-') as temporary:
        stage=Path(temporary)
        shutil.copytree(root/'native-app',stage/'native-app')
        for name in ['PhoneThemeFiles.cpp','PhoneThemeFiles.h']:
            data=subprocess.check_output(['git','show',manifest['consumer_source_ref']+':native-app/'+name],cwd=root)
            (stage/'native-app'/name).write_bytes(data)
        verify(stage)
        (stage/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
        stage.rename(target)
print('Readonly pre-font consumer snapshot verified:',manifest['consumer_source_ref'])
