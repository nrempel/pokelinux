#!/usr/bin/env python3
"""Run a bounded ARM64 Linux guest; never treat virtual graphics as Pixel proof."""
import argparse
import datetime
import json
import os
import re
import subprocess
from pathlib import Path

REQUIRED_CHECKS = ('boot', 'arch_arm64', 'stock_abi', 'software_egl', 'hyprland',
                   'wayland_render', 'touch', 'touch_release', 'compositor_shutdown', 'shared_output_ownership')


def qemu_command(project, results, provision=False):
    project, results = Path(project).resolve(), Path(results).resolve()
    for path in (project, results):
        if ',' in str(path):
            raise ValueError('QEMU shared paths cannot contain commas')
    return [
        'xvfb-run', '-a', '-s', '-screen 0 800x600x24 -nolisten tcp',
        'qemu-system-aarch64', '-machine', 'virt,gic-version=3',
        '-cpu', 'max', '-accel', 'tcg,thread=multi', '-smp', '4', '-m', '4096',
        '-kernel', str(project / 'sim/boot/Image'),
        '-initrd', str(project / 'sim/boot/initramfs-linux.img'),
        '-append', 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 '
                   'systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service '
                   + ('' if provision else 'systemd.mask=systemd-networkd-wait-online.service ')
                   + 'pixel.mode=' + ('provision' if provision else 'test'),
        '-drive', f'file={project}/sim/rootfs.img,if=none,id=root,format=raw',
        '-device', 'virtio-blk-pci,drive=root',
        '-device', 'virtio-gpu-gl-pci,xres=800,yres=600',
        '-device', 'virtio-keyboard-pci',
        '-device', 'virtio-rng-pci',
        '-global', 'virtio-net-pci.romfile=',
        '-nic', 'user,model=virtio-net-pci' if provision else 'none',
        '-fsdev', f'local,id=project,path={project},security_model=none,readonly=on',
        '-device', 'virtio-9p-pci,fsdev=project,mount_tag=project',
        '-fsdev', f'local,id=results,path={results},security_model=mapped-xattr',
        '-device', 'virtio-9p-pci,fsdev=results,mount_tag=results',
        '-display', 'sdl,gl=on', '-serial', 'stdio', '-monitor', 'none', '-no-reboot',
    ]


def validate_result(result):
    if 'error' in result:
        raise ValueError('Guest reported an error: ' + str(result['error']))
    if result.get('hardware_verified') is not False:
        raise ValueError('Simulation cannot verify Pixel hardware')
    missing = [key for key in REQUIRED_CHECKS if result.get('checks', {}).get(key) is not True]
    if missing:
        raise ValueError('Guest checks missing or failed: ' + ', '.join(missing))


def validate_factory_boundary(returncode, output, trace):
    if returncode != 77 or 'INITIALIZE=0 VERSION=0.0 ERROR=0x3003' not in output:
        raise ValueError('Unexpected factory EGL outcome')
    if not re.search(r'powervr\.ini[^\n]* = [0-9]+', trace):
        raise ValueError('Factory configuration was not opened')
    if not re.search(r'/dev/dri/renderD\d+[^\n]* = -1 errno=2', trace):
        raise ValueError('Trace does not establish absent virtual GPU devices')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('provision', 'test'))
    parser.add_argument('--timeout', type=int, default=1800)
    parser.add_argument('--project', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    results = args.project / 'sim/results' / (args.mode + '-' + stamp)
    results.mkdir(parents=True)
    command = qemu_command(args.project, results, args.mode == 'provision')
    (results / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    print('VM results:', results, flush=True)
    with (results / 'serial.log').open('w') as output:
        completed = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT,
                                   timeout=args.timeout, check=False,
                                   env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', SDL_VIDEODRIVER='x11'))
    if completed.returncode:
        raise SystemExit(f'QEMU failed with {completed.returncode}; see {results}/serial.log')
    result_file = results / ('provision.json' if args.mode == 'provision' else 'result.json')
    result = json.loads(result_file.read_text())
    if args.mode == 'test':
        validate_result(result)
    elif result.get('provisioned') is not True:
        raise SystemExit('Guest provisioning failed')
    if args.mode == 'provision':
        (args.project / 'sim/provisioned.json').write_text(json.dumps(result, indent=2) + '\n')
    else:
        (args.project / 'sim/latest-result.json').write_text(json.dumps({
            'results': str(results), 'result': result}, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
