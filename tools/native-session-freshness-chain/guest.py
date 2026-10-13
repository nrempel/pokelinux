#!/usr/bin/env python3
"""Install actual static ARM helpers at the paths embedded by Cargo, then test."""
import hashlib
import json
import os
import shutil
import subprocess
import traceback
from pathlib import Path

R = Path('/results')
try:
    selection = json.loads((R / 'selection.json').read_text())
    assert selection['arch'] == 'arm'
    for value in selection['artifacts'].values():
        source = R / value['frozen_path']
        assert hashlib.sha256(source.read_bytes()).hexdigest() == value['sha256']
        target = Path(value['compiled_path'])
        assert target.is_relative_to('/cache')
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        target.chmod(0o755)
    result = subprocess.run(['python3', str(R / 'source/execute.py'), '--results', str(R)], timeout=150)
    (R / 'guest-exit-code').write_text(str(result.returncode) + '\n')
except BaseException:
    (R / 'guest-error.txt').write_text(traceback.format_exc())
    print(traceback.format_exc(), flush=True)
finally:
    os.sync()
    subprocess.run(['systemctl', 'poweroff'], check=False)
