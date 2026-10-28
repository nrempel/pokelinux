#!/usr/bin/env python3
"""Actual native-helper preservation/refusal controls; only genuine generated files."""
import argparse
import configparser
import hashlib
import json
import os
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from owned import Scope, now

parser = argparse.ArgumentParser()
for name in ['provisioner', 'assets', 'deployment', 'output']:
    parser.add_argument('--' + name, required=True)
args = parser.parse_args()
out = Path(args.output)
out.mkdir()
scope = Scope(out)
work = now() + 60_000_000_000
close = work + 5_000_000_000
result = {'passed': False, 'PID1_failure_propagation_executed': False}

def fingerprint(root):
    return {str(p.relative_to(root)): {'sha256': hashlib.sha256(p.read_bytes()).hexdigest(),
            'inode': p.stat().st_ino, 'uid': p.stat().st_uid, 'mode': p.stat().st_mode,
            'mtime_ns': p.stat().st_mtime_ns}
            for p in root.rglob('*') if p.is_file()}

def invoke(label, expected):
    code, _ = scope.run([args.provisioner, str(config), args.assets], label, min(work, now() + 20_000_000_000), limit=1024**2)
    assert (code == 0) == expected, (label, code)
    return code

try:
    # Validate actual reusable staged dependencies, not another fixture unit.
    units = Path(args.deployment) / 'usr/lib/systemd/system'
    for dependent in ['pokelinux-normal-session.target', 'pokelinux-session-owner.service',
                      'pokelinux-normal-appearance.service', 'pokelinux-omarchy-shell.service']:
        body = configparser.ConfigParser(interpolation=None)
        body.read(units / (dependent + '.d/theme.conf'))
        assert body['Unit']['Requires'].split() == ['pokelinux-omarchy-theme.service']
        assert body['Unit']['After'].split() == ['pokelinux-omarchy-theme.service']
    body = configparser.ConfigParser(interpolation=None)
    body.read(units / 'pokelinux-omarchy-theme.service')
    assert body['Service']['Type'] == 'oneshot' and body['Service']['Restart'] == 'no'
    assert body['Service']['TimeoutStartSec'] == '30s' and body['Service']['TimeoutStopSec'] == '5s'
    result['normal_staged_dependency_declarations_validated'] = True
    base = Path('/run/theme-selection')
    base.mkdir(mode=0o755)
    home, runtime = base / 'home', base / 'runtime'
    for directory in [home, runtime]:
        directory.mkdir(mode=0o700)
        os.chown(directory, 1000, 1000)
    config = base / 'appearance.json'
    config.write_text(json.dumps({'home': str(home), 'runtime': str(runtime)}))
    config.chmod(0o644)
    invoke('initial-original-setter.log', True)
    current = home / '.local/state/omarchy/current'
    assert (current / 'theme/colors.toml').is_file() and (current / 'theme/shell.toml').is_file()
    # A user retains their selection marker; no palette is created by this test.
    marker = current / 'theme.name'
    marker.write_text('Retained user selection\n')
    before = fingerprint(home)
    invoke('preserve-complete-selection.log', True)
    assert fingerprint(home) == before
    result['complete_existing_selection_preserved_byte_inode_mode_time'] = True
    # Nonempty theme.name with missing mandatory source must be refused untouched.
    (current / 'theme/colors.toml').unlink()
    before = fingerprint(home)
    result['partial_selection_actual_exit'] = invoke('refuse-partial-selection.log', False)
    assert marker.read_text() == 'Retained user selection\n' and fingerprint(home) == before
    assert not (current / 'theme/colors.toml').exists()
    result['partial_existing_selection_refused_untouched'] = True
    result['passed'] = True
except BaseException as error:
    result.update(passed=False, error=type(error).__name__ + ': ' + str(error))
finally:
    result['cleanup_errors'] = scope.close(close)
    result['receipts'] = scope.receipts
    result['passed'] = bool(result['passed'] and not result['cleanup_errors'] and all(
        v['actual_direct_parent_wait'] and v['stdout_EOF'] and v['wait_before_original_cutoff']
        and v['EOF_before_original_cutoff'] for v in scope.receipts))
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'passed': result['passed'], 'error': result.get('error')}))
raise SystemExit(0 if result['passed'] else 1)
