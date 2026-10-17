"""Independent sixth case: manager Stop while the actual admitted writer is held.

Source only until separately reviewed/frozen selection. No socket Stop, StopAck,
production owner import, transport shortcut, child kill or independent child reap.
"""
import json
import os
from pathlib import Path
import signal

from wire_assertions import status


def run(g):
    case = 'manager-stop-held-writer'
    assert g['selected']['case'] == case
    g['assert_proposed_ready']()
    now, until = g['now'], g['case_until']
    main_pid, incarnation = g['main_pid'], g['incarnation']
    digest = g['expected']['payload_sha256']
    assert now() < until and os.geteuid() == os.getegid() == 0
    records = []
    stopped = None

    def current(label):
        value = g['read_current_status'](label, until)
        assert value['phase'] == 'observed-active' and value['failure'] is None
        request = status(value, main_pid, incarnation, digest)
        records.append({'status': value})
        return value, request

    def zero(label):
        witness = g['read_independent_peer_state'](label, until)
        capture, report = witness['capture'], witness['report']
        assert capture['controller_uid'] == report['controller_uid'] == 1000
        assert capture['epoch'] == capture['count'] == capture['total'] == 0
        assert capture['owner_pid'] == report['owner_pid'] == g['worker_pid']
        assert capture['owner_uid'] == report['owner_uid'] == 1500
        assert report['registers'] == 1
        assert report['raw_other'] == report['segments'] == report['clears'] == 0
        records.append({'independent_peer': witness})
        return witness

    plain, request = current('manager-case-healthy-current-status')
    assert request is None
    assert plain['last_observation']['kind'] == 'raw'
    assert plain['last_observation']['serial'] == 7
    zero('manager-case-initial-zero105')
    configured, request = current('manager-case-config-alone-no-dispatch')
    assert request is None and configured['revision'] == plain['revision']
    zero('manager-case-configured-zero105')
    marker = Path(str(g['JOURNAL']) + '.add-pdn')
    assert not os.path.lexists(marker)

    try:
        stopped = g['writer_watch']
        assert g['alive'](stopped)
        signal.pidfd_send_signal(stopped, signal.SIGSTOP)
        g['wait_stopped_role']('writer', until)
        deadline = min(until - 5_000_000_000, now() + 8_000_000_000)
        body = {'schema': 1, 'incarnation': incarnation, 'attempt': 1, 'serial': 1,
                'request': 105, 'slot': 1, 'payload_sha256': digest, 'deadline_ns': deadline}
        rpc_until = min(until, now() + 3_000_000_000)
        code, data, _ = g['command'](['python3', g['FIXTURE'] / 'operator_client.py',
                '--socket', g['SOCKET'], '--server-pid', str(main_pid),
                '--until-ns', str(rpc_until), '--admit',
                json.dumps(body, separators=(',', ':'), sort_keys=True)],
                'manager-case-one-root-admit', rpc_until, expected=0, user=False, pipe=True)
        value = json.loads(data)
        assert code == 0 and value['classification'] == 'operator-reply'
        reply = value['reply']
        assert reply['admitted'] is True and reply['reason'] == 'admitted-pending-durability'
        assert reply['incarnation'] == incarnation and reply['attempt'] == 1
        records.append({'operator': value})
        first = None
        for index in range(3):
            value, request = current('manager-case-held-writer-responsive-' + str(index))
            assert request is not None and request.deadline == deadline
            assert request.outcome == 'outcome-unknown' and request.response is None
            assert value['revision'] == configured['revision']
            if first is None:
                first = request
            else:
                assert request.intent == first.intent and request.revision >= first.revision
            assert g['alive'](stopped) and g['alive'](g['main_watch'])
            assert g['alive'](g['worker_watch']) and not os.path.lexists(marker)
        zero('manager-case-admitted-held-zero105')
        assert 'StopAck_snapshot' not in g['result']
        manager = g['manager_stop'](until)
        assert manager['actual_systemctl_wait_exit'] == 0 and manager['StopAck_claimed'] is False
        assert manager['diagnostic_snapshot']['request_history'] is not None
        assert 'StopAck_snapshot' not in g['result']
        closure = g['close_unit_and_refuse_restart'](first)
        assert closure['native_radio_worker_wait']['wait_signal'] == 9
        assert closure['native_radio_worker_wait']['wait_code'] is None
        assert closure['native_writer_wait']['wait_signal'] == 9
        assert closure['native_writer_wait']['wait_code'] is None
        assert closure['Main_exit'] == 0 and closure['restart_exit'] == 78
        assert closure['unchanged_markers'] and closure['no_successor_lookup_or_registration_or105']
        assert g['result']['systemd_normal_Main_actualwait']['SERVICE_RESULT'] == 'success'
        assert g['result']['StopAck_claimed'] is False
        return {'classification': 'actual-manager-stop-case', 'case': case, 'oracle_satisfied': True,
                'records': records, 'manager_Stop': manager, 'closure': closure,
                'StopAck_claimed': False, 'response_crash_persistence_claimed': False}
    finally:
        if stopped is not None and g['alive'](stopped):
            # Only release the exact still-owned control stop for owner cleanup.
            # Native kill/wait remains exclusively with Main's Rust Supervisors.
            signal.pidfd_send_signal(stopped, signal.SIGCONT)
