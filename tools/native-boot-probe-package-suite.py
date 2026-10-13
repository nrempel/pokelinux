#!/usr/bin/env python3
"""Actual factory red and independently decoded offline candidate checks.

This verifier never imports the packager's CPIO/LZ4 code or executes any ELF.
Original payloads remain private. It exercises unchanged pinned AOSP tools and
the previously frozen Rust inspector at their actual source/binary boundaries.
"""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
import pathlib
import stat
import struct
import subprocess
import sys

INIT_SHA = 'cbf938d20127b02dc84afe2bd0e1fc4c66bd6f2d33d33c436e7a8c9fe37d205c'
CHILD_SHA = '3b15eb4a9fd289e6664c917861f98bbe385e117e26b28abb149b3021e4b19e63'
MAGIC = bytes.fromhex('02214c18')
BLOCK = 8 * 1024 * 1024
REPO = pathlib.Path(__file__).resolve().parent.parent


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def sha(body):
    return hashlib.sha256(body).hexdigest()


def decode_stream(body):
    """Decode actual concatenated legacy frames, not a pre-split mock stream."""
    assert len(body) <= 64 * 1024 * 1024, 'compressed stream bound'
    lib = ctypes.CDLL(ctypes.util.find_library('lz4'))
    lib.LZ4_decompress_safe.argtypes = [ctypes.c_void_p, ctypes.c_void_p,
                                      ctypes.c_int, ctypes.c_int]
    lib.LZ4_decompress_safe.restype = ctypes.c_int
    position, frames = 0, []
    while position < len(body):
        assert body[position:position + 4] == MAGIC, 'actual legacy magic'
        start = position
        position += 4
        blocks = []
        while position < len(body) and body[position:position + 4] != MAGIC:
            assert position + 4 <= len(body), 'complete block size'
            length, = struct.unpack_from('<I', body, position)
            position += 4
            assert 0 < length <= BLOCK + BLOCK // 255 + 16, 'bounded compressed block'
            assert position + length <= len(body), 'complete compressed block'
            buffer = ctypes.create_string_buffer(BLOCK)
            count = lib.LZ4_decompress_safe(body[position:position + length], buffer,
                                            length, BLOCK)
            assert 0 < count <= BLOCK, 'liblz4 decode success'
            blocks.append(buffer.raw[:count])
            position += length
            assert sum(map(len, blocks)) <= 128 * 1024 * 1024, 'frame output bound'
        assert blocks, 'nonempty actual frame'
        assert all(len(block) == BLOCK for block in blocks[:-1]), 'legacy filled blocks'
        frames.append({'compressed_sha256': sha(body[start:position]),
                       'body': b''.join(blocks), 'blocks': len(blocks)})
        assert len(frames) <= 8, 'frame count bound'
    return frames


def archive(body, strict=False):
    """Independent newc inspection, retaining content only for the current frame."""
    cursor, records = 0, {}
    while cursor + 110 <= len(body):
        assert body[cursor:cursor + 6] == b'070701', 'newc magic'
        values = [int(body[cursor + 6 + i * 8:cursor + 14 + i * 8], 16)
                  for i in range(13)]
        namesize, size = values[11], values[6]
        assert 1 <= namesize <= 4096 and size <= 128 * 1024 * 1024, 'newc bounds'
        name_end = cursor + 110 + namesize
        assert name_end <= len(body) and body[name_end - 1] == 0, 'terminated name'
        name = body[cursor + 110:name_end - 1].decode('utf-8')
        data_start = (name_end + 3) // 4 * 4
        data_end = data_start + size
        assert data_end <= len(body), 'bounded data extent'
        data = body[data_start:data_end]
        cursor = (data_end + 3) // 4 * 4
        if name == 'TRAILER!!!':
            assert size == 0 and not any(body[cursor:]), 'trailer and zero padding'
            return records
        if strict:
            assert name not in records, 'no duplicate paths in the new native archive'
        assert name and not name.startswith('/') and '..' not in name.split('/'), 'safe path'
        records[name] = {'mode': values[1], 'uid': values[2], 'gid': values[3],
                         'nlink': values[4], 'mtime': values[5], 'bytes': size,
                         'sha256': sha(data), 'data': data}
        assert len(records) <= 10000, 'entry bound'
    raise AssertionError('required actual CPIO trailer')


def boot_ramdisk(path):
    with path.open('rb') as stream:
        header = stream.read(4096)
        assert header[:8] == b'ANDROID!', 'boot header magic'
        kernel, ramdisk, _, size = struct.unpack_from('<4I', header, 8)
        assert size == 1584 and struct.unpack_from('<I', header, 40)[0] == 4, 'header v4'
        stream.seek(4096 + (kernel + 4095) // 4096 * 4096)
        payload = stream.read(ramdisk)
        assert len(payload) == ramdisk, 'actual boot ramdisk extent'
    return header, payload


def factory_red(args):
    path = args.authenticated_selection.parent / 'selected/init_boot.img'
    proof = json.loads(args.authenticated_selection.read_text())
    record = next(record for record in proof['selected'] if record['name'] == 'init_boot.img')
    assert path.stat().st_size == record['bytes'] and digest(path) == record['sha256']
    _, compressed = boot_ramdisk(path)
    frames = decode_stream(compressed)
    assert len(frames) == 1
    init = archive(frames[0]['body'])['init']
    assert stat.S_ISREG(init['mode']) and init['data'][:4] == b'\x7fELF'
    actual = init['sha256']
    print(json.dumps({'boundary': 'actual-authenticated-factory-init-boot-CPIO-init',
                      'stock_init_sha256': actual, 'required_native_init_sha256': INIT_SHA,
                      'stock_init_bytes': init['bytes'], 'native_init_present': actual == INIT_SHA,
                      'phone_accessed': False}), flush=True)
    assert actual == INIT_SHA, 'actual stock /init does not implement pinned native PID1'


def invoke(args, output, extra=None):
    command = [sys.executable, str(REPO / 'tools/native-boot-probe-package.py'),
               '--authenticated-selection', str(args.authenticated_selection),
               '--private-reference-root', str(args.private_reference_root),
               '--inspector', str(args.inspector), '--native-init', str(args.native_init),
               '--native-child', str(args.native_child), '--private-output-dir', str(output)]
    if extra:
        command.extend(extra)
    return subprocess.run(command, capture_output=True, text=True)


def check_candidate(args, output):
    image = output / 'init_boot.img'
    header, compressed = boot_ramdisk(image)
    stock_header, stock_compressed = boot_ramdisk(
        args.authenticated_selection.parent / 'selected/init_boot.img')
    expected_header = bytearray(stock_header)
    struct.pack_into('<I', expected_header, 12, len(compressed))
    assert header == expected_header, 'original header unchanged except ramdisk size'
    assert image.stat().st_size == 4096 + (len(compressed) + 4095) // 4096 * 4096
    assert image.stat().st_size <= 8 * 1024 * 1024, 'original partition capacity'
    assert struct.unpack_from('<I', header, 1580)[0] == 0, 'no copied signature'
    assert compressed[:4] == stock_compressed[:4] == MAGIC, 'actual Google legacy framing'
    assert compressed == (output / 'ramdisk.lz4').read_bytes()
    frames = decode_stream(compressed)
    assert len(frames) == 1 and frames[0]['body'] == (output / 'ramdisk.cpio').read_bytes()
    entries = archive(frames[0]['body'], strict=True)
    assert set(entries) == {'dev', 'proc', 'sys', 'init', 'probe-child'}, 'minimal native archive'
    for name, entry in entries.items():
        expected_mode = (stat.S_IFDIR | 0o755) if name in {'dev', 'proc', 'sys'} else (stat.S_IFREG | 0o755)
        assert entry['mode'] == expected_mode and entry['uid'] == entry['gid'] == 0
        assert entry['mtime'] == 0 and entry['nlink'] == 1, 'deterministic ownership and no hard links'
        assert not stat.S_ISLNK(entry['mode']), 'no generic Android init symlink'
    assert entries['init']['data'] == args.native_init.read_bytes()
    assert entries['init']['sha256'] == INIT_SHA
    assert entries['probe-child']['data'] == args.native_child.read_bytes()
    assert entries['probe-child']['sha256'] == CHILD_SHA
    original = subprocess.run([sys.executable, str(args.private_reference_root / 'unpack_bootimg.py'),
                               '--boot_img', str(image), '--out', str(output / 'independent-original-unpack')],
                              check=True, capture_output=True, text=True)
    assert original.returncode == 0
    assert (output / 'independent-original-unpack/ramdisk').read_bytes() == compressed
    inspected = json.loads(subprocess.run([str(args.inspector), str(image)], check=True,
                                         capture_output=True, text=True).stdout)
    assert inspected['kind'] == 'boot-v4' and inspected['page_size'] == 4096
    assert inspected['kernel']['size'] == inspected['signature']['size'] == 0
    assert inspected['trailing']['size'] == 0
    assert inspected['ramdisk'] == {'offset': 4096, 'size': len(compressed)}
    assert not any(image.read_bytes()[4096 + len(compressed):]), 'only new zero page padding'
    # Use the actual retained unchanged original extractions, each rebound to its
    # authenticated selected container by the earlier seven comparisons.
    comparison = json.loads((args.private_reference_root / 'comparison-summary.json').read_text())
    compared = {record['image']: record for record in comparison['results']}
    selected = args.authenticated_selection.parent / 'selected'
    vendors = []
    for name in ['vendor_kernel_boot.img', 'vendor_boot.img']:
        record = compared[name]
        assert digest(selected / name) == record['image_sha256']
        binding = record['components']['vendor_ramdisk00']
        with (selected / name).open('rb') as stream:
            stream.seek(binding['extent']['offset'])
            body = stream.read(binding['extent']['size'])
        assert len(body) == binding['extent']['size'] and sha(body) == binding['sha256']
        vendors.append(body)
    combined = b''.join([*vendors, compressed])
    streams = decode_stream(combined)
    assert len(streams) == 3, 'three actual concatenated legacy frames'
    assert [frame['compressed_sha256'] for frame in streams] == [sha(body) for body in [*vendors, compressed]]
    overlay, vendor_init = {}, None
    for index, frame in enumerate(streams):
        current = archive(frame['body'])
        if index == 1:
            vendor_init = current['init']
            assert stat.S_ISLNK(vendor_init['mode']) and vendor_init['data'] == b'/system/bin/init'
        overlay.update({name: {**record, 'source_frame': index} for name, record in current.items()})
    assert stat.S_ISREG(overlay['init']['mode']) and overlay['init']['sha256'] == INIT_SHA
    assert overlay['init']['source_frame'] == 2 and overlay['init']['uid'] == 0
    assert overlay['probe-child']['source_frame'] == 2 and overlay['probe-child']['sha256'] == CHILD_SHA
    manifest = json.loads((output / 'manifest.json').read_text())
    assert manifest['candidate_class'] == 'UNQUALIFIED_PIXEL_NATIVE_BOOT_PROBE_CANDIDATE'
    for flag in ['avb_verified', 'bootloader_accepted', 'kernel_kmi_qualified',
                 'pixel_kernel_runtime_verified', 'physical_device_accessed', 'deployment_admitted',
                 'full_phone_init', 'persistent_storage_mounted', 'original_vendor_executed']:
        assert manifest['qualifications'][flag] is False, 'no qualification escalation: ' + flag
    assert manifest['unsigned'] is True and manifest['local_only'] is True
    assert manifest['immutable_inputs_unchanged'] is True
    assert manifest['root_init']['sha256'] == INIT_SHA and manifest['root_init']['owner'] == 'native-rust-pid1-probe'
    assert manifest['image']['sha256'] == digest(image)
    assert manifest['fragment_choice']['bootloader_behavior_qualified'] is False
    assert len(manifest['remaining_gates']) >= 7
    return {'image_sha256': digest(image), 'image_bytes': image.stat().st_size,
            'generic_init_sha256': INIT_SHA, 'generic_child_sha256': CHILD_SHA,
            'independent_original_unpack_and_rust_inspector': True,
            'actual_three_frame_overlay': True, 'vendor_init_target': vendor_init['data'].decode(),
            'generic_init_replaces_vendor_symlink': True, 'full_deployment_refused': True}


def green(args):
    args.private_evidence_dir = args.private_evidence_dir.resolve()
    assert not any((path / '.git').exists() for path in [args.private_evidence_dir, *args.private_evidence_dir.parents]), 'evidence must remain outside every Git worktree'
    assert args.private_red_log.stat().st_size <= 65536, 'bounded original-red log'
    red_text = args.private_red_log.read_text()
    red = json.loads(red_text.splitlines()[0])
    assert red['boundary'] == 'actual-authenticated-factory-init-boot-CPIO-init'
    assert red['stock_init_sha256'] != INIT_SHA and red['required_native_init_sha256'] == INIT_SHA
    assert red['native_init_present'] is False
    assert 'AssertionError: actual stock /init does not implement pinned native PID1' in red_text
    args.private_evidence_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    proof = json.loads(args.authenticated_selection.read_text())
    selected = args.authenticated_selection.parent / 'selected'
    before = {record['name']: digest(selected / record['name']) for record in proof['selected']}
    results = []
    for label in ['first', 'reproduced']:
        output = args.private_evidence_dir / label
        result = invoke(args, output)
        assert result.returncode == 0, result.stdout + result.stderr
        results.append(check_candidate(args, output))
    assert results[0] == results[1], 'actual packaging reproducibility'
    # Negative paths use actual file/source boundaries; no mocked artifact store.
    mutated = args.private_evidence_dir / 'changed-native-init'
    body = bytearray(args.native_init.read_bytes())
    body[-1] ^= 1
    mutated.write_bytes(body)
    rejected = args.private_evidence_dir / 'refused-changed-init'
    result = invoke(args, rejected, ['--native-init', str(mutated)])
    assert result.returncode == 2 and 'native executable pin mismatch' in result.stderr
    assert not rejected.exists(), 'no image emitted for changed native ELF'
    changed_child = args.private_evidence_dir / 'changed-native-child'
    body = bytearray(args.native_child.read_bytes())
    body[-1] ^= 1
    changed_child.write_bytes(body)
    rejected = args.private_evidence_dir / 'refused-changed-child'
    result = invoke(args, rejected, ['--native-child', str(changed_child)])
    assert result.returncode == 2 and 'native executable pin mismatch' in result.stderr
    assert not rejected.exists()
    # The authenticated stock Android init itself must be refused as a native payload.
    _, compressed = boot_ramdisk(selected / 'init_boot.img')
    stock_init = archive(decode_stream(compressed)[0]['body'])['init']['data']
    assert sha(stock_init) == red['stock_init_sha256'], 'red log is bound to this actual original'
    assert len(stock_init) == red['stock_init_bytes']
    stock_path = args.private_evidence_dir / 'original-stock-init.private'
    stock_path.write_bytes(stock_init)
    rejected = args.private_evidence_dir / 'refused-stock-init'
    result = invoke(args, rejected, ['--native-init', str(stock_path)])
    assert result.returncode == 2 and 'native executable pin mismatch' in result.stderr
    assert not rejected.exists()
    result = invoke(args, args.private_evidence_dir / 'first')
    assert result.returncode == 2 and 'output label already exists' in result.stderr
    assert digest(args.private_evidence_dir / 'first/init_boot.img') == results[0]['image_sha256']
    rejected = REPO / 'native-boot-package-forbidden-output'
    result = invoke(args, rejected)
    assert result.returncode == 2 and 'outside every Git worktree' in result.stderr
    assert not rejected.exists()
    changed_selection = args.private_evidence_dir / 'changed-selection.json'
    changed_proof = dict(proof)
    changed_proof['sha256'] = '0' * 64
    changed_selection.write_text(json.dumps(changed_proof))
    rejected = args.private_evidence_dir / 'refused-unauthenticated-selection'
    result = invoke(args, rejected, ['--authenticated-selection', str(changed_selection)])
    assert result.returncode == 2 and 'whole official archive authentication required' in result.stderr
    assert not rejected.exists()
    # Mutate an actual private original producer copy, preserving upstream notices.
    changed_reference = args.private_evidence_dir / 'changed-original-source'
    changed_reference.mkdir(mode=0o700)
    for name in ['mkbootimg.py', 'unpack_bootimg.py', 'include/bootimg/bootimg.h', 'gki/generate_gki_certificate.py']:
        target = changed_reference / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((args.private_reference_root / name).read_bytes())
    producer = changed_reference / 'mkbootimg.py'
    producer.write_bytes(producer.read_bytes() + b'\n# private original-source mutation negative\n')
    rejected = args.private_evidence_dir / 'refused-changed-original-source'
    result = invoke(args, rejected, ['--private-reference-root', str(changed_reference)])
    assert result.returncode == 2 and 'pinned original source changed' in result.stderr
    assert not rejected.exists()
    changed_inspector = args.private_evidence_dir / 'changed-inspector'
    body = bytearray(args.inspector.read_bytes())
    body[-1] ^= 1
    changed_inspector.write_bytes(body)
    rejected = args.private_evidence_dir / 'refused-changed-inspector'
    result = invoke(args, rejected, ['--inspector', str(changed_inspector)])
    assert result.returncode == 2 and 'frozen bounded Rust inspector binary changed' in result.stderr
    assert not rejected.exists()
    after = {record['name']: digest(selected / record['name']) for record in proof['selected']}
    assert before == after, 'all original selected inputs unchanged'
    report = {'schema': 1, 'actual_original_red_precedes_implementation': True,
              'actual_original_red': {**red, 'log_sha256': digest(args.private_red_log),
                                     'frozen_pre_implementation_verifier_sha256': digest(args.private_red_suite)},
              'candidate_checks': results[0], 'reproducible_two_actual_builds': True,
              'negative_groups': ['changed-native-init', 'changed-native-child', 'actual-stock-Android-init',
                                  'existing-output-label', 'Git-output', 'unauthenticated-selection',
                                  'changed-original-AOSP-source', 'changed-Rust-inspector'],
              'immutable_selected_files_rechecked': len(after), 'immutable_inputs_unchanged': True,
              'phone_accessed': False, 'pixel_kernel_booted': False, 'avb_verified': False}
    (args.private_evidence_dir / 'green-summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['factory-red', 'green'])
    p.add_argument('--authenticated-selection', required=True, type=pathlib.Path)
    p.add_argument('--private-reference-root', type=pathlib.Path)
    p.add_argument('--inspector', type=pathlib.Path)
    p.add_argument('--native-init', type=pathlib.Path)
    p.add_argument('--native-child', type=pathlib.Path)
    p.add_argument('--private-evidence-dir', type=pathlib.Path)
    p.add_argument('--private-red-log', type=pathlib.Path)
    p.add_argument('--private-red-suite', type=pathlib.Path)
    args = p.parse_args()
    if args.action == 'factory-red':
        factory_red(args)
    else:
        for name in ['private_reference_root', 'inspector', 'native_init', 'native_child', 'private_evidence_dir',
                     'private_red_log', 'private_red_suite']:
            if getattr(args, name) is None:
                p.error('green requires --' + name.replace('_', '-'))
        green(args)


if __name__ == '__main__':
    main()
