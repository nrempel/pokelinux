#!/usr/bin/env python3
"""Install explicit already-built helpers inside a disposable root fixture only."""
import argparse
import json
import os
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--daemon', type=Path, required=True)
parser.add_argument('--launcher', type=Path, required=True)
parser.add_argument('--install', type=Path, default=Path('/opt/service-restoration'))
args = parser.parse_args()
assert os.getuid() == os.geteuid() == 0
assert os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1'
assert args.install.is_absolute() and args.install.parent == Path('/opt')
args.install.mkdir(mode=0o755)
for source, name in [(args.daemon, 'lifecycle-daemon'), (args.launcher, 'lifecycle-launcher'),
                     (Path(__file__).with_name('app.py'), 'app.py')]:
    target = args.install / name
    shutil.copyfile(source.resolve(strict=True), target)
    target.chmod(0o755 if name != 'app.py' else 0o644)
    assert target.stat().st_uid == 0
python = Path(shutil.which('python3')).resolve(strict=True)
(args.install / 'python-path').write_text(str(python) + '\n')
(args.install / 'python-path').chmod(0o644)
print(json.dumps({'install': str(args.install), 'python': str(python), 'built': False,
                  'executed_fixture': False}))
