#!/usr/bin/env python3
"""Exercise original-font file staging; discovery and rendering are separate."""
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
result = {'passed': False, 'font_discovery_or_rendering_executed': False}
resources = {
    'usr/share/fonts/omarchy/omarchy.ttf': (
        'default/fonts/omarchy/omarchy.ttf',
        'de860dce13dda83739a32eea787c422cb81e8a30f1b0bdc2cfc8829ee944e0ff'),
    'usr/share/doc/pokelinux/omarchy-font/README.md': (
        'default/fonts/omarchy/README.md',
        '300c2fb653194b3cd39e5045584e3dd1e624e7d351fb68a0eb45643f9ed34ee3'),
    'usr/share/doc/pokelinux/omarchy-font/LICENSE': (
        'LICENSE',
        '717ba1949502290f8e47688ae2e323acd06c8ca47aec9f7596b15f678c1af4a2'),
}
try:
    spec = importlib.util.spec_from_file_location('theme_stage', args.stage)
    stage = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(stage)
    root, manifest = args.output / 'root', args.output / 'manifest.json'
    rows = stage.stage(args.archive, args.provisioner, root, manifest)
    assert rows == json.loads(manifest.read_text())['files']
    result['stage_file_count'] = len(rows)
    observed = {}
    with tarfile.open(args.archive) as archive:
        for target, (source, digest) in resources.items():
            member = archive.getmember('omarchy-c668141e9c42b13c80c9ca4ea108e11708c5e8a5/' + source)
            assert member.isfile()
            original = archive.extractfile(member).read()
            assert hashlib.sha256(original).hexdigest() == digest
            path = root / target
            assert path.is_file(), 'original font resource is not installed: ' + target
            info = path.lstat()
            assert stat.S_ISREG(info.st_mode) and info.st_nlink == 1
            assert info.st_uid == info.st_gid == 0 and stat.S_IMODE(info.st_mode) == 0o644
            assert path.read_bytes() == original
            expected = {'bytes': len(original), 'sha256': digest, 'mode': 0o644}
            assert rows[target] == expected
            observed[target] = expected
    result.update(passed=True, installed_original_resources=observed)
except BaseException as error:
    result['error'] = type(error).__name__ + ': ' + str(error)
finally:
    (args.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
raise SystemExit(0 if result['passed'] else 1)
