#!/usr/bin/env python3
"""Index selected actual per-app lifetime RED separately from setup failures."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'artifacts/native-display-app-generations'


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--label', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    assert not args.output.exists(), 'Preserve each frozen public index'
    out = BASE / args.label
    selection = json.loads((out / 'selection.json').read_text())
    result = json.loads((out / 'result.json').read_text())
    assert result['desired_boundary_red'] and result['prerequisites_qualified'] and not result['passed']
    assert result['all_tracked_children_finally_absent_or_directly_waited'] and result['source_failed_before_desired_boundary'] is False
    assert len(result['groups']) == 7 and sum(g['passed'] for g in result['groups']) == 6
    failed = next(g for g in result['groups'] if not g['passed'])
    assert failed['id'] == 'future_per_app_display_generation_withdraws_old_listener_after_exact_app_reap'
    registry = failed['actual_allowed_registry']
    assert registry['allowed_bound'] == ['wl_compositor', 'xdg_wm_base'] and registry['uid'] == registry['gid'] == 11010
    assert registry['pid'] != failed['actual_reaped_app_pid']
    health = failed['post_reap_health']
    assert health['actor_snapshot']['principals'] == 0 and health['actor_snapshot']['ready'] and not health['actor_snapshot']['locked']
    assert health['display']['state'] == 'Ready' and health['display']['occupied'] and health['display']['admission']
    reap = result['app_parent_reap_inference']
    assert reap['app_pidfd_death_observed'] and reap['app_proc_absent'] and reap['original_parent_pidfd_living']
    assert reap['independent_syscall_trace'] is False
    assert result['Root_history_boundary']['app_read'] == {'reply': {'result': 'denied'}}
    for name, digest in selection['files'].items():
        assert sha(out / name) == digest, name
    source = out / 'source/rust-build'
    core = selection['rust_selection']['sources']
    for name, digest in core.items():
        assert sha(source / name) == digest, name
    actor = source / 'native-lifecycle/src/service.rs'
    actor_text = actor.read_text()
    assert '.filter_map(|(app, c)| match c.try_wait()' in actor_text
    assert 'authority.revoke(&name);\n                children.remove(&name);' in actor_text
    setup_path = Path(__file__).with_name('setup-evidence.json')
    setup = json.loads(setup_path.read_text())
    assert not setup['selected'] and not setup['desired_boundary_red'] and setup['cleanup_complete']
    for name, digest in setup['proof_checksums'].items():
        assert sha(ROOT / name) == digest, name
    screens = []
    for path, observation in [
        (BASE / setup['label'] / 'normal-unknown-source-denied.png', 'Unselected pre-Source setup: Messages access denied'),
        (out / 'normal-unknown-source-denied.png', 'Selected normal Qt: Messages access denied'),
        (out / 'normal-current-focus-grant-conversation.png', 'Selected normal Qt: Local receipt history and conversation sender +15550100; modem receive/send unavailable. Literal message body not visually qualified.'),
    ]:
        screens.append({'path': str(path.relative_to(ROOT)), 'sha256': sha(path), 'visually_reviewed': True, 'observation': observation})
    own = Path(__file__).parent
    evidence = {
        'scope': result['boundary_scope'], 'selected_label': args.label,
        'selected_result_sha256': sha(out / 'result.json'), 'selected_selection_sha256': sha(out / 'selection.json'),
        'selected_groups': {'passed_controls': 6, 'desired_future_boundary_failed': 1},
        'original_source': result['source'], 'app_parent_reap': reap, 'post_reap_same_live_service_health': health,
        'actual_post_reap_registry': registry, 'data_boundary_probe': result['Root_history_boundary'],
        'selected_service_elf_sha256': selection['service_elf_sha256'], 'normal_qt_elf_sha256': selection['normal_qt_elf_sha256'],
        'historical_runtime_source_checksums': core, 'actor_try_wait_source_sha256': sha(actor),
        'freshness_corrected_core_selected': False, 'independent_parent_wait_trace_qualified': False,
        'source_revival_permitted': False, 'no_cold_Qt_reopen': True, 'physical_phone_qualified': False,
        'cleanup': {'tracked_unique_pids': len({v['pid'] for v in result['cleanup']}),
                    'direct_python_parent_wait_or_final_helper_death_absence': result['all_tracked_children_finally_absent_or_directly_waited'],
                    'limit': 'Python directly waits its children. Service-owned helper records use pidfd death/proc absence; app parent reap additionally uses the retained original living parent and trusted compiled try_wait inference. No independent syscall trace.'},
        'visual_review': screens,
        'proof_checksums': {str(path.relative_to(ROOT)): sha(path) for path in sorted(out.rglob('*')) if path.is_file()},
        'unselected_setup_checksums': setup['proof_checksums'], 'setup_index_sha256': sha(setup_path),
        'current_authored_tools_checksums': {str(path.relative_to(ROOT)): sha(path) for path in sorted(own.iterdir())
             if path.is_file() and path != args.output.resolve() and path.suffix != '.pyc'},
        'limits': [
            'This is a new per-app lifetime requirement; a surviving service-epoch realm is consistent with the old selected contract.',
            'No new per-app implementation or GREEN was executed here; no generation replacement or relaunch is qualified.',
            'Original Source remains healthy through the post-reap registry. Whole-service Stop during final cleanup intentionally produces terminal Source/display transitions afterward.',
            'Close-to-observed-absence timing is an observation, not a hard cancellation or original-read deadline bound.',
            'Allowed display metadata/registry does not grant data, raw history, admin, sensor/capture or modem effect authority.',
        ],
    }
    args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'evidence': str(args.output), 'sha256': sha(args.output),
                      'selected_paths': len(evidence['proof_checksums']), 'setup_paths': len(evidence['unselected_setup_checksums']),
                      'runtime_sources': len(core), 'current_tools': len(evidence['current_authored_tools_checksums'])}, indent=2))


if __name__ == '__main__':
    main()
