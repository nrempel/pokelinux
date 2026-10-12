#!/usr/bin/env python3
"""Explicit Qt 6.12 name-resolution patch for the pinned Omarchy fixture."""
import difflib,hashlib,json,re
from pathlib import Path

def changes(root, manifest):
    result=[]
    for name,digest in sorted(manifest['files'].items()):
        if not name.endswith('.qml'):continue
        data=(root/name).read_bytes()
        assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==digest,name
        before=data.decode()
        if not re.search(r'\bColor\.',before):continue
        after=re.sub(r'\bColor\.', 'Commons.Color.', before)
        after=re.sub(r'^import ', 'import qs.Commons as Commons\nimport ', after, count=1, flags=re.M)
        result.append((name,before,after))
    return result

def apply(root, manifest):
    edits=changes(root,manifest)  # Verify every input before writing any replacement.
    for name,before,after in edits:(root/name).write_text(after)
    return {name:hashlib.sha256(after.encode()).hexdigest() for name,before,after in edits}

if __name__=='__main__':
    project=Path(__file__).resolve().parents[1]
    root=project/'sim/omarchy-source'
    manifest=json.loads((root/'source-manifest.json').read_text())
    edits=changes(root,manifest)
    patch=''.join(''.join(difflib.unified_diff(before.splitlines(keepends=True),after.splitlines(keepends=True),fromfile='a/'+name,tofile='b/'+name)) for name,before,after in edits)
    (project/'sim/omarchy-qt612.patch').write_text(patch)
    print('Generated namespace-only patch for',len(edits),'QML files; upstream source unchanged.')
