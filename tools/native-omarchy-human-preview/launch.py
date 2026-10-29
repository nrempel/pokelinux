#!/usr/bin/env python3
"""Create and optionally run one disposable preview from Harbour's qualified seed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat

BASE_SHA = '7e139e5d7bf814ffe3f0469630b8f166885ab45741112923555c26f7ea4f16b7'
COMMAND_SHA = '1357d611542da896f08c1fe7dbe632ca9167df182bed1da87ea9f17ee7116cd2'
FLOOR = 2 * 1024**3
BUILD_VOLUME = Path('/tmp/pokelinux-native-callback-span-build-tmpfs-root-1')


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_json(path):
    assert path.stat().st_size <= 8 * 1024**2
    return json.loads(path.read_text())


def relative(name):
    path = Path(name)
    assert path.parts and not path.is_absolute() and '..' not in path.parts
    return path


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def capacity(required=0):
    assert shutil.disk_usage('/tmp').free >= FLOOR + required
    assert shutil.disk_usage(BUILD_VOLUME).free >= FLOOR, 'build-volume free-space floor'


def runtime_capacity():
    capacity()
    fields = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    value, unit = fields['MemAvailable'].split()
    assert unit == 'kB' and int(value) * 1024 >= 12 * 1024**3, '12GiB available memory required'


def prepare(base, name):
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,31}', name), 'invalid case name'
    assert os.getuid() == os.getgid() == 1000, 'prepare as Harbour ubuntu, not root'
    assert digest(base / 'selection.json') == BASE_SHA, 'unqualified seed selection'
    assert digest(base / 'command.json') == COMMAND_SHA, 'unqualified seed command'
    selection = read_json(base / 'selection.json')
    command = read_json(base / 'command.json')
    target = Path('/tmp') / ('pokelinux-native-x86-composition-' + name)
    assert len(str(target / 'viewer.sock').encode()) < 108
    assert not target.exists() and not target.is_symlink(), 'output already exists; never reused'
    required = sum((base / relative(n)).stat().st_size for n in selection['files'])
    capacity(required + 16 * 1024**2)
    for name_in, expected in selection['files'].items():
        source = base / relative(name_in)
        assert stat.S_ISREG(source.lstat().st_mode) and digest(source) == expected
    target.mkdir(mode=0o700)
    # Copy selected inputs only, never runtime outputs, sockets or private captures.
    for name_in in selection['files']:
        source = base / relative(name_in)
        output = target / name_in
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, output)
        output.chmod(stat.S_IMODE(source.stat().st_mode))
        assert output.stat().st_nlink == 1
        assert digest(output) == selection['files'][name_in], 'seed changed during copy'
    manifest_path = target / 'deployment/selected-profile.json'
    manifest = read_json(manifest_path)
    for name_in, row in manifest['files'].items():
        row['path'] = str(target / 'deployment/root' / relative(name_in))
    write_json(manifest_path, manifest)
    selection['cold_installed_bundle']['files'] = manifest['files']
    selection['cold_installed_bundle']['manifest_sha256'] = digest(manifest_path)
    selection['label'] = name
    selection['human_preview']['host_case_dir'] = str(target)
    selection['scope'] = 'Disposable normal-entry Harbour preview; new session unexecuted, no human or phone qualification inherited'
    selection['launcher_seed'] = {'selection_sha256': BASE_SHA, 'command_sha256': COMMAND_SHA}
    selection['launch_performed'] = False
    # Print readiness from the existing owned runner loop; no monitor child or detached worker.
    runner = target / 'source/run-selected.py'
    source = runner.read_text()
    needle = "    if live:cpu_capture('existing-host-loop')\n"
    assert source.count(needle) == 1
    replacement = """    if live:
     cpu_capture('existing-host-loop')
     ready_path=R/'viewer-input/human-ready.json'
     if 'viewer_ready_notification'not in record and ready_path.is_file():
      ready=load(ready_path,16384)
      assert ready['selection_sha256']==x.selection_sha256 and ready['host_socket_path']==str(R/'viewer.sock')
      if now()<ready['host_no_client_until_ns']and (R/'viewer.sock').is_socket():
       record['viewer_ready_notification']={'observed_host_ns':now(),'socket':str(R/'viewer.sock')}
       print(json.dumps({'viewer_ready':True,'socket':str(R/'viewer.sock'),'first_connection_until_host_ns':ready['host_no_client_until_ns'],'maximum_dwell_until_host_ns':ready['host_dwell_until_ns'],'human_connection_qualified':False}),flush=True)
"""
    mode = stat.S_IMODE(runner.stat().st_mode)
    runner.chmod(mode | 0o200)
    runner.write_text(source.replace(needle, replacement))
    runner.chmod(mode)
    selection['files'] = {n: digest(target / n) for n in selection['files']}
    write_json(target / 'selection.json', selection)
    command[command.index('--name') + 1] = 'pokelinux-native-x86-composition-' + name
    command[command.index('--cidfile') + 1] = str(target / 'owned-container-id')
    results = [i + 1 for i, value in enumerate(command[:-1]) if value == '-v' and command[i + 1].endswith(':/results:rw')]
    assert len(results) == 1
    command[results[0]] = str(target) + ':/results:rw'
    write_json(target / 'command.json', command)
    capacity()
    argv = ['sudo', '-n', '/usr/bin/python3', '-B', str(runner), '--selection-dir', str(target), '--selection-sha256', digest(target / 'selection.json'), '--command-sha256', digest(target / 'command.json')]
    write_json(target / 'launch-command.json', argv)
    return target, argv


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', type=Path, required=True, help='retained qualified Harbour seed directory')
    parser.add_argument('--name', required=True, help='unique lowercase case name, up to32 characters')
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    if not args.prepare_only:
        runtime_capacity()
    target, argv = prepare(args.base.resolve(strict=True), args.name)
    print(json.dumps({'prepared': str(target), 'will_run': not args.prepare_only, 'no_phone_access': True, 'connection': 'Wait for viewer_ready, then forward the reported Unix socket through your existing SSH login to a loopback VNC port.'}), flush=True)
    if not args.prepare_only:
        # Replace this process: the existing runner retains all subprocess/container custody.
        runtime_capacity()
        os.execvp(argv[0], argv)


if __name__ == '__main__':
    main()
