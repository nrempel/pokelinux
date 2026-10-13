#!/usr/bin/env python3
"""Index separate actual candidate successes and the retained successor failure."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).parent
BASE = ROOT / 'artifacts/native-display-app-generations'
BUILD = 'app-generation-candidate-build-1'
LABELS = {
    'lifetime': 'app-generation-lifetime-green-arm-1',
    'cancel': 'app-generation-cancellation-green-arm-1',
    'held-init-stop': 'app-generation-held-init-stop-green-arm-1',
}


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block := stream.read(1048576):
            digest.update(block)
    return digest.hexdigest()


def main():
    build = BASE / BUILD
    selected = json.loads((build / 'selection.json').read_text())
    assert (build / 'exit-code').read_text().strip() == '0'
    assert selected['core_snapshot_verified_unchanged']
    for name, value in selected['sources'].items():
        assert sha(build / 'source' / name) == value
    for name, value in selected['bins'].items():
        assert sha(build / 'bins' / name) == value
    core = Path(selected['core_checkout'])
    for name, value in selected['core_source_checksums'].items():
        assert sha(core / name) == value, ('current core changed', name)
    epochs = {}
    for case, label in LABELS.items():
        out = BASE / label
        selection = json.loads((out / 'selection.json').read_text())
        result = json.loads((out / 'result.json').read_text())
        assert selection['rust_selection'] == selected
        assert selection['case'] == case
        for name, value in selection['files'].items():
            assert sha(out / name) == value
        assert result['all_tracked_children_finally_absent_or_directly_waited']
        assert all(v['passed'] for v in result['groups'])
        assert result['passed'] == (case != 'cancel')
        epochs[case] = {
            'label': label, 'result_sha256': sha(out / 'result.json'),
            'selection_sha256': sha(out / 'selection.json'),
            'aggregate_passed': result['passed'],
            'groups': result['groups'],
            'tracked_cleanup_outcomes': len(result['cleanup']),
            'all_tracked_cleanup_complete': True,
            'error': result.get('error'),
            'source_failed_events': [v for v in result['source_events'] if v['event'] == 'source_failed'],
        }
    cancel = json.loads((BASE / LABELS['cancel'] / 'result.json').read_text())
    assert len(cancel['groups']) == 1
    assert any(v['event'] == 'source_failed' and v['kind'] == 'InvalidData' for v in cancel['source_events'])
    paths = [build] + [BASE / label for label in LABELS.values()]
    proof = {str(q.relative_to(ROOT)): sha(q) for p in paths for q in sorted(p.rglob('*')) if q.is_file()}
    authored = {str(q.relative_to(ROOT)): sha(q) for q in sorted(TOOLS.rglob('*'))
                if q.is_file() and (q.name.startswith('green-') or 'app-context-fixture' in q.parts)
                and q.name != 'green-evidence.json' and '__pycache__' not in q.parts}
    value = {
        'scope': 'Actual ARM per-app context lifetime and held-initialization Stop pass separately. Pending EOF cancellation succeeds narrowly; its full cold-successor case FAILED. No aggregate full-matrix pass.',
        'selected_build': BUILD, 'selected_service_elf_sha256': selected['bins']['messages-app-context-fixture'],
        'retained_proof_root_checkout': str(ROOT),
        'selected_core_root_checkout': str(core),
        'selected_source_library_sha256': selected['source_library_sha256'],
        'core_snapshot_verified_unchanged': True,
        'current_core_source_checksums': selected['core_source_checksums'],
        'epochs': epochs,
        'proof_checksums': proof,
        'current_authored_tools_checksums': authored,
        'historical_RED_indexes': {name: sha(TOOLS / name) for name in ['setup-evidence.json', 'evidence.json', 'init-red-evidence.json', 'cancel-red-evidence.json']},
        'current_driver_equals_every_executed_driver': False,
        'failed_successor_diagnosis': {
            'retained_mirrored_frames': len(cancel['baseline_frames']),
            'first_rejected_frame_retained': False,
            'last_accepted_heartbeat_retained': False,
            'exact_InvalidData_predicate_qualified': False,
            'qualified': 'Actual Source InvalidData and subsequent context withdrawal; broad cold-successor availability gate. Semantic-change logs cannot diagnose a heartbeat gap or first rejected packet.',
            'authored_read_only_notes': 'tools/native-display-app-generations/green-source-failure-diagnosis.md',
        },
        'driver_revision_scope': 'Each actual epoch freezes its exact guest/run/source bytes. Held-init Stop support was added after lifetime freeze; later current diagnostic asserts command-line length before indexing. Current authored pins do not relabel earlier runtime drivers.',
        'visual_review': {
            'epoch': LABELS['lifetime'], 'reviewer': 'Sol implementation agent',
            'frames': ['first-generation-unknown-Source-denied.png', 'first-generation-current-focus-grant-history.png', 'second-generation-new-current-focus-existing-grant-history.png'],
            'observed': 'Unknown Source screen says Messages access denied. Both admitted generation screens say Local receipt history with modem receive/send unavailable and show +15550100. No literal message body visibility is claimed.',
        },
        'wait_scope': 'Direct Python subprocess waits are actual. Live-parent principal removal and SDK occupied=false bind to the selected compiled actor Child::try_wait and Supervisor Child::try_wait plus recorded pidfd death/proc absence where tracked. No independent helper-parent wait syscall trace. Failed cancellation successor SDK392 is not one of its18 tracked cleanup rows; its actual fixture terminal occupied=false event records native wait completion.',
        'limits': [
            'Original compositor/adapter and unchanged normal Qt are actual execution; provisioner/profile/seed/probes are authored isolated integration inputs.',
            'No accepted old client disconnect promise, metadata permission grant, Source revival, receive/backend/phone, secure presentation or production PAM lock qualification.',
            'The failed cancellation successor source InvalidData is retained as a real availability gate; its empty command-line IndexError is a secondary driver diagnostic. No successful successor/history qualification there.',
            'Observed intervals are not a hard realtime or existing read-stamp cancellation bound. Source100ms, setup1s and RPC2s remain unchanged.',
        ],
        'full_matrix_passed': False,
    }
    output = TOOLS / 'green-evidence.json'
    output.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'proof_paths': len(proof), 'current_authored': len(authored), 'current_core': len(selected['core_source_checksums']), 'index_sha256': sha(output)}, indent=2))


if __name__ == '__main__':
    main()
