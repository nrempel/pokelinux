"""Pure paired oracle; Main process ownership is recorded by the existing consumer."""
MARKER = 'STATIC_MAIN_ENTRY_MUST_DERIVE_3S_CUTOFF_AND_REJECT_ABSOLUTE_ARG'


def qualify(static, absolute, *, static_consumer_exit, absolute_consumer_exit):
    for value, mode in [(static, 'static'), (absolute, 'absolute')]:
        assert value['entry'] == mode and value['observation_qualified']
        assert value['closure']['closed'] and value['closure']['wait_all_ECHILD']
        assert value['all_parent_handles_closed'] and value['namespace_handle_closed']
        assert not value['closure']['adopted_actual_waits']
        assert value['network_after'] == value['network_before']
        assert value['network_mutating_requests_admitted'] == 0
        assert value['first_statement_placement_runtime_proved'] is False
    desired = bool(static['passed'] and absolute['passed'])
    red = bool(static.get('argv_rejected_before_owned_admission')
               and absolute['legacy_absolute_control_observed']
               and not static['passed'] and not absolute['passed'])
    assert (desired and static_consumer_exit == absolute_consumer_exit == 0
            or red and static_consumer_exit == absolute_consumer_exit == 1)
    return {'passed': desired, 'behavior_RED_qualified': red,
            'legacy_control_proves_baseline_environment': bool(
                absolute['legacy_absolute_control_observed']),
            'static_entry_cutoff_derivation_observed': bool(
                static['derived_cutoff_in_parent_entry_interval']),
            'first_statement_placement_is_source_evidence_only': True,
            'no_installed_boot_or_interface_resolution_claim': True}


def assert_desired_after_cleanup(review):
    assert review['passed'], MARKER
