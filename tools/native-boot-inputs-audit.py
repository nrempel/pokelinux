#!/usr/bin/env python3
"""Read-only factory kernel/ramdisk metadata audit; no payload execution or flashing.

Requires full-archive authentication plus hash-rechecked selected originals and
unchanged AOSP extraction from native-boot-inputs-reference.py. LZ4 decoding uses
the host open-source liblz4 with fixed input/output bounds. Only clean metadata
is written; no factory executable, config body or ramdisk payload is published.
"""
import argparse
import collections
import ctypes
import ctypes.util
import hashlib
import json
import pathlib
import re
import stat
import struct
import subprocess
import zlib

FACTORY_SHA = '3721e6accc2e87d851ee7966d794318078ec74c7cc32343daa10968264d6bb62'
FLAGS = ['CONFIG_ARM64_4K_PAGES', 'CONFIG_ARM64_16K_PAGES', 'CONFIG_ARM64_64K_PAGES',
         'CONFIG_BLK_DEV_INITRD', 'CONFIG_RD_LZ4', 'CONFIG_NAMESPACES', 'CONFIG_PID_NS',
         'CONFIG_DEVTMPFS', 'CONFIG_TMPFS', 'CONFIG_PROC_FS', 'CONFIG_SYSFS', 'CONFIG_UNIX',
         'CONFIG_TTY', 'CONFIG_VT', 'CONFIG_SERIAL_EARLYCON', 'CONFIG_SERIAL_8250_CONSOLE',
         'CONFIG_SERIAL_AMBA_PL011_CONSOLE', 'CONFIG_SERIAL_SAMSUNG_CONSOLE',
         'CONFIG_PSTORE', 'CONFIG_PSTORE_RAM', 'CONFIG_PSTORE_CONSOLE', 'CONFIG_PSTORE_PMSG',
         'CONFIG_USB_GADGET', 'CONFIG_USB_CONFIGFS', 'CONFIG_USB_CONFIGFS_ACM',
         'CONFIG_USB_CONFIGFS_F_FS', 'CONFIG_USB_DWC3', 'CONFIG_USB_DWC3_DUAL_ROLE',
         'CONFIG_MODVERSIONS', 'CONFIG_MODULE_SIG', 'CONFIG_MODULE_SIG_FORCE',
         'CONFIG_MODULE_SIG_PROTECT', 'CONFIG_SECURITY_SELINUX',
         'CONFIG_SECURITY_SELINUX_BOOTPARAM', 'CONFIG_IKCONFIG', 'CONFIG_PRINTK']


def sha(body):
    return hashlib.sha256(body).hexdigest()


def file_sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def legacy_lz4(path):
    if path.stat().st_size > 64 * 1024 * 1024:
        raise ValueError('compressed input exceeds audit bound')
    body = path.read_bytes()
    if body[:4] != bytes.fromhex('02214c18'):
        raise ValueError('expected selected legacy LZ4 framing')
    library = ctypes.CDLL(ctypes.util.find_library('lz4'))
    decode = library.LZ4_decompress_safe
    decode.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
    decode.restype = ctypes.c_int
    at, result = 4, bytearray()
    while at < len(body):
        if at + 4 > len(body):
            raise ValueError('truncated LZ4 block size')
        length = struct.unpack_from('<I', body, at)[0]
        at += 4
        if not 0 < length <= 8 * 1024 * 1024 + 65536 or at + length > len(body):
            raise ValueError('LZ4 block outside bounded input')
        output = ctypes.create_string_buffer(8 * 1024 * 1024)
        count = decode(body[at:at + length], output, length, len(output))
        if count < 0 or len(result) + count > 128 * 1024 * 1024:
            raise ValueError('LZ4 decoding failed or exceeds output bound')
        result.extend(output.raw[:count])
        at += length
    return bytes(result)


def kernel(path):
    body = legacy_lz4(path)
    if body[56:60] != b'ARM\x64':
        raise ValueError('missing original ARM64 Image magic')
    page_flag = (struct.unpack_from('<Q', body, 24)[0] >> 1) & 3
    release = re.search(rb'Linux version ([A-Za-z0-9._+\-]+)', body)
    marker = body.find(b'IKCFG_ST')
    if marker < 0 or not release:
        raise ValueError('missing embedded release/config reference')
    decoder = zlib.decompressobj(16 + zlib.MAX_WBITS)
    config = decoder.decompress(body[marker + 8:], 1024 * 1024)
    if not decoder.eof or not decoder.unused_data.startswith(b'IKCFG_ED'):
        raise ValueError('embedded kernel config outside bounded envelope')
    lines = config.decode('ascii').splitlines()
    flags = {}
    for key in FLAGS:
        line = next((line for line in lines if line.startswith(key + '=')
                     or line == '# ' + key + ' is not set'), None)
        flags[key] = line.split('=', 1)[1] if line and '=' in line else 'n' if line else 'absent'
    expected = 1 if flags['CONFIG_ARM64_4K_PAGES'] == 'y' else 2 if flags['CONFIG_ARM64_16K_PAGES'] == 'y' else 0
    if page_flag != expected:
        raise ValueError('ARM64 Image page flags disagree with embedded config')
    return {'compressed_sha256': file_sha(path), 'image_sha256': sha(body),
            'image_bytes': len(body), 'kernel_release': release[1].decode(),
            'image_page_flag': page_flag, 'config_sha256': sha(config), 'config_flags': flags}


def cpio(path):
    body = legacy_lz4(path)
    at, entries, modules, init, text = 0, [], [], None, {}
    while at + 110 <= len(body):
        if body[at:at + 6] != b'070701':
            raise ValueError('invalid selected newc record')
        fields = [int(body[at + 6 + i * 8:at + 14 + i * 8], 16) for i in range(13)]
        namesize, size = fields[11], fields[6]
        if not 1 <= namesize <= 4096 or size > 128 * 1024 * 1024:
            raise ValueError('cpio field exceeds bound')
        endname = at + 110 + namesize
        if endname > len(body) or body[endname - 1] != 0:
            raise ValueError('truncated cpio name')
        name = body[at + 110:endname - 1].decode('utf-8')
        start = (endname + 3) & ~3
        end = start + size
        if end > len(body):
            raise ValueError('cpio payload outside archive')
        data = body[start:end]
        at = (end + 3) & ~3
        if name == 'TRAILER!!!':
            if any(body[at:]):
                raise ValueError('uninterpreted nonzero cpio tail')
            break
        entry = {'path': name, 'mode': oct(fields[1]), 'bytes': size, 'sha256': sha(data)}
        entries.append(entry)
        if len(entries) > 10000:
            raise ValueError('too many cpio entries')
        if name == 'init':
            init = {**entry, 'kind': 'regular' if stat.S_ISREG(fields[1]) else 'symlink' if stat.S_ISLNK(fields[1]) else 'other'}
            if stat.S_ISLNK(fields[1]):
                init['target'] = data.decode('utf-8')
        if name.endswith('.ko'):
            modules.append({'sha256': sha(data), 'vermagic': [v.decode('ascii') for v in re.findall(rb'vermagic=([^\x00]+)', data)]})
        if name.endswith('modules.load') or name.endswith('modules.dep'):
            text[name] = {'sha256': sha(data), 'bytes': size, 'lines': len(data.splitlines())}
    else:
        raise ValueError('missing cpio trailer')
    return {'compressed_sha256': file_sha(path), 'newc_sha256': sha(body),
            'newc_bytes': len(body), 'entries': len(entries), 'init': init,
            'module_count': len(modules), 'module_vermagic_counts': dict(collections.Counter(
                value for module in modules for value in module['vermagic'])), 'module_metadata': text}


def dlkm(image):
    listing = subprocess.run(['debugfs', '-R', 'ls -p /lib/modules', str(image)],
                             check=True, capture_output=True).stdout.decode('ascii')
    records, text = [], {}
    for line in listing.splitlines():
        fields = line.split('/')
        if len(fields) < 7:
            continue
        name = fields[5]
        if not re.fullmatch(r'[A-Za-z0-9_.-]+', name) or not name.endswith(('.ko', '.dep', '.load')):
            continue
        size = int(fields[6])
        if size > 16 * 1024 * 1024:
            raise ValueError('module metadata entry exceeds bound')
        data = subprocess.run(['debugfs', '-R', 'cat /lib/modules/' + name, str(image)],
                              check=True, capture_output=True).stdout
        if len(data) != size:
            raise ValueError('debugfs entry size disagrees with listing')
        if name.endswith('.ko'):
            records.extend(v.decode('ascii') for v in re.findall(rb'vermagic=([^\x00]+)', data))
        else:
            text[name] = {'sha256': sha(data), 'bytes': size, 'lines': len(data.splitlines())}
    return {'image_sha256': file_sha(image), 'module_count': len(records),
            'module_vermagic_counts': dict(collections.Counter(records)), 'module_metadata': text}


def dtb(path):
    if path.stat().st_size > 8 * 1024 * 1024:
        raise ValueError('DTB payload exceeds bound')
    body, at, records = path.read_bytes(), 0, []
    while at < len(body):
        if at + 40 > len(body) or body[at:at + 4] != bytes.fromhex('d00dfeed'):
            raise ValueError('invalid concatenated FDT header')
        size = struct.unpack_from('>I', body, at + 4)[0]
        version = struct.unpack_from('>I', body, at + 20)[0]
        if size < 40 or size > len(body) - at or version != 17:
            raise ValueError('unsupported or truncated selected FDT')
        records.append({'offset': at, 'bytes': size, 'version': version,
                        'sha256': sha(body[at:at + size])})
        at += size
    return records


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('authenticated_selection', type=pathlib.Path)
    p.add_argument('private_reference_root', type=pathlib.Path)
    args = p.parse_args()
    root = args.private_reference_root.resolve()
    repo = pathlib.Path(__file__).resolve().parent.parent
    if root == repo or repo in root.parents:
        p.error('audit outputs must remain outside repository')
    proof = json.loads(args.authenticated_selection.read_text())
    if (proof.get('whole_archive_authenticated') is not True or proof.get('sha256') != FACTORY_SHA
            or proof.get('bytes') != 11378664978 or proof.get('index_reads_rebound') != 38):
        raise ValueError('whole official archive authentication required')
    selected = args.authenticated_selection.parent / 'selected'
    for entry in proof['selected']:
        if not re.fullmatch(r'[A-Za-z0-9_.-]+', entry['name']):
            raise ValueError('unsafe selection name')
        path = selected / entry['name']
        if path.stat().st_size != entry['bytes'] or file_sha(path) != entry['sha256']:
            raise ValueError('authenticated selected bytes changed')
    unpacked = root / 'comparison-unpacked'
    comparison = json.loads((root / 'comparison-summary.json').read_text())
    if comparison.get('reference_commit') != 'd2bb0af5ba6d3198a3e99529c97eda1be0b5a093':
        raise ValueError('pinned original comparison required')
    compared = {entry['image']: entry for entry in comparison['results']}
    for name in ['boot.img', 'init_boot.img', 'vendor_boot.img', 'vendor_kernel_boot.img']:
        record = compared[name]
        if file_sha(selected / name) != record['image_sha256']:
            raise ValueError('selected original differs from comparison')
        for component, binding in record['components'].items():
            extracted = unpacked / name / component
            if file_sha(extracted) != binding['sha256'] or extracted.stat().st_size != binding['extent']['size']:
                raise ValueError('original unpacked component changed')
            h = hashlib.sha256()
            with (selected / name).open('rb') as source:
                source.seek(binding['extent']['offset'])
                remaining = binding['extent']['size']
                while remaining:
                    data = source.read(min(remaining, 65536))
                    if not data:
                        raise ValueError('comparison extent truncated')
                    h.update(data)
                    remaining -= len(data)
            if h.hexdigest() != binding['sha256']:
                raise ValueError('original component differs from authenticated bytes')
    report = {'factory_sha256': FACTORY_SHA, 'whole_archive_authenticated': True,
              'kernel_4k': kernel(unpacked / 'boot.img/kernel'), 'kernel_16k': kernel(selected / 'kernel_16k'),
              'dtb_records': dtb(unpacked / 'vendor_kernel_boot.img/dtb'),
              'selected_artifacts': [{k: entry[k] for k in ['name', 'bytes', 'sha256']} for entry in proof['selected']],
              'ramdisks': {name: cpio(unpacked / name) for name in [
                  'init_boot.img/ramdisk', 'vendor_boot.img/vendor_ramdisk00',
                  'vendor_boot.img/vendor_ramdisk01', 'vendor_kernel_boot.img/vendor_ramdisk00']},
              'dlkm': {name: dlkm(selected / name) for name in ['system_dlkm.img', 'vendor_dlkm.img']},
              'avb_verified': False, 'kernel_kmi_qualified': False,
              'original_vendor_payload_executed': False, 'guest_pixel_kernel_executed': False, 'phone_accessed': False}
    report['external_16k_ramdisk_equals_vendor_fragment'] = file_sha(selected / 'ramdisk_16k.img') == file_sha(unpacked / 'vendor_boot.img/vendor_ramdisk01')
    path = root / 'audit-summary.json'
    path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'audit_summary': str(path), 'kernel_kmi_qualified': False, 'phone_accessed': False}))


if __name__ == '__main__':
    main()
