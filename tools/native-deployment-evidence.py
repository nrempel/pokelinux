#!/usr/bin/env python3
"""Clean file-identity + actual CLI evidence. Never copies original payloads."""
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[1]
OUTPUT = PROJECT / 'artifacts/native-deployment/correction-elf-resident'
BINARY = OUTPUT / 'x86_64-unknown-linux-gnu/release/pokelinux-deployment'
PROFILE_PATH = PROJECT / 'native-deployment/profiles/pixel10-frankel.json'
REFERENCES_PATH = PROJECT / 'native-deployment/profiles/pixel10-recorded-inputs.json'


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def main():
    references = json.loads(REFERENCES_PATH.read_text())
    records = []
    audits = {}
    for reference in references['references']:
        audit_path = PROJECT / reference['audit']
        audit = json.loads(audit_path.read_text())
        rows = audit['files'] if isinstance(audit, dict) else audit
        prior = next(row for row in rows if row['path'].lstrip('/') == reference['path'])
        if prior['sha256'] != reference['sha256'] or prior['size'] != reference['size']:
            raise RuntimeError('clean reference pin differs from recorded original audit')
        path = PROJECT / reference['root'] / reference['path']
        if path.is_symlink() or not path.is_file():
            raise RuntimeError('selected original reference is absent or unsafe')
        digest = sha(path)
        if digest != reference['sha256'] or path.stat().st_size != reference['size']:
            raise RuntimeError('selected original reference hash/size changed')
        records.append({'path': '/' + reference['path'], 'sha256': digest,
                        'size': path.stat().st_size, 'original_record_match': True})
        audits[reference['audit']] = sha(audit_path)
    profile = json.loads(PROFILE_PATH.read_text())
    with tempfile.TemporaryDirectory(prefix='pokelinux-deployment-cli-') as temp:
        folder = Path(temp)
        stage = folder / 'stage'
        stage.mkdir()
        inventory = {key: profile[key] for key in ['schema', 'device', 'architecture', 'component_set']}
        inventory.update({'profile': profile['id'], 'artifacts': [], 'packages': [],
                          'files': [], 'services': [], 'enable_capabilities': []})
        inventory_path = folder / 'inventory.json'
        inventory_path.write_text(json.dumps(inventory))
        command = [str(BINARY), str(PROFILE_PATH), str(inventory_path), str(stage)]
        completed = subprocess.run(command, text=True, capture_output=True, timeout=15)
        report = json.loads(completed.stdout)
        assert completed.returncode == 1 and not report['deployment_admitted']
        assert not list(stage.iterdir()), 'read-only CLI changed the staging tree'
        unresolved = {issue['subject']: issue['scope'] for issue in report['issues']
                      if issue['code'] == 'unresolved-gate'}
        assert unresolved == {gate['id']: gate['scope'] for gate in profile['gates']}
        assert sum(scope == 'offline' for scope in unresolved.values()) == 9
        assert unresolved['physical-device'] == 'physical'
        assert {issue['subject'] for issue in report['issues'] if issue['code'] == 'unpinned-artifact'} == {
            'google-modules-complete', 'google-firmware-complete'}
        assert sum(issue['code'] == 'resource-owner' for issue in report['issues']) == len(profile['resources'])
        inventory['unexpected_field'] = 'authored negative fixture'
        inventory_path.write_text(json.dumps(inventory))
        invalid = subprocess.run(command, text=True, capture_output=True, timeout=15)
        assert invalid.returncode == 2 and not invalid.stdout and invalid.stderr == 'manifest input rejected\n'
        assert not list(stage.iterdir())
        # Authored oversized negative manifest; source contents are not reported.
        huge_profile = dict(profile)
        huge_profile['gates'] = [dict(gate) for gate in profile['gates']]
        huge_profile['gates'][0]['evidence'] = 'authored oversized fixture ' * 100000
        huge_path = folder / 'oversized-profile.json'
        huge_path.write_text(json.dumps(huge_profile))
        inventory.pop('unexpected_field')
        inventory_path.write_text(json.dumps(inventory))
        oversized = subprocess.run([str(BINARY), str(huge_path), str(inventory_path), str(stage)],
                                   text=True, capture_output=True, timeout=15)
        assert oversized.returncode == 2 and not oversized.stdout and oversized.stderr == 'manifest input rejected\n', 'oversized manifest was parsed instead of refused'
        assert not list(stage.iterdir())
    evidence = {
        'schema': 1,
        'original_reference_scope': 'Recorded private original audit files independently rehashed; no vendor execution, raw contents or payload copies',
        'original_references': records,
        'original_audit_record_sha256': audits,
        'cli': {'binary_sha256': sha(BINARY), 'profile_sha256': sha(PROFILE_PATH),
                'recorded_inputs_sha256': sha(REFERENCES_PATH), 'pixel_refusal_exit_code': completed.returncode,
                'pixel_report': report, 'unknown_field_exit_code': invalid.returncode,
                'staging_mutated': False, 'oversized_manifest_exit_code': oversized.returncode},
        'limits': ['No actual matched deployment image/package set exists here.',
                   'Kernel/module digests and complete firmware set are unavailable.',
                   'AOSP confirms Pixel 10/frankel; exact CP3A set, whole ZIP SHA256, AVB, KMI and runtime ownership remain unqualified.',
                   'Nine offline deployment gates remain distinct from the physical-device gate.',
                   'No original vendor service executed, physical phone accessed or privileged service configuration changed.']}
    OUTPUT.mkdir(exist_ok=True)
    (OUTPUT / 'reference-and-cli-evidence.json').write_text(json.dumps(evidence, indent=2) + '\n')
    print(json.dumps({'original_rehash_matches': len(records), 'pixel_deployment_admitted': False,
                      'offline_gates': 9, 'physical_gates': 1, 'cli_read_only': True,
                      'unknown_field_refused': True, 'oversized_manifest_refused': True}))


if __name__ == '__main__':
    main()
