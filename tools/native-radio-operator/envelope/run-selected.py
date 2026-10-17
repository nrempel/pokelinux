#!/usr/bin/env python3
"""Run a frozen installed public radio service ARM tracer with exact owned-CID absence closure."""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

sys.dont_write_bytecode = True
MAIN = Path('/home/ubuntu/Projects/pokelinux')
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def absent(value, identity):
    return value.returncode == 1 and any(text in value.stderr for text in
        ['No such object: ' + identity, 'No such container: ' + identity])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--selection-dir', required=True, type=Path)
    parser.add_argument('--selection-sha256', required=True)
    parser.add_argument('--command-sha256', required=True)
    args = parser.parse_args()
    root = args.selection_dir.resolve()
    assert root.parent == Path('/tmp') and root.name.startswith('pokelinux-radio-operator-green-arm-')
    assert sha(root / 'selection.json') == args.selection_sha256
    assert sha(root / 'command.json') == args.command_sha256
    selected = json.loads((root / 'selection.json').read_text())
    command = json.loads((root / 'command.json').read_text())
    assert selected['phase'] == 'radio-operator-green-arm' and selected['network'] == 'none'
    assert selected['Rust_Supervisor_owns_worker_wait'] and not selected['independent_guest_worker_reap_selected']
    assert selected['systemd_owns_service_Main_wait'] and selected['production_unit_bytes_installed_unchanged']
    assert selected['actual_status_CLIwait0_and_parsed_active_RAW_required_before_operator_oracle']
    assert selected['selected_slot_id'] == 1 and selected['GREEN_writer_channel_and_Ready_required']
    assert selected['single_current_status_history_key'] == 'last_observation'
    assert selected['request_history_is_independent']
    assert selected['installed_unit_KillMode'] == 'mixed'
    assert selected['installed_unit_ExecStop_present'] is False
    assert selected['case'] in ['accepted','expired-before-admission','permit-expires','stop-before-consumption','writer-held','manager-stop-held-writer']
    assert selected['expected_outcome'] == 'GREEN' and selected['strict_post_wait_cutoff_checks']
    assert set(selected['binaries']) == {'manager', 'peer', 'operator-controller', 'radio-worker', 'radio-service', 'radio-status'}
    assert not selected['automatic_retries_selected']
    assert not selected['PAM_or_authentication_selected'] and not selected['vendor_or_private_inputs_selected']
    assert not selected['phone_accessed'] and not selected['host_devices_attached']
    assert command[:len(DOCKER)] == DOCKER and '--privileged' not in command and '--device' not in command
    assert command[command.index('--network') + 1] == 'none'
    name = command[command.index('--name') + 1]
    assert name == 'pokelinux-radio-operator-' + selected['label']
    assert command[command.index('--cidfile') + 1] == str(root / 'owned-container-id')
    assert sha(__file__) == selected['files']['source/run-selected.py']
    assert all(not (root / entry).exists() for entry in ['execution.json', 'result.json', 'console.log',
                                                       'owned-container-id', 'output.lock'])
    lockpath = Path(selected['shared_input_lock'])
    assert lockpath == MAIN / 'artifacts/native-session-installed-app/vm-production.lock'
    image = subprocess.check_output(DOCKER + ['image', 'inspect', '--format', '{{.Id}}', selected['runtime_image']],
                                    text=True, timeout=10).strip()
    assert image == selected['runtime_image']
    daemon = subprocess.check_output(DOCKER + ['info', '--format', '{{.ID}}'], text=True, timeout=10).strip()
    assert daemon
    prior = subprocess.run(DOCKER + ['container', 'inspect', name], capture_output=True, text=True, timeout=5)
    assert absent(prior, name), 'exact label must be absent before owned run'
    record = {'schema': 1, 'scope': selected['scope'], 'runtime_image': image,
        'selection_sha256': args.selection_sha256, 'command_sha256': args.command_sha256,
        'runner_sha256': sha(__file__), 'docker_daemon_id_before': daemon, 'container_name': name,
        'direct_container_cli_wait': False, 'owned_container_absence_qualified': False,
        'timed_out': False, 'ownership_observations': [], 'normal_app_executed': False,
        'phone_accessed': False, 'hardware_qualified': False}

    def closure():
        for relative, digest in selected['files'].items():
            path = Path(relative)
            assert not path.is_absolute() and '..' not in path.parts
            assert sha(root / path) == digest, relative
        for path, digest in selected['external_inputs'].items():
            assert sha(path) == digest, path
        assert sha(root / 'selection.json') == args.selection_sha256
        assert sha(root / 'command.json') == args.command_sha256

    with (root / 'output.lock').open('xb') as output_lock, lockpath.open('rb') as input_lock:
        fcntl.flock(output_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fcntl.flock(input_lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        record['per_label_exclusive_lock_held'] = record['shared_readonly_input_lock_held'] = True
        closure()
        child = None
        cid = None
        live = False
        started = time.monotonic()
        registration_deadline = None
        next_inspection = 0
        error = None

        def published():
            nonlocal cid
            path = root / 'owned-container-id'
            if not path.exists():
                return
            body = path.read_bytes()
            assert len(body) <= 128
            value = body.decode('ascii').strip()
            if len(value) != 64 or any(c not in '0123456789abcdef' for c in value):
                return
            if cid is None:
                cid = value
                record['exact_owned_container_id'] = cid
            assert value == cid, 'owned CID publication changed'

        def inspection(phase):
            assert cid is not None
            result = subprocess.run(DOCKER + ['container', 'inspect', '--format',
                '{"id":{{json .Id}},"name":{{json .Name}},"image":{{json .Config.Image}},"state":{{json .State}}}', cid],
                capture_output=True, text=True, timeout=5)
            assert len(record['ownership_observations']) < 256
            row = {'phase': phase, 'identity': cid, 'exit': result.returncode,
                   'stdout': result.stdout[:8192], 'stderr': result.stderr[:8192],
                   'completed_monotonic_ns': time.monotonic_ns(), 'actual_inspect_CLI_wait': True}
            record['ownership_observations'].append(row)
            if result.returncode == 0:
                value = json.loads(result.stdout)
                assert value['id'] == cid and value['name'] == '/' + name and value['image'] == image
                return result, value['state']
            return result, None

        try:
            with (root / 'console.log').open('x') as log:
                record['started_monotonic_ns'] = time.monotonic_ns()
                child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
                record['owned_cli_pid'] = child.pid
                while child.poll() is None:
                    published()
                    now = time.monotonic()
                    if now >= started + selected['outer_deadline_seconds']:
                        record['timed_out'] = True
                        raise TimeoutError('owned service ARM outer deadline')
                    if cid is not None and not live:
                        if registration_deadline is None:
                            registration_deadline = now + 30
                        if now >= registration_deadline:
                            raise TimeoutError('same owned CID registration deadline')
                        if now >= next_inspection:
                            try:
                                observed, state = inspection('same-owned-CID-registration')
                                inspected_at = time.monotonic()
                                record['ownership_observations'][-1].update({
                                    'original_registration_cutoff_monotonic_ns': int(registration_deadline * 1e9),
                                    'inspection_completed_before_original_registration_cutoff': inspected_at < registration_deadline})
                                if inspected_at >= registration_deadline:
                                    raise TimeoutError('live owned CID inspection completed at or after original registration cutoff')
                                if state and state['Running'] and state['Pid'] > 1:
                                    live = True
                                    record['live_owned_identity_confirmed'] = True
                                    record['container_init_pid'] = state['Pid']
                                elif state and state['Status'] in ['exited', 'dead']:
                                    raise RuntimeError('same owned CID terminal during registration')
                                # Transient absent/unavailable inspection is retained and retried;
                                # no dependent QMP operation or one-failure guest kill exists here.
                            except subprocess.TimeoutExpired:
                                record['ownership_observations'].append({'phase': 'registration-inspect-timeout',
                                    'identity': cid, 'completed_monotonic_ns': time.monotonic_ns()})
                            next_inspection = time.monotonic() + .2
                    time.sleep(.1)
                published()
                record['returncode'] = child.wait()
                record['direct_container_cli_wait'] = True
                observed_wait_ns = time.monotonic_ns()
                observed_wait = observed_wait_ns / 1e9
                record['actual_CLI_wait_observed_monotonic_ns'] = observed_wait_ns
                record['actual_CLI_wait_before_original_outer_cutoff'] = observed_wait < started + selected['outer_deadline_seconds']
                if not record['actual_CLI_wait_before_original_outer_cutoff']:
                    record['timed_out'] = True
                    raise TimeoutError('actual owned Docker CLI wait observed after original outer cutoff')
        except BaseException as exception:
            error = type(exception).__name__ + ': ' + str(exception)
            if child is not None:
                published()
                if child.poll() is None and cid is not None:
                    try:
                        killed = subprocess.run(DOCKER + ['kill', cid], capture_output=True, text=True, timeout=5)
                        record['exact_owned_container_kill'] = {'exit': killed.returncode,
                            'stdout': killed.stdout[:2048], 'stderr': killed.stderr[:2048]}
                    except Exception as cleanup_error:
                        record['owned_container_kill_error'] = str(cleanup_error)
                try:
                    record['returncode'] = child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    child.kill()
                    record['returncode'] = child.wait(timeout=5)
                    record['owned_CLI_killed_for_bounded_host_wait'] = True
                record['direct_container_cli_wait'] = True
        finally:
            record['completed_monotonic_ns'] = time.monotonic_ns()
            if error:
                record['error'] = error
            try:
                assert cid is not None, 'no exact owned CID captured'
                until = time.monotonic() + 30
                while time.monotonic() < until:
                    observed, state = inspection('exact-owned-CID-after-direct-CLI-wait')
                    inspected_at = time.monotonic()
                    record['ownership_observations'][-1].update({
                        'original_closure_cutoff_monotonic_ns': int(until * 1e9),
                        'inspection_completed_before_original_closure_cutoff': inspected_at < until})
                    if inspected_at >= until:
                        raise TimeoutError('owned CID absence inspection completed at or after original closure cutoff')
                    if absent(observed, cid):
                        record['owned_container_absence_qualified'] = True
                        break
                    if state:
                        record['terminal_container_state'] = state
                    time.sleep(.2)
            except Exception as exception:
                record['terminal_probe_error'] = str(exception)
            if not record['owned_container_absence_qualified']:
                record['unresolved_owned_container_handle'] = cid or name
                record['successor_runtime_permitted'] = False
            record['direct_CLI_wait_scope'] = 'specific owned Docker CLI only'
            record['container_closure_authority'] = 'separate exact-owned-CID NoSuch inspection'
            try:
                record['docker_daemon_id_after'] = subprocess.check_output(
                    DOCKER + ['info', '--format', '{{.ID}}'], text=True, timeout=10).strip()
                assert record['docker_daemon_id_after'] == daemon
                record['same_docker_daemon_before_after'] = True
            except Exception as exception:
                record['same_docker_daemon_before_after'] = False
                record['daemon_closure_error'] = str(exception)
            try:
                closure()
                record['all_original_and_selected_inputs_unchanged'] = True
            except Exception as exception:
                record['all_original_and_selected_inputs_unchanged'] = False
                record['input_closure_error'] = str(exception)
            try:
                guest = json.loads((root / 'result.json').read_text()) if (root / 'result.json').exists() else None
            except (OSError, json.JSONDecodeError) as exception:
                guest = None
                record['guest_result_error'] = str(exception)
            record['guest_result_sha256'] = sha(root / 'result.json') if guest else None
            record['guest_reported_requirement_passed'] = bool(guest and guest.get('passed'))
            record['guest_reported_expected_outcome'] = bool(guest and guest.get('expected_outcome_observed'))
            record['guest_reported_desired_boundary_red'] = bool(guest and guest.get('desired_boundary_red'))
            record['guest_case_closure_qualified'] = bool(guest and guest.get('guest_case_closure_qualified'))
            qualified_closure = bool(record.get('returncode') == 0 and record['direct_container_cli_wait']
                and record.get('actual_CLI_wait_before_original_outer_cutoff') and live and not error
                and not record['timed_out'] and record['owned_container_absence_qualified']
                and record['same_docker_daemon_before_after'] and record['all_original_and_selected_inputs_unchanged']
                and record['guest_case_closure_qualified'])
            record['outer_runtime_and_guest_closure_qualified'] = qualified_closure
            record['expected_outcome_observed'] = bool(qualified_closure and record['guest_reported_expected_outcome'])
            record['desired_boundary_red'] = bool(qualified_closure and record['guest_reported_desired_boundary_red'])
            record['passed'] = bool(qualified_closure and record['guest_reported_requirement_passed'])
            with (root / 'execution.json').open('x') as output:
                json.dump(record, output, indent=2)
                output.write('\n')
    print(json.dumps({k: record.get(k) for k in ['passed', 'returncode', 'guest_result_sha256',
        'owned_container_absence_qualified', 'all_original_and_selected_inputs_unchanged', 'error']}, indent=2))
    return 0 if record['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
