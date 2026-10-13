#!/usr/bin/env python3
"""Freeze an actual display measurement selection; this command never boots a VM."""
import argparse
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH',
          'docker', '--host=unix:///var/run/docker.sock']
IMAGE = 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--label', required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--phase', choices=('red', 'green'), default='red')
    p.add_argument('--protocol-input', type=Path, help='Pinned matching public screencopy XML for green only')
    a = p.parse_args()
    assert a.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
    assert not a.output.exists(), 'preserve existing plan/evidence'
    a.output.mkdir(mode=0o700)
    source = a.output / 'source'; source.mkdir()
    for name in ('guest.py', 'measure.py', 'qt-output-probe.cpp'):
        shutil.copy2(HERE / name, source / name)
    shutil.copy2(REPO / 'sim/display-profiles/pixel10.json', source / 'pixel10.json')
    if a.phase == 'green':
        assert a.protocol_input is not None, 'green requires separately pinned public protocol input'
        assert sha(a.protocol_input) == '0f50f6187965b02782a12ba346ad950d6031454f6af6e1ac5b341a279ef16d56'
        for name in ('physical-size-adapter.cpp', 'raw-output-capture.cpp', 'public-api-pins.json'):
            shutil.copy2(HERE / name, source / name)
        shutil.copy2(a.protocol_input, source / 'wlr-screencopy-unstable-v1.xml')
    else:
        assert a.protocol_input is None, 'no capture selected for red'
    shared = MAIN / 'artifacts/native-session-installed-app/vm-production.lock'
    vm = MAIN / 'artifacts/native-session-installed-app/vm-production/root.img'
    with shared.open('rb') as lock:
        fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        selection = {'phase': a.phase, 'label': a.label, 'entry': '/results/source/guest.py',
            'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
            'sources': {p.name: sha(p) for p in sorted(source.iterdir())},
            'input_root_sha256': sha(vm), 'kernel_sha256': sha(MAIN / 'sim/boot/Image'),
            'initrd_sha256': sha(MAIN / 'sim/boot/initramfs-linux.img'),
            'runtime_image': IMAGE, 'shared_input_lock': str(shared),
            'input_lock_mode': 'shared-readonly; take again across runtime',
            'guest_cpus': 4, 'guest_memory_mib': 4096, 'container_cpus': 6, 'container_memory': '8g',
            'native_pixels': {'width': 1080, 'height': 2424},
            'physical_density': ('normal public IOutput.physicalSize from selected65x146mm simulation profile; rawDRM/baseEDID remains independently recorded' if a.phase == 'green' else 'deliberately unchanged defaultEDID; negative boundary only'),
            'requested_native_surface_dpr': 2.625,
            'measurement_contract_version': 2,
            'physical_qt_coordinate_basis': 'QScreen physicalDPI times actual target QWindow DPR; less than1logicalpixel rounding per axis; QScreenDPR diagnostic retained',
            'capture_gate': ('only after actual metadata qualifies; full native SHM output buffer encoded without resize; PNG decoded pixel hash must match oriented native pixels' if a.phase == 'green' else 'no capture'),
            'public_protocol_url': ('https://raw.githubusercontent.com/hyprwm/Hyprland/efb50993780079460b0cbed1363e2166a2de1d9f/protocols/wlr-screencopy-unstable-v1.xml' if a.phase == 'green' else None),
            'supported_scale_config': 'debug.disable_scale_checks=true permits fractional logical bounds; actual Qt screen and surface DPR measured independently',
            'public_monitor_source_commit': 'efb50993780079460b0cbed1363e2166a2de1d9f',
            'public_monitor_source_sha256': '9cf88e154eb5dae676c79d37b5b055ca6134838857cecdbb89a3b747a6821927',
            'public_output_protocol_source_sha256': '74ab6553e39c3587eb5b3850dbc8eb69b85e4a2aa738ca2353ea2837739c0f35',
            'user_ui_selected': False,
            'raw_measurement_png_selected': a.phase == 'green',
            'physical_phone_accessed': False, 'network': 'none', 'qemu_mode': 'system',
            'frozen_vm_or_kernel_modified': False, 'snapshot_overlay': 'own container /tmp tmpfs'}
    qemu = ['qemu-system-aarch64', '-machine', 'virt,gic-version=3', '-cpu', 'max',
        '-accel', 'tcg,thread=multi', '-smp', '4', '-m', '4096',
        '-kernel', '/workspace/sim/boot/Image', '-initrd', '/workspace/sim/boot/initramfs-linux.img',
        '-append', 'console=ttyAMA0 root=/dev/vda rw rootwait panic=10 systemd.mask=sshd.service systemd.mask=serial-getty@ttyAMA0.service systemd.mask=systemd-networkd-wait-online.service',
        '-drive', 'file=/vm/root.img,if=none,id=root,format=raw,snapshot=on',
        '-device', 'virtio-blk-pci,drive=root', '-device', 'virtio-gpu-gl-pci,xres=1080,yres=2424',
        '-device', 'virtio-keyboard-pci', '-device', 'virtio-rng-pci', '-nic', 'none',
        '-fsdev', 'local,id=project,path=/workspace,security_model=none,readonly=on', '-device', 'virtio-9p-pci,fsdev=project,mount_tag=project',
        '-fsdev', 'local,id=results,path=/results,security_model=mapped-xattr', '-device', 'virtio-9p-pci,fsdev=results,mount_tag=results',
        '-display', 'sdl,gl=on', '-serial', 'stdio', '-monitor', 'none', '-no-reboot']
    shell = '#!/bin/bash\nset -euo pipefail\nexport LIBGL_ALWAYS_SOFTWARE=1 SDL_VIDEODRIVER=x11 XDG_RUNTIME_DIR=/tmp TMPDIR=/tmp\nexec timeout --signal=TERM --kill-after=10s 600s xvfb-run -a -s \'-screen 0 1080x2424x24 -dpi 100 -nolisten tcp\' ' + __import__('shlex').join(qemu) + '\n'
    (source / 'boot.sh').write_text(shell); selection['sources']['boot.sh'] = sha(source / 'boot.sh')
    command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-pixel10-display-' + a.label,
        '--network', 'none', '--cpus', '6', '--memory', '8g', '--pids-limit', '512',
        '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges', '--user', f'{os.getuid()}:{os.getgid()}',
        '--tmpfs', '/tmp:rw,exec,nosuid,nodev,size=2g', '-v', str(MAIN) + ':/workspace:ro',
        '-v', str(vm) + ':/vm/root.img:ro', '-v', str(a.output.resolve()) + ':/results:rw',
        IMAGE, 'bash', '/results/source/boot.sh']
    (a.output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
    (a.output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    for path in source.iterdir(): path.chmod(path.stat().st_mode & ~0o222)
    print(json.dumps({'selection': str(a.output / 'selection.json'),
        'selection_sha256': sha(a.output / 'selection.json'), 'command_sha256': sha(a.output / 'command.json'),
        'runtime_started': False}, indent=2))


if __name__ == '__main__':
    main()
