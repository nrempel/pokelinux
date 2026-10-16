"""Authored source-only oracle; no implementation/runtime evidence is present.
Root supplies authenticated actual callsite records and independently retained
main-process/custody facts. Missing keys/records never become synthetic success.
"""

def classify(c):
    assert c['schema'] == 1 and c['case'] in ('BACKEND_READY', 'STOP_HELD')
    assert c['run'] == 1
    assert c['expected_shutdown_outcome'] == 'CLEAN_PENDING_NO_STOP_RETIRE'
    assert c['profile']['matching_native_cohort']
    assert c['profile']['AQ_SONAMEs'] == ['libaquamarine.so.15']
    assert c['profile']['mandatory_DRM_only']
    assert c['profile']['real_seatd_udev_virtio_primary_render_qualified']
    assert c['profile']['UID_GID'] == [1000, 1000] and c['profile']['extra_groups'] == []
    assert c['profile']['Pixel10_native'] == [1080, 2424, 2.625]
    assert c['profile']['Source_lease_ns'] == 100000000
    assert c['profile']['Source_cadence_ns'] == 25000000
    assert not c['profile']['standalone_or_headless_or_AQ14_fallback']
    assert not c['profile']['Source_packets_fabricated_or_retimed']
    main, worker = c['main_PID'], c['worker_TID']
    assert main > 1 and worker > 1 and worker != main
    records = c['records']; assert 0 < len(records) <= 64
    assert 0 < c['record_bytes'] <= 65536
    for v in records:
        assert v['schema'] == v['run'] == 1
        assert v['actual_sender_PID_UID_GID'] == [main, 1000, 1000]
        assert 0 < v['observed_boottime_ns'] <= v['Root_received_at_ns'] < c['work_until_ns']
        assert v['tid'] in (main, worker)
    def one(event, tid):
        values = [v for v in records if v['event'] == event]
        assert len(values) == 1, event
        v = values[0]; assert v['tid'] == tid
        return v
    def t(v): return v['observed_boottime_ns']
    def call(v):
        assert 0 < v['enter_ns'] <= v['return_ns'] <= t(v)
    batch = one('BROKER_INITIAL_BATCH_READY', main)
    assert batch['events'] == 3 and batch['graphics_devices'] == 1
    assert batch['real_service_lease_retained'] and not batch['queue_advanced']
    assert batch['session_generation'] > 0
    identity = batch['device']
    assert identity['device_lease_id'] > 0 and identity['device_generation'] > 0
    assert identity['primary_rdev'] > 0 and identity['render_rdev'] > 0
    assert identity['primary_rdev'] != identity['render_rdev']
    captured = one('RUST_BOOTSTRAP_CAPTURED', main); call(captured)
    assert t(batch) <= captured['enter_ns'] and captured['code'] == 0
    assert captured['events'] == 3 and captured['CLOEXEC_duplicates_owned']
    assert captured['source_pending_and_service_FDs_still_open']
    marked = one('BROKER_PREFIX_MARKED', main); call(marked)
    assert captured['return_ns'] <= marked['enter_ns']
    assert marked['sequences'] == [1, 2, 3]
    assert marked['pending_duplicates_closed'] and marked['original_service_lease_retained']
    assert marked['post_prefix_suffix_count'] == 0 and not marked['transport_failed']
    assert not marked['preworker_revocation_latched']
    watch = one('MAIN_WATCHES_REGISTERED', main)
    assert watch['broker_sources_control_only']
    assert watch['completion_borrowed'] and watch['startup_borrowed']
    assert watch['completion_fd'] >= 0 and watch['startup_fd'] >= 0
    assert watch['completion_fd'] != watch['startup_fd']
    created = one('NATIVE_HOLDER_CREATED', worker)
    assert t(captured) <= t(created) and created['CPU_only_empty']
    assert t(marked) <= t(created)
    mirror = one('MIRROR_ENUM_APPLIED', worker); call(mirror)
    assert t(created) <= mirror['enter_ns']
    assert mirror['session_generation'] == batch['session_generation']
    assert mirror['applied_sequences'] == [1, 2, 3] and mirror['all_apply_results'] == [0, 0, 0]
    assert mirror['owned_FDs_duplicated_from_Rust_borrows']
    assert mirror['device'] == identity and mirror['enumerated'] and mirror['active']
    backend = one('AQ_BACKEND_CREATED', worker); call(backend)
    assert mirror['return_ns'] <= backend['enter_ns'] and backend['nonnull']
    assert backend['request'] == 'ONE_MANDATORY_DRM' and backend['retained_before_start']
    assert backend['factory'] == 'createRenderSessionHolder'
    assert backend['CPU_only_empty'] and backend['retained_before_initialize']
    initialized = one('AQ_BACKEND_INITIALIZED', worker); call(initialized)
    assert backend['return_ns'] <= initialized['enter_ns']
    assert initialized['method'] == 'initializeRenderSession' and initialized['returned_true']
    assert initialized['child_retained_before_native_init'] and initialized['native_graph_retained']
    assert initialized['device'] == identity and initialized['request'] == 'ONE_MANDATORY_DRM'
    started = one('AQ_BACKEND_STARTED', worker); call(started)
    assert backend['return_ns'] <= started['enter_ns']
    assert initialized['return_ns'] <= started['enter_ns']
    assert started['output_subscription_retained_before_start']
    assert started['returned_true'] and started['allocator_retained']
    output = one('OUTPUT_COPIED', worker)
    # newOutput is synchronously emitted DURING backend.start(), not after it.
    assert started['enter_ns'] <= t(output) <= started['return_ns']
    assert output['device'] == identity and output['owner_registry_bound_to_real_output_mode']
    for k in ('output_id', 'output_generation', 'mode_id', 'mode_generation', 'connector_id'):
        assert output[k] > 0
    assert output['width'] > 0 and output['height'] > 0
    assert output['copied_values_only'] and not output['display_mutation_or_presentation_claim']
    returned = one('BOOTSTRAP_CANDIDATE_RETURNED', worker)
    assert started['return_ns'] <= t(returned)
    assert returned['session_generation'] == batch['session_generation']
    assert returned['outputs'] == 1 and returned['output_id'] == output['output_id']
    control = c['early_control_wire']
    assert control['transport'] == 'UNIX_STREAM'
    assert control['framing'] == 'RAW_UTF8_THEN_CLIENT_SHUT_WR_EOF'
    assert control['listener_nonblocking'] and control['accept_nonblocking']
    assert control['per_client_read_nonblocking'] and control['per_client_write_nonblocking']
    assert control['maximum_clients'] == 16 and control['maximum_request_bytes'] == 64
    assert control['maximum_response_bytes'] == 2048 and control['client_cutoff_ns'] == 2000000000
    def wire_request(q):
        actual = q['wire']; encoded = q['request'].encode('utf-8')
        assert actual['sent_request_utf8_hex'] == encoded.hex() and 0 < len(encoded) <= 64
        assert actual['client_SHUT_WR_completed'] and actual['server_request_EOF_observed']
        assert actual['accepted_socket_nonblocking'] and actual['read_and_write_nonblocking']
        assert actual['original_client_until_ns'] == actual['accepted_at_ns'] + 2000000000
        assert 0 < actual['accepted_at_ns'] <= actual['server_request_EOF_at_ns']
        assert actual['server_request_EOF_at_ns'] <= actual['dispatch_deadline_checked_at_ns']
        assert actual['dispatch_deadline_checked_at_ns'] <= actual['last_send_deadline_checked_at_ns']
        assert actual['last_send_deadline_checked_at_ns'] < actual['original_client_until_ns']
        assert actual['last_send_deadline_checked_at_ns'] <= q['return_ns']
        assert 0 < actual['send_attempt_count'] == actual['send_deadline_check_count']
        assert actual['client_received_response_EOF'] and actual['response_utf8_valid']
        assert type(actual['response_bytes']) is int and 0 < actual['response_bytes'] <= 2048
    queries = c['public_queries']; assert 0 < len(queries) <= 128
    for q in queries:
        assert q['request'] == 'owner-startup snapshot'
        wire_request(q)
        assert 0 < q['enter_ns'] <= q['reply']['observed_boottime_ns'] <= q['return_ns'] < c['work_until_ns']
        assert q['reply']['main_tid'] == main
        assert q['reply']['run'] == 1 and not q['reply']['failed']
        assert q['return_ns'] - q['enter_ns'] < c['frozen_request_bound_ns']
    stop = c['public_stop']
    assert stop['request'] == 'owner-startup stop'
    wire_request(stop)
    assert 0 < stop['enter_ns'] <= stop['reply']['observed_boottime_ns'] <= stop['return_ns'] < c['work_until_ns']
    assert stop['reply']['code'] == 0 and stop['reply']['main_tid'] == main
    assert stop['reply']['stopping'] and stop['reply']['join_code'] == -1
    assert stop['return_ns'] - stop['enter_ns'] < c['frozen_request_bound_ns']
    if c['case'] == 'BACKEND_READY':
        accepted = one('CONTROL_CANDIDATE_ACCEPTED', main)
        assert t(watch) <= t(accepted) and t(returned) <= t(accepted) < c['startup_not_after_ns']
        assert accepted['actual_Rust_ready_and_discovery'] and not accepted['failed']
        assert accepted['post_prefix_suffix_count'] == 0 and not accepted['transport_failed']
        assert not accepted['preworker_revocation_latched']
        assert accepted['output_id'] == output['output_id']
        assert accepted['output_generation'] == output['output_generation']
        assert accepted['copied_catalogue_bound_to_same_discovery']
        assert not accepted['shell_ready_or_native_frame_claim']
        assert t(accepted) < stop['enter_ns']
        assert any(q['reply']['phase'] == 'BACKEND_READY' for q in queries)
    else:
        hold = one('TEST_HOLD_ENTERED', worker)
        assert started['return_ns'] <= t(hold) < stop['enter_ns']
        assert hold['native_graph_retained']
        release = c['Root_hold_release']
        assert stop['return_ns'] < release['sent_at_ns'] < c['work_until_ns']
        assert release['sent_at_ns'] < c['startup_not_after_ns']
        assert release['actual_owned_channel_send'] and release['phase'] == 'AFTER_BACKEND_BEFORE_CANDIDATE'
        assert release['sent_at_ns'] <= t(returned)
        assert t(returned) < c['startup_not_after_ns']
        assert any(t(hold) < q['enter_ns'] <= q['return_ns'] < release['sent_at_ns'] for q in queries)
        assert not any(v['event'] == 'CONTROL_CANDIDATE_ACCEPTED' for v in records)
        assert any(q['enter_ns'] > t(returned) and q['reply']['stopping'] and not q['reply']['ready'] for q in queries)
    # This is an actual native callback return observation, not a fabricated
    # Rust Pending receipt. Current FFI maps exactly status0/retired0 to Pending.
    pending = one('NATIVE_SHUTDOWN_PENDING', worker); call(pending)
    assert stop['enter_ns'] <= pending['enter_ns'] and t(returned) <= pending['enter_ns']
    assert pending['callback_status'] == 0 and pending['resources_retired'] == 0
    assert pending['device'] == identity and pending['native_graph_retained']
    assert pending['holder_retained'] and pending['mirror_retained']
    assert pending['backend_retained'] and pending['allocator_and_outputs_retained']
    assert not pending['StopRetire_authority_claim'] and not pending['native_drain_claim']
    if c['case'] == 'STOP_HELD':
        # Worker cannot enter shutdown_progress until the bootstrap callback
        # has actually returned to Rust. This causal witness also precedes the
        # original startup deadline; expiry cannot explain nonacceptance.
        assert pending['enter_ns'] < c['startup_not_after_ns']
    post_return_queries = [q for q in queries if
        q['enter_ns'] > max(t(returned), pending['return_ns'], stop['return_ns']) and
        q['reply']['stopping'] and not q['reply']['ready']]
    assert post_return_queries, 'public stopping/nonready query after actual return/Pending'
    retained = c['retained_before_disposal']
    assert stop['return_ns'] < retained['observed_boottime_ns'] < c['close_until_ns']
    assert t(returned) < retained['observed_boottime_ns']
    assert t(pending) < retained['observed_boottime_ns']
    assert any(q['return_ns'] < retained['observed_boottime_ns'] for q in post_return_queries)
    assert retained['executor_retained'] and retained['native_owner_graph_retained']
    assert retained['actual_Rust_stopping'] and not retained['actual_Rust_ready']
    assert not retained['actual_Rust_failed']
    assert type(retained['actual_Rust_stage']) is int and 0 <= retained['actual_Rust_stage'] <= 8
    assert retained['actual_Rust_stage'] not in (6, 7)  # fault-quarantine/exited are not clean Pending
    assert retained['pidfd_alive'] and retained['exact_worker_task_exists']
    assert retained['worker_tid'] == worker and retained['actual_Rust_join_code'] == 2
    assert not retained['released'] and retained['both_borrowed_watches_registered']
    assert retained['service_lease_retained'] and not retained['native_retirement_claim']
    closure = c['owned_process_closure']
    assert closure['exact_PID'] == main and closure['pidfd_signal'] == 'SIGKILL'
    assert closure['actual_wait_exit'] == -9 and closure['actual_merged_PIPE_EOF']
    assert retained['observed_boottime_ns'] < closure['signal_at_ns'] <= closure['waited_at_ns'] < c['close_until_ns']
    assert closure['descendants_closed'] and closure['same_daemon_exact_CID_absent']
    assert closure['original_inputs_unchanged'] and not closure['native_cleanup_or_join_claim']
    return {'qualified': True, 'scope': 'ACTUAL_MAIN_BROKER_WORKER_BACKEND_COMPONENT_WITH_RETAINED_STOP_CUSTODY',
            'case': c['case'], 'Source_GREEN': False, 'full_UI_or_frame': False,
            'native_drain_or_service_retirement': False, 'phone': False,
            'shutdown_outcome': 'CLEAN_PENDING_NO_STOP_RETIRE',
            'fault_quarantine_qualified_as_clean_pending': False}
