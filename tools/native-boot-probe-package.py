#!/usr/bin/env python3
"""Build one unsigned, unqualified native Pixel RAM probe candidate offline.

Only init_boot's generic ramdisk is authored. The authenticated matched Google
images are read and hash-bound; no kernel, DTB, DTBO, module or vendor payload is
changed or executed. Original AOSP tools stay private and unchanged. This is not
a deployer, AVB verifier, general init builder or bootloader instruction tool.
"""
import argparse
import ctypes
import ctypes.util
import hashlib
import importlib.util
import json
import os
import pathlib
import stat
import struct
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
PINS_FILE = REPO / 'native-boot-package/input-pins.json'
MAX_NATIVE = 4 * 1024 * 1024
BLOCK = 8 * 1024 * 1024
CAPACITY = 8 * 1024 * 1024
MAGIC = bytes.fromhex('02214c18')


def sha(body):
    return hashlib.sha256(body).hexdigest()


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def small_json(path):
    if path.stat().st_size > 1024 * 1024:
        raise ValueError('metadata exceeds 1 MiB bound')
    return json.loads(path.read_text())


def regular(path, bound):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_size > bound:
        raise ValueError('bounded regular file required: ' + path.name)
    return info


def outside_worktrees(path):
    if any((parent / '.git').exists() for parent in [path, *path.parents]):
        raise ValueError('private paths must be outside every Git worktree')


def load_reference(root, pins):
    outside_worktrees(root)
    wrapper = REPO / 'tools/native-boot-inputs-reference.py'
    if digest(wrapper) != pins['reference_wrapper_sha256']:
        raise ValueError('frozen authored reference wrapper changed')
    spec = importlib.util.spec_from_file_location('pinned_boot_reference', wrapper)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.sources(root, False)
    return module


def selection(args, pins):
    proof = small_json(args.authenticated_selection)
    archive = pins['factory_archive']
    if (proof.get('whole_archive_authenticated') is not True or
            proof.get('sha256') != archive['sha256'] or proof.get('bytes') != archive['bytes'] or
            proof.get('index_reads_rebound') != 38 or proof.get('schema') != 1):
        raise ValueError('whole official archive authentication required')
    expected = {record['name']: record for record in pins['selected']}
    entries = proof.get('selected', [])
    if len(entries) != len(expected) or len({record['name'] for record in entries}) != len(entries):
        raise ValueError('exact frozen selected input set required')
    for record in entries:
        if record['name'] not in expected or any(
                record[key] != expected[record['name']][key] for key in ['bytes', 'sha256']):
            raise ValueError('authenticated selected input pin changed')
    root = (args.authenticated_selection.parent / 'selected').resolve()
    outside_worktrees(root)
    return root, proof


def verify_originals(root, pins):
    records = []
    for entry in pins['selected']:
        path = root / entry['name']
        regular(path, 512 * 1024 * 1024)
        if path.stat().st_size != entry['bytes'] or digest(path) != entry['sha256']:
            raise ValueError('authenticated selected original changed: ' + entry['name'])
        records.append(dict(entry))
    return records


def native(path, pin):
    info = regular(path, MAX_NATIVE)
    body = path.read_bytes()
    if info.st_size != pin['bytes'] or sha(body) != pin['sha256']:
        raise ValueError('native executable pin mismatch: ' + path.name)
    if len(body) < 64 or body[:7] != b'\x7fELF\x02\x01\x01':
        raise ValueError('ELF64 little-endian executable required')
    kind, machine, version = struct.unpack_from('<HHI', body, 16)
    entry, phoff = struct.unpack_from('<QQ', body, 24)
    ehsize, phsize, count = struct.unpack_from('<HHH', body, 52)
    if (kind != 2 or machine != 183 or version != 1 or ehsize != 64 or phsize != 56 or
            not 1 <= count <= 128 or phoff < 64 or phoff + phsize * count > len(body)):
        raise ValueError('static ARM64 ELF header required')
    loads, entry_found = 0, False
    for index in range(count):
        ptype, flags, offset, vaddr, _, size, memsize, alignment = struct.unpack_from(
            '<IIQQQQQQ', body, phoff + index * phsize)
        if ptype in {2, 3}:
            raise ValueError('native probe must have no PT_INTERP or PT_DYNAMIC')
        if ptype == 1:
            if size > len(body) - min(offset, len(body)) or offset > len(body) or memsize < size:
                raise ValueError('ELF load outside bounded actual file')
            if alignment > 1 and (alignment & (alignment - 1) or vaddr % alignment != offset % alignment):
                raise ValueError('invalid ELF load alignment')
            loads += 1
            entry_found |= bool(flags & 1 and vaddr <= entry < vaddr + size)
    if not loads or not entry_found:
        raise ValueError('ELF entry must lie in file-backed executable load')
    return body


def cpio_entry(name, mode, body, inode):
    encoded = name.encode('ascii') + b'\0'
    # All non-directory entries have one link; separate unique inodes avoid
    # carrying hard-link ownership between independently concatenated archives.
    fields = [inode, mode, 0, 0, 1, 0, len(body), 0, 0, 0, 0, len(encoded), 0]
    record = b'070701' + ''.join(f'{value:08x}' for value in fields).encode('ascii') + encoded
    record += b'\0' * (-len(record) % 4)
    record += body
    return record + b'\0' * (-len(record) % 4)


def ramdisk(init, child):
    records = [('dev', stat.S_IFDIR | 0o755, b''), ('proc', stat.S_IFDIR | 0o755, b''),
               ('sys', stat.S_IFDIR | 0o755, b''), ('init', stat.S_IFREG | 0o755, init),
               ('probe-child', stat.S_IFREG | 0o755, child), ('TRAILER!!!', 0, b'')]
    body = b''.join(cpio_entry(name, mode, content, inode)
                    for inode, (name, mode, content) in enumerate(records, 1))
    return body + b'\0' * (-len(body) % 512)


def legacy_lz4(body):
    library_name = ctypes.util.find_library('lz4')
    if not library_name:
        raise ValueError('host open-source liblz4 required')
    library = ctypes.CDLL(library_name)
    library.LZ4_compressBound.argtypes = [ctypes.c_int]
    library.LZ4_compressBound.restype = ctypes.c_int
    library.LZ4_compress_default.argtypes = [ctypes.c_void_p, ctypes.c_void_p,
                                            ctypes.c_int, ctypes.c_int]
    library.LZ4_compress_default.restype = ctypes.c_int
    library.LZ4_versionNumber.restype = ctypes.c_int
    result = bytearray(MAGIC)
    for offset in range(0, len(body), BLOCK):
        block = body[offset:offset + BLOCK]
        capacity = library.LZ4_compressBound(len(block))
        compressed = ctypes.create_string_buffer(capacity)
        count = library.LZ4_compress_default(block, compressed, len(block), capacity)
        if count <= 0:
            raise ValueError('liblz4 compression failed')
        result.extend(struct.pack('<I', count))
        result.extend(compressed.raw[:count])
    # Legacy framing has implicit EOF, with no modern frame descriptor,
    # zero end marker, checksum or content-size footer.
    return bytes(result), library.LZ4_versionNumber()


def inspect(binary, path):
    result = subprocess.run([str(binary), str(path)], check=True, capture_output=True, text=True)
    if len(result.stdout) > 1024 * 1024:
        raise ValueError('inspector report exceeds bound')
    return json.loads(result.stdout)


def extent_hash(image, extent):
    h = hashlib.sha256()
    with image.open('rb') as stream:
        stream.seek(extent['offset'])
        remaining = extent['size']
        while remaining:
            body = stream.read(min(remaining, 65536))
            if not body:
                raise ValueError('truncated original component extent')
            h.update(body)
            remaining -= len(body)
    return h.hexdigest()


def original_components(root, binary):
    reports, records = {}, {}
    for name in ['boot.img', 'init_boot.img', 'vendor_boot.img', 'vendor_kernel_boot.img']:
        report = inspect(binary, root / name)
        reports[name] = report
        fields = ['kernel', 'ramdisk'] if report['kind'] == 'boot-v4' else ['dtb', 'bootconfig']
        records[name] = {field: {'extent': report[field], 'sha256': extent_hash(root / name, report[field])}
                         for field in fields if report[field]['size']}
        for index, fragment in enumerate(report['fragments']):
            records[name][f'vendor_ramdisk{index:02}'] = {
                'extent': fragment['extent'], 'sha256': extent_hash(root / name, fragment['extent'])}
    # This tool embodies one explicit observed 4 KiB candidate; it does not
    # infer a bootloader selection rule from fragment types or names.
    if (reports['init_boot.img']['kernel']['size'] != 0 or
            reports['init_boot.img']['signature']['size'] != 0 or
            reports['init_boot.img']['page_size'] != 4096 or
            reports['boot.img']['ramdisk']['size'] != 0):
        raise ValueError('selected current Google boot/init layout changed')
    expected = [('vendor_boot.img', [1, 0], [b'', b'16K']),
                ('vendor_kernel_boot.img', [1], [b''])]
    for name, kinds, names in expected:
        fragments = reports[name]['fragments']
        if ([fragment['ramdisk_type'] for fragment in fragments] != kinds or
                [bytes(fragment['name']) for fragment in fragments] != names):
            raise ValueError('selected current Google vendor fragment layout changed')
    return reports, records


def compare_original_components(reference_root, reference, selected, records):
    prior = small_json(reference_root / 'comparison-summary.json')
    if prior.get('reference_commit') != reference.COMMIT:
        raise ValueError('pinned original-code factory comparisons required')
    compared = {record['image']: record for record in prior['results']}
    for name, components in records.items():
        record = compared[name]
        if record.get('original_component_bytes_equal') is not True or record['image_sha256'] != digest(selected / name):
            raise ValueError('original comparison container binding changed')
        for component, binding in components.items():
            if binding != record['components'][component]:
                raise ValueError('original source/binary extent binding changed')
            extracted = reference_root / 'comparison-unpacked' / name / component
            if extracted.stat().st_size != binding['extent']['size'] or digest(extracted) != binding['sha256']:
                raise ValueError('retained original-code extraction changed')


def run(args):
    pins = small_json(PINS_FILE)
    output = args.private_output_dir.resolve()
    outside_worktrees(output)
    if output.exists():
        raise ValueError('output label already exists; preserve prior evidence')
    input_root, proof = selection(args, pins)
    if any(output == path or path in output.parents or output in path.parents
           for path in [input_root, args.private_reference_root.resolve(),
                        args.native_init.resolve(), args.native_child.resolve(),
                        args.authenticated_selection.resolve()]):
        raise ValueError('private output must not overlap any input')
    reference_root = args.private_reference_root.resolve()
    reference = load_reference(reference_root, pins)
    regular(args.inspector, 32 * 1024 * 1024)
    if digest(args.inspector) != pins['inspector_sha256']:
        raise ValueError('frozen bounded Rust inspector binary changed')
    originals = verify_originals(input_root, pins)
    selection_digest = digest(args.authenticated_selection)
    init = native(args.native_init, pins['native_payload']['init'])
    child = native(args.native_child, pins['native_payload']['probe-child'])
    reports, components = original_components(input_root, args.inspector)
    compare_original_components(reference_root, reference, input_root, components)
    raw = ramdisk(init, child)
    compressed, lz4_version = legacy_lz4(raw)
    if 4096 + (len(compressed) + 4095) // 4096 * 4096 > CAPACITY:
        raise ValueError('candidate exceeds original init_boot partition')
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    (output / 'ramdisk.cpio').write_bytes(raw)
    (output / 'ramdisk.lz4').write_bytes(compressed)
    osv = reports['init_boot.img']['os_version']
    version = '.'.join(str((osv >> shift) & 127) for shift in [25, 18, 11])
    patch = f'{2000 + ((osv >> 4) & 127):04}-{osv & 15:02}'
    image = output / 'init_boot.img'
    reference.original(reference_root, 'mkbootimg.py', [
        '--header_version', '4', '--pagesize', '4096', '--os_version', version,
        '--os_patch_level', patch, '--ramdisk', output / 'ramdisk.lz4', '--output', image])
    reference.original(reference_root, 'unpack_bootimg.py', [
        '--boot_img', image, '--out', output / 'original-unpack'])
    report = inspect(args.inspector, image)
    if (report['kind'] != 'boot-v4' or report['page_size'] != 4096 or
            report['ramdisk']['size'] != len(compressed) or report['kernel']['size'] != 0 or
            report['signature']['size'] != 0 or report['trailing']['size'] != 0 or
            image.stat().st_size > CAPACITY or
            (output / 'original-unpack/ramdisk').read_bytes() != compressed):
        raise ValueError('original producer/unpacker and bounded inspector disagree')
    with (input_root / 'init_boot.img').open('rb') as stream:
        original_header = bytearray(stream.read(4096))
    struct.pack_into('<I', original_header, 12, len(compressed))
    with image.open('rb') as stream:
        if stream.read(4096) != original_header:
            raise ValueError('candidate header changed beyond generic ramdisk size')
        stream.seek(4096 + len(compressed))
        if any(stream.read(4096)):
            raise ValueError('new container page padding must be zero')
    reference.sources(reference_root, False)
    if verify_originals(input_root, pins) != originals or digest(args.authenticated_selection) != selection_digest:
        raise ValueError('immutable original inputs changed during packaging')
    if (digest(args.native_init) != pins['native_payload']['init']['sha256'] or
            digest(args.native_child) != pins['native_payload']['probe-child']['sha256']):
        raise ValueError('native payload inputs changed during packaging')
    manifest = {
        'schema': 1, 'candidate_class': pins['candidate_class'], 'unsigned': True, 'local_only': True,
        'factory_archive': {**pins['factory_archive'], 'authentication_evidence': 'inherited-whole-archive-plus-38-rebound-index-reads',
                            'whole_archive_rehashed_by_this_tool': False},
        'immutable_selected_inputs': originals, 'immutable_components': components,
        'immutable_inputs_unchanged': True,
        'replacement_scope': 'init_boot-generic-ramdisk-only',
        'image': {'name': 'init_boot.img', 'bytes': image.stat().st_size, 'sha256': digest(image),
                  'partition_capacity': CAPACITY, 'header_version': 4, 'container_page_size': 4096,
                  'stock_header_preserved_except_ramdisk_size': True, 'copied_signature_or_avb_footer': False,
                  'os_version_field_preserved': osv},
        'generic_ramdisk': {'cpio_sha256': sha(raw), 'cpio_bytes': len(raw),
                            'compressed_sha256': sha(compressed), 'compressed_bytes': len(compressed),
                            'compression': 'legacy-lz4-8MiB-blocks-implicit-EOF', 'liblz4_version_number': lz4_version,
                            'paths': ['dev', 'proc', 'sys', 'init', 'probe-child'], 'uid': 0, 'gid': 0,
                            'mtime': 0, 'hard_links': False},
        'root_init': {**pins['native_payload']['init'], 'path': '/init', 'mode': '0100755', 'uid': 0, 'gid': 0,
                      'owner': 'native-rust-pid1-probe', 'replaces_vendor_symlink': '/system/bin/init',
                      'no_interpreter_or_dynamic_segment': True},
        'diagnostic_child': {**pins['native_payload']['probe-child'], 'path': '/probe-child', 'mode': '0100755'},
        'fragment_choice': {'kernel': 'unchanged-boot.img-4KiB',
                            'include': ['vendor_kernel_boot.img/vendor_ramdisk00', 'vendor_boot.img/vendor_ramdisk00'],
                            'exclude': ['vendor_boot.img/vendor_ramdisk01:16K'],
                            'generic_loaded_last': True, 'no_padding_between_compressed_frames': True,
                            'order_scope': 'offline-decoder-candidate-choice', 'bootloader_behavior_qualified': False},
        'original_format_execution': {'reference_commit': reference.COMMIT, 'source_sha256': reference.SOURCE_HASHES,
                                      'producer_unmodified': True, 'unpacker_unmodified': True,
                                      'candidate_ramdisk_roundtrip_equal': True,
                                      'inspector_sha256': pins['inspector_sha256'], 'inspector_geometry_agrees': True},
        'qualifications': {name: False for name in [
            'avb_verified', 'bootloader_accepted', 'kernel_kmi_qualified', 'pixel_kernel_runtime_verified',
            'physical_device_accessed', 'deployment_admitted', 'full_phone_init',
            'persistent_storage_mounted', 'original_vendor_executed', 'temporary_fastboot_boot_qualified']},
        'remaining_gates': ['AVB-signing-chain-and-bootloader-admission', 'temporary-RAM-boot-across-all-required-partitions',
                             'bootloader-4KiB-fragment-selection-and-order', 'unchanged-Pixel-kernel-native-probe-runtime',
                             'diagnostic-console-USB-pstore-observability', 'watchdog-thermal-charging-safe-early-runtime',
                             'kernel-KMI-protected-symbol-CRC-and-module-admission', 'native-full-phone-init-SELinux-and-service-integration',
                             'later-authorized-physical-device-validation'],
        'evidence_limits': ['No stock Google kernel boot or vendor executable execution occurred.',
                            'Arch ARM native runtime evidence is separate from this offline container proof.',
                            'Retained vendor archives contain original userspace files but this generic init starts only its diagnostic child.',
                            'The full Pixel smartphone deployment profile remains refused.']}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'candidate_class': pins['candidate_class'], 'image_sha256': digest(image),
                      'image_bytes': image.stat().st_size, 'unsigned': True, 'local_only': True,
                      'deployment_admitted': False, 'physical_device_accessed': False}))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ['authenticated-selection', 'private-reference-root', 'inspector',
                 'native-init', 'native-child', 'private-output-dir']:
        p.add_argument('--' + name, type=pathlib.Path, required=True)
    args = p.parse_args()
    # Original Python modules stay immutable even at the cache-file boundary.
    os.environ['PYTHONDONTWRITEBYTECODE'] = '1'
    try:
        run(args)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print('native boot candidate refused: ' + str(error), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
