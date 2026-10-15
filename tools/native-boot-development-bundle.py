#!/usr/bin/env python3
"""Build a private, offline development diagnostic bundle; never operate a phone."""
import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
AVB_COMMIT = '761178607206f4cb2af79ed9eec52d8cbd814adb'
AVB_SHA = 'e5a664a38db623da00f080219bc0ee60a640a9dc4a872803616fae4938ac749b'
CAPACITY = 8 * 1024 * 1024
ROLLBACK = 1791158400
SALT = hashlib.sha256(b'Pokelinux diagnostic init_boot v1').hexdigest()
UNCHANGED = ['boot.img', 'vendor_boot.img', 'vendor_kernel_boot.img', 'dtbo.img']


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run(args):
    package = load('compact_package', REPO / 'tools/native-boot-probe-package.py')
    output = args.private_output_dir.resolve()
    package.outside_worktrees(output)
    if output.exists():
        raise ValueError('output label already exists; preserve prior evidence')
    args.private_avbtool = args.private_avbtool.resolve()
    package.outside_worktrees(args.private_avbtool)
    package.regular(args.private_avbtool, 1024 * 1024)
    if package.digest(args.private_avbtool) != AVB_SHA:
        raise ValueError('pinned original avbtool changed')
    for value in vars(args).values():
        if isinstance(value, pathlib.Path) and value != args.private_output_dir:
            value = value.resolve()
            if value == output or value in output.parents or output in value.parents:
                raise ValueError('private output must not overlap any input')
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    compact_args = argparse.Namespace(**vars(args))
    compact_args.private_output_dir = output / 'compact'
    package.run(compact_args)
    (output / 'candidate').mkdir()
    shutil.copyfile(output / 'compact/init_boot.img', output / 'candidate/init_boot.img')
    image = output / 'candidate/init_boot.img'
    compact = (output / 'compact/init_boot.img').read_bytes()
    pins = package.small_json(package.PINS_FILE)
    input_root, _ = package.selection(args, pins)
    originals = package.verify_originals(input_root, pins)
    selection_sha = package.digest(args.authenticated_selection)
    executions = []

    def checked(argv, label):
        result = subprocess.run(list(map(str, argv)), capture_output=True, timeout=30)
        (output / (label + '.stdout')).write_bytes(result.stdout)
        (output / (label + '.stderr')).write_bytes(result.stderr)
        executions.append({'argv': list(map(str, argv)), 'actual_wait_exit': result.returncode,
                           'both_output_streams_EOF': True,
                           'stdout_sha256': package.sha(result.stdout),
                           'stderr_sha256': package.sha(result.stderr)})
        if result.returncode != 0:
            raise ValueError('original tool refused: ' + label)

    avb_command = [sys.executable, args.private_avbtool]
    checked([*avb_command, 'add_hash_footer', '--image', image,
             '--partition_name', 'init_boot', '--partition_size', CAPACITY,
             '--algorithm', 'NONE', '--hash_algorithm', 'sha256', '--salt', SALT,
             '--rollback_index', ROLLBACK, '--rollback_index_location', 0,
             '--flags', 0], 'original-avb-produce')
    # NONE checks format and salted payload integrity. It supplies no signature,
    # trusted key or device admission. Keep all stock verification policy intact.
    for action in ['info_image', 'verify_image']:
        checked([*avb_command, action, '--image', image], 'original-avb-' + action)
    avb = load('original_avb', args.private_avbtool)
    handler = avb.ImageHandler(str(image), read_only=True)
    try:
        footer, header, descriptors, size = avb.Avb()._parse_image(handler)
    finally:
        handler._image.close()
    if (footer is None or size != CAPACITY or len(descriptors) != 1 or
            not isinstance(descriptors[0], avb.AvbHashDescriptor)):
        raise ValueError('fresh complete init_boot AVB layout required')
    descriptor = descriptors[0]
    if (header.algorithm_type != 0 or header.authentication_data_block_size != 0 or
            header.hash_size != 0 or header.signature_size != 0 or
            header.public_key_size != 0 or header.public_key_metadata_size != 0 or
            header.flags != 0 or header.rollback_index != ROLLBACK or
            header.rollback_index_location != 0 or descriptor.flags != 0 or
            descriptor.partition_name != 'init_boot' or descriptor.hash_algorithm != 'sha256' or
            descriptor.image_size != len(compact) or descriptor.salt.hex() != SALT or
            descriptor.digest != hashlib.sha256(bytes.fromhex(SALT) + compact).digest()):
        raise ValueError('unsigned diagnostic AVB metadata disagrees with selected contract')
    body = image.read_bytes()
    if (footer.original_image_size != len(compact) or
            footer.vbmeta_offset != (len(compact) + 4095) // 4096 * 4096 or
            footer.vbmeta_size != 256 + header.auxiliary_data_block_size or
            footer.vbmeta_offset + footer.vbmeta_size > CAPACITY - 64 or
            body[:len(compact)] != compact or
            any(body[len(compact):footer.vbmeta_offset]) or
            any(body[footer.vbmeta_offset + footer.vbmeta_size:-64]) or
            body[-64:] == (input_root / 'init_boot.img').read_bytes()[-64:]):
        raise ValueError('complete image prefix, new footer or zero tail disagrees')
    report = package.inspect(args.inspector, image)
    compact_report = package.inspect(args.inspector, output / 'compact/init_boot.img')
    if (report['file_size'] != CAPACITY or report['kind'] != 'boot-v4' or
            report['page_size'] != 4096 or report['kernel']['size'] != 0 or
            report['signature']['size'] != 0 or
            report['ramdisk'] != compact_report['ramdisk'] or
            report['trailing'] != {'offset': len(compact), 'size': CAPACITY - len(compact)}):
        raise ValueError('Rust inspector disagrees on complete partition geometry')
    checked([sys.executable, args.private_reference_root / 'unpack_bootimg.py',
             '--boot_img', image, '--out', output / 'original-full-unpack'], 'original-full-unpack')
    if ((output / 'original-full-unpack/ramdisk').read_bytes() !=
            (output / 'compact/ramdisk.lz4').read_bytes()):
        raise ValueError('original unpacker disagrees on exact native ramdisk')
    expected = {row['name']: row for row in originals}

    def copy_original(name, directory):
        target = output / directory / name
        target.parent.mkdir(exist_ok=True)
        shutil.copyfile(input_root / name, target)
        row = expected[name]
        if target.stat().st_size != row['bytes'] or package.digest(target) != row['sha256']:
            raise ValueError('restoration or unchanged asset copy disagrees')
        return {'path': directory + '/' + name, 'bytes': row['bytes'], 'sha256': row['sha256']}

    restoration = copy_original('init_boot.img', 'restore')
    unchanged = [copy_original(name, 'unchanged') for name in UNCHANGED]
    reference = package.load_reference(args.private_reference_root.resolve(), pins)
    reference.sources(args.private_reference_root.resolve(), False)
    if (package.verify_originals(input_root, pins) != originals or
            package.digest(args.authenticated_selection) != selection_sha or
            package.digest(args.private_avbtool) != AVB_SHA or
            package.digest(args.native_init) != pins['native_payload']['init']['sha256'] or
            package.digest(args.native_child) != pins['native_payload']['probe-child']['sha256']):
        raise ValueError('selected inputs changed during development packaging')
    manifest = {
        'schema': 1,
        'classification': 'OFFLINE_DEVELOPMENT_DIAGNOSTIC_BUNDLE_PIXEL_UNQUALIFIED',
        'local_only': True, 'bundle_format_verified': True,
        'factory_archive': pins['factory_archive'],
        'archive_authentication': 'inherited whole-archive authentication; all14 selected files rehashed',
        'whole_archive_rehashed_by_this_tool': False,
        'scope': {'device': 'Pixel10/frankel', 'factory_build': 'CP3A.261005.005',
                  'intended_partition': 'init_boot', 'target_slot': None,
                  'partition_write_authorized': False,
                  'only_authored_partition': 'candidate/init_boot.img',
                  'unchanged_assets_are_reference_inputs_not_a_write_plan': True,
                  'module_firmware_and_board_runtime_qualified': False},
        'image': {'path': 'candidate/init_boot.img', 'bytes': CAPACITY,
                  'sha256': package.digest(image), 'header_version': 4,
                  'container_page_size': 4096, 'compact_prefix_bytes': len(compact),
                  'compact_prefix_sha256': package.sha(compact),
                  'native_payload': pins['native_payload'],
                  'ramdisk_lz4_sha256': package.digest(output / 'compact/ramdisk.lz4')},
        'avb': {'algorithm': 'NONE', 'signed': False, 'flags': 0,
                'rollback_index': ROLLBACK, 'header_rollback_index_location': 0,
                'rollback_index_origin': 'retained factory metadata; device stored index unknown',
                'salt_hex': SALT, 'protected_bytes': descriptor.image_size,
                'salted_payload_sha256': descriptor.digest.hex(),
                'footer_offset': CAPACITY - 64, 'vbmeta_offset': footer.vbmeta_offset,
                'vbmeta_bytes': footer.vbmeta_size, 'copied_OEM_signature_or_footer': False,
                'original_info_and_local_hash_verify_passed': True,
                'original_NONE_helper_success_is_not_signature_verification': True,
                'verification_policy_changed': False,
                'actual_unlocked_device_error_policy': 'unqualified'},
        'restoration': {**restoration,
                        'origin': 'exact authenticated retained factory member',
                        'is_verified_backup_of_current_phone_state': False},
        'unchanged_inputs': unchanged,
        'compact_package_manifest_sha256': package.digest(output / 'compact/manifest.json'),
        'original_tools': {'avb_commit': AVB_COMMIT, 'avbtool_sha256': AVB_SHA,
                           'mkbootimg_commit': reference.COMMIT,
                           'source_sha256': [{'path': k, 'sha256': v}
                                             for k, v in reference.SOURCE_HASHES.items()],
                           'Rust_inspector_sha256': pins['inspector_sha256']},
        'source_bindings': [{'path': name, 'sha256': package.digest(REPO / name)} for name in [
            'tools/native-boot-development-bundle.py', 'tools/native-boot-probe-package.py',
            'tools/native-boot-inputs-reference.py', 'native-boot-package/input-pins.json']],
        'diagnostic_profile': {'PID1': 'native Rust restricted RAM probe',
                               'child_actual_wait_success_exit': 37,
                               'kernel_ring_readback_ACKs_required': True,
                               'completion_action': 'PowerOff request; physical behavior unqualified',
                               'persistent_message_reporting_integrated': True,
                               'phone_diagnostic_retrieval_qualified': False,
                               'synthetic_ramoops_addresses_added': False},
        'qualifications': {name: False for name in [
            'physical_device_accessed', 'bootloader_accepted', 'deployment_admitted',
            'phone_current_state_backup_verified', 'stock_trust_chain_accepted',
            'unlocked_device_policy_qualified', 'phone_kernel_runtime_verified',
            'phone_ramdisk_selection_qualified', 'phone_recovery_qualified',
            'full_smartphone_system_qualified']},
        'remaining_experiment_gates': [
            'separate authorization for any physical access or partition change',
            'verify actual device/build/slot/partition geometry and rollback constraints',
            'establish actual development-device loader policy and ramdisk selection',
            'independent recovery access and exact matching current-state restoration set',
            'retrievable execution-correlated diagnostics including early failure',
            'board startup, bounded shutdown, watchdog, thermal and charging observations'],
        'full_phone_deployment_profile': 'unchanged; refused; separate from diagnostic format checks'}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    (output / 'original-execution.json').write_text(json.dumps(executions, indent=2) + '\n')
    print(json.dumps({'bundle_format_verified': True, 'candidate_bytes': CAPACITY,
                      'candidate_sha256': manifest['image']['sha256'],
                      'unsigned': True, 'deployment_admitted': False,
                      'physical_device_accessed': False}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['authenticated-selection', 'private-reference-root', 'inspector',
                 'native-init', 'native-child', 'private-avbtool', 'private-output-dir']:
        parser.add_argument('--' + name, type=pathlib.Path, required=True)
    args = parser.parse_args()
    os.environ['PYTHONDONTWRITEBYTECODE'] = '1'
    try:
        run(args)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError,
            subprocess.TimeoutExpired, json.JSONDecodeError) as error:
        print('development diagnostic bundle refused: ' + str(error), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
