"""Pure desired Main receive policy; assert only after the existing outer closure."""
MARKER = 'IDLE_EMPTY_MAIN_RECEIVE_MUST_KEEP_SAME_WORKER_FOR_FRESH_OBSERVATION'


def need(value, message):
    if not value:
        raise ValueError(message)


def common(r):
    need(r['prerequisites_qualified'] and r['custody_qualified']
         and not r.get('error') and not r['cleanup_errors'], 'actual setup and owned closure')
    need(not r['retry_or_reconnect_or_rollback'] and not r['Settings_or_VM_or_phone_executed'],
         'only selected normal host boundary')
    need(r['actual_effect_requests'] == 0 and r['initial_independent_readback']['ipv4'] == []
         and r['after_trace_independent_readback']['ipv4'] == [], 'zero actions and independent address absence')
    t = r['trace']
    need(t['one_fixture_owned_Main_only'] and not t['packet_payload_or_process_memory_read']
         and not t['syscall_results_changed'] and not t['effect_resubmission'], 'metadata-only owned Main trace')
    need(t['detached_at_actual_nonterminal_stop'] and t['Scope_retains_Main_terminal_wait']
         and not t.get('unexpected_terminal_tracer_wait'), 'normal Scope retains actual Main terminal wait')
    roles = t['source_role_proof']['roles']
    need(roles['roles_source_qualified'] and roles['readiness_poll_nfds'] == 3
         and roles['identity_poll_nfds'] == 2 and not roles['Main_other_three_fd_poll_paths'],
         'reviewed pinned source roles, not nfds alone')
    need(r['functional_completed_before_original_case'], 'one original functional aggregate')


def receive_failure(r):
    cause = r['actual_Main_worker_closure']['first_retirement']
    need(cause['reason'] == 'receive_failure' and cause['operation'] == 'receive_message'
         and cause['io_kind'] == 'TimedOut', 'actual Main first receive timeout, never a later Stop cause')
    stop = r['trace']['trace_stops'][0]
    until, detected = cause['original_until_ns'], cause['detected_at_ns']
    need(stop['kernel_stopped_at_ns'] < until <= stop['kernel_stopped_at_ns'] + 50_000_000
         and stop['actual_resume_started_at_ns'] >= until and detected >= stop['actual_resume_started_at_ns'],
         'same original50ms envelope actually straddles the controlled stop')
    need(r['worker_terminal_before_cleanup'] and r['actual_Main_exit'] == 0,
         'original worker terminal before cleanup with Main actual wait0')
    return {'original_envelope_until_ns': until, 'detected_at_ns': detected}


def qualify_idle(r, log):
    common(r)
    need(r['case'] == 'idle', 'selected Main idle case')
    park, trace = r['worker_safe_silence'], r['trace']
    need(park['safe_boundary'] == 'loop_sleep_entry_after_completed_observation_send_and_postchecks'
         and park['observation_send']['bytes'] in [48, 61]
         and park['SIGSTOP_via_retained_pidfd'] and park['detach_signal'] == 0
         and park['group_stop_confirmed'] and park['TracerPid'] == 0
         and park['normal_Main_retains_terminal_wait'], 'safe worker silence outside worker I/O envelopes')
    stops = trace['trace_stops']
    need(len(stops) == 1 and stops[0]['label'] == 'silent-worker-before-Main-auth'
         and stops[0]['requested_hold_ns'] == 80_000_000, 'one fixed80ms Main stop')
    anchor = trace['idle_gate_attribution']; no_data = anchor['prior_actual_no_data']
    need(park['group_stop_confirmed_at_ns'] < no_data['entry_ns'],
         'actual no-data observation occurs after the exact worker is safely stopped')
    need(no_data['kind'] in ['private_recvmsg_EAGAIN', 'source_qualified_three_fd_readiness_zero']
         and no_data['exit_ns'] < anchor['loop_sleep_exit_ns'] < stops[0]['kernel_stopped_at_ns'],
         'actual no-data, normal loop sleep, next gettid ordering')
    for value in [trace['worker_silence_before_hold'], trace['worker_silence_after_hold_before_Main_resume']]:
        need(value['group_stop_T'] and value['TracerPid'] == 0 and value['original_pidfd_nonterminal'],
             'same safely parked worker stays silent through Main hold')
    need(trace['post_hold_private_receives'] == [], 'no post-hold private receive before returning to loop')
    desired = bool(r.get('same_worker_fresh_after_idle') and not r['worker_terminal_before_cleanup'])
    detail = {}
    if desired:
        after = r['after_trace_inspect']['state']
        need(after['worker'] == r['initial_private_inspect']['state']['worker']
             and int(after['observed_at_ns']) > r['worker_SIGCONT_requested_at_ns']
             and after['available'] and after['configured'] is False and not after['pending'],
             'actual later fresh fact after same-worker SIGCONT')
        need(r['actual_Main_worker_closure']['first_retirement']['reason'] == 'external_stop',
             'no earlier candidate retirement hidden by cleanup')
    else:
        need(no_data['kind'] == 'private_recvmsg_EAGAIN', 'baseline RED requires actual empty receive')
        detail = receive_failure(r)
    return {'passed': desired, 'behavior_RED_qualified': not desired,
            'actual_outcome_confirmed': True, 'detail': detail}


def qualify_consumed(r, log):
    common(r)
    need(r['case'] == 'consumed', 'selected consumed Main record')
    trace, stop = r['trace'], r['trace']['trace_stops'][0]
    need(len(trace['trace_stops']) == 1 and stop['label'] == 'consumed-Main-receive-envelope-expiry'
         and stop['requested_hold_ns'] == 80_000_000, 'one controlled consumed-receive stop')
    frame = trace['actual_consumed_frame']
    need(frame['bytes'] in [48, 61] and not frame['contents_inspected']
         and frame['exit_stop_at_ns'] <= stop['kernel_stopped_at_ns'], 'actual private record consumed before hold')
    detail = receive_failure(r)
    need(frame['exit_stop_at_ns'] < detail['original_envelope_until_ns'],
         'selected record was consumed before its original envelope expired')
    need(trace['post_hold_private_receives'] == [], 'no timeout forgiveness or second receive')
    return {'passed': True, 'actual_consumed_bytes': frame['bytes'],
            'actual_Main_receive_failure': detail, 'no_late_success_or_retry': True}


def qualify_parent_death(r):
    need(r['prerequisites_qualified'] and r['custody_qualified'] and not r.get('error')
         and not r['cleanup_errors'] and r['case'] == 'parent-death'
         and r['actual_effect_requests'] == 0 and r['parent_death_control_passed'], 'unchanged parent-death control')
    need(r['actual_Main_exit'] == -9 and r['actual_Main_wait_and_EOF_before_adopted_worker_wait']
         and r['known_worker_adopted_actual_wait']['returncode'] == -9
         and r['independent_worker_pidfd_terminal_after_Main_wait'], 'actual known owned worker death and wait')
    return {'passed': True, 'actual_known_worker_parent_death_wait_signal': 9,
            'ordinary_Main_worker_wait_claimed': False}


def qualify_all(idle, consumed, death, *, all_outer_cleanup_qualified):
    need(all_outer_cleanup_qualified and consumed['passed'] and death['passed']
         and idle['actual_outcome_confirmed'], 'all controls and actual outer cleanup')
    return {'passed': bool(idle['passed']), 'behavior_RED_qualified': bool(idle['behavior_RED_qualified']),
            'consumed_expiry_control_passed': True, 'parent_death_control_passed': True}


def assert_desired_after_cleanup(review):
    assert review['passed'], MARKER
