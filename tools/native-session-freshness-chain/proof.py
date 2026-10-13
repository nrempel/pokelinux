#!/usr/bin/env python3
"""Index only separately executed, complete chronological process RED/GREEN epochs."""
import argparse
import json
from pathlib import Path
from common import BASE, ROOT, hashes, label, sha, verify, write_json
from run import EXPECTED_RED


def checked_epoch(name, phase, arch, build_name):
    out = BASE / name
    selection = json.loads((out / 'selection.json').read_text())
    result = json.loads((out / 'result.json').read_text())
    assert selection['phase'] == phase and selection['arch'] == arch and selection['rust_build'] == build_name
    verify(out, selection['files'])
    build = BASE / build_name
    assert selection['rust_selection_sha256'] == sha(build / 'selection.json')
    assert result['completed'] and result['all_observed_helpers_absent_after_cleanup']
    assert len(result['test_names']) == 34 and result['counts']['ignored'] == 0 and result['counts']['filtered'] == 0
    failed = {name for name, status in result['test_names'].items() if status == 'FAILED'}
    if phase == 'red':
        assert failed == EXPECTED_RED and result['test_exit_code'] == 101
    else:
        assert not failed and result['passed'] and result['test_exit_code'] == 0
    return {'label': name, 'selection_sha256': sha(out / 'selection.json'),
            'result_sha256': sha(out / 'result.json'), 'test_log_sha256': sha(out / 'test.log'),
            'test_elf_sha256': selection['artifacts']['process']['sha256'],
            'counts': result['counts'], 'failed': sorted(failed),
            'kernel': result['kernel'], 'architecture': result['architecture'], 'runtime_uid': result['uid'],
            'observed_helpers': len(result['owned_processes_observed']),
            'orphan_waits': result['orphan_waits'],
            'cleanup_evidence_limit': result['cleanup_evidence_limit']}


def main():
    parser = argparse.ArgumentParser()
    for name in ('red-build', 'red-host', 'red-arm', 'green-build', 'green-host', 'green-arm'):
        parser.add_argument('--' + name, required=True, type=label)
    parser.add_argument('--unselected-output', action='append', type=label, default=[])
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    assert not args.output.exists(), 'Do not replace an earlier frozen evidence index'
    names = [args.red_build, args.red_host, args.red_arm, args.green_build, args.green_host, args.green_arm]
    epochs = [checked_epoch(getattr(args, phase + '_' + arch), phase, arch, getattr(args, phase + '_build'))
              for phase in ('red', 'green') for arch in ('host', 'arm')]
    sources = {}
    for phase in ('red', 'green'):
        build = BASE / getattr(args, phase + '_build')
        selection = json.loads((build / 'selection.json').read_text())
        verify(build / 'source', selection['source_checksums'])
        for artifacts in selection['artifacts'].values():
            for artifact in artifacts.values():
                assert sha(build / artifact['frozen_path']) == artifact['sha256']
        sources[phase] = {name: value for name, value in selection['source_checksums'].items()
                          if name.startswith(('native-session/', 'native-lifecycle/', 'native-kernel-identity/'))}
    unchanged = {name: value for name, value in sources['red'].items() if sources['green'].get(name) == value}
    changed = {name: {'red': value, 'green': sources['green'].get(name)} for name, value in sources['red'].items()
               if sources['green'].get(name) != value}
    assert set(changed) == {'native-session/src/lib.rs', 'native-session/tests/process.rs'}, changed
    red_test = (BASE / args.red_build / 'source/native-session/tests/process.rs').read_text()
    green_test = (BASE / args.green_build / 'source/native-session/tests/process.rs').read_text()
    assert red_test.replace('    let (_p, o) = start(0);', '    let (_p, mut o) = start(0);') == green_test, 'Test conditions changed beyond explicit snapshot mutability binding'
    checksums = {str((BASE / name / relative).relative_to(ROOT)): value
                 for name in names for relative, value in hashes(BASE / name).items()}
    unselected = {str((BASE / name / relative).relative_to(ROOT)): value
                  for name in args.unselected_output for relative, value in hashes(BASE / name).items()}
    own = Path(__file__).parent
    current = {str(path.relative_to(ROOT)): sha(path) for path in sorted(own.iterdir())
               if path.is_file() and path != args.output.resolve() and path.suffix != '.pyc'}
    write_json(args.output, {
        'scope': 'Actual host Linux and actual ARM Linux SourceObserver process-chain tests with authored synthetic peers; no compositor, installed app, vendor backend or phone qualification',
        'chronology': 'Selected RED freezes baseline implementation with newly authored tests; Root was notified of both executed four-failure REDs before correcting Observer; GREEN has a separate source/build/runtime freeze',
        'epochs': epochs, 'proof_checksums': checksums, 'unselected_setup_checksums': unselected,
        'red_core_checksums': sources['red'], 'green_core_checksums': sources['green'],
        'unchanged_core_checksums': unchanged, 'changed_core_checksums': changed,
        'test_source_adjustment': 'Only existing unknown_source_has_no_positive_state adds mut to Observer binding for snapshot(&mut self); all assertions/timing conditions and peer bytes remain unchanged',
        'current_authored_tools_checksums': current,
        'negative_controls': 'Real sampling gap, observed snapshot expiry, expired intermediate frame and previously returned timeout stay terminal; malformed frames/credentials/pidfds remain separately exercised by the full suite',
        'authored_wire_fields': 'Exact contiguous boundary, backwards intermediate and expired intermediate fields are synthetic explicit boundary inputs over actual sockets; timely heartbeat/cycle positives use the sender actual BOOTTIME',
        'deadline_limit': 'Each new accepted Source sample keeps its own 100ms lease; the tests assert old captured deadlines remain expired. No source recreation, old-request renewal, physical-device or hard realtime scheduling claim',
    })
    print(json.dumps({'evidence': str(args.output), 'sha256': sha(args.output), 'proof_paths': len(checksums),
                      'unselected_paths': len(unselected), 'changed_core': changed}, indent=2))


if __name__ == '__main__':
    main()
