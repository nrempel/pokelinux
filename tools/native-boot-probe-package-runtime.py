#!/usr/bin/env python3
"""Consume an exact candidate generic LZ4 ramdisk on the retained Arch kernel.

Only the authored generic ramdisk is booted. No stock Google kernel, vendor
userspace, root disk, network or physical device is attached. AOSP extraction
and the pinned Rust inspector must agree before QEMU receives the LZ4 bytes.
"""
import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import stat
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parent.parent
KERNEL = pathlib.Path('/home/ubuntu/Projects/pokelinux/sim/boot/Image')
KERNEL_SHA = '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b'
DOCKER_IMAGE = '561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
HISTORICAL_IMAGE_SHA = '588cabd9399e1ece30cd35373c07d5de8ee5c6654bae692879aeb912c9ac1faf'
DOCKER = ['docker', '--host=unix:///var/run/docker.sock']


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def load(name):
    spec = importlib.util.spec_from_file_location(name, REPO / 'tools' / (name.replace('_', '-') + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def private(path):
    if any((parent / '.git').exists() for parent in [path, *path.parents]):
        raise ValueError('private output must be outside every Git worktree')


def observed_native_completion(console, code, timed_out):
    child = re.search(r'POKELINUX_CHILD_EXEC pid=(\d+) parent=1', console)
    waited = re.search(r'POKELINUX_CHILD_WAIT pid=(\d+) actual_wait=true exit=Some\(37\)', console)
    return (not timed_out and code == 0 and
            'POKELINUX_NATIVE_PID1 pid=1 uid=0' in console and
            'POKELINUX_PROBE_KERNEL release=7.1.6-1-aarch64-ARCH' in console and
            child is not None and waited is not None and child.group(1) == waited.group(1) and
            'POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false' in console and
            'reboot: Power down' in console and 'POKELINUX_PROBE_FAILURE' not in console and
            'Kernel panic' not in console)


def run(args):
    package = load('native_boot_probe_package')
    decoder = load('native_boot_probe_package_suite')
    pins = package.small_json(REPO / 'native-boot-package/input-pins.json')
    image = args.candidate_image.resolve()
    output = args.private_output_dir.resolve()
    private(output)
    if output.exists():
        raise ValueError('output label already exists; preserve prior runtime evidence')
    if not re.fullmatch(r'[a-zA-Z0-9_-]+', output.name):
        raise ValueError('invalid immutable output label')
    if any(output == path or output in path.parents or path in output.parents
           for path in [image, KERNEL, args.inspector.resolve(), args.private_reference_root.resolve()]):
        raise ValueError('private output must not overlap inputs')
    package.regular(image, 8 * 1024 * 1024)
    image_sha = digest(image)
    manifest = package.small_json(image.parent / 'manifest.json')
    if (manifest['candidate_class'] != pins['candidate_class'] or
            manifest['image']['sha256'] != image_sha or manifest['unsigned'] is not True or
            manifest['qualifications']['deployment_admitted'] is not False):
        raise ValueError('exact unsigned unqualified candidate binding required')
    if image_sha != HISTORICAL_IMAGE_SHA:
        raise ValueError('runtime consumer requires the exact frozen candidate')
    package.regular(args.inspector, 32 * 1024 * 1024)
    if digest(args.inspector) != pins['inspector_sha256']:
        raise ValueError('frozen Rust inspector binary changed')
    if digest(KERNEL) != KERNEL_SHA:
        raise ValueError('retained Arch kernel changed')
    reference = package.load_reference(args.private_reference_root.resolve(), pins)
    report = package.inspect(args.inspector, image)
    if (report['kind'] != 'boot-v4' or report['page_size'] != 4096 or
            report['kernel']['size'] or report['signature']['size'] or report['trailing']['size']):
        raise ValueError('bounded kernel-free init_boot v4 candidate required')
    ramdisk_sha = package.extent_hash(image, report['ramdisk'])
    if ramdisk_sha != manifest['generic_ramdisk']['compressed_sha256']:
        raise ValueError('manifest ramdisk binding changed')
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    reference.original(args.private_reference_root, 'unpack_bootimg.py', [
        '--boot_img', image, '--out', output / 'original-unpack'])
    ramdisk = output / 'original-unpack/ramdisk'
    if (ramdisk.stat().st_size != report['ramdisk']['size'] or digest(ramdisk) != ramdisk_sha):
        raise ValueError('actual original unpacker and Rust extent disagree')
    frames = decoder.decode_stream(ramdisk.read_bytes())
    if len(frames) != 1:
        raise ValueError('exact single generic LZ4 frame required')
    entries = decoder.archive(frames[0]['body'], strict=True)
    for name in ['init', 'probe-child']:
        if (not stat.S_ISREG(entries[name]['mode']) or
                entries[name]['sha256'] != pins['native_payload'][name]['sha256']):
            raise ValueError('actual generic native ELF pin mismatch')
    if args.mode == 'red' and any(name in entries for name in ['dev/console', 'dev/null']):
        raise ValueError('historical RED unexpectedly has startup nodes')
    node_records = {}
    for name in ['dev/console', 'dev/null']:
        node_records[name] = ({key: value for key, value in entries[name].items() if key != 'data'}
                              if name in entries else None)
    selection = {'candidate_image_sha256': image_sha, 'kernel_sha256': KERNEL_SHA,
                 'exact_extracted_lz4_sha256': ramdisk_sha,
                 'inspector_sha256': digest(args.inspector), 'consumer_sha256': digest(pathlib.Path(__file__)),
                 'original_unpacker_sha256': digest(args.private_reference_root / 'unpack_bootimg.py'),
                 'cpio_startup_nodes': node_records, 'generic_only': True}
    (output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
    env = os.environ.copy()
    env['PYTHONDONTWRITEBYTECODE'] = '1'
    for key in ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_TLS', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']:
        env.pop(key, None)
    actual_image = subprocess.run(DOCKER + ['image', 'inspect', DOCKER_IMAGE, '--format', '{{.Id}}'],
                                  check=True, capture_output=True, text=True, env=env).stdout.strip()
    if actual_image != 'sha256:' + DOCKER_IMAGE:
        raise ValueError('pinned local Docker QEMU image required')
    name = 'pokelinux-boot-candidate-' + output.name
    qemu = ['qemu-system-aarch64', '-machine', 'virt,gic-version=3', '-cpu', 'max', '-smp', '2', '-m', '256',
            '-nic', 'none', '-nographic', '-monitor', 'none', '-no-reboot', '-kernel', '/inputs/Image',
            '-initrd', '/inputs/generic.lz4',
            '-append', 'console=ttyAMA0 earlycon=pl011,0x9000000 rdinit=/init panic=1']
    command = DOCKER + ['run', '--rm', '--name', name, '--network', 'none', '--read-only', '--cap-drop', 'ALL',
                       '--cpus', '2', '--memory', '512m', '--pids-limit', '64',
                       '-v', f'{KERNEL}:/inputs/Image:ro', '-v', f'{ramdisk}:/inputs/generic.lz4:ro',
                       DOCKER_IMAGE] + qemu
    (output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    started = time.monotonic()
    timed_out = False
    with (output / 'console.log').open('wb') as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, env=env)
        try:
            code = process.wait(timeout=45)
        except subprocess.TimeoutExpired:
            timed_out = True
            subprocess.run(DOCKER + ['kill', name], env=env, check=True, timeout=5,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            code = process.wait(timeout=5)
    console = (output / 'console.log').read_text(errors='replace')
    if digest(image) != image_sha or digest(ramdisk) != ramdisk_sha or digest(KERNEL) != KERNEL_SHA:
        raise ValueError('immutable runtime inputs changed')
    reference.sources(args.private_reference_root, False)
    actual_child = re.search(r'POKELINUX_CHILD_EXEC pid=(\d+) parent=1', console)
    actual_wait = re.search(r'POKELINUX_CHILD_WAIT pid=(\d+) actual_wait=true exit=Some\(37\)', console)
    expected_red = (args.mode == 'red' and not timed_out and code == 0 and
                    'Warning: unable to open an initial console.' in console and
                    'Attempted to kill init! exitcode=0x00000006' in console and
                    'Run /init as init process' in console and 'POKELINUX_NATIVE_PID1' not in console)
    native_completed = observed_native_completion(console, code, timed_out)
    passed = args.mode == 'green' and native_completed
    result = {'schema': 1, 'boundary': 'actual-produced-init_boot-original-extracted-LZ4-to-Arch-kernel',
              'scope': 'standalone-authored-generic-only-no-vendor-ramdisk', **selection,
              'mode': args.mode, 'passed': passed, 'expected_red': expected_red,
              'observed_native_runtime_green': native_completed,
              'docker_actual_wait': True, 'docker_exit': code, 'timed_out': timed_out,
              'elapsed_seconds': round(time.monotonic() - started, 3),
              'console_sha256': digest(output / 'console.log'),
              'child_exec_pid': int(actual_child.group(1)) if actual_child else None,
              'actual_wait_child_exit': 37 if actual_wait else None,
              'pixel_kernel_booted': False, 'phone_accessed': False, 'vendor_userspace_executed': False,
              'root_disk_attached': False, 'network_attached': False, 'deployment_admitted': False}
    (output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
    return 0 if passed else 1


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('mode', choices=['red', 'green'])
    for name in ['candidate-image', 'private-reference-root', 'inspector', 'private-output-dir']:
        p.add_argument('--' + name, type=pathlib.Path, required=True)
    args = p.parse_args()
    os.environ['PYTHONDONTWRITEBYTECODE'] = '1'
    sys.dont_write_bytecode = True
    try:
        return run(args)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print('native boot candidate runtime refused: ' + str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
