#!/usr/bin/env python3
"""Own dedicated ARM QEMU clone. Never touches the compositor fixture VM."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

REPO = Path(__file__).resolve().parents[2]
BASE = REPO / 'artifacts/native-session-authority'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
IMAGE = 'pixel-graphics-sim:20261011-b472'


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as file:
        while block := file.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def hashes(directory):
    return {str(path.relative_to(directory)): sha(path) for path in sorted(directory.rglob('*'))
            if path.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build', required=True, help='immutable crossbuild label')
    parser.add_argument('--label', required=True)
    parser.add_argument('--case', action='append')
    args = parser.parse_args()
    for label in (args.build, args.label):
        assert label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in label)
    BASE.mkdir(parents=True, exist_ok=True)
    with (BASE / 'vm.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        selected = BASE / args.build
        selection = json.loads((selected / 'selection.json').read_text())
        assert selection['target'] == 'aarch64-unknown-linux-gnu'
        for name, expected in selection['sources'].items():
            assert sha(selected / 'source' / name) == expected, name
        for name, expected in selection['elf_sha256'].items():
            assert sha(selected / 'bins' / name) == expected, name
        output = BASE / args.label
        assert not output.exists(), 'preserve existing proof ' + str(output)
        output.mkdir()
        shutil.copytree(selected / 'source', output / 'source')
        shutil.copytree(selected / 'bins', output / 'bins')
        shutil.copyfile(Path(__file__), output / 'vm.py')
        shutil.copyfile(Path(__file__).with_name('vm-guest.py'), output / 'vm-guest.py')
        selection['selected_build'] = args.build
        selection['selected_cases'] = args.case or []
        selection['kernel_image_sha256'] = sha(REPO / 'sim/boot/Image')
        selection['initrd_sha256'] = sha(REPO / 'sim/boot/initramfs-linux.img')
        selection['vm_image'] = IMAGE
        selection['vm_image_id'] = subprocess.check_output(DOCKER + ['image', 'inspect', IMAGE, '--format', '{{.Id}}']).decode().strip()
        selection['qemu_mode'] = 'system'
        selection['vm_guest_sha256'] = sha(output / 'vm-guest.py')
        vm = BASE / 'vm'
        vm.mkdir(exist_ok=True)
        image = vm / 'root.img'
        if not image.exists():
            subprocess.run(['cp', '--reflink=auto', '--sparse=always', str(REPO / 'sim/rootfs.img'), str(image)], check=True)
            entry = vm / 'entry'
            entry.write_text('''#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python3 /results/vm-guest.py
''')
            commands = ['rm /usr/local/bin/pixel-sim-boot',
                        'write ' + str(entry) + ' /usr/local/bin/pixel-sim-boot',
                        'set_inode_field /usr/local/bin/pixel-sim-boot mode 0100755']
            with (vm / 'clone.log').open('wb') as log:
                for command in commands:
                    subprocess.run(['debugfs', '-w', '-R', command, str(image)],
                                   stdout=log, stderr=subprocess.STDOUT, check=True)
        selection['owned_vm_root_sha256'] = sha(image)
        (output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
        qemu = ['qemu-system-aarch64', '-machine', 'virt,gic-version=3', '-cpu', 'max',
                '-accel', 'tcg,thread=multi', '-smp', '3', '-m', '2048',
                '-kernel', '/workspace/sim/boot/Image', '-initrd', '/workspace/sim/boot/initramfs-linux.img',
                '-append', 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service '
                    'systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service',
                '-drive', 'file=/vm/root.img,if=none,id=root,format=raw,snapshot=on',
                '-device', 'virtio-blk-pci,drive=root', '-device', 'virtio-rng-pci', '-nic', 'none',
                '-fsdev', 'local,id=project,path=/workspace,security_model=none,readonly=on',
                '-device', 'virtio-9p-pci,fsdev=project,mount_tag=project',
                '-fsdev', 'local,id=results,path=/results,security_model=mapped-xattr',
                '-device', 'virtio-9p-pci,fsdev=results,mount_tag=results',
                '-nographic', '-monitor', 'none', '-no-reboot']
        (output / 'command.json').write_text(json.dumps(qemu, indent=2) + '\n')
        name = 'pokelinux-session-authority-vm-' + args.label
        container = subprocess.check_output(DOCKER + ['create', '--name', name, '--pull=never',
            '--network', 'none', '--cpus', '4', '--memory', '4g', '--pids-limit', '128',
            '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
            '--user', f'{os.getuid()}:{os.getgid()}', '-v', str(REPO) + ':/workspace:ro',
            '-v', str(vm) + ':/vm:ro', '-v', str(output) + ':/results:rw', IMAGE,
            'timeout', '--signal=TERM', '--kill-after=10s', '360s', *qemu]).decode().strip()
        selection['container_id'] = container
        (output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
        try:
            with (output / 'serial.log').open('wb') as log:
                completed = subprocess.run(DOCKER + ['start', '-a', container], stdout=log, stderr=subprocess.STDOUT)
            (output / 'exit-code').write_text(str(completed.returncode) + '\n')
        finally:
            subprocess.run(DOCKER + ['rm', '-f', container], stdout=subprocess.DEVNULL, check=True)
        result_path = output / 'result.json'
        if not result_path.exists():
            print((output / 'serial.log').read_text()[-4000:])
            raise AssertionError('guest did not publish actual execution result')
        result = json.loads(result_path.read_text())
        (output / 'proof-sha256.json').write_text(json.dumps({key: value for key, value in hashes(output).items()
            if not key.startswith(('source/', 'bins/')) and key != 'proof-sha256.json'}, indent=2) + '\n')
        print((output / 'execution.log').read_text() if (output / 'execution.log').exists() else '', end='')
        print(json.dumps({'passed': result['passed'], 'kernel': result.get('kernel'),
                          'error': result.get('error'), 'pidfs_identity_qualified': result.get('pidfs_identity_qualified')}, indent=2))
        assert completed.returncode == 0, 'owned QEMU process failed/timed out'
        assert result.get('kernel'), result.get('traceback', 'guest setup result absent')
        assert result['kernel']['machine'] == 'aarch64' and result['qemu_mode'] == 'system'
        assert result['pidfs_identity_qualified'], 'actual PID_FS_MAGIC not qualified'
        if selection['phase'] == 'red':
            cases = result['cases']
            assert all(case['setup_complete'] for case in cases)
            assert next(case['passed'] for case in cases if case['case'] == 'unknown_denied')
            failures = {case['case'] for case in cases if not case['passed']}
            assert {'root_controls_unknown_denied', 'source_only_read'} <= failures
            assert all(not case.get('cleanup_error') for case in cases)
            print('Qualified actual ARM kernel red missing-ingress and manual-controls bypass')
            return 0
        assert result['passed']
        return 0


if __name__ == '__main__':
    raise SystemExit(main())
