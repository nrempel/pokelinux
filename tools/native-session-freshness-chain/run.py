#!/usr/bin/env python3
"""Separate immutable host/ARM epochs; the desired RED remains an actual failure."""
import argparse
import fcntl
import json
import os
import shutil
import subprocess
from pathlib import Path
from common import BASE, DOCKER, MAIN, VM_IMAGE, hashes, image_id, label, sha, verify, write_json

EXPECTED_RED = {
    'timely_actual_queued_heartbeat_is_drained_before_cached_expiry',
    'timely_actual_queued_lock_cycle_preserves_every_semantic_revision',
    'exactly_contiguous_actual_wire_leases_do_not_extend_old_deadline',
    'backwards_intermediate_actual_sample_cannot_hide_in_fresh_final_frame',
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--label', type=label, required=True)
    parser.add_argument('--build', type=label, required=True)
    parser.add_argument('--arch', choices=['host', 'arm'], required=True)
    parser.add_argument('--phase', choices=['red', 'green'], required=True)
    parser.add_argument('--test', default=None, help='Optional exact libtest name; default full process suite')
    args = parser.parse_args()
    BASE.mkdir(parents=True, exist_ok=True)
    with (BASE / (args.label + '.lock')).open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        out = BASE / args.label
        out.mkdir()
        out.chmod(0o770)  # Isolated Root runs with the owning host GID; no DAC capability.
        build = BASE / args.build
        bs = json.loads((build / 'selection.json').read_text())
        assert bs['phase'] == args.phase
        verify(build / 'source', bs['source_checksums'])
        source = out / 'source'
        source.mkdir()
        for name in ('run.py', 'common.py', 'execute.py', 'guest.py', 'boot.sh'):
            shutil.copyfile(Path(__file__).with_name(name), source / name)
        shutil.copytree(build / 'source', source / 'rust-build')
        shutil.copyfile(build / 'selection.json', source / 'rust-build-selection.json')
        artifacts = bs['artifacts'][args.arch]
        for value in artifacts.values():
            assert sha(build / value['frozen_path']) == value['sha256']
            target = out / value['frozen_path']
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(build / value['frozen_path'], target)
            target.chmod(0o755)
        selection = {
            'phase': args.phase, 'arch': args.arch, 'test': args.test, 'rust_build': args.build,
            'rust_selection_sha256': sha(build / 'selection.json'), 'artifacts': artifacts,
            'scope': 'actual Root process sockets, SCM credentials/pidfds and SourceObserver freshness chains with authored synthetic peer; no physical phone or compositor epoch',
            'network': 'none', 'runtime_uid': 0, 'image_id': image_id(VM_IMAGE),
            'files': hashes(out), 'container_cpus': 6 if args.arch == 'arm' else 2,
            'container_memory': '8g' if args.arch == 'arm' else '2g',
        }
        shared = None
        try:
            if args.arch == 'arm':
                vm = MAIN / 'artifacts/native-session-installed-app/vm-production'
                shared = (MAIN / 'artifacts/native-session-installed-app/vm-production.lock').open('a')
                fcntl.flock(shared, fcntl.LOCK_SH | fcntl.LOCK_NB)
                selection.update({'backing_sha256': sha(vm / 'root.img'),
                    'kernel_sha256': sha(MAIN / 'sim/boot/Image'), 'initrd_sha256': sha(MAIN / 'sim/boot/initramfs-linux.img'),
                    'readonly_backing_snapshot': True, 'input_lock': 'shared; image preparers require exclusive', 'guest_vcpus': 4})
                command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-freshness-arm-' + args.label,
                    '--network', 'none', '--cpus', '6', '--memory', '8g', '--pids-limit', '256', '--cap-drop', 'ALL',
                    '--security-opt', 'no-new-privileges', '--user', f'{os.getuid()}:{os.getgid()}',
                    '-v', str(MAIN) + ':/workspace:ro', '-v', str(vm) + ':/vm:ro', '-v', str(out) + ':/results:rw',
                    selection['image_id'], 'bash', '/results/source/boot.sh']
            else:
                command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-freshness-host-' + args.label,
                    '--network', 'none', '--cpus', '2', '--memory', '2g', '--pids-limit', '256', '--cap-drop', 'ALL',
                    '--security-opt', 'no-new-privileges', '--user', f'0:{os.getgid()}',
                    '-v', str(out) + ':/results:rw', '-v', str(out / 'cache-tree/host') + ':/cache:ro',
                    selection['image_id'], 'python3', '/results/source/execute.py', '--results', '/results']
            write_json(out / 'selection.json', selection)
            write_json(out / 'command.json', command)
            with (out / 'serial.log').open('w') as log:
                run = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
            (out / 'exit-code').write_text(str(run.returncode) + '\n')
        finally:
            if shared:
                shared.close()
        if not (out / 'result.json').is_file():
            print((out / 'serial.log').read_text()[-6000:])
            raise RuntimeError('Runtime bootstrap did not produce a completed actual-test result')
        result = json.loads((out / 'result.json').read_text())
        print(json.dumps(result, indent=2, sort_keys=True))
        assert run.returncode == 0 and result['completed'] and result['all_observed_helpers_absent_after_cleanup']
        if args.phase == 'red':
            required = {args.test} if args.test else EXPECTED_RED
            actual = {name for name, status in result['test_names'].items() if status == 'FAILED'}
            assert actual == required and result['test_exit_code'] == 101, 'Unexpected failure set is not selected RED'
            raise SystemExit(1)
        assert result['passed']


if __name__ == '__main__':
    main()
