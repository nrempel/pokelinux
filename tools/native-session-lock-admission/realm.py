#!/usr/bin/env python3
"""Unexecuted setup helpers for the original Omarchy/Pixel10 ARM fixture."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import tarfile


OMARCHY = 'c668141e9c42b13c80c9ca4ea108e11708c5e8a5'
ARCHIVE = '8cc6b1d9d903c606600395e3b3d80ad9f15bd3ab78f0f19b0f49041b1df82013'
QUICKSHELL = 'c52b32a7c87df436cd09da98cf47e2341797d2159d30e4208456bba821fafb78'
LOCK_SERVICE = '3f0a265a09f7957e8f5d74666595d3136ec64899c414426190460f9e2098e701'


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def stage_omarchy(source, command, start, env, result):
    """Actual immutable originals; file presence is not PAM authentication."""
    source = Path(source)
    assert sha('/usr/bin/quickshell') == QUICKSHELL
    archive = source / 'omarchy-public.tar.gz'
    assert sha(archive) == ARCHIVE
    extracted = Path('/tmp/omarchy-lock-original')
    extracted.mkdir(mode=0o755)
    with tarfile.open(archive) as bundle:
        bundle.extractall(extracted, filter='data')
    root = extracted / ('omarchy-' + OMARCHY)
    manifest = json.loads((source / 'omarchy-public-manifest.json').read_text())
    assert manifest['commit'] == OMARCHY and manifest['archive_sha256'] == ARCHIVE
    for name, expected in manifest['files'].items():
        assert sha(root / name) == expected, name
    assert sha(root / 'shell/plugins/lock/Service.qml') == LOCK_SERVICE
    # This is the already qualified Qt6.12 Color namespace appearance patch.
    # Its implementation and original input manifest must be separately frozen.
    from omarchy_theme_patch import apply
    patch = apply(root, json.loads((source / 'omarchy-source-manifest.json').read_text()))
    installed = Path('/usr/share/omarchy')
    assert not installed.exists()
    shutil.copytree(root, installed)
    result['original_omarchy'] = {'commit': OMARCHY, 'archive_sha256': ARCHIVE,
        'Quickshell_ELF_sha256': QUICKSHELL, 'appearance_namespace_patch': patch,
        'timer_or_authentication_logic_changed': False,
        'installed_resource_authority_qualified': False}
    assert not Path('/usr/bin/fprintd-list').exists(), 'fingerprint helper is outside this selected realm'
    setup_env = {'PATH': '/usr/share/omarchy/bin:/usr/local/bin:/usr/bin:/bin',
                 'OMARCHY_INSTALL_USER': 'alarm', 'USER': 'alarm',
                 'OMARCHY_PATH': str(installed), 'HOME': '/root'}
    command([str(installed / 'bin/omarchy-apply-lock')], 'original-lock-PAM-definition-setup.log', env=setup_env)
    pam = Path('/etc/pam.d/omarchy-lock-password')
    text = (root / 'bin/omarchy-apply-lock').read_text()
    delimiter = "as_root tee /etc/pam.d/omarchy-lock-password >/dev/null <<'EOF'\n"
    assert text.count(delimiter) == 1
    expected = text.split(delimiter, 1)[1].split('\nEOF\n', 1)[0] + '\n'
    assert pam.read_text() == expected and pam.stat().st_uid == 0
    result['original_PAM_definition'] = {'sha256': sha(pam),
        'exact_original_definition': True, 'PAM_authentication_executed': False,
        'trusted_unlock_qualified': False}
    config = Path('/home/alarm/.config/omarchy')
    config.mkdir(parents=True, exist_ok=True)
    target = config / 'shell.json'
    shutil.copyfile(installed / 'config/omarchy/shell.json', target)
    os.chown(config, 1000, 1000)
    os.chown(target, 1000, 1000)
    env.update(OMARCHY_PATH=str(installed), PATH=str(installed / 'bin') + ':/usr/local/bin:/usr/bin')
    shell = start(['/usr/bin/quickshell', '-n', '-p', str(installed / 'shell')],
                  'original-omarchy-shell.log', env=env, user=True)
    return installed, shell


def apply_display(source, binaries, command, env, result, suffix=''):
    """Finish actual measurement before any Source lease or app focus starts."""
    from measure import load_profile, qualify
    source, binaries = Path(source), Path(binaries)
    receipt_dir = Path(env['POKELINUX_DISPLAY_PROFILE_RECEIPT']).parent
    receipt_dir.mkdir(mode=0o700)
    os.chown(receipt_dir, 1000, 1000)
    # These environment values must also be in the compositor's original exec
    # environment; the guest adapter supplies them before starting Hyprland.
    assert env['POKELINUX_DISPLAY_PROFILE'] == str(source / 'pixel10.json')
    assert env['POKELINUX_DISPLAY_PROFILE_RECEIPT'] == str(receipt_dir / 'applied.json')
    command(['hyprctl', 'plugin', 'load', str(binaries / 'pixel10-physical-size-adapter.so')],
            'pixel10-profile-load' + suffix + '.log', env=env, user=True)
    observed = receipt_dir / 'metadata.json'
    command([str(binaries / 'pixel10-output-probe'), str(observed)],
            'pixel10-actual-metadata' + suffix + '.log', env=env, user=True)
    profile = load_profile(source / 'pixel10.json')
    observation = json.loads(observed.read_text())
    qualification = qualify(profile, observation)
    assert qualification['qualified'], qualification['failures']
    applied = json.loads((receipt_dir / 'applied.json').read_text())
    assert applied['profile_sha256'] == sha(source / 'pixel10.json')
    result['Pixel10_profile'] = {'profile_sha256': sha(source / 'pixel10.json'),
        'observation': observation, 'qualification': qualification,
        'public_profile_application': applied, 'hardware_verified': False}
    return qualification


def capture_native(name, output, source, binaries, command, env, qualification):
    """No generic export scaling; exact native announced buffer and pixel proof."""
    from measure import load_profile, png_pixels
    assert qualification['qualified']
    output, source, binaries = Path(output), Path(source), Path(binaries)
    capture_dir = Path('/run/user/1000/native-lock-captures')
    capture_dir.mkdir(mode=0o700, exist_ok=True)
    os.chown(capture_dir, 1000, 1000)
    png, receipt = capture_dir / (name + '.png'), capture_dir / (name + '.json')
    command([str(binaries / 'pixel10-raw-output-capture'), 'Virtual-1', str(png), str(receipt)],
            name + '-native-capture.log', env=env, user=True)
    proof = json.loads(receipt.read_text())
    assert png_pixels(png) == load_profile(source / 'pixel10.json')['native_pixels']
    assert proof['native_buffer_width'] == 1080 and proof['native_buffer_height'] == 2424
    assert proof['png_decode_pixel_hash_matches_native_buffer']
    assert not proof['resize_or_export_scale_used'] and sha(png) == proof['png_sha256']
    shutil.copyfile(png, output / png.name)
    shutil.copyfile(receipt, output / receipt.name)
    return {'file': png.name, 'sha256': sha(png), 'actual_native_capture': proof,
            'profile_qualification': qualification, 'phone_hardware_verified': False}
