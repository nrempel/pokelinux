#!/usr/bin/env python3
"""Freeze correction proof while preserving the historical 21-test checkpoint."""
import hashlib
import json
import subprocess
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[1]
ORIGINAL = PROJECT / 'artifacts/native-deployment'
ARTIFACTS = ORIGINAL / 'correction-elf-resident'
OLD_INDEX_SHA = '2345588fa096d142ab917ef69db97aef8f04fe64c038f196213461acfc278b40'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    old_index = ORIGINAL / 'final-evidence-index.json'
    assert sha(old_index) == OLD_INDEX_SHA, 'historical index changed'
    previous = json.loads(old_index.read_text())
    snapshot = ARTIFACTS / 'frozen-baseline'
    old_provenance = PROJECT / 'docs/evidence/native-deployment-provenance.json'
    assert sha(old_provenance) == sha(snapshot / old_provenance.name), 'historical provenance changed'
    assert sha(snapshot / old_index.name) == OLD_INDEX_SHA
    for name, digest in previous['source_sha256'].items():
        assert sha(snapshot / 'sources' / name) == digest, name
    for target, digest in previous['binary_sha256'].items():
        assert sha(ORIGINAL / target / 'release/pokelinux-deployment') == digest, 'historical binary changed'
    for name, digest in previous['artifacts_sha256'].items():
        assert sha(ORIGINAL / name) == digest, 'historical execution evidence changed'

    sources = list((PROJECT / 'native-deployment').rglob('*'))
    sources += [PROJECT / name for name in ['tools/build-native-deployment.sh',
                'tools/build-native-deployment-correction.sh', 'tools/native-deployment-resident-suite.py',
                'tools/native-deployment-evidence.py', 'tools/index-native-deployment.py',
                'docs/native-deployment.md']]
    source_hashes = {str(path.relative_to(PROJECT)): sha(path) for path in sources if path.is_file()}
    logs = {name: sha(ARTIFACTS / name) for name in ['red-elf-loads.log', 'red-metadata-bound.log',
            'red-resident.log', 'red-resident.json', 'green-build.log', 'green-resident.json',
            'green-cli-reference.log', 'reference-and-cli-evidence.json']}
    green_build = (ARTIFACTS / 'green-build.log').read_text()
    assert green_build.count('test result: ok. 29 passed; 0 failed') == 2
    assert '21 passed; 7 failed' in (ARTIFACTS / 'red-elf-loads.log').read_text()
    assert '0 passed; 1 failed' in (ARTIFACTS / 'red-metadata-bound.log').read_text()
    red_resident = json.loads((ARTIFACTS / 'red-resident.json').read_text())
    green_resident = json.loads((ARTIFACTS / 'green-resident.json').read_text())
    assert not red_resident['report']['inventory_valid']
    assert green_resident['report']['inventory_valid'] and not green_resident['report']['deployment_admitted']
    assert red_resident['ordinary_resource_bytes'] == green_resident['ordinary_resource_bytes'] == 192 * 1024 * 1024
    assert red_resident['child_address_space_limit_bytes'] == green_resident['child_address_space_limit_bytes'] == 96 * 1024 * 1024
    assert not red_resident['stage_changed'] and not green_resident['stage_changed']
    cli_evidence = json.loads((ARTIFACTS / 'reference-and-cli-evidence.json').read_text())
    toolchain = json.loads((ORIGINAL / 'toolchain.json').read_text())
    image_id = subprocess.check_output(['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT',
                '-u', 'DOCKER_TLS', '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH',
                'docker', '--host=unix:///var/run/docker.sock', 'image', 'inspect',
                toolchain['image_tag'], '--format', '{{.Id}}'], text=True).strip()
    assert image_id == toolchain['image_id'], 'build image identity changed'
    result = {
        'schema': 1, 'date_utc': '2026-10-12',
        'scope': 'Corrected read-only native staging admission: ELF structural load geometry and bounded resident payload inspection',
        'repository_head_at_index': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=PROJECT, text=True).strip(),
        'supersedes': {'reason': 'Independent review found malformed PT_LOAD/entry acceptance and cumulative whole-payload retention in the prior 21-test source',
                      'previous_groups_per_target': 21, 'previous_index_sha256': OLD_INDEX_SHA,
                      'previous_clean_provenance_sha256': sha(old_provenance),
                      'previous_clean_provenance': 'docs/evidence/native-deployment-provenance.json',
                      'previous_binary_sha256': previous['binary_sha256'],
                      'previous_source_snapshot_matches': len(previous['source_sha256']),
                      'historical_index_provenance_logs_and_binaries_unchanged': True},
        'source_sha256': dict(sorted(source_hashes.items())),
        'executed_reds': {
            'actual_compiled_elf_mutations': {'groups_failed': 7, 'prior_groups_passed': 21,
                'covers': ['file extent beyond actual payload and overflow', 'file bytes exceed memory extent',
                           'virtual extent overflow', 'non-power-of-two alignment', 'address/offset incongruence',
                           'entry outside file-backed executable loads', 'malformed non-executable load']},
            'excessive_valid_package_metadata': {'groups_failed': 1},
            'actual_constrained_resident_load': red_resident},
        'final_validation': {
            'admission_groups_per_target': 29,
            'targets': ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu (user-QEMU)'],
            'fmt': 'pass', 'clippy_warnings_denied': 'pass', 'release_builds': 'pass',
            'fixture_origin': 'Authored temporary devices/packages/resources/contracts; actual compiled target Rust ELF positives and explicit mutant negatives',
            'actual_constrained_resident_load': green_resident,
            'stream_chunk_bytes': 64 * 1024, 'metadata_body_limit_bytes': 1024 * 1024,
            'retained_payload_body_map': False,
            'original_rehash_matches': 6, 'cli_pixel_deployment_admitted': False,
            'cli_unknown_and_oversized_input': 'refused', 'cli_staging_mutation': False},
        'artifacts_sha256': logs,
        'binary_sha256': {target: sha(ARTIFACTS / target / 'release/pokelinux-deployment')
                          for target in ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu']},
        'compiled_rust_elf_fixture_sha256': {target: sha(ARTIFACTS / target / 'native-elf-fixture')
                          for target in ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu']},
        'original_reference_and_cli_evidence': cli_evidence,
        'public_sources': [
            {'url': 'https://source.android.com/docs/setup/reference/build-numbers', 'checked_date_utc': '2026-10-12',
             'supports': 'AOSP lists Pixel 10 with codename frankel; does not authenticate CP3A selected factory set'},
            {'url': 'https://gabi.xinuos.com/elf/07-pheader.html', 'checked_date_utc': '2026-10-12',
             'supports': 'ELF program-header file/memory sizes, alignment/congruence, load ordering and interpreter placement'}],
        'qualification_limits': {'model_codename_association': 'Verified from authoritative AOSP table',
            'exact_selected_factory_build_set_authentication': False,
            'complete_kernel_module_firmware_set': 'Unresolved; three required artifact pins remain null',
            'whole_factory_zip_sha256': False, 'avb_signature': False, 'kernel_kmi_matching': False,
            'offline_requirements_unresolved': 9, 'physical_requirements_unresolved': 1,
            'elf_system_loader_execution_and_transitive_abi_closure': False,
            'target_kernel_page_size_or_virtual_address_qualification': False,
            'universal_process_rss_or_storage_latency_guarantee': False,
            'installed_native_services_and_runtime_ownership': False,
            'arm64_guest_kernel': False, 'arm64_binder': False, 'original_vendor_service_execution': False,
            'physical_phone_accessed': False, 'privileged_host_services_changed': False,
            'proprietary_payload_or_raw_audit_publication': False, 'deployment_report_is_launch_authority': False},
        'build_toolchain': toolchain,
    }
    assert result['binary_sha256']['x86_64-unknown-linux-gnu'] == green_resident['binary_sha256'] == cli_evidence['cli']['binary_sha256']
    assert result['supersedes']['previous_binary_sha256']['x86_64-unknown-linux-gnu'] == red_resident['binary_sha256']
    (PROJECT / 'docs/evidence/native-deployment-correction-provenance.json').write_text(json.dumps(result, indent=2) + '\n')
    (ARTIFACTS / 'final-evidence-index.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'source_files': len(source_hashes), 'admission_groups_per_target': 29,
                      'original_checkpoint_unchanged': True,
                      'corrected_peak_rss_kib': green_resident['child_peak_rss_kib'],
                      'deployment_admitted': False,
                      'correction_index_sha256': sha(ARTIFACTS / 'final-evidence-index.json')}))


if __name__ == '__main__':
    main()
