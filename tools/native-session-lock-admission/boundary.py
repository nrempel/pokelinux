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

    for uid, value in [(0, {'op': 'lock'}), (11010, {'op': 'lock'}),
                       (1000, {'op': 'unlock'}), (1000, {'op': 'lock', 'locked': False}),
                       (1000, {'op': 'grant', 'app': 'org.omarchy.Messages', 'permission': 'capture', 'allowed': True}),
                       (1000, {'op': 'launch', 'app': 'org.omarchy.Messages'})]:
        negative = scope.lock_rpc(value, uid)
        assert negative == {'actual_uid': uid, 'reply': {'result': 'denied'}}, negative
    assert scope.admin({'op': 'snapshot'})['locked']
    result['groups'].append({'id': 'actual_ARM_unauthorized_UID_and_non_lock_packets_denied', 'passed': True})
    started = scope.now()
    snapshot = scope.admin({'op': 'snapshot'})
    result['groups'].append({'id': 'snapshot_responsive_with_original_locker_held', 'passed': True,
                             'snapshot': snapshot, 'elapsed_ns': scope.now() - started})
    signal.pidfd_send_signal(scope.shell_fd, signal.SIGKILL)
    scope.wait(lambda: scope.shell.poll() is not None, 2)
    scope.shell.wait(timeout=.25)
    assert scope.shell.returncode == -signal.SIGKILL
    assert select.select([scope.shell_fd], [], [], 0)[0] and not Path(f'/proc/{scope.shell.pid}').exists()
    after = scope.admin({'op': 'snapshot'})
    assert after['ready'] and after['locked'] and scope.actor.poll() is None
    assert not scope.original_focus()['locked']
    result['groups'].append({'id': 'actual_original_shell_death_does_not_unlock_or_recover', 'passed': True,
                             'snapshot_after_actual_wait': after, 'shell_wait_returncode': scope.shell.returncode})
    result['native_lock_admission_qualified'] = True
    result['lock_entry_prerequisites_qualified'] = True
