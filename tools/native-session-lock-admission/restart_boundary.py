#!/usr/bin/env python3
"""Genuine installed native-before-original lock entry with a retained delayed locker."""
from pathlib import Path
import json
import re
import select
import signal

MARKER = re.compile(r'omarchy lock (\S+) lock-requested(?:\r?\n|$)')


def run(scope):
    result = scope.result
    before = scope.current_app_health()
    baseline = scope.wait(lambda: next((p for p in scope.wire_pairs() if p['reply'].get('page')), None), 3)
    offset = scope.shell_log.stat().st_size
    invoked = scope.now()
    action = scope.start_original_lock_action()
    until = invoked + 3_000_000_000
    marker = None
    while scope.now() < until:
        assert scope.shell.poll() is None
        with scope.shell_log.open('rb') as log:
            log.seek(offset)
            tail = log.read(65537)
        assert len(tail) <= 65536
        found = MARKER.search(tail.decode('utf-8', errors='replace'))
        if found:
            marker, observed = found.group(0), scope.now()
            signal.pidfd_send_signal(scope.shell_fd, signal.SIGSTOP)
            signaled = scope.now()
            break
        scope.sleep(.00025)
    assert marker is not None, 'fixed native entry did not execute the original lock action'
    state = scope.wait(lambda: (value if (value := Path(f'/proc/{scope.shell.pid}/stat').read_text().split()[2]) in ('T', 't') else None), .25)
    held = scope.now()
    health = scope.current_app_health(expected_locked=True)
    assert not scope.original_focus()['locked']
    assert health['current_original_focus']['pid'] == scope.app_pid
    action_log = scope.shell_log.parent / 'original-installed-system-lock.log'
    ack = next(json.loads(line) for line in action_log.read_text().splitlines()
               if line.startswith('{') and json.loads(line).get('event') == 'native_lock_intent_ack')
    assert invoked <= ack['acknowledged_at_ns'] < observed < held < ack['until_ns']
    assert not ack['secure_presentation_qualified']
    witness = {'invoked_at_ns': invoked, 'actual_ack': ack, 'marker_observed_at_ns': observed,
               'SIGSTOP_requested_at_ns': signaled, 'T_state_observed_at_ns': held,
               'original_wall_clock_marker': marker, 'actual_shell_pid': scope.shell.pid,
               'actual_shell_state': state, 'before': before, 'while_held': health,
               'actual_pre_lock_read': baseline, 'sample_budget_ns': 100_000_000,
               'original_timer_ms': 500, 'PAM_authentication_executed': False,
               'cached_pixels_prove_new_read': False, 'secure_presentation_qualified': False}
    scope.write_witness('native-lock-before-original-marker.json', witness)
    result['groups'].append({'id': 'actual_native_ACK_precedes_original_request_and_delayed_QML', 'passed': True, **witness})

    # The normal app's own unchanged polling worker makes this new read. Real
    # kernel credentials and request/reply fd pairing are retained in the trace.
    fresh_offset = scope.trace_offset()
    denied = scope.wait(lambda: next((p for p in scope.wire_pairs(fresh_offset)
                                      if p['reply'] == {'result': 'denied'}), None), 3)
    after_read = scope.current_app_health(expected_locked=True)
    assert Path(f'/proc/{scope.shell.pid}/stat').read_text().split()[2] in ('T', 't')
    assert after_read['current_original_focus']['pid'] == scope.app_pid
    assert not after_read['current_original_focus']['locked']
    result['groups'].append({'id': 'normal_app_fresh_read_denied_while_original_locker_T_held',
                             'passed': True, 'actual_request_reply': denied, 'health': after_read,
                             'capture_policy_scope': 'separate genuine registered-principal host actor group; normal Qt has no capture command'})
    fresh_unlocked = scope.wait(lambda: next((e for e in reversed(scope.original_source_packets())
        if not e['frame']['locked'] and e['frame']['sampled_at_ns'] > held + 100_000_000), None), 3)
    assert scope.admin({'op': 'snapshot'})['locked']
    result['groups'].append({'id': 'new_original_unlocked_Source_facts_cannot_clear_native_root_intent',
                             'passed': True, 'actual_fresh_source_fact': fresh_unlocked})

    restart = scope.restart_display_epoch()
    snapshot = scope.admin({'op': 'snapshot'})
    health = scope.current_app_health(expected_locked=snapshot['locked'])
    offset = scope.trace_offset()
    read = scope.wait(lambda: next((p for p in scope.wire_pairs(offset)
        if (p['reply'] == {'result': 'denied'} if snapshot['locked'] else p['reply'].get('page'))), None), 4)
    source = scope.wait(lambda: next((p for p in reversed(scope.original_source_packets())
        if not p['frame']['locked'] and (p['frame'].get('focus') or {}).get('pid') == scope.app_pid), None), 3)
    after = scope.current_app_health(expected_locked=snapshot['locked'])
    observation = {'id': 'pending_native_lock_survives_actual_healthy_actor_and_changed_display_restart',
        'passed': snapshot['locked'] and read['reply'] == {'result': 'denied'},
        'restart': restart, 'health': health, 'actual_fresh_authenticated_read': read,
        'actual_original_Source_fact': source, 'after': after}
    scope.write_witness('pending-lock-restart-boundary.json', observation)
    result['groups'].append(observation)
    result['desired_boundary_red'] = not observation['passed']
    result['expected_red_qualified'] = bool(result['desired_boundary_red'] and read['reply'].get('page'))
    assert observation['passed'], 'acknowledged native lock intent lost across healthy actor replacement'
    started = scope.now()
    status = scope.admin({'op': 'snapshot'})
    assert status['ready'] and status['locked']
    result['groups'].append({'id': 'restored_pending_lock_snapshot_responsive', 'passed': True,
        'snapshot': status, 'elapsed_ns': scope.now() - started})
    result['native_lock_admission_qualified'] = True
    result['lock_entry_prerequisites_qualified'] = True
    result['pending_lock_restart_qualified'] = True
