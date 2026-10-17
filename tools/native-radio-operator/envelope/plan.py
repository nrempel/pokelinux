#!/usr/bin/env python3
"""SOURCE ONLY: select one fresh GREEN epoch from reviewed actual build artifacts.

No build or runtime action occurs here. Root reviews the frozen source first,
then separately authorizes planning and execution from actual candidate bytes.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct

HERE = Path(__file__).resolve().parent
DRAFT = HERE.parent
MAIN = Path('/home/ubuntu/Projects/pokelinux')
IMAGE = 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
EXPECTED = {
    MAIN / 'artifacts/native-session-installed-app/vm-production/root.img': '31e3ccf69a5496f61284abdcd049e8273e70e840407bab4e744c778f21e25842',
    MAIN / 'sim/boot/Image': '2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b',
    MAIN / 'sim/boot/initramfs-linux.img': 'a9bb29d29abda94748c9948f903c3c1704f829eea477652b9a4201a61f36a647'}
ROLES = {'manager', 'peer', 'operator-controller', 'radio-worker', 'radio-service', 'radio-status'}
CASES = ('accepted', 'expired-before-admission', 'permit-expires', 'stop-before-consumption', 'writer-held', 'manager-stop-held-writer')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def arm_elf(path):
    with Path(path).open('rb') as stream:
        header = stream.read(64)
    assert len(header) == 64 and header[:6] == b'\x7fELF\x02\x01'
    assert struct.unpack_from('<H', header, 18)[0] == 183


def actual_Cargo_link(qualification_path, qualification, bins):
    """Read real successful Cargo JSON streams; a copied summary is insufficient."""
    assert set(qualification['artifacts']) == ROLES
    streams = qualification['Cargo_build_streams']
    assert set(streams) == {'fixture-arm-cargo.jsonl', 'production-arm-cargo.jsonl'}
    observed = {}
    for stream_name, proof in streams.items():
        path = qualification_path.parent / stream_name
        assert sha(path) == proof['sha256']
        messages = [json.loads(line) for line in path.read_text().splitlines()]
        finished = [row for row in messages if row.get('reason') == 'build-finished']
        assert len(finished) == 1 and finished[0]['success'] is True
        roots = [row for row in messages if row.get('reason') == 'compiler-artifact'
                 and row.get('executable') and row['target']['kind'] == ['bin']
                 and row['profile']['test'] is False]
        assert len(roots) == proof['root_bin_count'] == 3
        assert proof['successful_build_finished'] is True
        for row in roots:
            name = 'operator-controller' if row['target']['name'] == 'controller' else row['target']['name']
            assert name in ROLES and name not in observed
            artifact = qualification['artifacts'][name]
            assert artifact['Cargo_source_stream'] == stream_name
            assert artifact['Cargo_source_stream_sha256'] == proof['sha256']
            assert artifact['Cargo_executable'] == row['executable']
            assert artifact['Cargo_target_name'] == row['target']['name']
            assert artifact['Cargo_package_id'] == row['package_id']
            assert artifact['Cargo_manifest_path'] == row['manifest_path']
            assert row['executable'] in row['filenames']
            relative = Path(row['executable']).relative_to('/results')
            assert relative.parts[:2] == ('target', 'aarch64-unknown-linux-gnu')
            actual_path = qualification_path.parent / relative
            supplied_path, digest = bins[name]
            assert actual_path.resolve(strict=True) == supplied_path == Path(artifact['actual_path']).resolve(strict=True)
            assert sha(actual_path) == digest == artifact['sha256']
            arm_elf(actual_path)
            observed[name] = digest
    assert set(observed) == ROLES


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--case', required=True, choices=CASES)
    parser.add_argument('--review-manifest-sha256', required=True)
    parser.add_argument('--candidate-source', required=True, type=Path)
    parser.add_argument('--candidate-pins', required=True, type=Path)
    parser.add_argument('--candidate-pins-sha256', required=True)
    parser.add_argument('--artifact-qualification', required=True, type=Path)
    parser.add_argument('--artifact-qualification-sha256', required=True)
    parser.add_argument('--bin', nargs=3, action='append', required=True, metavar=('ROLE', 'PATH', 'SHA256'))
    parser.add_argument('--build-proof', nargs=2, action='append', required=True, metavar=('PATH', 'SHA256'))
    args = parser.parse_args()
    out = args.output.resolve()
    assert out.parent == Path('/tmp') and out.name.startswith('pokelinux-radio-operator-green-arm-') and not out.exists()
    manifest = DRAFT / 'REVIEW-MANIFEST.json'
    assert sha(manifest) == args.review_manifest_sha256
    review = json.loads(manifest.read_text())
    for relative, entry in review['files'].items():
        assert sha(DRAFT / relative) == entry['sha256'], relative
    source_root = args.candidate_source.resolve(strict=True)
    pins_path = args.candidate_pins.resolve(strict=True)
    assert sha(pins_path) == args.candidate_pins_sha256
    pins = json.loads(pins_path.read_text())
    assert pins['source_pins'] and pins['whole_HEAD_equality_required'] is False
    for relative, digest in pins['source_pins'].items():
        path = Path(relative)
        assert not path.is_absolute() and '..' not in path.parts
        assert sha(source_root / path) == digest
    bins = {name: (Path(path).resolve(strict=True), digest) for name, path, digest in args.bin}
    assert len(args.bin) == len(bins) == 6 and set(bins) == ROLES
    qualification_path = args.artifact_qualification.resolve(strict=True)
    assert sha(qualification_path) == args.artifact_qualification_sha256
    qualification = json.loads(qualification_path.read_text())
    assert qualification['strict_build_passed'] and qualification['actual_CLI_wait_exit'] == 0
    assert qualification['actual_PIPE_EOF'] and qualification['fresh_exact_CID_absent']
    assert qualification['same_daemon'] and qualification['immutable_inputs_unchanged']
    assert qualification['harness_manifest_sha256'] == args.review_manifest_sha256
    assert qualification['production_source_pins_sha256'] == args.candidate_pins_sha256
    actual_Cargo_link(qualification_path, qualification, bins)
    for path, digest in args.build_proof:
        assert sha(path) == digest
    origins = {}
    out.mkdir(mode=0o700)
    (out / 'bins').mkdir()
    (out / 'source/workspace').mkdir(parents=True)

    def copy(source, target):
        source = Path(source).resolve(strict=True)
        digest = sha(source)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        target.chmod(0o444)
        assert sha(target) == sha(source) == digest
        origins[str(source)] = digest

    for relative in review['files']:
        copy(DRAFT / relative, out / 'source/harness' / relative)
    copy(manifest, out / 'source/harness/REVIEW-MANIFEST.json')
    for name in ['guest.py', 'boot.sh', 'run-selected.py', 'systemd-exit-observer.py', 'plan.py']:
        copy(HERE / name, out / 'source' / name)
    for name in ['consumer.py', 'green_adapter.py', 'operator_client.py', 'wire_assertions.py', 'manager_stop_consumer.py', 'config.json', 'expected.json']:
        copy(DRAFT / name, out / 'source' / name)
    copy(pins_path, out / 'source/production-source-pins.json')
    for relative in pins['source_pins']:
        copy(source_root / relative, out / 'source/repository' / relative)
    for name, (path, digest) in bins.items():
        copy(path, out / 'bins' / name)
        (out / 'bins' / name).chmod(0o555)
    proofs = [(str(qualification_path), args.artifact_qualification_sha256), *args.build_proof]
    for index, (path, digest) in enumerate(proofs, 1):
        copy(path, out / 'source/build-proofs' / (str(index) + '-' + Path(path).name))
    for name in qualification['Cargo_build_streams']:
        copy(qualification_path.parent / name, out / 'source/build-proofs' / name)
    lockpath = MAIN / 'artifacts/native-session-installed-app/vm-production.lock'
    with lockpath.open('rb') as lock:
        fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        for path, digest in EXPECTED.items():
            assert sha(path) == digest
            origins[str(path)] = digest
    selected = {'schema': 1, 'phase': 'radio-operator-green-arm', 'label': out.name,
        'case': args.case, 'expected_outcome': 'GREEN',
        'scope': 'one fresh installed Rust operator epoch with real native child channels, durable writer wait, independent Binder peer and full owned closure',
        'runtime_image': IMAGE, 'binaries': {name: digest for name, (path, digest) in bins.items()},
        'source_closure': pins['source_pins'], 'production_source_pins_sha256': args.candidate_pins_sha256,
        'harness_manifest_sha256': args.review_manifest_sha256,
        'whole_HEAD_equality_required': False, 'build_performed_by_this_envelope': False,
        'build_proof_hashes': proofs,
        'Rust_Supervisor_owns_worker_wait': True, 'systemd_owns_service_Main_wait': True,
        'independent_guest_worker_reap_selected': False, 'production_unit_bytes_installed_unchanged': True,
        'actual_status_CLIwait0_and_parsed_active_RAW_required_before_operator_oracle': True,
        'GREEN_writer_channel_and_Ready_required': True, 'strict_post_wait_cutoff_checks': True,
        'selected_slot_id': 1, 'single_current_status_history_key': 'last_observation',
        'request_history_is_independent': True, 'service_uid_gid': 1500,
        'fixture_manager_peer_controller_uid_gid': 1000, 'operator_uid_gid': 0,
        'fixture_supplementary_groups': [], 'binder_device': '/dev/binderfs/radio',
        'binder_device_mode': '0666 private guest fixture only; no deployment permission claim',
        'service_name': 'org.pokelinux.fixture.arm64-radio',
        'fixed_prestart_config': '/etc/pokelinux/radio-add-pdn.json',
        'config_ownership': 'root:1500 file0640 parent0750; data only, never dispatch authority',
        'installed_unit_KillMode': 'mixed', 'installed_unit_ExecStop_present': False,
        'unit_Stop_seconds': 5, 'native_worker_original_cleanup_budget_ns': 2000000000,
        'setup_observation_deadline_seconds': 10, 'whole_first_start_case_original_cutoff_seconds': 29,
        'status_client_original_RPC_or_Stop_deadline_ns': 3000000000,
        'same_journal_refusal_observation_budget_ns': 5000000000,
        'status_JSON_maximum_bytes': 16384, 'operator_helper_maximum_selected_output_bytes': 16384,
        'retained_channel_maximum_bytes': 8192,
        'exact_worker_lookup_count': 1, 'exact_peer_registration_count': 1,
        'exact_native_decoded_RAW_observed_event_count': 1,
        'expected_capture105_count': 1 if args.case == 'accepted' else 0,
        'no_deadline_renewals_selected': True, 'automatic_retries_selected': False,
        'PAM_or_authentication_selected': False, 'vendor_or_private_inputs_selected': False,
        'phone_accessed': False, 'hardware_qualified': False, 'host_devices_attached': False,
        'guest_cgroup_kill_selected': False, 'network': 'none', 'guest_cpus': 6, 'guest_memory_mib': 4096,
        'container_cpus': 8, 'container_memory': '7g', 'outer_deadline_seconds': 600,
        'guest_QEMU_deadline_seconds': 540, 'overlay_maximum_tmpfs_bytes': 2147483648,
        'raw_backing_clone_permitted': False, 'shared_input_lock': str(lockpath),
        'external_inputs': origins,
        'consumer_source_pins': {name: sha(DRAFT / name) for name in
            ['consumer.py', 'green_adapter.py', 'operator_client.py', 'wire_assertions.py', 'manager_stop_consumer.py']},
        'files': {str(path.relative_to(out)): sha(path) for tree in [out / 'source', out / 'bins']
                  for path in sorted(tree.rglob('*')) if path.is_file()}}
    command = DOCKER + ['run', '--rm', '--pull=never', '--name', 'pokelinux-radio-operator-' + out.name,
        '--cidfile', str(out / 'owned-container-id'), '--network', 'none', '--cpus', '8', '--memory', '7g',
        '--pids-limit', '512', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
        '--user', f'{os.getuid()}:{os.getgid()}', '--tmpfs', '/tmp:rw,exec,nosuid,nodev,size=2g',
        '-v', str(out / 'source/workspace') + ':/workspace:ro',
        '-v', str(MAIN / 'artifacts/native-session-installed-app/vm-production/root.img') + ':/vm/root.img:ro',
        '-v', str(MAIN / 'sim/boot/Image') + ':/boot-inputs/Image:ro',
        '-v', str(MAIN / 'sim/boot/initramfs-linux.img') + ':/boot-inputs/initramfs-linux.img:ro',
        '-v', str(out) + ':/results:rw', IMAGE, 'bash', '/results/source/boot.sh']
    for name, value in [('selection.json', selected), ('command.json', command)]:
        (out / name).write_text(json.dumps(value, indent=2) + '\n')
        (out / name).chmod(0o444)
    print(json.dumps({'selection_dir': str(out), 'case': args.case,
          'selection_sha256': sha(out / 'selection.json'), 'command_sha256': sha(out / 'command.json'),
          'runtime_started': False}, indent=2))


if __name__ == '__main__':
    main()
