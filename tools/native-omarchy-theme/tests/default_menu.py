#!/usr/bin/env python3
"""Actual original-menu file staging; no menu action or GUI is executed."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import tarfile

sys.dont_write_bytecode = True
parser = argparse.ArgumentParser(description=__doc__)
for name in ['stage', 'archive', 'provisioner', 'output']:
    parser.add_argument('--' + name, required=True, type=Path)
args = parser.parse_args()
if os.geteuid() != 0:
    parser.error('private installation control requires UID0')
args.output.mkdir(mode=0o700)
result = {'passed': False, 'menu_action_or_GUI_executed': False}
try:
    spec = importlib.util.spec_from_file_location('theme_stage', args.stage)
    stage = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(stage)
    root, manifest = args.output / 'root', args.output / 'manifest.json'
    rows = stage.stage(args.archive, args.provisioner, root, manifest)
    assert rows == json.loads(manifest.read_text())['files']
    result['stage_file_count'] = len(rows)
    source = 'default/omarchy/omarchy-menu.jsonc'
    target = 'usr/share/pokelinux/omarchy/' + source
    digest = '8c791cfc457d6c50b90a6636a6f28c9415a2fa6e4edf84826906457b75649689'
    with tarfile.open(args.archive) as archive:
        member = archive.getmember('omarchy-c668141e9c42b13c80c9ca4ea108e11708c5e8a5/' + source)
        assert member.isfile() and member.size == 52910
        original = archive.extractfile(member).read(52911)
    assert len(original) == 52910 and hashlib.sha256(original).hexdigest() == digest
    path = root / target
    result['original_menu_exists'] = path.is_file()
    assert path.is_file(), 'original default menu is not installed: ' + target
    info = path.lstat()
    assert stat.S_ISREG(info.st_mode) and info.st_nlink == 1
    assert info.st_uid == info.st_gid == 0 and stat.S_IMODE(info.st_mode) == 0o644
    assert path.read_bytes() == original
    expected = {'bytes': len(original), 'sha256': digest, 'mode': 0o644}
    assert rows[target] == expected
    result.update(passed=True, installed_original_menu={
        'path': target, **expected, 'uid': info.st_uid, 'gid': info.st_gid,
        'nlink': info.st_nlink})
except BaseException as error:
    result['error'] = type(error).__name__ + ': ' + str(error)
finally:
    (args.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
raise SystemExit(0 if result['passed'] else 1)
