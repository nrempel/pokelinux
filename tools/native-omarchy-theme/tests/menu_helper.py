#!/usr/bin/env python3
"""Actual root-owned staging boundary; no helper, service or GUI is executed."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import stat
import sys
import tarfile

sys.dont_write_bytecode = True

parser = argparse.ArgumentParser(description=__doc__)
for name in ['stage', 'archive', 'provisioner', 'output']:
    parser.add_argument('--' + name, required=True, type=Path)
args = parser.parse_args()
if os.geteuid() != 0:
    parser.error('root-owned private staging control requires UID0')
args.output.mkdir(mode=0o700)
result = {'schema': 1, 'passed': False, 'GUI_or_helper_executed': False}
try:
    spec = importlib.util.spec_from_file_location('theme_stage', args.stage)
    stage = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(stage)
    pins = json.loads((stage.HERE / 'assets.json').read_text())
    archive_hash = hashlib.sha256(args.archive.read_bytes()).hexdigest()
    assert archive_hash == pins['archive_sha256'], 'original public archive differs'
    member = 'omarchy-' + pins['commit'] + '/bin/omarchy-shell'
    with tarfile.open(args.archive) as archive:
        info = archive.getmember(member)
        assert info.isfile() and info.size == 2766
        original = archive.extractfile(info).read(2767)
    original_hash = hashlib.sha256(original).hexdigest()
    assert original_hash == '538ece231aba80154c76ab48df8ab013b2f02c5171241d08bddbf08a5fe5ac34'
    result['original_source'] = {'archive_sha256': archive_hash, 'member': member,
                                 'bytes': len(original), 'sha256': original_hash}
    root = args.output / 'root'
    manifest = args.output / 'manifest.json'
    rows = stage.stage(args.archive, args.provisioner, root, manifest)
    result['stage_file_count'] = len(rows)
    helper = root / 'usr/bin/omarchy-shell'
    result['public_helper_exists'] = helper.is_file()
    assert helper.is_file(), 'original menu helper is not exported at /usr/bin/omarchy-shell'
    info = helper.lstat()
    result['actual_public_helper'] = {
        'bytes': info.st_size, 'sha256': hashlib.sha256(helper.read_bytes()).hexdigest(),
        'mode': stat.S_IMODE(info.st_mode), 'uid': info.st_uid, 'gid': info.st_gid,
        'nlink': info.st_nlink, 'regular': stat.S_ISREG(info.st_mode)}
    assert stat.S_ISREG(info.st_mode) and info.st_nlink == 1
    assert info.st_uid == 0 and info.st_gid == 0 and stat.S_IMODE(info.st_mode) == 0o755
    assert helper.read_bytes() == original, 'public helper differs from original selected source'
    expected = {'bytes': len(original), 'sha256': original_hash, 'mode': 0o755}
    document = json.loads(manifest.read_text())
    assert document['files'] == rows and rows['usr/bin/omarchy-shell'] == expected
    assert sorted(p.name for p in (root / 'usr/bin').iterdir()) == ['omarchy-shell']
    assert shutil.which('omarchy-shell', path=str(root / 'usr/bin')) == str(helper)
    result['manifest_matches_original'] = True
    result['resolves_from_public_bin_only'] = True
    result['passed'] = True
except BaseException as error:
    result['error'] = type(error).__name__ + ': ' + str(error)
finally:
    (args.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'passed': result['passed'], 'error': result.get('error')}))
raise SystemExit(0 if result['passed'] else 1)
