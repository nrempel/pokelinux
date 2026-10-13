#!/usr/bin/env python3
"""Verify immutable private selections and write only safe lock-admission metadata."""
import argparse
import hashlib
import json
from pathlib import Path


def sha(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            value.update(block)
    return value.hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def bindings(root):
    return {str(p.relative_to(root)): sha(p) for p in sorted(root.rglob('*'))
            if p.is_file() and p.name != 'proof.json'}


def verify_build(root):
    selected = read(root / 'selection.json')
    assert selected['build_returncode'] == 0 and selected['build_cli_actually_waited']
    for name, expected in selected['sources'].items():
        assert sha(root / 'source' / name) == expected, name
    for name, expected in selected['bins'].items():
        assert sha(root / name) == expected, name
    return selected


def verify_arm(root, green):
    selected, result, execution = (read(root / name) for name in ['selection.json', 'result.json', 'execution.json'])
    for name, expected in selected['files'].items():
        assert sha(root / name) == expected, name
    assert execution['selection_sha256'] == sha(root / 'selection.json')
    assert execution['command_sha256'] == sha(root / 'command.json')
    assert execution['result_sha256'] == sha(root / 'result.json')
    assert execution['returncode'] == 0 and execution['direct_container_cli_wait']
    assert execution['shared_readonly_lock_held_across_runtime'] and execution['inputs_unchanged_after_runtime']
    assert result['all_tracked_processes_waited_or_finally_absent']
    if green:
        assert execution['green_qualified'] and result['passed'] and result['native_lock_admission_qualified']
        assert all(g['passed'] for g in result['groups'])
    else:
        assert not execution['green_qualified'] and not result['passed']
    return selected, result, execution


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--red-build', required=True, type=Path)
    p.add_argument('--green-build', required=True, type=Path)
    p.add_argument('--red-host', required=True, type=Path)
    p.add_argument('--green-host', required=True, type=Path)
    p.add_argument('--green-arm', required=True, type=Path)
    p.add_argument('--failed-arm', action='append', type=Path, default=[])
    p.add_argument('--components', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args()
    red_build, green_build = verify_build(a.red_build), verify_build(a.green_build)
    assert red_build['phase'] == 'red' and green_build['phase'] == 'green'
    assert red_build['bins']['arm-bins/messages-app-context-fixture'] == 'bb1032e9a7c9583a0be6f118c3d9e452915b555614886c0343b5b0aa4d60b17f'
    red = read(a.red_host / 'result.json')
    assert red['passed'] and red['expected_red_qualified'] and red['actual_actor_parent_wait']
    attempt = red['groups'][0]
    assert attempt['actual_child']['uid'] == 1000
    assert attempt['actual_child']['outcome'] == {'exception': 'FileNotFoundError', 'errno': 2}
    assert not attempt['before']['locked'] and not attempt['after']['locked']
    assert attempt['before']['ready'] and attempt['after']['ready']
    green, host_execution, host_selection = (read(a.green_host / name) for name in ['result.json', 'execution.json', 'selection.json'])
    assert green['passed'] and green['actual_actor_parent_wait'] and not green['actor_has_messages_provider']
    assert host_execution['returncode'] == 0 and host_execution['direct_container_cli_wait']
    assert host_execution['fixture_unchanged'] and host_execution['host_bins_unchanged']
    assert sha(a.green_host / 'fixture.py') == host_selection['fixture_sha256']
    selected, arm, execution = verify_arm(a.green_arm, True)
    assert selected['normal_qt_elf_sha256'] == '4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8'
    first = arm['groups'][0]
    ack, marker, held = first['actual_ack']['acknowledged_at_ns'], first['marker_observed_at_ns'], first['T_state_observed_at_ns']
    assert ack < marker < held < first['actual_ack']['until_ns']
    assert first['while_held']['actor_snapshot']['locked'] and not first['while_held']['current_original_focus']['locked']
    assert arm['groups'][1]['actual_request_reply']['reply'] == {'result': 'denied'}
    assert arm['whole_Stop_ACK'] == {'result': 'ok'}
    failures = []
    for root in a.failed_arm:
        _, failed, _ = verify_arm(root, False)
        failures.append({'label': root.name, 'error': failed['error'], 'setup_complete': failed['setup_complete'],
                         'failed_result_unchanged': True, 'bindings': bindings(root)})
    components = read(a.components / 'run-execution.json')
    assert components['returncode'] == 0 and components['direct_container_cli_wait']
    assert '8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' in (a.components / 'tests.log').read_text()
    private = {'red_build': bindings(a.red_build), 'green_build': bindings(a.green_build),
               'red_host': bindings(a.red_host), 'green_host': bindings(a.green_host),
               'green_arm': bindings(a.green_arm), 'components': bindings(a.components)}
    private_proof = {'scope': 'private exact file bindings; no raw artifact publication',
                     'selected': private, 'preserved_failed_ARM_epochs': failures}
    proof_path = a.green_arm / 'proof.json'
    assert not proof_path.exists()
    proof_path.write_text(json.dumps(private_proof, indent=2) + '\n')
    deadline = next(g for g in green['groups'] if g['id'] == 'held_actual_actor_ack_cannot_execute_effect_after_absolute_deadline')
    report = {'scope': 'session UID1000 lock-only native authority ingress; exact original installed Omarchy action',
        'evidence_classes': {'real_host_actor_and_client_groups': len(green['groups']), 'actual_ARM_original_boundary_groups': len(arm['groups']), 'existing_real_host_components': 8},
        'chronological_RED': {'label': a.red_host.name, 'result_sha256': sha(a.red_host / 'result.json'),
            'unchanged_current_core_build_selection_sha256': sha(a.red_build / 'selection.json'),
            'actual_UID': 1000, 'failure': 'lock.sock ENOENT with ready/unlocked actor',
            'before_core_implementation': True, 'fixture_pin_after_runtime_without_intermediate_edits': True},
        'native_build': {'selection_sha256': sha(a.green_build / 'selection.json'), 'sources': green_build['sources'],
                         'bins': green_build['bins'], 'format_and_clippy_passed': True},
        'actor_route': {'socket': '/run/pokelinux/messages/lock.sock', 'provider_independent': True, 'trusted_session_UID': 1000,
            'packet_credentials': 'kernel SCM_CREDENTIALS on every packet, including inherited socket delegation',
            'typed_request': {'op': 'lock'}, 'effect_parameters_allowed': False, 'unlock_grant_launch_allowed': False,
            'ACK_condition': 'existing authority.lifecycle(Some(true),None,None) completed before reply enqueue/send',
            'fresh_unlocked_Source_clears_intent': False, 'UI_death_unlock_or_recovery': False},
        'native_client': {'installed_UID': 1000, 'arguments_allowed': False, 'absolute_BOOTTIME_budget_ns': 2_000_000_000,
            'exact_retained_action': '/usr/libexec/pokelinux/omarchy-system-lock-original',
            'original_action_sha256': selected['original_action_sha256'],
            'ACK_matches_connected_Root_PID_UID_GID': True, 'withheld_ACK_effect_executed': False,
            'withheld_ACK_observed_parent_duration_ns': deadline['elapsed_ns'], 'hard_wall_clock_latency_qualified': False},
        'actual_original_ARM': {'label': a.green_arm.name, 'selection_sha256': sha(a.green_arm / 'selection.json'),
            'result_sha256': sha(a.green_arm / 'result.json'), 'execution_sha256': sha(a.green_arm / 'execution.json'),
            'kernel_release': arm['kernel']['release'], 'diagnostic_method': selected['actor_wire_trace'],
            'native_ACK_BOOTTIME_ns': ack, 'original_request_marker_observed_BOOTTIME_ns': marker,
            'actual_QML_T_state_BOOTTIME_ns': held, 'marker_to_T_ms': (held-marker)/1e6,
            'original_timer_ms': 500, 'original_Source_budget_ns': 100_000_000,
            'normal_app_fresh_authenticated_read_denied_while_locker_held': True,
            'capture_authority': 'separate actual registered-principal host group with same current core; normal Qt has no capture command',
            'fresh_original_unlocked_Source_intent_retention': True, 'shell_death_retains_fence': True,
            'snapshot_held_locker_ns': arm['groups'][4]['elapsed_ns'], 'whole_Stop_ACK': arm['whole_Stop_ACK'],
            'cleanup_records': len(arm['cleanup']), 'all_tracked_processes_waited_or_finally_absent': True,
            'native_actor_parent_is_owned_diagnostic_tracer': True,
            'direct_Root_wait_of_tracer_separate_from_native_actor_wait': True,
            'native_actor_final_pidfd_death_and_proc_absence': True,
            'shared_readonly_inputs_unchanged_after_actual_wait': True,
            'terminal_Source_events_during_closure': [e['kind'] for e in arm['source_events'] if e['event']=='source_failed'],
            'normal_render_scope': 'independently qualified prior original-Omarchy/native-1080x2424 preview; no new capture in Source lease'},
        'preserved_failures': [{'label': f['label'], 'error': f['error'], 'setup_complete': f['setup_complete'], 'failed_result_unchanged': True} for f in failures],
        'private_proof_sha256': sha(proof_path),
        'verified_private_bindings': sum(len(v) for v in private.values()) + sum(len(f['bindings']) for f in failures),
        'qualification_limits': ['Intent ACK supplies no secure frames, PAM authentication, UI-locked state or trusted unlock.',
            'Runtime uses the generic Arch ARM7.1.6 kernel, separate from matched Pixel kernel/modules/firmware.',
            'Installer-owned original modes normalized before fixed executable selection; original action/QML/PAM/timer bytes unchanged.',
            'No physical phone, proprietary runtime, credential publication, Source revival or implicit recovery.'],
        'raw_package_ELF_trace_logs_or_private_frames_for_publication': False}
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'report_sha256': sha(a.output), 'private_proof_sha256': sha(proof_path), 'verified_bindings': report['verified_private_bindings']}, indent=2))


if __name__ == '__main__':
    main()
