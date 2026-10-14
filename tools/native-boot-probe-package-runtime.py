#!/usr/bin/env python3
"""Consume the current manifest-bound generic LZ4 ramdisk on the retained Arch kernel.

Only the authored generic ramdisk is booted. No Google kernel, vendor userspace,
root disk, network or physical device is attached. Original AOSP extraction,
the pinned Rust inspector, actual CPIO nodes and current native pins must agree.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parent.parent
KERNEL = pathlib.Path('/home/ubuntu/Projects/pokelinux/sim/boot/Image')
KERNEL_SHA = '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b'
DOCKER_IMAGE = 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
DOCKER = ['docker', '--host=unix:///var/run/docker.sock']


def digest(path):
    with pathlib.Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def load(name, path=None):
    path = path or REPO / 'tools' / (name.replace('_', '-') + '.py')
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def private(path):
    if any((parent / '.git').exists() for parent in [path, *path.parents]):
        raise ValueError('private output must be outside every Git worktree')


def absent(result, identity):
    return result.returncode == 1 and any(value in result.stderr for value in
        ['No such container: ' + identity, 'No such object: ' + identity])


def observed_native_completion(console, code, timed_out, diagnostics):
    checked = diagnostics.validate(console, mode='green')
    if (not isinstance(checked, dict) or type(checked.get('passed')) is not bool or
            not isinstance(checked.get('acks'), list) or not isinstance(checked.get('reason'), str)):
        raise ValueError('current diagnostic checker API required')
    children = re.findall(r'^POKELINUX_CHILD_EXEC pid=([1-9][0-9]*) parent=1$', console, re.M)
    waits = []
    for ack in checked['acks']:
        if not isinstance(ack, dict) or not isinstance(ack.get('message'), str):
            raise ValueError('typed diagnostic acknowledgement required')
        waited = re.fullmatch(r'POKELINUX_CHILD_WAIT pid=([1-9][0-9]*) '
                              r'actual_wait=true exit=Some\(37\)', ack['message'])
        if waited:
            waits.append(waited[1])
    matching_child = len(children) == 1 and waits == children and int(children[0]) > 1
    completed = not timed_out and code == 0 and checked['passed'] and matching_child
    return bool(completed), checked, int(children[0]) if matching_child else None


def owned_run(command, output, env, name):
    """Wait the owned CLI and independently establish exact owned-CID absence."""
    record = {'direct_cli_wait': False, 'timed_out': False, 'exact_CID_absence': False,
              'live_identity_confirmed': False, 'ownership_observations': [],
              'container_closure_authority': 'separate exact-owned-CID inspection',
              'CLI_wait_scope': 'owned Docker CLI only'}
    child, cid = None, None
    cidpath = output / 'owned-container-id'
    fmt = '{"id":{{json .Id}},"name":{{json .Name}},"image":{{json .Config.Image}},"state":{{json .State}}}'

    def invoke(arguments, timeout=5):
        return subprocess.run(DOCKER + arguments, capture_output=True, text=True, env=env, timeout=timeout)

    def published_cid():
        nonlocal cid
        if cidpath.exists():
            body = cidpath.read_bytes()
            if len(body) > 128:
                raise ValueError('bounded owned CID publication required')
            value = body.decode('ascii').strip()
            if re.fullmatch('[0-9a-f]{64}', value):
                if cid is not None and cid != value:
                    raise ValueError('owned CID publication changed')
                cid = value
        return cid

    def observe(result, phase):
        record['ownership_observations'].append({'phase': phase, 'exit': result.returncode,
            'stdout': result.stdout[:8192], 'stderr': result.stderr[:8192],
            'observed_monotonic_ns': time.monotonic_ns()})
        if len(record['ownership_observations']) > 256:
            raise ValueError('ownership observation bound exceeded')

    def identity(result):
        value = json.loads(result.stdout)
        if value['id'] != cid or value['name'] != '/' + name or value['image'] != DOCKER_IMAGE:
            raise ValueError('exact owned CID/name/image identity mismatch')
        return value['state']

    with (output / 'output.lock').open('xb') as lock, (output / 'console.log').open('xb') as log:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        record['started_monotonic_ns'] = time.monotonic_ns()
        try:
            before = invoke(['info', '--format', '{{.ID}}'])
            if before.returncode or not before.stdout.strip():
                raise ValueError('Docker daemon identity unavailable before launch')
            record['daemon_before'] = before.stdout.strip()
            prior = invoke(['container', 'inspect', name])
            record['prior_label_probe'] = {'exit': prior.returncode, 'stderr': prior.stderr[:8192]}
            if not absent(prior, name):
                raise ValueError('actual container-label absence required before launch')
            child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, env=env)
            record['owned_cli_pid'] = child.pid
            deadline = time.monotonic() + 45
            registration = min(deadline, time.monotonic() + 10)
            while child.poll() is None:
                if time.monotonic() >= deadline:
                    record['timed_out'] = True
                    raise TimeoutError('owned generic-kernel runtime deadline')
                if not record['live_identity_confirmed']:
                    if time.monotonic() >= registration:
                        raise TimeoutError('same-owned-CID registration deadline')
                    if published_cid() is not None:
                        live = invoke(['container', 'inspect', '--format', fmt, cid], timeout=3)
                        observe(live, 'same-CID-registration')
                        if live.returncode == 0:
                            state = identity(live)
                            if state['Running'] and state['Pid'] > 1:
                                record['live_identity_confirmed'] = True
                            elif state['Status'] in ['dead', 'exited']:
                                break
                        elif not absent(live, cid):
                            raise ValueError('owned-CID inspection prerequisite failed')
                        # A published CID may briefly precede Docker registration.
                        # Keep observing this same owned handle within the bound.
                time.sleep(.05)
            record['docker_exit'] = child.wait(timeout=max(.001, deadline - time.monotonic()))
            record['direct_cli_wait'] = True
        except BaseException as error:
            record['error'] = type(error).__name__ + ': ' + str(error)
        finally:
            if child is not None:
                try:
                    published_cid()
                except Exception as error:
                    record['CID_publication_error'] = str(error)
                if child.poll() is None and cid is not None:
                    try:
                        killed = invoke(['kill', cid])
                        record['exact_owned_kill'] = {'exit': killed.returncode, 'stderr': killed.stderr[:8192]}
                    except Exception as error:
                        record['exact_owned_kill_error'] = str(error)
                if not record['direct_cli_wait']:
                    try:
                        record['docker_exit'] = child.wait(timeout=10)
                        record['direct_cli_wait'] = True
                    except subprocess.TimeoutExpired:
                        # This closes only the CLI handle; container closure is
                        # established separately below and may remain unresolved.
                        child.kill()
                        record['CLI_killed_for_bounded_host_wait'] = True
                        try:
                            record['docker_exit'] = child.wait(timeout=5)
                            record['direct_cli_wait'] = True
                        except subprocess.TimeoutExpired:
                            record['unresolved_owned_CLI_pid'] = child.pid
                if cid is not None:
                    terminal_deadline = time.monotonic() + 5
                    while True:
                        try:
                            terminal = invoke(['container', 'inspect', '--format', fmt, cid], timeout=3)
                            observe(terminal, 'same-CID-terminal-or-absence')
                            if absent(terminal, cid):
                                record['exact_CID_absence'] = True
                                break
                            if terminal.returncode == 0:
                                state = identity(terminal)
                                record['last_container_state'] = state
                                if state['Running']:
                                    break
                            else:
                                record['terminal_probe_error'] = terminal.stderr[:8192]
                                break
                        except Exception as error:
                            record['terminal_probe_error'] = str(error)
                            break
                        if time.monotonic() >= terminal_deadline:
                            break
                        time.sleep(.05)
                if not record['exact_CID_absence']:
                    record['unresolved_owned_container_handle'] = cid or name
                    record['successor_runtime_permitted'] = False
            record['exact_owned_CID'] = cid
            try:
                after = invoke(['info', '--format', '{{.ID}}'])
                record['daemon_after'] = after.stdout.strip() if after.returncode == 0 else None
                record['same_daemon'] = bool(record.get('daemon_before') and
                    record['daemon_after'] == record['daemon_before'])
            except Exception as error:
                record['daemon_after_error'] = str(error)
                record['same_daemon'] = False
            record['completed_monotonic_ns'] = time.monotonic_ns()
    return record


def run(args):
    package = load('native_boot_probe_package')
    decoder = load('native_boot_probe_package_suite')
    checker_path = REPO / 'tools/native-boot-probe/diagnostics.py'
    package.regular(checker_path, 1024 * 1024)
    diagnostics = load('boot_diagnostic_checker', checker_path)
    pins_path = REPO / 'native-boot-package/input-pins.json'
    pins = package.small_json(pins_path)
    image = args.candidate_image.resolve()
    manifest_path = image.parent / 'manifest.json'
    output = args.private_output_dir.resolve()
    private(output)
    if output.exists():
        raise ValueError('output label already exists; preserve prior runtime evidence')
    if not re.fullmatch(r'[a-zA-Z0-9_-]+', output.name):
        raise ValueError('invalid immutable output label')
    if any(output == path or output in path.parents or path in output.parents for path in
           [image, KERNEL, args.inspector.resolve(), args.private_reference_root.resolve()]):
        raise ValueError('private output must not overlap inputs')
    package.regular(image, 8 * 1024 * 1024)
    package.regular(manifest_path, 1024 * 1024)
    image_sha = digest(image)
    manifest = package.small_json(manifest_path)
    if (manifest['schema'] != 2 or manifest['candidate_class'] != pins['candidate_class'] or
            manifest['image']['sha256'] != image_sha or manifest['image']['bytes'] != image.stat().st_size or
            manifest['unsigned'] is not True or manifest['immutable_inputs_unchanged'] is not True or
            manifest['immutable_selected_inputs'] != pins['selected'] or
            any(manifest['qualifications'][key] is not False for key in
                ['deployment_admitted', 'pixel_kernel_runtime_verified', 'avb_verified', 'physical_device_accessed'])):
        raise ValueError('exact current unsigned unqualified candidate manifest required')
    for name, field in [('init', 'root_init'), ('probe-child', 'diagnostic_child')]:
        if any(manifest[field][key] != pins['native_payload'][name][key] for key in ['bytes', 'sha256']):
            raise ValueError('manifest current native payload pin mismatch')
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
    if (ramdisk_sha != manifest['generic_ramdisk']['compressed_sha256'] or
            report['ramdisk']['size'] != manifest['generic_ramdisk']['compressed_bytes']):
        raise ValueError('manifest compressed ramdisk binding changed')
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    reference.original(args.private_reference_root, 'unpack_bootimg.py', [
        '--boot_img', image, '--out', output / 'original-unpack'])
    ramdisk = output / 'original-unpack/ramdisk'
    if ramdisk.stat().st_size != report['ramdisk']['size'] or digest(ramdisk) != ramdisk_sha:
        raise ValueError('actual original unpacker and Rust extent disagree')
    frames = decoder.decode_stream(ramdisk.read_bytes())
    if len(frames) != 1:
        raise ValueError('exact single generic LZ4 frame required')
    entries = decoder.archive(frames[0]['body'], strict=True)
    node = decoder.check_generic(entries, pins['native_payload'])
    if (decoder.sha(frames[0]['body']) != manifest['generic_ramdisk']['cpio_sha256'] or
            len(frames[0]['body']) != manifest['generic_ramdisk']['cpio_bytes'] or
            manifest['generic_ramdisk']['paths'] != ['dev', 'dev/kmsg', 'proc', 'sys', 'init', 'probe-child'] or
            manifest['generic_ramdisk']['device_nodes'] != [node]):
        raise ValueError('actual current CPIO and manifest disagree')
    selected_files = [image, manifest_path, KERNEL, ramdisk, args.inspector.resolve(), checker_path,
                      pins_path, pathlib.Path(__file__).resolve(),
                      REPO / 'tools/native-boot-probe-package.py',
                      REPO / 'tools/native-boot-probe-package-suite.py',
                      REPO / 'tools/native-boot-inputs-reference.py']
    selected_files.extend(args.private_reference_root / name for name in reference.SOURCE_HASHES)
    frozen = {str(path): digest(path) for path in selected_files}
    selection = {'candidate_image_sha256': image_sha, 'candidate_manifest_sha256': digest(manifest_path),
                 'kernel_sha256': KERNEL_SHA, 'exact_extracted_lz4_sha256': ramdisk_sha,
                 'current_native_payload': pins['native_payload'], 'cpio_device_nodes': [node],
                 'diagnostic_checker_sha256': digest(checker_path), 'generic_only': True,
                 'selected_input_hashes': frozen}
    (output / 'selection.json').write_text(json.dumps(selection, indent=2) + '\n')
    env = os.environ.copy()
    env['PYTHONDONTWRITEBYTECODE'] = '1'
    for key in ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_TLS', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']:
        env.pop(key, None)
    actual_image = subprocess.run(DOCKER + ['image', 'inspect', DOCKER_IMAGE, '--format', '{{.Id}}'],
        check=True, capture_output=True, text=True, env=env, timeout=5).stdout.strip()
    if actual_image != DOCKER_IMAGE:
        raise ValueError('pinned local Docker QEMU image required')
    name = 'pokelinux-boot-candidate-' + output.name
    qemu = ['qemu-system-aarch64', '-machine', 'virt,gic-version=3', '-cpu', 'max', '-smp', '2', '-m', '256',
            '-nic', 'none', '-nographic', '-monitor', 'none', '-no-reboot', '-kernel', '/inputs/Image',
            '-initrd', '/inputs/generic.lz4',
            '-append', 'console=ttyAMA0 earlycon=pl011,0x9000000 rdinit=/init panic=1']
    command = DOCKER + ['run', '--rm', '--pull=never', '--name', name,
        '--cidfile', str(output / 'owned-container-id'), '--network', 'none', '--read-only',
        '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges', '--cpus', '2', '--memory', '512m',
        '--pids-limit', '64', '-v', f'{KERNEL}:/inputs/Image:ro',
        '-v', f'{ramdisk}:/inputs/generic.lz4:ro', DOCKER_IMAGE] + qemu
    (output / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
    execution = owned_run(command, output, env, name)
    try:
        execution['all_inputs_unchanged'] = all(digest(path) == pin for path, pin in frozen.items())
    except OSError as error:
        execution['all_inputs_unchanged'] = False
        execution['input_closure_error'] = str(error)
    try:
        reference.sources(args.private_reference_root, False)
        execution['original_sources_unchanged'] = True
    except (ValueError, OSError) as error:
        execution['original_sources_unchanged'] = False
        execution['source_closure_error'] = str(error)
    (output / 'execution.json').write_text(json.dumps(execution, indent=2) + '\n')
    package.regular(output / 'console.log', 4 * 1024 * 1024)
    console = (output / 'console.log').read_text(errors='replace')
    try:
        native_completed, checked, child_pid = observed_native_completion(
            console, execution.get('docker_exit'), execution['timed_out'], diagnostics)
    except ValueError as error:
        native_completed, child_pid = False, None
        checked = {'passed': False, 'acks': [], 'reason': 'diagnostic checker API failure: ' + str(error)}
    passed = bool(native_completed and execution['live_identity_confirmed'] and
        execution['direct_cli_wait'] and execution['exact_CID_absence'] and
        execution['same_daemon'] and execution['all_inputs_unchanged'] and execution['original_sources_unchanged'] and
        not execution.get('error'))
    result = {'schema': 2, 'boundary': 'actual-current-init_boot-original-extracted-LZ4-to-Arch-kernel',
        'scope': 'standalone-authored-generic-only-no-vendor-ramdisk', **selection,
        'mode': 'green', 'passed': passed, 'native_completion_observed': native_completed,
        'diagnostic_readback': checked, 'docker_actual_wait': execution['direct_cli_wait'],
        'docker_exit': execution.get('docker_exit'), 'timed_out': execution['timed_out'],
        'exact_owned_CID_absence': execution['exact_CID_absence'], 'same_daemon': execution['same_daemon'],
        'all_inputs_unchanged': execution['all_inputs_unchanged'],
        'elapsed_seconds': round((execution['completed_monotonic_ns'] - execution['started_monotonic_ns']) / 1e9, 3),
        'console_sha256': digest(output / 'console.log'), 'child_exec_pid': child_pid,
        'actual_wait_child_exit': 37 if child_pid is not None else None,
        'pixel_kernel_booted': False, 'phone_accessed': False, 'vendor_userspace_executed': False,
        'root_disk_attached': False, 'network_attached': False, 'pstore_retention_qualified': False,
        'deployment_admitted': False}
    (output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
    return 0 if passed else 1


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('mode', choices=['green'])
    for name in ['candidate-image', 'private-reference-root', 'inspector', 'private-output-dir']:
        p.add_argument('--' + name, type=pathlib.Path, required=True)
    args = p.parse_args()
    os.environ['PYTHONDONTWRITEBYTECODE'] = '1'
    sys.dont_write_bytecode = True
    try:
        return run(args)
    except (ValueError, KeyError, OSError, AssertionError, subprocess.CalledProcessError,
            subprocess.TimeoutExpired, json.JSONDecodeError) as error:
        print('native boot candidate runtime refused: ' + str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
