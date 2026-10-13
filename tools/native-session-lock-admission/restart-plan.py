#!/usr/bin/env python3
"""Freeze one actual restart boundary with retained original input pins."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--original', required=True, type=Path)
    p.add_argument('--build', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--phase', choices=['red', 'green'], required=True)
    a = p.parse_args()
    assert not a.output.exists()
    original = json.loads((a.original / 'selection.json').read_text())
    for name, expected in original['files'].items():
        assert sha(a.original / name) == expected, name
    build = json.loads((a.build / 'selection.json').read_text())
    assert build['build_returncode'] == 0 and build['build_cli_actually_waited']
    for name, expected in build['sources'].items():
        assert sha(a.build / 'source' / name) == expected
        assert sha(ROOT / name) == expected
    for name, expected in build['bins'].items():
        assert sha(a.build / name) == expected
    a.output.mkdir(mode=0o700)
    source, bins = a.output / 'source', a.output / 'bins'
    shutil.copytree(a.original / 'source', source)
    shutil.copytree(a.original / 'bins', bins)
    for tree in [source, bins]:
        for path in tree.rglob('*'):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
    shutil.rmtree(source / 'rust-build')
    shutil.copytree(a.build / 'source', source / 'rust-build')
    shutil.copyfile(a.build / 'selection.json', source / 'rust-build-selection.json')
    for name in ['guest.py', 'realm.py', 'restart_boundary.py', 'restart-plan.py', 'restart-run.py']:
        shutil.copyfile(HERE / name, source / name)
    for path in (a.build / 'arm-bins').glob('*'):
        if path.suffix != '.d':
            shutil.copyfile(path, bins / path.name)
    boot = (source / 'boot.sh').read_text()
    boot = boot.replace('exec timeout', 'qemu-img create -f qcow2 -F raw -b /vm/root.img /tmp/lock-restart-overlay.qcow2 > /results/overlay-create.log\ntimeout')
    boot = boot.replace('file=/vm/root.img,if=none,id=root,format=raw,snapshot=on',
                        'file=/tmp/lock-restart-overlay.qcow2,if=none,id=root,format=qcow2')
    boot += '\nqemu-img info --output=json /tmp/lock-restart-overlay.qcow2 > /results/overlay.json\n'
    boot += 'test "$(stat -c %s /tmp/lock-restart-overlay.qcow2)" -le 268435456\n'
    (source / 'boot.sh').write_text(boot)
    selection = {**original, 'phase': 'original-lock-restart-' + a.phase,
        'restart_boundary': True, 'restart_kind': 'healthy actor Stop/actual wait; changed original compositor/display/Source epoch',
        'same_root_policy_directory': True, 'root_policy_reset': False,
        'original_locker_T_held_across_restart': True, 'Source_continuation_qualified': False,
        'native_build_selection_sha256': sha(a.build / 'selection.json'),
        'current_core_source_sha256': build['sources'],
        'explicit_overlay': 'private qcow2 over readonly /vm/root.img; final bytes <=256MiB',
        'label': a.output.name, 'files': {str(q.relative_to(a.output)): sha(q)
            for tree in [source, bins] for q in sorted(tree.rglob('*')) if q.is_file()}}
    command = json.loads((a.original / 'command.json').read_text())
    for i, part in enumerate(command):
        if part == str(a.original.resolve()) + ':/results:rw':
            command[i] = str(a.output.resolve()) + ':/results:rw'
        elif part.startswith('pokelinux-native-lock-'):
            command[i] = 'pokelinux-native-lock-' + a.output.name
    assert str(a.output.resolve()) + ':/results:rw' in command
    for path in bins.iterdir():
        path.chmod(0o555)
    for path in source.rglob('*'):
        if path.is_file():
            path.chmod(path.stat().st_mode & ~0o222)
    (a.output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
    (a.output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    print(json.dumps({'selection_sha256': sha(a.output / 'selection.json'), 'command_sha256': sha(a.output / 'command.json'),
        'runtime_started': False, 'selected_inputs': len(selection['files'])}, indent=2))


if __name__ == '__main__':
    main()
