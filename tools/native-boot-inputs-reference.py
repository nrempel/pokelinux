#!/usr/bin/env python3
"""Pin unchanged AOSP sources; compare geometry against original code privately.

Downloaded sources retain their embedded licenses. Factory files and original
unpacked payloads belong only in a private directory outside the repository.
This tool never runs a factory executable or touches a phone.
"""
import argparse
import base64
import hashlib
import json
import pathlib
import subprocess
import urllib.request

COMMIT = 'd2bb0af5ba6d3198a3e99529c97eda1be0b5a093'
BASE = 'https://android.googlesource.com/platform/system/tools/mkbootimg/+/' + COMMIT
SOURCE_HASHES = {
    'mkbootimg.py': '37d84b3d162e0bc62e36c1f4e1c63c85ea0caa9f29be023eb2f8efe006ad948c',
    'unpack_bootimg.py': 'a9d260978a63bd06a24b6347e7dee8a28ff96639793caea15dff6aa491316308',
    'include/bootimg/bootimg.h': '9adba595989e15b3343969cdc508df666e0c05d3b8a38aa362bd9945302c814e',
    'gki/generate_gki_certificate.py': '1bb1feec68a13da18d581aa2c631798f86f6bc10b55d587b2dd31446a0f8a203',
}
FACTORY_SHA = '3721e6accc2e87d851ee7966d794318078ec74c7cc32343daa10968264d6bb62'
FACTORY_SIZE = 11378664978
ORIGINAL_IMAGES = ['boot.img', 'init_boot.img', 'vendor_boot.img', 'vendor_kernel_boot.img']


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def sources(root, fetch):
    for name, expected in SOURCE_HASHES.items():
        path = root / name
        if fetch and not path.exists():
            with urllib.request.urlopen(BASE + '/' + name + '?format=TEXT', timeout=30) as response:
                body = base64.b64decode(response.read(1024 * 1024), validate=True)
            if hashlib.sha256(body).hexdigest() != expected:
                raise ValueError('unexpected pinned AOSP source hash')
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        if digest(path) != expected:
            raise ValueError('pinned original source changed: ' + name)


def original(root, script, args):
    return subprocess.run(['python3', str(root / script), *map(str, args)],
                          check=True, capture_output=True, text=True).stdout


def prepare(root):
    sources(root, True)
    p = root / 'fixtures'
    p.mkdir(exist_ok=True)
    for name, length, byte in [('kernel', 8197, 65), ('generic', 4097, 66),
                               ('platform', 123, 67), ('dlkm', 9001, 68),
                               ('dtb', 127, 69), ('bootconfig', 9, 70)]:
        (p / name).write_bytes(bytes([byte]) * length)
    original(root, 'mkbootimg.py', ['--header_version', '4', '--kernel', p / 'kernel',
             '--ramdisk', p / 'generic', '--cmdline', 'console=ttyS0', '--output', p / 'boot-v4.img'])
    original(root, 'mkbootimg.py', ['--header_version', '4', '--pagesize', '4096',
             '--vendor_ramdisk', p / 'platform', '--ramdisk_type', 'dlkm', '--ramdisk_name', 'modules',
             '--board_id0', '17', '--vendor_ramdisk_fragment', p / 'dlkm', '--dtb', p / 'dtb',
             '--vendor_bootconfig', p / 'bootconfig', '--vendor_cmdline', 'bootconfig',
             '--vendor_boot', p / 'vendor-v4.img'])
    original(root, 'mkbootimg.py', ['--header_version', '4', '--pagesize', '2048',
             '--vendor_ramdisk', p / 'platform', '--vendor_boot', p / 'vendor-2048-v4.img'])
    (root / 'reference-files.json').write_text(json.dumps({
        'commit': COMMIT, 'source_sha256': SOURCE_HASHES,
        'fixture_sha256': {p.name: digest(p) for p in sorted(p.glob('*.img'))}}, indent=2) + '\n')


def compare_one(root, binary, image, label):
    report = json.loads(subprocess.run([str(binary), str(image)], check=True,
                                      capture_output=True, text=True).stdout)
    output = root / 'comparison-unpacked' / label
    original(root, 'unpack_bootimg.py', ['--boot_img', image, '--out', output])
    extents = {}
    for field in ['kernel', 'ramdisk', 'dtb', 'bootconfig', 'signature']:
        path = output / ('boot_signature' if field == 'signature' else field)
        if path.exists():
            extents[path.name] = report[field]
    for index, fragment in enumerate(report['fragments']):
        extents[f'vendor_ramdisk{index:02}'] = fragment['extent']
    matches = {}
    for name, extent in extents.items():
        original_path = output / name
        if original_path.stat().st_size != extent['size']:
            raise ValueError('original unpacker disagrees on component size')
        h = hashlib.sha256()
        with image.open('rb') as source:
            source.seek(extent['offset'])
            remaining = extent['size']
            while remaining:
                body = source.read(min(remaining, 65536))
                if not body:
                    raise ValueError('truncated compared component')
                h.update(body)
                remaining -= len(body)
        if h.hexdigest() != digest(original_path):
            raise ValueError('original unpacker disagrees on component bytes')
        matches[name] = {'extent': extent, 'sha256': h.hexdigest()}
    return {'image': label, 'image_sha256': digest(image), 'kind': report['kind'],
            'container_page_size': report['page_size'], 'components': matches,
            'original_component_bytes_equal': True, 'avb_verified': False,
            'kernel_kmi_qualified': False, 'phone_accessed': False}


def compare(root, binary, selection):
    sources(root, False)
    results = [compare_one(root, binary, root / 'fixtures' / name, 'authored-' + name)
               for name in ['boot-v4.img', 'vendor-v4.img', 'vendor-2048-v4.img']]
    if selection:
        proof = json.loads(selection.read_text())
        if (proof.get('whole_archive_authenticated') is not True
                or proof.get('sha256') != FACTORY_SHA or proof.get('bytes') != FACTORY_SIZE
                or proof.get('index_reads_rebound') != 38):
            raise ValueError('full selected archive authentication proof required')
        records = {entry['name']: entry for entry in proof['selected']}
        for name in ORIGINAL_IMAGES:
            path = selection.parent / 'selected' / name
            if path.stat().st_size != records[name]['bytes'] or digest(path) != records[name]['sha256']:
                raise ValueError('authenticated selected input changed')
            results.append(compare_one(root, binary, path, name))
    (root / 'comparison-summary.json').write_text(json.dumps({
        'reference_commit': COMMIT, 'results': results}, indent=2) + '\n')
    print(json.dumps({'compared': len(results), 'original_component_bytes_equal': True,
                      'summary': str(root / 'comparison-summary.json')}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'compare'])
    parser.add_argument('private_reference_root', type=pathlib.Path)
    parser.add_argument('--inspector', type=pathlib.Path)
    parser.add_argument('--authenticated-selection', type=pathlib.Path)
    args = parser.parse_args()
    root = args.private_reference_root.resolve()
    if root == pathlib.Path(__file__).resolve().parent.parent or pathlib.Path(__file__).resolve().parent.parent in root.parents:
        parser.error('original sources and payloads must remain outside the repository')
    root.mkdir(parents=True, exist_ok=True)
    if args.action == 'prepare':
        prepare(root)
    else:
        if not args.inspector:
            parser.error('compare requires --inspector')
        compare(root, args.inspector.resolve(), args.authenticated_selection)


if __name__ == '__main__':
    main()
