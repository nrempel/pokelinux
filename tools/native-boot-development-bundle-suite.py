#!/usr/bin/env python3
"""Fixed offline development-bundle behavior; no native payload, VM or device execution."""
import argparse, hashlib, importlib.util, json, pathlib, stat, struct, subprocess, sys
CAPACITY = 8 * 1024 * 1024
AVB_SHA = 'e5a664a38db623da00f080219bc0ee60a640a9dc4a872803616fae4938ac749b'
SALT = hashlib.sha256(b'Pokelinux diagnostic init_boot v1').digest()
UNCHANGED = ['boot.img', 'vendor_boot.img', 'vendor_kernel_boot.img', 'dtbo.img']
QUALIFICATIONS = ['physical_device_accessed', 'bootloader_accepted', 'deployment_admitted', 'phone_current_state_backup_verified', 'stock_trust_chain_accepted']

def sha(body):
    return hashlib.sha256(body).hexdigest()

def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def load_module(path):
    spec = importlib.util.spec_from_file_location('independent_package_witness', path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m

def command(argv, label, evidence):
    p = subprocess.run([str(x) for x in argv], capture_output=True)
    assert len(p.stdout) <= 1024 * 1024 and len(p.stderr) <= 1024 * 1024, 'bounded offline tool output'
    (evidence / (label + '.stdout')).write_bytes(p.stdout)
    (evidence / (label + '.stderr')).write_bytes(p.stderr)
    return p

def metadata(body):
    assert len(body) == CAPACITY
    magic, major, minor, original, offset, size, reserved = struct.unpack('>4sIIQQQ28s', body[-64:])
    assert magic == b'AVBf' and major == 1 and (minor == 0) and (not any(reserved)), 'canonical fresh footer'
    assert 0 < original <= offset and offset % 4096 == 0 and (256 <= size <= 64 * 1024) and (offset + size <= len(body) - 64), 'bounded AVB span'
    blob = body[offset:offset + size]
    assert blob[:4] == b'AVB0'
    auth, aux = struct.unpack_from('>QQ', blob, 12)
    algorithm = struct.unpack_from('>I', blob, 28)[0]
    hash_off, hash_size, sig_off, sig_size, key_off, key_size, km_off, km_size, desc_off, desc_size = struct.unpack_from('>10Q', blob, 32)
    rollback = struct.unpack_from('>Q', blob, 112)[0]
    flags, location = struct.unpack_from('>II', blob, 120)
    assert size == 256 + auth + aux and (not any(blob[192:256])), 'exact metadata size/reserved padding'
    return {'raw_footer': body[-64:], 'original': original, 'offset': offset, 'size': size, 'blob': blob, 'auth': auth, 'aux': aux, 'algorithm': algorithm, 'hash_off': hash_off, 'hash_size': hash_size, 'sig_off': sig_off, 'sig_size': sig_size, 'key_off': key_off, 'key_size': key_size, 'km_off': km_off, 'km_size': km_size, 'desc_off': desc_off, 'desc_size': desc_size, 'rollback': rollback, 'flags': flags, 'location': location}

def check_none_hash(image, compact, stock):
    body = image.read_bytes()
    m = metadata(body)
    assert m['algorithm'] == 0 and m['auth'] == m['hash_size'] == m['sig_size'] == m['key_size'] == m['km_size'] == 0, 'NONE has no signature/auth/key'
    assert m['hash_off'] == m['sig_off'] == 0
    assert 0 <= m['key_off'] <= m['aux'] and 0 <= m['km_off'] <= m['aux'], 'zero-sized key extents stay within auxiliary block'
    assert m['flags'] == 0 and m['location'] == 0 and (m['rollback'] == 1791158400), 'no disabled-verification flags; fixed rollback field and location'
    assert m['original'] == len(compact) and body[:len(compact)] == compact, 'entire compact prefix preserved'
    assert m['offset'] == (len(compact) + 4095) // 4096 * 4096 and (not any(body[len(compact):m['offset']]))
    assert not any(body[m['offset'] + m['size']:-64]), 'all AVB partition gap padding zero'
    aux = m['blob'][256:]
    assert m['auth'] == 0 and m['aux'] % 64 == 0 and (m['desc_off'] + m['desc_size'] <= len(aux))
    assert not any(aux[:m['desc_off']]) and (not any(aux[m['desc_off'] + m['desc_size']:])), 'all unused auxiliary padding zero'
    d = aux[m['desc_off']:m['desc_off'] + m['desc_size']]
    assert len(d) >= 132
    tag, following, covered = struct.unpack_from('>QQQ', d)
    algorithm = d[24:56]
    name_n, salt_n, digest_n, flags = struct.unpack_from('>4I', d, 56)
    assert tag == 2 and following + 16 == len(d) and (len(d) % 8 == 0), 'exactly one hash descriptor, no second descriptor'
    assert algorithm == b'sha256' + b'\x00' * 26 and (not any(d[72:132])) and (flags == 0)
    assert name_n == 9 and salt_n == digest_n == 32 and (covered == len(compact))
    start = 132
    name = d[start:start + name_n]
    start += name_n
    salt = d[start:start + salt_n]
    start += salt_n
    actual = d[start:start + digest_n]
    start += digest_n
    assert name == b'init_boot' and salt == SALT and (actual == hashlib.sha256(SALT + compact).digest())
    assert not any(d[start:]), 'descriptor padding zero'
    old = metadata(stock)
    assert old['key_size'] > 0 and m['key_size'] == 0, 'empty candidate key cannot equal retained stock child key'
    assert m['raw_footer'] != old['raw_footer'] and m['blob'] != old['blob'], 'no stale stock footer/VBMeta copied'
    return {'algorithm': 'NONE', 'signed': False, 'flags': 0, 'rollback_index': m['rollback'], 'header_rollback_location_observed': m['location'], 'image_size': covered, 'salt_hex': salt.hex(), 'digest_hex': actual.hex(), 'stock_expected_child_key_match': False}

def candidate(args, bundle, label, suite, pins):
    image = bundle / 'candidate/init_boot.img'
    compact_dir = bundle / 'compact'
    compact = compact_dir / 'init_boot.img'
    prior = suite.check_candidate(args, compact_dir)
    stock_root = args.authenticated_selection.parent / 'selected'
    stock = (stock_root / 'init_boot.img').read_bytes()
    header, compressed = suite.boot_ramdisk(image)
    ch, cp = suite.boot_ramdisk(compact)
    assert header == ch and compressed == cp
    original = command([sys.executable, args.private_reference_root / 'unpack_bootimg.py', '--boot_img', image, '--out', bundle / 'independent-full-unpack'], label + '-original-unpack', args.private_evidence_dir)
    assert original.returncode == 0 and (bundle / 'independent-full-unpack/ramdisk').read_bytes() == compressed
    inspected = command([args.inspector, image], label + '-rust-inspect', args.private_evidence_dir)
    assert inspected.returncode == 0
    report = json.loads(inspected.stdout)
    end = 4096 + (len(compressed) + 4095) // 4096 * 4096
    assert report['kind'] == 'boot-v4' and report['header_version'] == 4 and (report['header_size'] == 1584) and (report['page_size'] == 4096) and (report['file_size'] == CAPACITY)
    assert report['kernel']['size'] == report['signature']['size'] == 0 and report['ramdisk'] == {'offset': 4096, 'size': len(compressed)}
    assert report['trailing'] == {'offset': end, 'size': CAPACITY - end}, 'Rust reports complete partition tail; it does not authenticate AVB'
    frames = suite.decode_stream(compressed)
    assert len(frames) == 1 and frames[0]['body'] == (compact_dir / 'ramdisk.cpio').read_bytes()
    entries = suite.archive(frames[0]['body'], strict=True)
    suite.check_generic(entries, pins['native_payload'])
    assert entries['init']['data'] == args.native_init.read_bytes() and entries['probe-child']['data'] == args.native_child.read_bytes()
    assert not any(image.read_bytes()[4096 + len(compressed):end]), 'boot page padding zero'
    avb = check_none_hash(image, compact.read_bytes(), stock)
    for action in ['info_image', 'verify_image']:
        result = command([sys.executable, args.private_avbtool, action, '--image', image], label + '-original-avb-' + action, args.private_evidence_dir)
        assert result.returncode == 0, 'original AVB ' + action + ' must accept local NONE structure/hash'
    manifest = json.loads((bundle / 'manifest.json').read_text())
    assert manifest['schema'] == 1 and manifest['bundle_format_verified'] is True
    assert all((manifest['qualifications'][k] is False for k in QUALIFICATIONS))
    assert manifest['avb']['algorithm'] == 'NONE' and manifest['avb']['signed'] is False and (manifest['avb']['flags'] == 0)
    assert manifest['image']['bytes'] == CAPACITY and manifest['image']['sha256'] == digest(image)
    selected = {x['name']: x for x in pins['selected']}
    restored = bundle / 'restore/init_boot.img'
    s = restored.lstat()
    source = stock_root / 'init_boot.img'
    assert stat.S_ISREG(s.st_mode) and restored.read_bytes() == stock and (s.st_size == selected['init_boot.img']['bytes']) and (digest(restored) == selected['init_boot.img']['sha256'])
    assert (s.st_dev, s.st_ino) != (source.stat().st_dev, source.stat().st_ino), 'restore is a separate asset, not hardlink/symlink'
    assert manifest['restoration']['path'] == 'restore/init_boot.img' and manifest['restoration']['bytes'] == s.st_size and (manifest['restoration']['sha256'] == digest(restored))
    rows = {x['path']: x for x in manifest['unchanged_inputs']}
    assert len(rows) == len(manifest['unchanged_inputs'])
    copies = {}
    for name in UNCHANGED:
        p = bundle / 'unchanged' / name
        st = p.lstat()
        expected = selected[name]
        relative = 'unchanged/' + name
        assert stat.S_ISREG(st.st_mode) and st.st_size == expected['bytes'] and (digest(p) == expected['sha256'] == digest(stock_root / name))
        assert rows[relative]['bytes'] == st.st_size and rows[relative]['sha256'] == expected['sha256']
        copies[relative] = {'bytes': st.st_size, 'sha256': expected['sha256']}
    return {'candidate_sha256': digest(image), 'candidate_bytes': CAPACITY, 'manifest_sha256': digest(bundle / 'manifest.json'), 'restore_sha256': digest(restored), 'unchanged': copies, 'avb': avb, 'compact_image_sha256': prior['image_sha256'], 'independent_full_original_unpack_and_Rust_inspector': True, 'full_production_deployment_admitted': False}

def main():
    p = argparse.ArgumentParser(description=__doc__)
    for n in ['repo', 'authenticated-selection', 'private-reference-root', 'inspector', 'native-init', 'native-child', 'private-avbtool', 'private-evidence-dir']:
        p.add_argument('--' + n, type=pathlib.Path, required=True)
    args = p.parse_args()
    args.repo = args.repo.resolve()
    args.private_evidence_dir = args.private_evidence_dir.resolve()
    assert not args.private_evidence_dir.exists()
    assert not any(((x / '.git').exists() for x in [args.private_evidence_dir, *args.private_evidence_dir.parents]))
    args.private_evidence_dir.mkdir(mode=448)
    pins = json.loads((args.repo / 'native-boot-package/input-pins.json').read_text())
    assert digest(args.private_avbtool) == AVB_SHA
    suite = load_module(args.repo / 'tools/native-boot-probe-package-suite.py')
    proof = json.loads(args.authenticated_selection.read_text())
    root = args.authenticated_selection.parent / 'selected'
    before = {x['name']: digest(root / x['name']) for x in proof['selected']}
    results = []
    for label in ['first', 'reproduced']:
        out = args.private_evidence_dir / label
        argv = [sys.executable, args.repo / 'tools/native-boot-development-bundle.py']
        for n in ['authenticated_selection', 'private_reference_root', 'inspector', 'native_init', 'native_child', 'private_avbtool']:
            argv.extend(['--' + n.replace('_', '-'), getattr(args, n)])
        argv.extend(['--private-output-dir', out])
        result = command(argv, label + '-producer', args.private_evidence_dir)
        assert result.returncode == 0, 'real producer setup failed: ' + result.stderr.decode(errors='replace')
        image = out / 'candidate/init_boot.img'
        assert image.is_file() and image.stat().st_size == CAPACITY, 'MISSING_COMPLETE_INIT_BOOT_8MIB: producer succeeded but candidate is not exactly8388608bytes'
        results.append(candidate(args, out, label, suite, pins))
    assert results[0] == results[1], 'candidate and manifest reproducible across two real builds'
    good = args.private_evidence_dir / 'first/candidate/init_boot.img'
    body = good.read_bytes()
    for label, data in [('payload-flip', body[:4128] + bytes([body[4128] ^ 1]) + body[4129:]), ('truncated-footer', body[:-64])]:
        d = args.private_evidence_dir / label
        d.mkdir()
        image = d / 'init_boot.img'
        image.write_bytes(data)
        rejected = command([sys.executable, args.private_avbtool, 'verify_image', '--image', image], label + '-original-avb-refusal', args.private_evidence_dir)
        assert rejected.returncode == 1, 'original AVB must refuse isolated mutated init_boot.img'
    after = {x['name']: digest(root / x['name']) for x in proof['selected']}
    assert before == after and len(after) == 14
    report = {'schema': 1, 'bundle_format_qualified': True, 'checks': results[0], 'two_real_builds_identical': True, 'original_avb_two_tamper_refusals': ['payload-flip/init_boot.img', 'truncated-footer/init_boot.img'], 'all14_selected_originals_unchanged': True, 'local_NONE_verification_is_signature_or_stock_acceptance': False, 'phone_current_build_slot_rollback_state': 'unknown; no current-state backup or deployment admission', 'physical_device_accessed': False, 'existing_compact_eight_refusal_groups': 'separately retained, unchanged; not rerun by this suite', 'production_validator': 'separate full production validator remains refused; this suite does not alter or admit its profile'}
    (args.private_evidence_dir / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
if __name__ == '__main__':
    main()
