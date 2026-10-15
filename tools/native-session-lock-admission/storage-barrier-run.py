#!/usr/bin/env python3
"""Run the ordinary storage-barrier fixture in one owned disposable container."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import uuid

IMAGE = 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
DOCKER = ['docker', '--host=unix:///var/run/docker.sock']
FIXTURE = Path(__file__).resolve().with_name('storage-barrier.py')
LABEL = 'io.pokelinux.storage-barrier-run'
BOUND = 60


def sha(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, default=FIXTURE)
    parser.add_argument('--common', type=Path)
    args = parser.parse_args()
    binary, fixture = args.binary.resolve(strict=True), args.fixture.resolve(strict=True)
    if not binary.is_file() or not fixture.is_file():
        parser.error('binary and fixture must be regular files')
    with binary.open('rb') as elf:
        if elf.read(4) != b'\x7fELF':
            parser.error('binary must be an ELF')
    output = args.output.absolute()
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    frozen, results = output / 'frozen', output / 'results'
    frozen.mkdir(mode=0o755)
    results.mkdir(mode=0o777)
    results.chmod(0o777)  # Container root has no DAC_OVERRIDE over this UID1000-owned bind.
    inputs = {}
    freeze = [('storage-barrier.py', fixture, 0o444), ('lifecycle-daemon', binary, 0o555)]
    if args.common:
        freeze.append(('storage-barrier-common.py', args.common.resolve(strict=True), 0o444))
    for name, original, mode in freeze:
        before = sha(original)
        target = frozen / name
        shutil.copyfile(original, target)
        target.chmod(mode)
        assert sha(target) == before == sha(original), f'input changed during freeze: {name}'
        inputs[name] = {'original': str(original), 'sha256': before}
    nonce = uuid.uuid4().hex
    name = 'pokelinux-storage-barrier-' + nonce
    env = os.environ.copy()
    for key in ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_TLS', 'DOCKER_TLS_VERIFY',
                'DOCKER_CERT_PATH']:
        env.pop(key, None)

    def docker(*args, timeout=5):
        return subprocess.run(DOCKER + list(args), env=env, stdin=subprocess.DEVNULL,
                              capture_output=True, text=True, timeout=timeout)

    def daemon_id():
        reply = docker('info', '--format', '{{.ID}}')
        if reply.returncode or not reply.stdout.strip():
            raise RuntimeError('cannot identify the explicitly selected Docker daemon: ' + reply.stderr)
        return reply.stdout.strip()

    def inspect(target):
        fmt = '{{json .Id}} {{json .Name}} {{json .State.Running}} {{json .State.Pid}} {{json (index .Config.Labels "' + LABEL + '")}}'
        reply = docker('inspect', target, '--format', fmt)
        if reply.returncode:
            return reply, None
        # The generated name, label and CID contain no whitespace.
        parts = reply.stdout.strip().split()
        if len(parts) != 5:
            raise RuntimeError('unexpected exact-container inspect output')
        cid, actual_name, running, pid, label = map(json.loads, parts)
        if not re.fullmatch('[0-9a-f]{64}', cid) or actual_name != '/' + name or label != nonce:
            raise RuntimeError('container ownership identity mismatch')
        return reply, {'cid': cid, 'name': actual_name, 'running': running, 'init_pid': pid}

    container_env = {'PATH': '/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin',
                     'TMPDIR': '/tmp', 'NATIVE_LIFECYCLE_ISOLATED': '1'}
    fixture_args = ['-I', '/source/storage-barrier.py', '--binary', '/bins/lifecycle-daemon',
                    '--results', '/results']
    if args.common:
        fixture_args += ['--common', '/source/storage-barrier-common.py']
    launch = ('import os,sys;os.environ.clear();os.environ.update(' + repr(container_env) +
              ');os.execv(sys.executable,[sys.executable]+' + repr(fixture_args) + ')')
    cidfile = output / 'cid'
    command = DOCKER + ['run', '--rm', '--pull=never', '--name', name, '--label', LABEL + '=' + nonce,
                       '--cidfile', str(cidfile), '--network', 'none', '--user', '0:0',
                       '--cap-drop', 'ALL', '--cap-add', 'SETUID', '--cap-add', 'SETGID',
                       '--cap-add', 'KILL', '--security-opt', 'no-new-privileges', '--read-only',
                       '--cpus', '2', '--memory', '512m', '--pids-limit', '128']
    for target, mode in [('/opt', '0755'), ('/run', '0755'), ('/var/lib', '0755'), ('/tmp', '0700')]:
        command += ['--tmpfs', f'{target}:rw,exec,nosuid,nodev,size=64m,mode={mode},uid=0,gid=0']
    mounts = [(frozen / 'storage-barrier.py', '/source/storage-barrier.py', True),
                                     (frozen / 'lifecycle-daemon', '/bins/lifecycle-daemon', True),
                                     (results, '/results', False)]
    if args.common:
        mounts.append((frozen / 'storage-barrier-common.py', '/source/storage-barrier-common.py', True))
    for source, target, readonly in mounts:
        command += ['--mount', f'type=bind,src={source},dst={target}' + (',readonly' if readonly else '')]
    command += ['--entrypoint', 'python3', IMAGE, '-I', '-c', launch]
    write(output / 'selection.json', {'image': IMAGE, 'inputs': inputs, 'runner_sha256': sha(Path(__file__)),
                                     'bound_seconds': BOUND, 'command': command})
    receipt = {'passed': False, 'timed_out': False, 'forced_cleanup': False,
               'actual_cli_wait': None, 'exact_live_container': None, 'cid': None}
    cli = None
    started = time.monotonic()
    try:
        receipt['daemon_before'] = daemon_id()
        with (output / 'console.log').open('xb') as console:
            cli = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL,
                                   stdout=console, stderr=subprocess.STDOUT)
            receipt['actual_cli_pid'] = cli.pid
            while cli.poll() is None:
                if cidfile.exists() and receipt['cid'] is None:
                    cid = cidfile.read_text().strip()
                    if cid and not re.fullmatch('[0-9a-f]{64}', cid):
                        raise RuntimeError('invalid owned CID file')
                    if cid:
                        receipt['cid'] = cid
                if receipt['cid'] and receipt['exact_live_container'] is None:
                    _, state = inspect(receipt['cid'])
                    if state and state['running'] and state['init_pid'] > 0:
                        receipt['exact_live_container'] = state
                if time.monotonic() - started >= BOUND:
                    receipt['timed_out'] = True
                    raise TimeoutError('original 60s container workload bound')
                time.sleep(.02)
            receipt['actual_cli_wait'] = cli.wait()
    except BaseException as error:
        receipt['error'] = repr(error)
    finally:
        try:
            if receipt['cid'] is None and cidfile.exists():
                cid = cidfile.read_text().strip()
                if re.fullmatch('[0-9a-f]{64}', cid):
                    receipt['cid'] = cid
            if receipt['cid'] is None:
                _, state = inspect(name)  # Verify nonce before adopting this run's exact CID.
                if state:
                    receipt['cid'] = state['cid']
            if cli is not None and cli.poll() is None:
                receipt['forced_cleanup'] = True
                if receipt['cid']:
                    _, state = inspect(receipt['cid'])
                    if state:
                        killed = docker('kill', receipt['cid'], timeout=10)
                        receipt['cleanup_kill_exit'] = killed.returncode
                try:
                    receipt['actual_cli_wait'] = cli.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    cli.kill()
                    receipt['cli_forced_kill'] = True
                    receipt['actual_cli_wait'] = cli.wait(timeout=5)
            elif cli is not None and receipt['actual_cli_wait'] is None:
                receipt['actual_cli_wait'] = cli.wait()
            if receipt['cid']:
                _, state = inspect(receipt['cid'])
                if state:
                    receipt['forced_cleanup'] = True
                    removed = docker('rm', '--force', receipt['cid'], timeout=10)
                    receipt['cleanup_remove_exit'] = removed.returncode
                absent = docker('inspect', receipt['cid'], '--format', '{{.Id}}')
                receipt['fresh_exact_cid_absence'] = {
                    'returncode': absent.returncode, 'stdout': absent.stdout, 'stderr': absent.stderr,
                    'confirmed': absent.returncode != 0 and not absent.stdout.strip() and
                    receipt['cid'] in absent.stderr and
                    any(s in absent.stderr.lower() for s in ['no such object', 'no such container'])}
            receipt['daemon_after'] = daemon_id()
            receipt['same_daemon'] = receipt.get('daemon_before') == receipt['daemon_after']
        except BaseException as error:
            receipt['cleanup_error'] = repr(error)
        finally:
            # A failed identity query must never skip the actual CLI wait.
            # Any forced fallback remains failed evidence, even if it closes.
            if cli is not None and receipt['actual_cli_wait'] is None:
                receipt['forced_cleanup'] = True
                try:
                    _, state = inspect(receipt['cid'] or name)
                    if state:
                        receipt['cid'] = state['cid']
                        docker('rm', '--force', state['cid'], timeout=10)
                except BaseException as error:
                    receipt['fallback_container_error'] = repr(error)
                try:
                    receipt['actual_cli_wait'] = cli.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    cli.kill()
                    receipt['actual_cli_wait'] = cli.wait(timeout=5)
        receipt['frozen_inputs_unchanged'] = {name: sha(frozen / name) == value['sha256']
                                               for name, value in inputs.items()}
        receipt['elapsed_seconds'] = time.monotonic() - started
        receipt['cleanup_complete'] = bool(receipt.get('same_daemon') and
            receipt.get('fresh_exact_cid_absence', {}).get('confirmed') and
            receipt['actual_cli_wait'] is not None)
        receipt['passed'] = bool(receipt['actual_cli_wait'] == 0 and receipt['cleanup_complete'] and
            receipt['exact_live_container'] and all(receipt['frozen_inputs_unchanged'].values()) and
            not receipt['forced_cleanup'] and not receipt['timed_out'] and 'error' not in receipt and
            'cleanup_error' not in receipt)
        write(output / 'execution.json', receipt)
    print(json.dumps(receipt, indent=2))
    if receipt['passed']:
        return 0
    code = receipt['actual_cli_wait']
    return code if isinstance(code, int) and 1 <= code <= 255 else 1


if __name__ == '__main__':
    raise SystemExit(main())
