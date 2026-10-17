"""Concrete source-only GREEN phases for the existing owned installed-radio envelope.

All command paths below are real selected executables/sockets/Binder controls.
No production library imports, direct TransportSession, fake ABI or worker reap.
"""
import json
import os
from pathlib import Path
import re
import stat

from wire_assertions import status as typed_status

CASES = ('accepted', 'expired-before-admission', 'permit-expires',
         'stop-before-consumption', 'writer-held', 'manager-stop-held-writer')


def make_adapter(g):
    selected = g['selected']
    assert selected['phase'] == 'radio-operator-green-arm'
    assert selected['expected_outcome'] == 'GREEN' and selected['case'] in CASES
    assert g['result']['setup_complete'] and g['result']['actual_status_client_setup_qualified']
    now, command, wait_for = g['now'], g['command'], g['wait_for']
    until, setup_until = g['case_until'], g['setup_until']
    fixture, journal = g['FIXTURE'], g['JOURNAL']
    worker_pid, main_pid, incarnation = g['worker_pid'], g['main_pid'], g['incarnation']
    expected = json.loads((g['R'] / 'source/expected.json').read_text())
    assert expected['worker_uid'] == 1500 and expected['controller_uid'] == 1000
    assert expected['operator_uid'] == 0 and expected['route'] == [1, 42, 105, 1]
    payload = bytes.fromhex(expected['payload_hex'])
    assert len(payload) == 663
    assert g['sha'](g['R'] / 'source/config.json') == g['result']['pre_start_config']['sha256']
    role = {}
    operation = {}
    stop = {}
    markers = (journal, Path(str(journal) + '.writer'), Path(str(journal) + '.add-pdn'))

    def one_event(name, deadline, required=True):
        def current():
            values = [event for event in g['events']() if event['event'] == name]
            assert len(values) <= 1, 'duplicate actual Main event: ' + name
            return values[0] if values else None
        return wait_for(current, deadline, 'actual Main ' + name) if required else current()

    def file_state(path):
        if not os.path.lexists(path):
            return None
        metadata = path.lstat()
        assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == metadata.st_gid == 1500
        assert stat.S_IMODE(metadata.st_mode) == 0o600 and metadata.st_size <= 16384
        return (metadata.st_dev, metadata.st_ino, metadata.st_uid, metadata.st_gid,
                metadata.st_mode, path.read_bytes())

    def marker_state():
        return tuple(file_state(path) for path in markers)

    def marker_hashes(values):
        import hashlib
        return [{'present': value is not None,
                 'sha256': None if value is None else hashlib.sha256(value[5]).hexdigest(),
                 'device': None if value is None else value[0],
                 'inode': None if value is None else value[1]}
                for value in values]

    def descriptor_diagnostic(pid, child=False):
        # Read bounded proc metadata only. No foreign FD is opened or transferred;
        # these counts do not claim independent socket-peer authentication.
        assert now() < setup_until
        paths = list(Path(f'/proc/{pid}/fd').iterdir())
        assert len(paths) <= 64
        sockets = 0
        cloexec_sockets = 0
        vanished = 0
        for path in paths:
            assert now() < setup_until
            try:
                target = os.readlink(path)
                assert len(target) <= 4096
                text = Path(f'/proc/{pid}/fdinfo/{path.name}').read_text()
            except FileNotFoundError:
                # Proc FD enumeration is not an atomic descriptor snapshot.
                # A transient client FD may close between these metadata reads.
                vanished += 1
                continue
            assert len(text) <= 4096
            flags = int(next(line.split()[1] for line in text.splitlines()
                             if line.startswith('flags:')), 8)
            if target.startswith('socket:['):
                sockets += 1
                cloexec_sockets += bool(flags & os.O_CLOEXEC)
        if child:
            assert os.readlink(f'/proc/{pid}/fd/0') == '/dev/null'
        assert cloexec_sockets >= 1 and now() < setup_until
        assert all(g['alive'](handle) for handle in
                   (g['main_watch'], g['worker_watch'], role['writer_watch']))
        return {'descriptor_count': len(paths), 'socket_count': sockets,
                'CLOEXEC_socket_count': cloexec_sockets,
                'vanished_diagnostic_entries': vanished,
                'child_stdin_devnull': child,
                'FD_continuity_claimed': False,
                'independent_peer_authentication_claimed': False}

    def bind_ready():
        ready = one_event('control-ready', setup_until)
        writer_start = one_event('writer-start', setup_until)
        assert ready['main_pid'] == writer_start['main_pid'] == main_pid
        assert ready['worker_pid'] == worker_pid
        writer_pid = ready['writer_pid']
        assert type(writer_pid) is int and writer_pid > 1
        assert writer_pid not in (worker_pid, main_pid, g['peer'].pid, g['manager'].pid, os.getpid())
        writer_incarnation = ready['writer_incarnation']
        assert re.fullmatch('[0-9a-f]{32}', writer_incarnation) and writer_incarnation != incarnation
        assert writer_start['writer_pid'] == writer_pid
        assert ready['incarnation'] == writer_start['incarnation'] == incarnation
        assert writer_start['writer_incarnation'] == writer_incarnation
        session_until = ready['until_ns']
        assert session_until == writer_start['until_ns'] == g['until_ns'] and now() < session_until
        assert ready['config_sha256'] == g['sha'](g['R'] / 'source/config.json')
        assert ready['payload_sha256'] == expected['payload_sha256']
        assert ready['payload_length'] == len(payload)
        assert ready['retained_worker_channel'] is True and ready['retained_writer_channel'] is True
        writer_watch = os.pidfd_open(writer_pid, os.O_NONBLOCK)
        g['watches'].append(writer_watch)
        role.update(writer_pid=writer_pid, writer_watch=writer_watch,
                    writer_incarnation=writer_incarnation, session_until=session_until)
        g.update(writer_pid=writer_pid, writer_watch=writer_watch)
        writer_argv = [str(g['INSTALL'] / 'radio-worker'), '--pokelinux-process-worker',
                       '--intent-writer', '--journal', str(markers[2]),
                       '--until-ns', str(session_until)]
        writer_identity = g['identity'](writer_pid, selected['binaries']['radio-worker'],
                                        writer_argv, main_pid)
        assert g['first_state']['ControlGroup'] in writer_identity['proc_cgroup']
        assert g['alive'](g['main_watch']) and g['alive'](g['worker_watch']) and g['alive'](writer_watch)
        first = file_state(markers[0]); second = file_state(markers[1])
        assert first is not None and second is not None and file_state(markers[2]) is None
        assert first[5] == ('admission-v1\n' + incarnation + '\n1\n').encode()
        assert second[5] == ('admission-v1\n' + writer_incarnation + '\n1\n').encode()
        for prefix, value in [('radio', first), ('writer', second)]:
            assert ready[prefix + '_launch_device'] == value[0]
            assert ready[prefix + '_launch_inode'] == value[1]
            assert ready[prefix + '_launch_uid'] == value[2]
            assert ready[prefix + '_launch_mode'] == value[4]
            assert 0 < ready[prefix + '_launch_sync_ns'] < now() < session_until
        role['launch_markers'] = (first, second)
        g['result']['actual_native_writer_binding'] = writer_identity
        g['result']['control_ready'] = ready
        g['result']['retained_channel_authentication_scope'] = (
            'Source-bound actual READY/GO and role-admitted Supervisor receipts; shared channel '
            'separate real Linux boundary checks. Proc FD counts are metadata only; '
            'writer durability/wait and independent peer105 exercise actual role channels.')
        g['result']['role_descriptor_diagnostics'] = {
            'Main': descriptor_diagnostic(main_pid),
            'worker': descriptor_diagnostic(worker_pid, True),
            'writer': descriptor_diagnostic(writer_pid, True)}
        active = g['cgroup_snapshot'](g['first_state']['ControlGroup'], 'both-ready-owned-roles')
        assert set(active['procs'].split()) == {str(main_pid), str(worker_pid), str(writer_pid)}
        g['result']['control_ready_qualified'] = True

    bind_ready()

    def assert_proposed_ready():
        assert g['result']['control_ready_qualified']
        assert all(g['alive'](handle) for handle in
                   (g['main_watch'], g['worker_watch'], role['writer_watch']))
        assert marker_state()[:2] == role['launch_markers']
        assert file_state(markers[2]) is None and now() < setup_until

    def read_current_status(label, deadline):
        value = g['status'](label, min(deadline, until, now() + 3_000_000_000))
        typed_status(value, main_pid, incarnation, expected['payload_sha256'])
        return value

    def control(action, label, deadline):
        action_until = min(deadline, until, now() + 3_000_000_000)
        code, data, pid = command(
            ['setpriv', '--reuid=1000', '--regid=1000', '--clear-groups',
             fixture / 'operator-controller', action, str(action_until)],
            label, action_until, expected=0, user=False, pipe=True)
        value = json.loads(data)
        assert value['controller_uid'] == 1000 and value['kind'] == action and value['ok'] is True
        lookup = f'SERVICE-LOOKUP pid={pid} uid=1000 found=true'
        wait_for(lambda: lookup in g['lines']('manager.log'), action_until, 'exact independent control lookup')
        assert g['lines']('manager.log').count(lookup) == 1
        return value

    def peer_state(label, deadline):
        capture = control('capture', label + '-capture', deadline)
        count = 1 if selected['case'] == 'accepted' else 0
        assert capture['version'] == 1 and capture['epoch'] == 0
        assert capture['count'] == capture['total'] == count
        assert capture['owner_pid'] == worker_pid and capture['owner_uid'] == 1500
        if count:
            assert [capture[k] for k in ('serial', 'client_id', 'request', 'slot')] == [1, 42, 105, 1]
            assert capture['payload_hex'] == expected['payload_hex']
            assert capture['calling_pid'] == 0 and capture['calling_uid'] == 1500
        else:
            assert capture['payload_hex'] == ''
        report = control('report', label + '-report', deadline)
        assert report['registers'] == 1
        assert report['raw_other'] == report['segments'] == report['clears'] == 0
        assert report['owner_pid'] == worker_pid and report['owner_uid'] == 1500
        return {'capture': capture, 'report': report}

    def wait_stopped_role(name, deadline):
        pid = worker_pid if name == 'worker' else role['writer_pid']
        handle = g['worker_watch'] if name == 'worker' else role['writer_watch']
        def stopped():
            assert g['alive'](handle) and g['alive'](g['main_watch'])
            text = Path(f'/proc/{pid}/status').read_text()
            assert len(text) <= 16384
            line = next(line for line in text.splitlines() if line.startswith('State:'))
            return line.split()[1] == 'T'
        wait_for(stopped, deadline, 'original bound ' + name + ' stopped')
        g['result']['held_role'] = {'role': name, 'pid': pid, 'actual_proc_state': 'T',
                                    'signal_authority': 'original bound pidfd only'}

    def assert_exact_intent_record(path, radio, deadline, digest):
        assert Path(path) == markers[2] and radio == incarnation
        assert digest == expected['payload_sha256']
        if 'intent' not in operation:
            value = read_current_status('intent-record-current-history', min(deadline, until))
            request = typed_status(value, main_pid, incarnation, digest)
            assert request is not None and request.deadline == deadline
            operation.update(intent=request.intent, deadline=deadline)
        intent = operation['intent']
        assert re.fullmatch('[0-9a-f]{32}', intent) and operation['deadline'] == deadline
        lines = ['pokelinux-add-pdn-intent-v1', 'radio=' + incarnation,
                 'writer=' + role['writer_incarnation'], 'intent=' + intent,
                 'attempt=1', 'serial=1', 'client_id=42', 'request=105', 'slot=1',
                 'payload_length=' + str(len(payload)), 'payload_sha256=' + digest,
                 'deadline_ns=' + str(deadline)]
        actual = file_state(path)
        assert actual is not None and actual[5] == ('\n'.join(lines) + '\n').encode()
        operation['marker'] = actual
        return actual

    def writer_wait(deadline, normal=False):
        row = one_event('writer-reap', deadline)
        assert row['writer_pid'] == role['writer_pid']
        assert row['writer_incarnation'] == role['writer_incarnation']
        assert row['incarnation'] == incarnation
        assert row['occupied'] is False and row['admitted'] is False
        assert 0 < row['waited_at_ns'] < now()
        if normal:
            assert row['wait_code'] == 0 and row['wait_signal'] is None
            assert row['original_cleanup_until_ns'] is None
            assert row['waited_at_ns'] < operation['deadline']
        else:
            assert row['wait_code'] is None and row['wait_signal'] == 9
            assert row['waited_at_ns'] < row['original_cleanup_until_ns']
            assert row['original_cleanup_until_ns'] - row['waited_at_ns'] <= 2_000_000_000
        assert not g['alive'](role['writer_watch'])
        assert not Path('/proc/' + str(role['writer_pid'])).exists()
        g['result']['native_writer_actualwait'] = row
        return row

    def wait_verified_writer_sync_and_reap(deadline):
        value = read_current_status('awaiting-durable-intent', deadline)
        request = typed_status(value, main_pid, incarnation, expected['payload_sha256'])
        assert request is not None and request.deadline == deadline
        assert request.outcome == 'outcome-unknown' and request.response is None
        operation.update(intent=request.intent, deadline=deadline)
        sync = one_event('intent-synced', deadline)
        for key, expected_value in [('incarnation', incarnation),
                ('writer_incarnation', role['writer_incarnation']), ('intent', request.intent),
                ('deadline_ns', deadline), ('payload_sha256', expected['payload_sha256']),
                ('payload_length', len(payload))]:
            assert sync[key] == expected_value
        marker = assert_exact_intent_record(markers[2], incarnation, deadline, expected['payload_sha256'])
        assert (sync['device'], sync['inode'], sync['uid'], sync['mode']) == (marker[0], marker[1], marker[2], marker[4])
        assert 0 < sync['sync_completed_at_ns'] < deadline
        waited = writer_wait(deadline, normal=True)
        permit = one_event('permit-released', deadline)
        assert permit['incarnation'] == incarnation
        assert permit['writer_incarnation'] == role['writer_incarnation']
        assert permit['intent'] == request.intent and permit['deadline_ns'] == deadline
        assert permit['session_until_ns'] == role['session_until']
        assert permit['writer_waited_at_ns'] == waited['waited_at_ns']
        assert permit['sync_completed_at_ns'] == sync['sync_completed_at_ns']
        assert sync['sync_completed_at_ns'] <= waited['waited_at_ns']
        assert waited['waited_at_ns'] <= permit['admitted_at_ns'] <= permit['sent_at_ns']
        assert permit['sent_at_ns'] < min(deadline, role['session_until'])
        names = [event['event'] for event in g['events']()]
        assert names.index('intent-synced') < names.index('writer-reap') < names.index('permit-released')
        assert g['alive'](g['main_watch']) and g['alive'](g['worker_watch'])
        operation.update(sync=sync, permit=permit)
        g['result']['durable_permission_sequence'] = {
            'intent_synced': sync, 'normal_writer_actualwait': waited,
            'permit_released': permit, 'ACK_and_actual_wait_both_required': True,
            'writer_ACK_EXIT_wait0_source_bound': True}

    def wait_request_response(deadline):
        def response():
            value = read_current_status('await-request-response', deadline)
            request = typed_status(value, main_pid, incarnation, expected['payload_sha256'])
            assert request is not None and request.intent == operation['intent']
            return value if request.response is not None else None
        return wait_for(response, min(deadline, until), 'actual typed attributed response')

    def wait_not_sent(deadline):
        def not_sent():
            value = read_current_status('await-expired-permit-not-sent', deadline)
            request = typed_status(value, main_pid, incarnation, expected['payload_sha256'])
            assert request is not None and request.intent == operation['intent']
            assert request.response is None
            return value if request.outcome == 'not-sent' else None
        return wait_for(not_sent, min(deadline, until), 'actual expired-permit NotSent')

    def stop_with_snapshot(deadline):
        assert not stop
        stop['pre_peer'] = peer_state('pre-socket-stop', deadline)
        stop['markers'] = marker_state()
        assert stop['markers'][:2] == role['launch_markers']
        if selected['case'] in ('expired-before-admission', 'writer-held'):
            assert stop['markers'][2] is None
        else:
            assert stop['markers'][2] == operation['marker']
        # One Stop budget starts before its actual RPC and is reused through
        # both native waits/PID1; no fresh closing budget after receiving ACK.
        stop['started_at_ns'] = now()
        stop['until'] = min(deadline, until, stop['started_at_ns'] + 5_000_000_000)
        g['result']['Stop_operation_started_at_ns'] = stop['started_at_ns']
        g['result']['Stop_operation_start_scope'] = 'caller operation before Popen/RPC; not kernel packet send'
        rpc_until = min(stop['until'], stop['started_at_ns'] + 3_000_000_000)
        code, data, _ = command(['python3', fixture / 'operator_client.py', '--socket', g['SOCKET'],
                '--server-pid', str(main_pid), '--until-ns', str(rpc_until), '--stop'],
                'service-account-stop-snapshot', rpc_until, expected=0, user=True, pipe=True)
        value = json.loads(data)
        assert value['classification'] == 'stop-snapshot'
        snapshot = value['snapshot']
        typed_status(snapshot, main_pid, incarnation, expected['payload_sha256'])
        assert snapshot['phase'] == 'stopping'
        assert now() < stop['until']
        g['result']['StopAck_snapshot'] = snapshot
        g['result']['Stop_original_until_ns'] = stop['until']
        return snapshot

    def manager_stop(deadline):
        # Separate sixth-case entry: real manager Stop, never socket Stop/StopAck.
        assert selected['case'] == 'manager-stop-held-writer' and not stop
        assert all(g['alive'](handle) for handle in
                   (g['main_watch'], g['worker_watch'], role['writer_watch']))
        value = read_current_status('pre-manager-stop-diagnostic-snapshot', deadline)
        request = typed_status(value, main_pid, incarnation, expected['payload_sha256'])
        assert value['phase'] == 'observed-active' and request is not None
        assert request.outcome == 'outcome-unknown' and request.response is None
        stop['pre_peer'] = peer_state('pre-manager-stop', deadline)
        stop['markers'] = marker_state()
        assert stop['markers'][:2] == role['launch_markers'] and stop['markers'][2] is None
        assert one_event('intent-synced', deadline, required=False) is None
        assert one_event('permit-released', deadline, required=False) is None
        stop['started_at_ns'] = now()
        stop['until'] = min(deadline, until, stop['started_at_ns'] + 5_000_000_000)
        g['result']['Stop_operation_started_at_ns'] = stop['started_at_ns']
        g['result']['Stop_operation_start_scope'] = 'caller operation before Popen/RPC; not kernel packet send'
        code, _, pid = command(['systemctl', 'stop', g['UNIT']],
                'manager-stop-held-writer', stop['until'], expected=0)
        assert code == 0 and now() < stop['until']
        assert 'StopAck_snapshot' not in g['result']
        g['result']['pre_manager_Stop_diagnostic_snapshot'] = value
        g['result']['manager_Stop_actualwait'] = next(
                row for row in g['result']['cleanup'] if row['pid'] == pid)
        g['result']['Stop_original_until_ns'] = stop['until']
        g['result']['Stop_authority'] = 'actual systemctl stop; no socket Stop selected'
        g['result']['StopAck_claimed'] = False
        return {'actual_systemctl_wait_exit': code, 'original_until_ns': stop['until'],
                'diagnostic_snapshot': value, 'StopAck_claimed': False}

    def close_unit_and_refuse_restart(first):
        assert stop and now() < stop['until']
        waited = g['native_reap'](stop['until'], main_alive=False)
        normal_writer = selected['case'] not in ('expired-before-admission', 'writer-held', 'manager-stop-held-writer')
        writer = writer_wait(stop['until'], normal=normal_writer)
        def main_stopped():
            return (not g['alive'](g['main_watch']) and
                    not Path('/proc/' + str(main_pid)).exists() and len(g['receipts']()) == 1)
        wait_for(main_stopped, stop['until'], 'native Stop followed by actual PID1 Main outcome')
        first_receipt = g['receipts']()[0]
        assert first_receipt['INVOCATION_ID'] == g['first_invocation']
        assert (first_receipt['EXIT_CODE'], first_receipt['EXIT_STATUS'], first_receipt['SERVICE_RESULT']) == ('exited', '0', 'success')
        # Main already exited under its socket Stop. Manager confirmation cannot
        # substitute a signal or wait for either retained native child.
        if selected['case'] != 'manager-stop-held-writer':
            command(['systemctl', 'stop', g['UNIT']], 'already-natively-stopped-unit', stop['until'])
        g['cgroup_snapshot'](g['first_state']['ControlGroup'], 'normal-stop-after-both-native-and-Main-waits', empty=True)
        assert marker_state() == stop['markers']
        assert now() < stop['until']
        g['result']['systemd_normal_Main_actualwait'] = first_receipt
        g['result']['normal_unit_Stop_qualified'] = True
        restart_until = min(until, now() + 5_000_000_000)
        code, _, _ = command(['systemctl', 'start', g['UNIT']], 'same-markers-restart', restart_until, expected=None)
        g['result']['restart_systemctl_start_actual_wait_exit'] = code
        def refused():
            g['live_fixture'](require_peer=True, require_main=False)
            return g['event']('start-refused') and len(g['receipts']()) == 2
        wait_for(refused, restart_until, 'same-markers refusal and separate PID1 outcome')
        refusal = g['event']('start-refused')
        assert refusal['journal_retained'] is True and refusal['child_spawned'] is False
        assert refusal['reason'] == 'registration-uncertain'
        second = g['receipts']()[1]
        assert second['INVOCATION_ID'] != g['first_invocation']
        assert (second['EXIT_CODE'], second['EXIT_STATUS'], second['SERVICE_RESULT']) == ('exited', '78', 'exit-code')
        state = g['show']('same-markers-refusal-properties', restart_until)
        assert state['InvocationID'] == second['INVOCATION_ID'] and state['MainPID'] == '0'
        assert state['ActiveState'] == 'failed' and state['ExecMainStatus'] == '78'
        g['cgroup_snapshot'](g['first_state']['ControlGroup'], 'same-markers-refusal-empty-cgroup', empty=True)
        post = peer_state('post-refusal', restart_until)
        assert post == stop['pre_peer']
        assert marker_state() == stop['markers']
        events = g['events']()
        assert sum(event['event'] == 'worker-start' for event in events) == 1
        assert sum(event['event'] == 'writer-start' for event in events) == 1
        manager_lines = g['lines']('manager.log')
        worker_lookup = f'SERVICE-LOOKUP pid={worker_pid} uid=1500 found=true'
        assert [line for line in manager_lines if re.fullmatch(r'SERVICE-LOOKUP pid=\d+ uid=1500 found=true', line)] == [worker_lookup]
        controllers = [owner['child'].pid for owner in g['children'] if str(fixture / 'operator-controller') in owner['argv']]
        assert len([line for line in manager_lines if line.startswith('SERVICE-LOOKUP ')]) == 1 + len(controllers)
        assert all(manager_lines.count(f'SERVICE-LOOKUP pid={pid} uid=1000 found=true') == 1 for pid in controllers)
        assert not any(line.startswith('ADD-PDN-CAPTURE-RESET ') for line in g['lines']('peer.log'))
        assert now() < until
        g['result']['same_journal_restart_refusal'] = {'event': refusal, 'systemd_Main_actualwait': second,
                'properties': state, 'new_child_spawned': False, 'new_registration': False,
                'no_additional105': True}
        g['result']['journal_retained_and_unchanged'] = True
        g['result']['all_markers_retained_and_unchanged'] = marker_hashes(stop['markers'])
        g['result']['whole_first_start_case_completed_before_29s_cutoff'] = True
        g['result']['case_original_cutoff_ns'] = until
        g['result']['case_completed_boottime_ns'] = now()
        return {'native_radio_worker_wait': waited, 'native_writer_wait': writer,
                'Main_exit': 0, 'restart_exit': 78, 'unchanged_markers': True,
                'no_successor_lookup_or_registration_or105': True,
                'pre_stop': stop['pre_peer'], 'post_refusal': post,
                'response_crash_persistence_claimed': False}

    adapter = dict(g)
    adapter.update(expected=expected, writer_pid=role['writer_pid'], writer_watch=role['writer_watch'],
        assert_proposed_ready=assert_proposed_ready, read_current_status=read_current_status,
        wait_stopped_role=wait_stopped_role, assert_exact_intent_record=assert_exact_intent_record,
        wait_verified_writer_sync_and_reap=wait_verified_writer_sync_and_reap,
        wait_request_response=wait_request_response, wait_not_sent=wait_not_sent,
        stop_with_snapshot=stop_with_snapshot, manager_stop=manager_stop,
        read_independent_peer_state=peer_state,
        close_unit_and_refuse_restart=close_unit_and_refuse_restart)
    return adapter
