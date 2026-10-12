#!/usr/bin/env python3
"""Stage the pinned, verified source paths exactly, including Linux case."""
import hashlib,io,json,tarfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
source=root/'sim/omarchy-source'
manifest=json.loads((source/'source-manifest.json').read_text())
with tarfile.open(root/'sim/omarchy-source.tar','w') as archive:
    for name,expected in sorted(manifest['files'].items()):
        data=(source/name).read_bytes()
        actual=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
        assert actual==expected,name
        info=tarfile.TarInfo(name);info.size=len(data);info.mode=int(manifest['modes'][name],8) & 0o777
        archive.addfile(info,io.BytesIO(data))
print('Staged',len(manifest['files']),'verified files at',manifest['commit'])
