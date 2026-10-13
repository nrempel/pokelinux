#!/usr/bin/env python3
"""Freeze one new installed-lock GREEN using the retained original input inventory."""
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
    p.add_argument('--original-selection', required=True, type=Path)
    p.add_argument('--build', required=True, type=Path)
    p.add_argument('--tracer', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args()
    assert not a.output.exists()
    original = json.loads((a.original_selection / 'selection.json').read_text())
    result = json.loads((a.original_selection / 'result.json').read_text())
    assert original['phase'] == 'original-lock-entry-red'
    assert result['expected_red_qualified'] and result['desired_boundary_red']
    assert result['all_tracked_processes_waited_or_finally_absent']
    for name, expected in original['files'].items():
        assert sha(a.original_selection / name) == expected, name
    build = json.loads((a.build / 'selection.json').read_text())
    assert build['phase'] == 'green' and build['build_returncode'] == 0 and build['build_cli_actually_waited']
    for name, expected in build['sources'].items():
        assert sha(a.build / 'source' / name) == expected
        assert sha(ROOT / name) == expected, 'current source differs from frozen build: ' + name
    for name, expected in build['bins'].items():
        assert sha(a.build / name) == expected
    a.output.mkdir(mode=0o700)
    source, bins = a.output / 'source', a.output / 'bins'
    shutil.copytree(a.original_selection / 'source', source)
    shutil.copytree(a.original_selection / 'bins', bins)
    for tree in [source, bins]:
        for path in tree.rglob('*'):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
    shutil.rmtree(source / 'rust-build')
    shutil.copytree(a.build / 'source', source / 'rust-build')
    shutil.copyfile(a.build / 'selection.json', source / 'rust-build-selection.json')
    for name in ['guest.py', 'boundary.py', 'realm.py', 'install.py', 'plan.py', 'run-selected.py']:
        shutil.copyfile(HERE / name, source / name)
    for path in (a.build / 'arm-bins').glob('*'):
        if path.suffix != '.d':
            shutil.copyfile(path, bins / path.name)
    tracer = json.loads((a.tracer / 'proof.json').read_text())
    assert tracer['signature_verified'] and tracer['ELF_machine'] == 'AArch64'
    assert sha(a.tracer / 'usr/bin/strace') == tracer['ELF_sha256']
    shutil.copyfile(a.tracer / 'usr/bin/strace', bins / 'strace')
    shutil.copyfile(a.tracer / 'proof.json', source / 'strace-public-package-proof.json')
    selected = {**original, 'phase': 'original-lock-entry-green',
        'original_RED_selection_sha256': sha(a.original_selection / 'selection.json'),
        'original_RED_result_sha256': sha(a.original_selection / 'result.json'),
        'original_RED_fixture_preserved': True,
        'native_build_selection_sha256': sha(a.build / 'selection.json'),
        'current_core_source_sha256': build['sources'],
        'service_elf_sha256': sha(bins / 'messages-app-context-fixture'),
        'native_lock_client_sha256': sha(bins / 'lifecycle-lock'),
        'original_action_sha256': 'f1953110658d819597d3c9d8df616350c362ee4c257227fcbdefac8275bbd596',
        'fixed_installed_lock_socket': '/run/pokelinux/messages/lock.sock',
        'fixed_retained_original_action': '/usr/libexec/pokelinux/omarchy-system-lock-original',
        'native_client_entry_budget_ns': 2_000_000_000,
        'actor_wire_trace': 'owned-parent strace --seccomp-bpf recvmsg/sendto, real authenticated app/source requests; no competing reader or injected requests/grants',
        'strace_dependency': tracer,
        'PAM_authentication_executed': False, 'secure_presentation_qualified': False,
        'label': a.output.name,
        'files': {str(q.relative_to(a.output)): sha(q) for tree in [source, bins]
                  for q in sorted(tree.rglob('*')) if q.is_file()}}
    command = json.loads((a.original_selection / 'command.json').read_text())
    for index, part in enumerate(command):
        if part == str(a.original_selection.resolve()) + ':/results:rw':
            command[index] = str(a.output.resolve()) + ':/results:rw'
        elif part.startswith('pokelinux-original-lock-'):
            command[index] = 'pokelinux-native-lock-' + a.output.name
    assert str(a.output.resolve()) + ':/results:rw' in command
    for path in bins.iterdir():
        path.chmod(0o555)
    for path in source.rglob('*'):
        if path.is_file():
            path.chmod(path.stat().st_mode & ~0o222)
    (a.output / 'selection.json').write_text(json.dumps(selected, indent=2) + '\n')
    (a.output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    print(json.dumps({'selection_sha256': sha(a.output / 'selection.json'), 'command_sha256': sha(a.output / 'command.json'),
                      'runtime_started': False, 'selected_inputs': len(selected['files'])}, indent=2))


if __name__ == '__main__':
    main()
