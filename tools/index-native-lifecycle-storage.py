#!/usr/bin/env python3
"""Index exact Cargo output for storage qualification; no executable glob selection."""
import argparse
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--architecture', choices=['aarch64', 'x86_64'], default='aarch64')
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
base = root / 'artifacts/native-lifecycle-storage'
target = args.architecture + '-unknown-linux-gnu'
items = []
for suffix, kind in [('bins', 'binary'), ('tests', 'library_test')]:
    manifest = base / (args.architecture + '-' + suffix + '-cargo.jsonl')
    raw = manifest.read_bytes()
    records = [json.loads(line) for line in raw.decode().splitlines()]
    selected = [r for r in records if r.get('reason') == 'compiler-artifact' and r.get('executable') and bool(r.get('profile', {}).get('test')) == (kind == 'library_test')]
    names = {'lifecycle-daemon', 'lifecycle-launcher', 'lifecycle-client'} if kind == 'binary' else {'pokelinux_lifecycle'}
    assert {r['target']['name'] for r in selected} == names and len(selected) == len(names)
    for record in selected:
        cargo_path = Path(record['executable'])
        relative = cargo_path.relative_to('/target')
        assert relative.parts[0] == target
        path = base / 'target' / relative
        items.append({'kind': kind, 'name': record['target']['name'], 'cargo_path': str(cargo_path), 'project_path': str(path.relative_to(root)), 'guest_path': '/workspace/' + str(path.relative_to(root)), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'bytes': path.stat().st_size, 'cargo_manifest': str(manifest.relative_to(root)), 'cargo_manifest_sha256': hashlib.sha256(raw).hexdigest()})
fixtures = ['tools/native-lifecycle-fixture.py', 'tools/native-lifecycle-storage-fixture.py', 'tools/native-lifecycle-storage-cleanup-fixture.py']
fixture_items = [{'path': p, 'sha256': hashlib.sha256((root / p).read_bytes()).hexdigest()} for p in fixtures]
original_groups = json.loads((root / 'docs/evidence/native-lifecycle-provenance.json').read_text())['execution']['x86_installed']['passed_groups']
new_groups = [
    'SIGSTOP_storage_process_status_stop_responsive_and_reaped',
    'uncertain_storage_owner_refuses_restart_after_reaping',
    'concurrent_same_principal_busy_refusal_consumes_permit',
    'synced_late_success_after_lifecycle_change_is_historical_only',
    'absolute_deadline_revokes_before_kill_reap_and_no_successor',
    'actual_worker_previous_token_reply_cannot_publish_current_admission',
    'paused_initial_storage_load_keeps_status_stop_responsive',
    'bounded_final_cleanup_reap_error_aborts_without_dropping_owned_child',
    'bounded_final_cleanup_reap_withheld_aborts_without_dropping_owned_child',
]
components = [
    'authority::tests::exhausted_lifecycle_revision_fences_prepared_work',
    'authority::tests::failed_admission_write_never_recovers_or_retries',
    'authority::tests::late_durable_success_grants_no_current_admission',
    'authority::tests::suspended_time_jump_expires_without_timer_delivery',
    'storage::tests::clean_seal_binds_exact_policy_and_unclean_owner_never_reopens',
    'storage::tests::clean_seal_rejects_changed_policy_without_overwriting_owner',
    'storage::tests::terminated_worker_slot_remains_owned_until_actual_reap',
]
helper = next(i for i in items if i['name'] == 'lifecycle-daemon')['guest_path']
manifest = {
    'target': target, 'runtime_qualified': False, 'artifacts': items, 'fixtures': fixture_items,
    'fixture_binary_root': '/workspace/artifacts/native-lifecycle-storage/target/' + target + '/debug',
    'fixture_environment': {'NATIVE_LIFECYCLE_ISOLATED': '1', 'LIFECYCLE_STORAGE_RUN_BASELINE': '1'},
    'component_environment': {'NATIVE_LIFECYCLE_TEST_ROOT': '/run', 'NATIVE_LIFECYCLE_STORAGE_HELPER': helper},
    'required_runtime': 'actual Linux kernel; isolated root; Python3; GNU libc; writable executable /opt plus private /run and /var/lib; fixture-owned child subreaper; core dumps disabled; no network/phone/device transport',
    'expected_installed_process_groups': original_groups + new_groups,
    'expected_installed_process_count': len(original_groups) + len(new_groups),
    'expected_component_tests': components,
    'expected_component_count': len(components),
    'cleanup_modes': ['error', 'withheld'],
    'mechanism_limits': 'SIGSTOP and explicitly injected wait/reap error/withheld wait with real owned children; no uninterruptible filesystem syscall claim',
}
name = 'arm64-artifacts.json' if args.architecture == 'aarch64' else 'x86-artifacts.json'
(base / name).write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps({'manifest': str((base / name).relative_to(root)), 'artifacts': len(items), 'installed_process_groups': manifest['expected_installed_process_count'], 'components': len(components), 'runtime_qualified': False}))
