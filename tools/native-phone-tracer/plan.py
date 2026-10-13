#!/usr/bin/env python3
"""Freeze exact tracer inputs without starting Docker or QEMU."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
PIXEL = Path('/tmp/pokelinux-pixel10-density-green-sol-3')
TRACE = Path('/tmp/pokelinux-lock-admission-arm-green-4')
IMAGE = 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
ROOT_SHA = '31e3ccf69a5496f61284abdcd049e8273e70e840407bab4e744c778f21e25842'
KERNEL_SHA = '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b'
INITRD_SHA = 'a9bb29d29abda94748c9948f903c3c1704f829eea477652b9a4201a61f36a647'
PEERS = {'incoming-peer': 'd3ace7cd833d10aa81217bae20d7a8765d06c3160167dbf69d848a9ce72560f4',
         'property-info': '7c81ebd6b1b2bde547675aed2f6ee4cdc0e0b2a4892160c5af08b92b0d0f4f85',
         'property-init': '27ba656934837f802ee05a7817ab83dd25fc1bbac57bd491a269dc5d338067a2',
         'incoming-manager': '40395c1363de8057735ae0a883f6d9498a73e7a4f2a119bf1157332a5eec6dbd',
         'incoming-probe': '270195bacdabd702a564bb2eb8a9f011bafd045e086afc7c1f5a16074dab5992'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--label', required=True)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--build', required=True, type=Path)
    p.add_argument('--build-selection-sha256', required=True)
    p.add_argument('--guest-sha256', required=True)
    a = p.parse_args()
    assert a.label and len(a.label) < 60 and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.label)
    output = a.output.resolve()
    assert Path('/dev/shm') in output.parents, 'RAM outputs required; no disk backing clone'
    assert not any((parent / '.git').exists() for parent in [output, *output.parents])
    assert not output.exists(), 'preserve selected labels'
    assert sha(HERE / 'guest.py') == a.guest_sha256
    assert sha(a.build / 'selection.json') == a.build_selection_sha256
    output.mkdir(mode=0o700)
    source, bins = output / 'source', output / 'bins'
    source.mkdir(); bins.mkdir(); (output / 'input').mkdir(); (output / 'placement').mkdir()
    origins = {}

    def copy(origin, destination, expected=None):
        origin = Path(origin)
        before = sha(origin)
        if expected is not None:
            assert before == expected, str(origin)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(origin, destination)
        assert sha(destination) == sha(origin) == before
        origins[str(origin)] = before

    for name in ['guest.py', 'realm.py', 'boot.sh', 'plan.py', 'run-selected.py', 'input.py', 'build-arm.py', 'frontend-capture.py', 'inspect-png.py']:
        copy(HERE / name, source / name)
    build = json.loads((a.build / 'selection.json').read_text())
    for name, record in build['sources_for_build'].items():
        copy(a.build / 'source' / name, source / 'rust-build' / name, record['sha256'])
    for name, record in build['sources_before_lock_resolution'].items():
        current = REPO / name
        if not current.exists() and name.startswith('artifacts/sms-binder/'):
            current = MAIN / name
        assert sha(current) == record['sha256'], 'current source changed after frozen build: ' + name
        origins[str(current)] = record['sha256']
    copy(a.build / 'selection.json', source / 'rust-build-selection.json', a.build_selection_sha256)
    for name in ['build-provenance.json', 'cargo-metadata.json']:
        copy(a.build / name, source / name)
    for name, record in build['bins'].items():
        copy(a.build / 'bins' / name, bins / name, record['sha256'])
    desktop = json.loads((a.build / 'desktop-launch/selection.json').read_text())
    assert desktop['base_selection_sha256'] == a.build_selection_sha256
    copy(a.build / 'desktop-launch/messages-launch', bins / 'messages-launch',
         '9996ef3639259a11f38a189052e58fa009047f205d306167b1f00d70b04c91cf')
    copy(a.build / 'desktop-launch/selection.json', source / 'messages-launch-selection.json')
    qt = MAIN / 'artifacts/native-messages/desktop/green-10'
    qt_result = json.loads((qt / 'result.json').read_text()); assert qt_result['passed']
    copy(qt / 'pokelinux-messages', bins / 'pokelinux-messages',
         '4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8')
    for path in sorted((qt / 'source/native-app').rglob('*')):
        if path.is_file(): copy(path, source / 'native-app' / path.relative_to(qt / 'source/native-app'))
    copy(MAIN / 'native-messages-ui/org.omarchy.Messages.desktop', source / 'org.omarchy.Messages.desktop')
    comp = MAIN / 'artifacts/native-session-compositor/green-owned-source-final'
    comp_result = json.loads((comp / 'result.json').read_text()); assert comp_result['passed']
    for name in ['adapter.so', 'probe.so']:
        copy(comp / 'bins' / name, bins / name,
             '312227400c39a793c2e591d0893c692d0ac258b8f5f9ad925ae2b8727fd53621' if name == 'adapter.so' else None)
    copy(comp / 'selection.json', source / 'compositor-build-selection.json')
    profile = json.loads((PIXEL / 'result.json').read_text())
    assert profile['passed'] and profile['physical_profile_qualified'] and profile['raw_native_capture_qualified']
    for name, record in profile['frozen_binaries'].items(): copy(PIXEL / 'binaries' / name, bins / name, record['sha256'])
    for name in ['pixel10.json', 'measure.py', 'public-api-pins.json']: copy(PIXEL / 'source' / name, source / name)
    copy(PIXEL / 'selection.json', source / 'pixel10-qualified-selection.json')
    copy(PIXEL / 'proof.json', source / 'pixel10-qualified-proof.json')
    for origin, name in [(MAIN / 'artifacts/native-theme/omarchy-public.tar.gz', 'omarchy-public.tar.gz'),
                         (MAIN / 'native-theme/public-manifest.json', 'omarchy-public-manifest.json'),
                         (MAIN / 'sim/omarchy-source/source-manifest.json', 'omarchy-source-manifest.json'),
                         (MAIN / 'tools/omarchy_theme_patch.py', 'omarchy_theme_patch.py')]: copy(origin, source / name)
    assert sha(source / 'omarchy-public.tar.gz') == '8cc6b1d9d903c606600395e3b3d80ad9f15bd3ab78f0f19b0f49041b1df82013'
    peer = MAIN / 'artifacts/native-messages-incoming-binder/selections/incoming-green-final-v2/binaries'
    for name, pin in PEERS.items(): copy(peer / name, bins / name, pin)
    copy(MAIN / 'tools/native-messages-incoming-binder/peer_session.py', source / 'peer_session.py')
    trace = json.loads((TRACE / 'source/strace-public-package-proof.json').read_text())
    assert trace['signature_verified'] and trace['ELF_sha256'] == '731bc2e1aedb562c7f106f55002865fb6eaea2b3fb18a52778ade83772888f96'
    copy(TRACE / 'bins/strace', bins / 'strace', trace['ELF_sha256'])
    copy(TRACE / 'source/strace-public-package-proof.json', source / 'strace-public-package-proof.json')
    android = MAIN / 'artifacts/runtime'
    manifest = json.loads((android / 'runtime-manifest.json').read_text())
    for record in manifest['files']:
        path = android / record['path'].lstrip('/')
        assert path.stat().st_size == record['size'] and sha(path) == record['sha256']
        origins[str(path)] = record['sha256']
    copy(android / 'runtime-manifest.json', source / 'android-runtime-manifest.json')
    backing = MAIN / 'artifacts/native-session-installed-app/vm-production/root.img'
    lockpath = backing.parent.parent / 'vm-production.lock'
    with lockpath.open('rb') as lock:
        fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        for path, pin in [(backing, ROOT_SHA), (MAIN / 'sim/boot/Image', KERNEL_SHA),
                          (MAIN / 'sim/boot/initramfs-linux.img', INITRD_SHA)]:
            assert sha(path) == pin
            origins[str(path)] = pin
    for name, pin in origins.items(): assert sha(name) == pin, 'input changed during preparation: ' + name
    for path in source.rglob('*'):
        if path.is_file(): path.chmod(0o444)
    for path in bins.iterdir(): path.chmod(0o555)
    selection = {'schema': 1, 'phase': 'native-phone-tracer', 'label': a.label,
                 'core_head': build['core_head'], 'fixture_binary': 'messages-phone-tracer',
                 'source_budget_ns': 100_000_000, 'Source_revival_or_reset_permitted': False,
                 'native_pixels': {'width': 1080, 'height': 2424}, 'physical_ppi': 422,
                 'native_window_dpr_profile': 2.625, 'normal_qt_replaced_or_instrumented': False,
                 'expected_hyprland_elf_sha256': comp_result['hyprland_elf_sha256'],
                 'runtime_image': IMAGE, 'shared_input_lock': str(lockpath), 'input_lock_mode': 'shared-readonly',
                 'output_lock_mode': 'exclusive-per-label', 'input_root_sha256': ROOT_SHA,
                 'kernel_sha256': KERNEL_SHA, 'initrd_sha256': INITRD_SHA,
                 'guest_cpus': 8, 'guest_memory_mib': 6144, 'container_cpus': 12, 'container_memory': '10g',
                 'outer_deadline_seconds': 600, 'network': 'none', 'host_devices_attached': False,
                 'overlay_scope': 'bounded-private-container-tmpfs-qcow2-with-readonly-raw-backing',
                 'phone_accessed': False, 'hardware_qualified': False, 'full_production_qualified': False,
                 'synthetic_incoming_peer': True, 'actual_android_runtime_readonly': True,
                 'QMP_input_whitelist': ['tab', 'ret'], 'input_method': 'actual-virtio-keyboard-send-key',
                 'capture_api': 'XGetImage', 'capture_device': 'owned sole SDL/X11 whole client window',
                 'capture_method_scope': 'unscaled SDL/X11 whole client window; no direct Wayland-buffer equality claim',
                 'actor_trace': 'owned parent strace --seccomp-bpf recvmsg/sendto --status=successful; per-actor completed calls only; parent is not socket actor',
                 'host_software_renderer_threads': 4,
                 'host_SDL_frontend': 'sdl,gl=on',
                 'whole_owned_SDL_window_at_root_origin_required': True,
                 'frontend_origin_placement_phase': 'after denied normal app render before first Source adapter load',
                 'capture_resize_or_crop_permitted': False,
                 'external_inputs': origins,
                 'files': {str(path.relative_to(output)): sha(path) for tree in [source, bins]
                           for path in sorted(tree.rglob('*')) if path.is_file()}}
    command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-phone-tracer-' + a.label,
        '--cidfile', str(output / 'owned-container-id'),
        '--network', 'none', '--cpus', '12', '--memory', '10g', '--pids-limit', '512', '--cap-drop', 'ALL',
        '--security-opt', 'no-new-privileges', '--user', f'{os.getuid()}:{os.getgid()}',
        '--tmpfs', '/tmp:rw,exec,nosuid,nodev,size=2g', '-v', f'{MAIN}:/workspace:ro',
        '-v', f'{backing}:/vm/root.img:ro', '-v', f'{output}:/results:rw',
        '-v', f'{android}:/android-runtime:ro', IMAGE, 'bash', '/results/source/boot.sh']
    (output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
    (output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    print(json.dumps({'selection_dir': str(output), 'selection_sha256': sha(output / 'selection.json'),
                      'command_sha256': sha(output / 'command.json'), 'runtime_started': False}, indent=2))


if __name__ == '__main__': main()
