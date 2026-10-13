#!/usr/bin/env python3
"""Actual held-Initializing SDK exposes a NEW future launch-readiness boundary."""
import argparse
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
BUILD = Path('/tmp/pokelinux-wayland-containment-root/artifacts/native-session-display-app/owned-display-preflight-fence-build-2')
BASE = ROOT / 'artifacts/native-display-app-generations'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
IMAGE = 'pixel-graphics-sim:20261011-b472'
SERVICE = '1a4d606e41e85422c999a16c3adda7072eaccb2999fe3d25bf9b61cf89e13c57'
QT = '4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8'


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--label', required=True)
    args = parser.parse_args()
    assert args.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.label)
    BASE.mkdir(exist_ok=True)
    with (BASE / (args.label + '.lock')).open('a') as own, (MAIN / 'artifacts/native-session-installed-app/vm-production.lock').open('a') as shared:
        fcntl.flock(own, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fcntl.flock(shared, fcntl.LOCK_SH | fcntl.LOCK_NB)
        out = BASE / args.label
        out.mkdir()
        source = out / 'source'
        source.mkdir()
        bins = out / 'bins'
        bins.mkdir()
        for name in ('init-red-run.py', 'init-red-guest.py', 'init-red-notes.md'):
            shutil.copyfile(Path(__file__).with_name(name), source / name)
        shutil.copyfile(source / 'init-red-guest.py', source / 'guest.py')
        for name in ('registry-probe.py', 'boundary-probe.py', 'boot.sh'):
            shutil.copyfile(Path(__file__).with_name(name), source / name)
        bs = json.loads((BUILD / 'selection.json').read_text())
        assert bs['bins']['messages-session-display-fixture'] == SERVICE
        for name, value in bs['sources'].items():
            assert sha(BUILD / 'source' / name) == value
        shutil.copytree(BUILD / 'source', source / 'rust-build')
        shutil.copyfile(BUILD / 'selection.json', source / 'rust-build-selection.json')
        for name, value in bs['bins'].items():
            assert sha(BUILD / 'bins' / name) == value
            shutil.copyfile(BUILD / 'bins' / name, bins / name)
        compositor = MAIN / 'artifacts/native-session-compositor/green-owned-source-final'
        comp = json.loads((compositor / 'result.json').read_text())
        assert comp['passed']
        for name, path in [('adapter.so', compositor / 'bins/adapter.so'), ('probe.so', compositor / 'bins/probe.so'), ('lock-client', compositor / 'lock-client')]:
            shutil.copyfile(path, bins / name)
        shutil.copyfile(compositor / 'selection.json', source / 'compositor-build-selection.json')
        qt = MAIN / 'artifacts/native-messages/desktop/green-10'
        proof = json.loads((qt / 'result.json').read_text())
        assert proof['passed'] and sha(qt / 'pokelinux-messages') == QT
        for name in ('pokelinux-messages', 'pokelinux-messages-observer'):
            assert sha(qt / name) == proof['production_binary'][name]
            shutil.copyfile(qt / name, bins / name)
        shutil.copyfile(qt / 'bins/messages-service', bins / 'messages-service')
        shutil.copytree(qt / 'source/native-app', source / 'native-app')
        for path in bins.iterdir():
            path.chmod(0o755)
        vm = MAIN / 'artifacts/native-session-installed-app/vm-production'
        selection = {
            'phase': 'green', 'test_phase': 'red', 'test_case': 'app-launch-before-context-Ready-red',
            'fixture_binary': 'messages-session-display-fixture', 'rust_selection': bs,
            'selected_runtime_build': str(BUILD), 'service_elf_sha256': SERVICE, 'normal_qt_elf_sha256': QT,
            'metadata_scope': 'Existing context instance is one service epoch; explicitly not an app generation or permission grant',
            'desired_future_boundary': 'No success/no registered normal Qt until app context SDK Ready; Snapshot/Stop responsive while SDK Initializing',
            'normal_production_qt': True, 'expected_hyprland_elf_sha256': comp['hyprland_elf_sha256'],
            'sample_budget_ns': 100000000, 'source_revival_permitted': False,
            'freshness_corrected_core_selected': False, 'no_runtime_rebuild': True, 'physical_phone_accessed': False,
            'input_vm_lock_mode': 'shared-readonly; preparers exclusive', 'qemu_smp': 4, 'container_cpus': 6,
            'container_memory': '8g', 'network': 'none', 'owned_vm_root_sha256': sha(vm / 'root.img'),
            'kernel_sha256': sha(MAIN / 'sim/boot/Image'), 'initrd_sha256': sha(MAIN / 'sim/boot/initramfs-linux.img'),
            'image_id': subprocess.check_output(DOCKER + ['image', 'inspect', IMAGE, '--format', '{{.Id}}'], text=True).strip(),
            'files': {str(p.relative_to(out)): sha(p) for tree in (source, bins) for p in sorted(tree.rglob('*')) if p.is_file()},
        }
        command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-display-init-admission-' + args.label,
            '--network', 'none', '--cpus', '6', '--memory', '8g', '--pids-limit', '512', '--cap-drop', 'ALL',
            '--security-opt', 'no-new-privileges', '--user', f'{os.getuid()}:{os.getgid()}',
            '-v', str(MAIN) + ':/workspace:ro', '-v', str(vm) + ':/vm:ro', '-v', str(out) + ':/results:rw',
            selection['image_id'], 'bash', '/results/source/boot.sh']
        (out / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
        (out / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
        with (out / 'serial.log').open('w') as log:
            completed = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        (out / 'exit-code').write_text(str(completed.returncode) + '\n')
        assert (out / 'result.json').is_file(), (out / 'serial.log').read_text()[-5000:]
        result = json.loads((out / 'result.json').read_text())
        print(json.dumps({k: result.get(k) for k in ('passed', 'desired_boundary_red', 'prerequisites_qualified', 'groups', 'error', 'cleanup_errors', 'all_tracked_children_finally_absent_or_directly_waited')}, indent=2))
        assert completed.returncode == 0 and result['setup_complete'] and result['all_tracked_children_finally_absent_or_directly_waited']
        assert result.get('prerequisites_qualified') and result['actual_premature_launch']['display']['state']=='Initializing'
        assert result['desired_boundary_red'] and result['red_detected'] and not result['passed']
        raise SystemExit(1)


if __name__ == '__main__':
    main()
