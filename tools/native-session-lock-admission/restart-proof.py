#!/usr/bin/env python3
"""Verify actual private restart epochs and emit safe authored metadata only."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def bindings(root):
    return {str(p.relative_to(root)): sha(p) for p in sorted(root.rglob('*'))
            if p.is_file() and p.name != 'restart-private-proof.json'}


def build(root):
    selected = read(root / 'selection.json')
    assert selected['build_returncode'] == 0 and selected['build_cli_actually_waited']
    for name, expected in selected['sources'].items():
        assert sha(root / 'source' / name) == expected, name
    for name, expected in selected['bins'].items():
        assert sha(root / name) == expected, name
    return selected


def arm(root, expected):
    selected, result, execution = (read(root / name) for name in ['selection.json', 'result.json', 'execution.json'])
    for name, digest in selected['files'].items():
        assert sha(root / name) == digest, name
    assert execution['selection_sha256'] == sha(root / 'selection.json')
    assert execution['command_sha256'] == sha(root / 'command.json')
    assert execution['result_sha256'] == sha(root / 'result.json')
    assert execution['returncode'] == 0 and execution['direct_container_cli_wait']
    assert execution['shared_readonly_lock_held_across_runtime'] and execution['inputs_unchanged_after_runtime']
    assert result['all_tracked_processes_waited_or_finally_absent']
    first = result['groups'][0]
    assert first['actual_ack']['acknowledged_at_ns'] < first['marker_observed_at_ns'] < first['T_state_observed_at_ns'] < first['actual_ack']['until_ns']
    assert first['while_held']['actor_snapshot']['locked'] and not first['while_held']['current_original_focus']['locked']
    assert selected['same_root_policy_directory'] and not selected['root_policy_reset']
    assert selected['restart_boundary'] and not selected['Source_continuation_qualified']
    group = next(g for g in result['groups'] if g['id'] == 'pending_native_lock_survives_actual_healthy_actor_and_changed_display_restart')
    restart = group['restart']
    assert restart['changed_display_epoch'] and restart['original_QML_remains_T_held']
    assert not restart['root_session_state_reset'] and not restart['Source_sequence_checks_relaxed']
    assert restart['old_actor_actual_wait_returncode'] == restart['old_source_actual_wait_returncode'] == 0
    assert group['after']['actor_snapshot']['ready'] and not group['after']['current_original_focus']['locked']
    assert group['actual_original_Source_fact']['frame']['source'] == restart['new_source_incarnation']
    if expected:
        assert execution['green_qualified'] and result['passed'] and all(g['passed'] for g in result['groups'])
        assert group['actual_fresh_authenticated_read']['reply'] == {'result': 'denied'}
        assert group['after']['actor_snapshot']['locked']
    else:
        assert execution['expected_red_qualified'] and result['expected_red_qualified'] and result['desired_boundary_red']
        assert not result['passed'] and group['actual_fresh_authenticated_read']['reply']['page'] == 'messages'
        assert not group['after']['actor_snapshot']['locked']
    assert restart['fresh_normal_app_ELF_sha256'] == '4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8'
    assert restart['fresh_Pixel10_profile_qualified']['qualified']
    assert restart['fresh_Pixel10_profile_qualified']['pixel_dimensions']['width'] == 1080
    assert restart['fresh_Pixel10_profile_qualified']['pixel_dimensions']['height'] == 2424
    overlay = read(root / 'overlay.json')
    assert overlay['format'] == 'qcow2' and overlay['full-backing-filename'] == '/vm/root.img'
    assert overlay['actual-size'] <= 268435456
    return selected, result, execution, group


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['red-build', 'green-build', 'red-arm', 'green-arm', 'host', 'components', 'output']:
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--failed', action='append', type=Path, default=[])
    a = parser.parse_args()
    red_build, green_build = build(a.red_build), build(a.green_build)
    assert red_build['phase'] == 'red' and green_build['phase'] == 'green'
    red, green = arm(a.red_arm, False), arm(a.green_arm, True)
    assert red[0]['native_build_selection_sha256'] == sha(a.red_build / 'selection.json')
    assert green[0]['native_build_selection_sha256'] == sha(a.green_build / 'selection.json')
    host, host_selection, host_execution = (read(a.host / name) for name in ['result.json', 'selection.json', 'execution.json'])
    assert host['passed'] and len(host['groups']) == 6 and all(g['passed'] for g in host['groups'])
    assert host['all_direct_children_actually_waited'] and not host['actual_UI_or_Source_runtime']
    assert host_selection['build_selection_sha256'] == sha(a.green_build / 'selection.json')
    assert host_execution['returncode'] == 0 and host_execution['direct_container_cli_wait']
    assert host_execution['fixture_unchanged'] and host_execution['bins_unchanged']
    assert sha(a.host / 'fixture.py') == host_selection['fixture_sha256']
    assert host_execution['result_sha256'] == sha(a.host / 'result.json')
    killed = next(g for g in host['groups'] if g['id'] == 'actual_actor_SIGKILL_after_synced_lock_before_ACK_retains_uncertainty')
    assert killed['persisted_before_actual_kill']['lock_pending']
    assert not killed['client']['authored_effect_executed'] and killed['successor']['actual_wait_returncode'] != 0
    components, component_selection = (read(a.components / name) for name in ['execution.json', 'selection.json'])
    assert components['compile_returncode'] == components['returncode'] == 0
    assert components['compile_cli_actually_waited'] and components['direct_container_cli_wait']
    assert components['ELF_unchanged_after_runtime']
    assert sha(a.components / 'component-tests') == component_selection['test_ELF_sha256']
    assert components['frozen_source_selection_sha256'] == sha(a.green_build / 'selection.json')
    count = re.search(r'test result: ok\. (\d+) passed; 0 failed;', (a.components / 'tests.log').read_text())
    assert count and int(count.group(1)) == 8
    private = {root.name: bindings(root) for root in [a.red_build, a.green_build, a.red_arm, a.green_arm, a.host, a.components, *a.failed]}
    failures = []
    for root in a.failed:
        execution = read(root / 'execution.json') if (root / 'execution.json').exists() else None
        result = read(root / 'result.json') if (root / 'result.json').exists() else None
        selected = read(root / 'selection.json') if (root / 'selection.json').exists() else None
        failures.append({'label': root.name, 'execution_returncode': execution.get('returncode') if execution else None,
                         'build_returncode': selected.get('build_returncode') if selected else None,
                         'error': result.get('error') if result else None,
                         'traceback': result.get('traceback') if result else None,
                         'desired_boundary_red': bool(result and result.get('desired_boundary_red')),
                         'failed_epoch_unchanged': True})
    proof = {'scope': 'private exact byte bindings; no raw artifact publication', 'selected': private,
             'failed_epochs': failures}
    private_path = a.green_arm / 'restart-private-proof.json'
    if private_path.exists():
        assert read(private_path) == proof
    else:
        private_path.write_text(json.dumps(proof, indent=2) + '\n')
    first = green[1]['groups'][0]
    report = {'scope': 'native pending lock survives actual healthy actor replacement with changed original compositor/display/Source epoch',
        'chronological_RED': {'label': a.red_arm.name, 'selection_sha256': sha(a.red_arm / 'selection.json'),
            'result_sha256': sha(a.red_arm / 'result.json'), 'execution_sha256': sha(a.red_arm / 'execution.json'),
            'pre_persistence_build_selection_sha256': sha(a.red_build / 'selection.json'),
            'fresh_unchanged_Qt_actual_authenticated_page_after_prior_native_lock_ACK': True,
            'same_root_policy_directory_and_clean_seal': True, 'changed_display_Source_epoch': True,
            'original_QML_T_held': True, 'whole_SIGKILL_reclassified_as_RED': False},
        'minimal_fix': {'field': 'Durable.lock_pending', 'owned_storage_process': True,
            'success_ACK_requires_confirmed_synced_replace': True, 'restored_before_ready': True,
            'unlocked_Source_can_clear_pending_intent': False, 'trusted_fresh_session_bootstrap': 'explicit root-owned storage lifecycle; never inferred from Source unlocked facts',
            'unknown_or_corrupt_recovery': 'denial through existing unclean/pending/seal/schema checks'},
        'native_build': {'selection_sha256': sha(a.green_build / 'selection.json'), 'sources': green_build['sources'],
            'bins': green_build['bins'], 'format_and_clippy_passed': True,
            'camera_provider_selected_in_original_ARM_runtime': False},
        'actual_original_ARM_GREEN': {'label': a.green_arm.name, 'selection_sha256': sha(a.green_arm / 'selection.json'),
            'result_sha256': sha(a.green_arm / 'result.json'), 'execution_sha256': sha(a.green_arm / 'execution.json'),
            'groups': len(green[1]['groups']), 'kernel': green[1]['kernel'], 'changed_display_Source_epoch': True,
            'same_policy_directory': True, 'fresh_Qt_authenticated_read': 'denied',
            'normal_Qt_ELF_sha256': green[3]['restart']['fresh_normal_app_ELF_sha256'],
            'native_ACK_BOOTTIME_ns': first['actual_ack']['acknowledged_at_ns'],
            'original_request_marker_BOOTTIME_ns': first['marker_observed_at_ns'],
            'QML_T_held_BOOTTIME_ns': first['T_state_observed_at_ns'],
            'Source_budget_ns': 100_000_000, 'original_timer_ms': 500,
            'Pixel10_native_pixels': {'width': 1080, 'height': 2424}, 'DPR': 2.625,
            'secure_presentation_or_new_capture_qualified': False,
            'explicit_readonly_backing_overlay_actual_bytes': read(a.green_arm / 'overlay.json')['actual-size'],
            'all_tracked_processes_waited_or_finally_absent': True,
            'direct_Root_wait_target': 'owned diagnostic strace parent',
            'native_actor_wait_scope': 'pidfd death and proc absence; native parent wait syscall not independently traced',
            'native_actor_direct_parent_wait_separate_from_Root_wait_of_owned_tracer': True},
        'real_host_controls': {'label': a.host.name, 'groups': [g['id'] for g in host['groups']],
            'result_sha256': sha(a.host / 'result.json'), 'whole_actor_SIGKILL': 'unclean same-directory restart refusal; separate from behavioral RED',
            'kill_before_ACK_authored_effect_executed': False, 'all_direct_children_actually_waited': True,
            'native_UI_or_Source_claim': False},
        'existing_components': {'passed': 8, 'exact_Cargo_selected_ELF_sha256': component_selection['test_ELF_sha256'],
            'execution_sha256': sha(a.components / 'execution.json')},
        'preserved_failures': [{**{k: v for k, v in failure.items() if k not in ['traceback', 'error']},
            'error_summary': (failure['error'].strip().splitlines()[-1][:240] if failure['error'] else None)}
            for failure in failures],
        'private_proof_sha256': sha(private_path), 'verified_private_bindings': sum(len(group) for group in private.values()),
        'qualification_limits': ['No trusted unlock/restore, PAM authentication or secure frames.',
            'Same Source continuation and fresh-endpoint reload on the old compositor remain unavailable.',
            'No physical phone, Pixel kernel/hardware, production install or camera merged runtime.',
            'No ACK refuses the original action; availability remains an explicit limitation.',
            'Storage failure fixtures are actual process/file controls, separate from power-loss or blocked physical IO.'],
        'raw_trace_package_binary_archive_or_private_frame_publication': False}
    a.output.write_text(json.dumps(report, indent=2) + chr(10))
    print(json.dumps({'report_sha256': sha(a.output), 'private_proof_sha256': sha(private_path),
                      'verified_private_bindings': report['verified_private_bindings']}, indent=2))


if __name__ == '__main__':
    main()
