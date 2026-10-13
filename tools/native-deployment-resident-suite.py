#!/usr/bin/env python3
"""Actual staged-file load under a child address-space limit, never a budget mock."""
import argparse
import hashlib
import json
import resource
import subprocess
import tempfile
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[1]
OUTPUT = PROJECT / 'artifacts/native-deployment/correction-elf-resident'
SIZE = 16 * 1024 * 1024
COUNT = 12
ADDRESS_LIMIT = 96 * 1024 * 1024
GATES = ['factory-integrity', 'kernel-kmi', 'vendor-userspace-reduction', 'bionic-abi',
         'native-init', 'selinux', 'backend-discovery', 'permissions',
         'native-service-integration', 'physical-device']


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(64 * 1024), b''):
            value.update(block)
    return value.hexdigest()


def child_limits():
    resource.setrlimit(resource.RLIMIT_AS, (ADDRESS_LIMIT, ADDRESS_LIMIT))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['red', 'green'])
    args = parser.parse_args()
    binary = (PROJECT / 'artifacts/native-deployment/x86_64-unknown-linux-gnu/release/pokelinux-deployment'
              if args.phase == 'red' else OUTPUT / 'x86_64-unknown-linux-gnu/release/pokelinux-deployment')
    probe = PROJECT / 'artifacts/native-deployment/x86_64-unknown-linux-gnu/native-elf-fixture'
    with tempfile.TemporaryDirectory(prefix='deployment-resident-') as temp:
        folder = Path(temp)
        stage = folder / 'stage'
        stage.mkdir()
        profile = {'schema': 1, 'id': 'authored-resident-fixture', 'device': 'authored-offline-device',
                   'architecture': 'x86_64', 'component_set': 'authored-resident-set',
                   'kernel_release': 'authored-kernel', 'evidence': 'authored-fixture',
                   'artifacts': [], 'resources': ['messages'],
                   'service_contracts': [{'id': 'messages', 'owns': ['messages'], 'dependencies': []}],
                   'gates': [{'id': gate, 'scope': 'physical' if gate == 'physical-device' else 'offline',
                              'state': 'qualified', 'evidence': 'authored test assertion only'} for gate in GATES],
                   'capabilities': []}
        inventory = {key: profile[key] for key in ['schema', 'device', 'architecture', 'component_set']}
        inventory.update({'profile': profile['id'], 'artifacts': [], 'files': [],
                          'packages': [{'name': 'messages', 'version': '1.0.0-1', 'architecture': 'x86_64',
                                        'component_set': profile['component_set'], 'metadata': '.PKGINFO',
                                        'dependencies': []}],
                          'services': [{'id': 'messages', 'runtime': 'native-rust',
                                        'executable': 'usr/bin/messages', 'package': 'messages',
                                        'owns': ['messages'], 'dependencies': []}], 'enable_capabilities': []})

        def add(name, content, kind, package, mode):
            path = stage / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
            path.chmod(mode)
            inventory['files'].append({'path': name, 'sha256': digest(path),
                                       'mode': mode, 'package': package, 'kind': kind})
            return path

        kernel = add('factory/kernel', b'authored kernel bytes; not a kernel image', 'kernel', None, 0o644)
        profile['artifacts'].append({'id': 'kernel', 'kind': 'kernel', 'sha256': digest(kernel),
                                     'source': 'authored temporary bytes only'})
        inventory['artifacts'].append({'id': 'kernel', 'path': 'factory/kernel',
                                       'component_set': profile['component_set'],
                                       'kernel_release': profile['kernel_release']})
        add('usr/bin/messages', probe.read_bytes(), 'native-executable', 'messages', 0o755)
        add('.PKGINFO', b'pkgname = messages\npkgver = 1.0.0-1\narch = x86_64\n', 'native-resource', 'messages', 0o644)
        for number in range(COUNT):
            name = f'usr/share/messages/ordinary-data-{number:02d}'
            path = stage / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('wb') as stream:
                stream.truncate(SIZE)  # Actual sparse zero-filled ordinary data; no huge in-memory fixture.
            path.chmod(0o644)
            inventory['files'].append({'path': name, 'sha256': digest(path), 'mode': 0o644,
                                       'package': 'messages', 'kind': 'native-resource'})
        profile_path, inventory_path = folder / 'profile.json', folder / 'inventory.json'
        profile_path.write_text(json.dumps(profile))
        inventory_path.write_text(json.dumps(inventory))
        before = {row['path']: (digest(stage / row['path']), (stage / row['path']).stat().st_size)
                  for row in inventory['files']}
        completed = subprocess.run([str(binary), str(profile_path), str(inventory_path), str(stage)],
                                   text=True, capture_output=True, timeout=30, preexec_fn=child_limits)
        peak_kib = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
        try:
            report = json.loads(completed.stdout)
        except json.JSONDecodeError:
            report = None
        after = {row['path']: (digest(stage / row['path']), (stage / row['path']).stat().st_size)
                 for row in inventory['files']}
        evidence = {'schema': 1, 'phase': args.phase, 'binary_sha256': digest(binary),
                    'fixture_origin': 'Authored ordinary staged files/package/profile; actual compiled Rust ELF probe',
                    'ordinary_resource_bytes': COUNT * SIZE, 'ordinary_resource_count': COUNT,
                    'bytes_per_resource': SIZE, 'manifest_bytes': inventory_path.stat().st_size,
                    'child_address_space_limit_bytes': ADDRESS_LIMIT, 'child_peak_rss_kib': peak_kib,
                    'returncode': completed.returncode, 'stderr': completed.stderr[:256],
                    'report': report, 'stage_changed': before != after,
                    'limits': 'Actual x86 child only; no guest-kernel/device or production-service evidence'}
        OUTPUT.mkdir(exist_ok=True)
        (OUTPUT / f'{args.phase}-resident.json').write_text(json.dumps(evidence, indent=2) + '\n')
        print(json.dumps({key: evidence[key] for key in ['phase', 'ordinary_resource_bytes',
                         'child_address_space_limit_bytes', 'child_peak_rss_kib', 'returncode', 'stage_changed']}))
        assert completed.returncode == 1 and report is not None and report['inventory_valid'], 'large conformant staged inventory failed under bounded child address space'
        assert not report['deployment_admitted'] and [item['code'] for item in report['issues']] == ['fixture-evidence']
        assert peak_kib < 64 * 1024, 'ordinary payload aggregate caused excessive resident memory'
        assert before == after


if __name__ == '__main__':
    main()
