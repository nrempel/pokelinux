#!/usr/bin/env python3
"""Freeze one authenticated diagnostic cause without relabeling prior proofs."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).parent
BASE = ROOT / 'artifacts/native-display-app-generations'
BUILD = 'app-generation-diagnostic-build-1'
EPOCH = 'app-generation-cancellation-diagnostic-arm-1'
SOURCE = 'd6fa11ae38c7cd6619b86f1b5ab8c996b6132d316274547c241418f335abaab0'
PATCH = 'c5006dd143eff76c37d5f716fac207fc8c44b88be57c1fb993e788e2a5a6d080'


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block := stream.read(1048576):
            digest.update(block)
    return digest.hexdigest()


def main():
    old = json.loads((TOOLS / 'green-evidence.json').read_text())
    for key in ['proof_checksums', 'current_authored_tools_checksums']:
        for name, value in old[key].items():
            assert sha(ROOT / name) == value, ('old proof changed', name)
    build, out = BASE / BUILD, BASE / EPOCH
    selected = json.loads((build / 'selection.json').read_text())
    selection = json.loads((out / 'selection.json').read_text())
    result = json.loads((out / 'result.json').read_text())
    assert sha(TOOLS / 'diagnostic-source.patch') == PATCH
    assert selected['source_library_sha256'] == SOURCE
    assert selected['diagnostic_only_source_differences'] == ['native-session/src/lib.rs']
    assert sha(build / 'source/native-session/src/lib.rs') == SOURCE
    original = BASE / 'app-generation-candidate-build-1/source'
    unchanged = {name: value for name, value in selected['core_source_checksums'].items()
                 if name != 'native-session/src/lib.rs'}
    assert len(unchanged) == 77
    for name, value in unchanged.items():
        assert sha(original / name) == value and sha(build / 'source' / name) == value
    assert selection['rust_selection'] == selected
    for name, value in selected['sources'].items():
        assert sha(build / 'source' / name) == value
    for name, value in selected['bins'].items():
        assert sha(build / 'bins' / name) == value
    for name, value in selection['files'].items():
        assert sha(out / name) == value
    assert result['diagnostic_trace_qualified'] and not result['diagnostic_trace_errors']
    assert not result['passed'] and len(result['groups']) == 1 and result['groups'][0]['passed']
    assert result['all_tracked_children_finally_absent_or_directly_waited']
    records = result['diagnostic_records']
    rejection, summary = records
    assert rejection['category'] == 10 and summary['category'] == 0
    assert rejection['sequence'] == rejection['previous_sequence'] + 1
    gap = rejection['sampled_at_ns'] - rejection['previous_sampled_at_ns']
    assert gap == 103208749 and gap > 100000000
    assert rejection['examination_boottime_ns'] == 0 and rejection['drain_count'] == 1
    assert summary['attempted'] == 2 and summary['sent_before_record'] == 1
    assert all(v['dropped_before_record'] == 0 for v in records)
    assert all(v['authenticated_kernel_credentials'] == [386, 0, 0] for v in records)
    adapter = Path('/home/ubuntu/Projects/pokelinux/artifacts/native-session-compositor/green-owned-source-final/source/adapter.cpp')
    assert sha(adapter) == '8a7a5c2a6a5a3a40aabe2583bdc2fda9fded8619804695dbec663c23ca6f7501'
    proof = {str(q.relative_to(ROOT)): sha(q) for p in [build, out]
             for q in sorted(p.rglob('*')) if q.is_file()}
    authored = {str(q.relative_to(ROOT)): sha(q) for q in sorted(TOOLS.glob('diagnostic-*'))
                if q.is_file() and q.name != 'diagnostic-evidence.json'}
    value = {
        'scope': 'One approved diagnostic-only actual epoch identifies its own greater-than100ms publisher sample gap. Aggregate failed; no production or earlier-failure requalification.',
        'selected_build': BUILD, 'selected_epoch': EPOCH,
        'retained_proof_root_checkout': str(ROOT),
        'original_source_sha256': selected['original_source_library_sha256'],
        'approved_diagnostic_source_sha256': SOURCE, 'approved_patch_sha256': PATCH,
        'diagnostic_source_differences': selected['diagnostic_only_source_differences'],
        'unchanged77_candidate_source_checksums': unchanged,
        'diagnostic_service_elf_sha256': selected['bins']['messages-app-context-fixture'],
        'result_sha256': sha(out / 'result.json'), 'selection_sha256': sha(out / 'selection.json'),
        'proof_checksums': proof, 'current_authored_tools_checksums': authored,
        'authenticated_trace_records': records, 'trace_qualified': True,
        'diagnosis': {'selected_predicate': 'Discontinuous sample chain', 'gap_ns': gap,
                      'original_budget_ns': 100000000, 'excess_ns': gap - 100000000,
                      'previous_original_until_ns': rejection['previous_sampled_at_ns'] + 100000000,
                      'own_live_sample_examination_performed': False,
                      'later_fd_type_examination_performed': False,
                      'scheduler_causality_qualified': False},
        'publisher_sample_origin': {'source_path': str(adapter), 'source_sha256': sha(adapter),
             'actual_adapter_elf_sha256': selection['files']['bins/adapter.so'],
             'actual_hyprland_elf_sha256': selection['expected_hyprland_elf_sha256'],
             'sample_clock': 'Actual retained adapter emit CLOCK_BOOTTIME encoded into authentic frames',
             'independent_publisher_clock_syscall_trace': False, 'mirrored_packets_retained': False},
        'cleanup': {'tracked_outcomes': len(result['cleanup']), 'all_tracked_complete': True,
                    'helper_parent_wait_independently_traced': False},
        'previous_current_green_index_sha256': sha(TOOLS / 'green-evidence.json'),
        'previous487_proof_and10_authored_pins_unchanged': True,
        'old_failed_successor_predicate_qualified': False,
        'old_failed_successor_reclassified': False, 'aggregate_passed': False,
        'limits': ['Diagnostic bookkeeping/setup/fault IO can perturb timing.',
                   'Category10 defaults are unexamined for fd/ancillary fields, not zero-handle observations.',
                   'No Source guard/deadline/frame/revival, production core, phone or vendor capability change.'],
    }
    path = TOOLS / 'diagnostic-evidence.json'
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'proof_paths': len(proof), 'authored_paths': len(authored),
                      'unchanged_core': len(unchanged), 'index_sha256': sha(path)}, indent=2))


if __name__ == '__main__':
    main()
