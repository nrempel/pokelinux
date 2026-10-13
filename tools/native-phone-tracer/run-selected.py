#!/usr/bin/env python3
"""Run one reviewed frozen tracer label, with actual owned Docker wait."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import time

MAIN = Path('/home/ubuntu/Projects/pokelinux')
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def absent(result, identity):
    diagnostic = result.stderr.decode(errors='replace') if isinstance(result.stderr, bytes) else result.stderr
    return result.returncode == 1 and any(text in diagnostic for text in
        ['No such object: ' + identity, 'No such container: ' + identity])


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--selection-dir', required=True, type=Path)
    p.add_argument('--selection-sha256', required=True)
    p.add_argument('--command-sha256', required=True)
    a = p.parse_args()
    root = a.selection_dir.resolve()
    assert Path('/dev/shm') in root.parents
    assert sha(root / 'selection.json') == a.selection_sha256
    assert sha(root / 'command.json') == a.command_sha256
    selected = json.loads((root / 'selection.json').read_text())
    command = json.loads((root / 'command.json').read_text())
    assert selected['phase'] == 'native-phone-tracer' and selected['network'] == 'none'
    assert selected['native_pixels'] == {'width': 1080, 'height': 2424}
    assert selected['source_budget_ns'] == 100_000_000 and not selected['Source_revival_or_reset_permitted']
    assert not selected['phone_accessed'] and not selected['host_devices_attached']
    assert command[:len(DOCKER)] == DOCKER
    assert '--privileged' not in command and '--device' not in command
    assert command[command.index('--network') + 1] == 'none'
    name = command[command.index('--name') + 1]
    assert name == 'pokelinux-phone-tracer-' + selected['label']
    assert command[command.index('--cidfile') + 1] == str(root / 'owned-container-id')
    assert all(not (root / file).exists() for file in ['execution.json', 'console.log', 'result.json', 'qmp.sock',
                                                      'owned-container-id', 'ownership-observer.json'])
    assert sha(__file__) == selected['files']['source/run-selected.py'], 'execute frozen reviewed runner'
    image = subprocess.check_output(DOCKER + ['image', 'inspect', '--format', '{{.Id}}', selected['runtime_image']], text=True).strip()
    assert image == selected['runtime_image']
    prior = subprocess.run(DOCKER + ['container', 'inspect', name], capture_output=True, timeout=5)
    assert absent(prior, name), 'must establish actual absence of container label'
    lockpath = Path(selected['shared_input_lock'])
    assert lockpath == MAIN / 'artifacts/native-session-installed-app/vm-production.lock'
    record = {'selection_sha256': a.selection_sha256, 'command_sha256': a.command_sha256,
              'runner_sha256': sha(__file__), 'runtime_image': image, 'container_name': name,
              'direct_container_cli_wait': False, 'timed_out': False, 'shared_readonly_lock_held': False,
              'per_label_exclusive_lock_held': False, 'keyboard_requests_completed': 0,
              'frontend_captures_completed': 0, 'frontend_constructor_placement_completed': 0}

    def closure():
        for relative, pin in selected['files'].items():
            path = Path(relative)
            assert not path.is_absolute() and '..' not in path.parts
            assert sha(root / path) == pin, 'frozen selected file changed: ' + relative
        for path, pin in selected['external_inputs'].items():
            assert sha(path) == pin, 'original input changed: ' + path
        assert sha(root / 'selection.json') == a.selection_sha256
        assert sha(root / 'command.json') == a.command_sha256

    with (root / 'output.lock').open('xb') as output_lock, lockpath.open('rb') as lock:
        fcntl.flock(output_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        record['shared_readonly_lock_held'] = record['per_label_exclusive_lock_held'] = True
        closure()
        spec = importlib.util.spec_from_file_location('tracer_keyboard', root / 'source/input.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        child = None
        cached_state = None
        known_container_id = None
        ownership_deadline = None
        observations = []
        previous_partial = None

        def observe(value):
            assert len(observations) < 256, 'bounded ownership inspection records'
            observations.append(value)
            temporary = root / 'ownership-observer.pending.json'
            temporary.write_text(json.dumps({'schema': 1, 'exact_published_owned_CID': known_container_id,
                'owned_cli_pid': child.pid if child is not None else None,
                'live_identity_confirmed': cached_state is not None,
                'observations': observations}, indent=2) + '\n')
            temporary.replace(root / 'ownership-observer.json')

        def owned():
            nonlocal cached_state, known_container_id, ownership_deadline, previous_partial
            assert child is not None and child.poll() is None, 'specific owned Docker CLI exited'
            if ownership_deadline is None:
                ownership_deadline = min(deadline, time.monotonic() + 30)
                record['ownership_registration_budget_seconds'] = 30
            while cached_state is None:
                assert child.poll() is None, 'owned Docker CLI exited before live identity confirmation'
                if time.monotonic() >= ownership_deadline:
                    raise TimeoutError('same-owned-CID registration/live-identity deadline')
                cidpath = root / 'owned-container-id'
                body = cidpath.read_bytes() if cidpath.exists() else b''
                assert len(body) <= 128, 'bounded Docker CID publication'
                value = body.decode('ascii').strip()
                if len(value) != 64 or any(c not in '0123456789abcdef' for c in value):
                    fingerprint = hashlib.sha256(body).hexdigest()
                    if fingerprint != previous_partial:
                        observe({'phase': 'CID-publication-in-progress', 'bytes': len(body), 'sha256': fingerprint,
                                 'observed_monotonic_ns': time.monotonic_ns(), 'owned_cli_live': True})
                        previous_partial = fingerprint
                    time.sleep(.1)
                    continue
                if known_container_id is None:
                    known_container_id = value
                    record['published_owned_container_id'] = value
                assert value == known_container_id, 'published owned CID changed'
                # Inspect only this CLI's complete published CID, never a name
                # lookup that may precede Docker's registration visibility.
                fmt = '{"id":{{json .Id}},"name":{{json .Name}},"image":{{json .Config.Image}},"state":{{json .State}}}'
                started = time.monotonic_ns()
                try:
                    result = subprocess.run(DOCKER + ['container', 'inspect', '--format', fmt, known_container_id],
                                            capture_output=True, text=True, timeout=5)
                except subprocess.TimeoutExpired as exception:
                    observe({'phase': 'same-CID-inspect-timeout', 'identity': known_container_id,
                             'started_monotonic_ns': started, 'completed_monotonic_ns': time.monotonic_ns(),
                             'stdout': str(exception.stdout or '')[:8192], 'stderr': str(exception.stderr or '')[:8192],
                             'owned_cli_live': child.poll() is None})
                    time.sleep(.1)
                    continue
                attempt = {'phase': 'same-CID-inspect', 'identity': known_container_id, 'actual_inspect_CLI_wait': True,
                           'exit': result.returncode, 'started_monotonic_ns': started,
                           'completed_monotonic_ns': time.monotonic_ns(),
                           'stdout': result.stdout[:8192], 'stderr': result.stderr[:8192],
                           'owned_cli_live': child.poll() is None}
                if result.returncode != 0:
                    attempt['classification'] = ('registration-not-yet-visible-while-owned-CLI-live'
                        if absent(result, known_container_id) and child.poll() is None else 'inspect-prerequisite-unavailable')
                    observe(attempt)
                    time.sleep(.1)
                    continue
                assert len(result.stdout) <= 65536
                value = json.loads(result.stdout)
                assert value['id'] == known_container_id and value['name'] == '/' + name and value['image'] == image
                state = value['state']
                if state['Running'] and state['Pid'] > 1:
                    cached_state = {'id': known_container_id, 'init_pid': state['Pid'], 'running': True}
                    record['container_id'] = known_container_id
                    record['container_init_pid'] = state['Pid']
                    attempt['classification'] = 'same-owned-CID-live-identity-confirmed'
                elif state['Status'] in ['exited', 'dead']:
                    attempt['classification'] = 'same-owned-CID-terminal-before-live-identity'
                    observe(attempt)
                    raise RuntimeError('owned container terminal before QMP prerequisites')
                else:
                    attempt['classification'] = 'same-owned-CID-starting-before-live-identity'
                observe(attempt)
                if cached_state is None:
                    time.sleep(.1)
            assert child.poll() is None
            return cached_state

        keyboard = module.Keyboard(root, owned)
        record['started_monotonic_ns'] = time.monotonic_ns()
        deadline = time.monotonic() + selected['outer_deadline_seconds']
        error = None
        try:
            with (root / 'console.log').open('x') as log:
                child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
                record['direct_cli_pid'] = child.pid
                while child.poll() is None:
                    if cached_state is None and (root / 'owned-container-id').exists():
                        owned()
                    if time.monotonic() >= deadline:
                        record['timed_out'] = True
                        raise TimeoutError('owned tracer outer deadline')
                    if keyboard.poll_placement():
                        record['frontend_constructor_placement_completed'] += 1
                    if keyboard.poll():
                        record['keyboard_requests_completed'] += 1
                    if keyboard.poll_capture():
                        record['frontend_captures_completed'] += 1
                    time.sleep(.1)
                record['returncode'] = child.wait()
                record['direct_container_cli_wait'] = True
        except BaseException as exception:
            error = type(exception).__name__ + ': ' + str(exception)
            if child is not None:
                if child.poll() is None:
                    if cached_state is None:
                        try:
                            owned()
                        except Exception:
                            pass
                    container_id = cached_state['id'] if cached_state is not None else None
                    if container_id is None and (root / 'owned-container-id').exists():
                        value = (root / 'owned-container-id').read_text().strip()
                        if len(value) == 64 and all(c in '0123456789abcdef' for c in value):
                            container_id = value
                    if container_id is not None:
                        try:
                            kill = subprocess.run(DOCKER + ['kill', container_id], capture_output=True, timeout=5)
                            record['owned_container_kill_exit'] = kill.returncode
                        except Exception as cleanup_error:
                            record['owned_container_kill_error'] = str(cleanup_error)
                    try:
                        record['returncode'] = child.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        record['returncode'] = child.wait(timeout=5)
                        record['owned_CLI_killed_for_bounded_host_wait'] = True
                else:
                    record['returncode'] = child.wait()
                record['direct_container_cli_wait'] = True
        finally:
            keyboard.close()
            record['completed_monotonic_ns'] = time.monotonic_ns()
            if error is not None:
                record['error'] = error
            container_id = cached_state['id'] if cached_state is not None else None
            if container_id is None and (root / 'owned-container-id').exists():
                value = (root / 'owned-container-id').read_text().strip()
                if len(value) == 64 and all(c in '0123456789abcdef' for c in value):
                    container_id = value
            record['exact_owned_container_id'] = container_id
            record['owned_container_terminal_or_absent'] = False
            record['owned_container_absence_qualified'] = False
            try:
                assert container_id is not None, 'no exact owned container identity captured'
                terminal = subprocess.run(DOCKER + ['container', 'inspect', '--format', '{{json .State}}', container_id],
                                          capture_output=True, text=True, timeout=5)
                record['terminal_container_probe_exit'] = terminal.returncode
                if terminal.returncode == 0:
                    state = json.loads(terminal.stdout)
                    record['terminal_container_state'] = state
                    record['owned_container_terminal_or_absent'] = bool(
                        not state['Running'] and state['Status'] in ['exited', 'dead'] and state['Pid'] == 0)
                elif absent(terminal, container_id):
                    record['owned_container_terminal_or_absent'] = True
                    record['owned_container_absence_qualified'] = True
                else:
                    record['terminal_container_probe_uncertain'] = terminal.stderr[:2048]
            except Exception as exception:
                record['terminal_container_probe_error'] = str(exception)
            if not record['owned_container_terminal_or_absent']:
                record['unresolved_owned_container_handle'] = container_id or name
                record['successor_runtime_permitted'] = False
            record['container_closure_authority'] = 'separate exact-owned-CID terminal/absence inspection'
            record['direct_CLI_wait_scope'] = 'owned Docker CLI only'
            record['ownership_observer_sha256'] = sha(root / 'ownership-observer.json') if observations else None
            try:
                closure()
                record['all_frozen_original_and_selected_inputs_unchanged'] = True
            except Exception as exception:
                record['all_frozen_original_and_selected_inputs_unchanged'] = False
                record['closure_error'] = str(exception)
            try:
                result = json.loads((root / 'result.json').read_text()) if (root / 'result.json').exists() else None
            except (OSError, json.JSONDecodeError) as exception:
                result = None
                record['guest_result_error'] = str(exception)
            record['result_sha256'] = sha(root / 'result.json') if result is not None else None
            record['guest_reported_passed'] = bool(result and result.get('passed'))
            record['passed'] = (record.get('returncode') == 0 and record['direct_container_cli_wait'] and
                                not record['timed_out'] and error is None and record['guest_reported_passed'] and
                                record['owned_container_terminal_or_absent'] and
                                record['all_frozen_original_and_selected_inputs_unchanged'])
            with (root / 'execution.json').open('x') as stream:
                json.dump(record, stream, indent=2)
                stream.write('\n')
    print(json.dumps(record, indent=2), flush=True)
    return 0 if record['passed'] else 1


if __name__ == '__main__':
    import sys
    sys.dont_write_bytecode = True
    raise SystemExit(main())
