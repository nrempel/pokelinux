#!/usr/bin/env python3
"""Owned actual-kernel display probe; no phone UI or service authority is selected."""
import base64
import grp
import hashlib
import json
import os
import shlex
import signal
import subprocess
import sys
import time
import traceback
from pathlib import Path

ROOT = Path('/results')
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'source'))
from measure import edid, load_profile, png_pixels, qualify

children = []
logs = []
result = {'passed': False, 'hardware_verified': False, 'phone_accessed': False,
          'phone_ui_selected': False, 'source_or_policy_selected': False}


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def freeze_binary(source, target):
    source, target = Path(source), Path(target)
    target.write_bytes(source.read_bytes())
    target.chmod(source.stat().st_mode & 0o777)
    assert sha(source) == sha(target)
    result.setdefault('frozen_binaries', {})[target.name] = {
        'sha256': sha(target), 'compiled_guest_mode': oct(source.stat().st_mode & 0o777),
        'frozen_guest_mode': oct(target.stat().st_mode & 0o777)}


def command(args, name, **kwargs):
    with (ROOT / name).open('w') as log:
        completed = subprocess.run(args, stdout=log, stderr=subprocess.STDOUT,
                                   timeout=180, **kwargs)
    if completed.returncode:
        raise RuntimeError('command failed: ' + name)


def start(args, name, **kwargs):
    log = (ROOT / name).open('w')
    logs.append(log)
    child = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, **kwargs)
    children.append(child)
    return child


def wait(check, child=None, timeout=150):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        if child is not None and child.poll() is not None:
            raise RuntimeError('owned child exited before readiness')
        value = check()
        if value:
            return value
        time.sleep(.05)
    raise TimeoutError('display fixture readiness')


try:
    selection = json.loads((ROOT / 'selection.json').read_text())
    assert os.getuid() == 0 and os.uname().machine == 'aarch64'
    assert os.uname().release == '7.1.6-1-aarch64-ARCH'
    assert sorted(p.name for p in Path('/sys/class/net').iterdir()) == ['lo']
    for name, expected in selection['sources'].items():
        assert sha(ROOT / 'source' / name) == expected, name
    profile = load_profile(ROOT / 'source/pixel10.json')
    result['profile_sha256'] = sha(ROOT / 'source/pixel10.json')
    result['kernel'] = {'release': os.uname().release, 'machine': os.uname().machine}
    result['hyprland_sha256'] = sha('/usr/bin/Hyprland')
    phase = selection['phase']
    assert phase in ('red', 'green')
    if phase == 'green':
        pins = json.loads((ROOT / 'source/public-api-pins.json').read_text())
        assert result['hyprland_sha256'] == pins['hyprland']['guest_elf_sha256']
        for path, expected in pins['installed_public_sdk'].items():
            assert sha(path) == expected, path
        assert sha(ROOT / 'source/wlr-screencopy-unstable-v1.xml') == pins['hyprland']['public_sources']['protocols/wlr-screencopy-unstable-v1.xml']
        result['matching_public_sdk_verified'] = True
    command(['modprobe', 'virtio_gpu'], 'modprobe.log')
    command(['udevadm', 'settle'], 'udev.log')
    command(['install', '-d', '-o', 'alarm', '-g', 'alarm', '-m', '700', '/run/user/1000'], 'runtime.log')
    # Compile the measurement program against the actual retained guest Qt ABI.
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'Qt6Gui', 'wayland-client', 'libdrm'], text=True))
    compiler = ['g++', '-std=c++17', '-O0', '-Wall', '-Wextra', '-Werror', '-fPIC',
                str(ROOT / 'source/qt-output-probe.cpp'), '-o', '/tmp/pixel10-output-probe', *flags]
    (ROOT / 'probe-build-command.json').write_text(json.dumps(compiler, indent=2) + '\n')
    command(compiler, 'probe-build.log')
    result['qt_probe_elf_sha256'] = sha('/tmp/pixel10-output-probe')
    if phase == 'green':
        frozen_bins = ROOT / 'binaries'; frozen_bins.mkdir(mode=0o700)
        freeze_binary('/tmp/pixel10-output-probe', frozen_bins / 'pixel10-output-probe')
        adapter_flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', 'hyprland', 'Qt6Core'], text=True))
        adapter_libs = shlex.split(subprocess.check_output(['pkg-config', '--libs', 'Qt6Core'], text=True))
        adapter_compiler = ['g++', '-std=c++23', '-fPIC', '-shared', '-O2', '-Wall', '-Wextra', '-Werror',
                            *adapter_flags, str(ROOT / 'source/physical-size-adapter.cpp'),
                            '-o', '/tmp/pixel10-physical-size-adapter.so', *adapter_libs]
        (ROOT / 'adapter-build-command.json').write_text(json.dumps(adapter_compiler, indent=2) + '\n')
        command(adapter_compiler, 'adapter-build.log')
        result['physical_size_adapter_elf_sha256'] = sha('/tmp/pixel10-physical-size-adapter.so')
        freeze_binary('/tmp/pixel10-physical-size-adapter.so', frozen_bins / 'pixel10-physical-size-adapter.so')
        protocol = str(ROOT / 'source/wlr-screencopy-unstable-v1.xml')
        command(['wayland-scanner', 'client-header', protocol, '/tmp/wlr-screencopy-client-protocol.h'], 'capture-header.log')
        command(['wayland-scanner', 'private-code', protocol, '/tmp/wlr-screencopy-protocol.c'], 'capture-protocol.log')
        command(['gcc', '-std=c11', '-fPIC', '-c', '/tmp/wlr-screencopy-protocol.c', '-o', '/tmp/wlr-screencopy-protocol.o'], 'capture-protocol-build.log')
        capture_compiler = ['g++', '-std=c++17', '-O0', '-Wall', '-Wextra', '-Werror', '-fPIC', '-I/tmp',
                            str(ROOT / 'source/raw-output-capture.cpp'), '/tmp/wlr-screencopy-protocol.o',
                            '-o', '/tmp/pixel10-raw-output-capture', *flags]
        (ROOT / 'capture-build-command.json').write_text(json.dumps(capture_compiler, indent=2) + '\n')
        command(capture_compiler, 'capture-build.log')
        result['native_capture_elf_sha256'] = sha('/tmp/pixel10-raw-output-capture')
        freeze_binary('/tmp/pixel10-raw-output-capture', frozen_bins / 'pixel10-raw-output-capture')
    command(['pacman', '-Q', 'hyprland', 'aquamarine', 'qt6-base', 'qt6-wayland'], 'packages.log')
    start(['seatd', '-g', 'video'], 'seatd.log', env=dict(os.environ, SEATD_VTBOUND='0'))
    (ROOT / 'monitor.lua').write_text(
        'hl.monitor({ output = "", mode = "1080x2424@60", position = "auto", scale = 2.625 })\n'
        'hl.permission("/usr/bin/grim", "screencopy", "allow")\n'
        + ('hl.permission("/tmp/pixel10-raw-output-capture", "screencopy", "allow")\n' if phase == 'green' else '') +
        'hl.config({ debug = { disable_scale_checks = true }, animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })\n')
    probe_data = Path('/tmp/pixel10-probe-data'); probe_data.mkdir(mode=0o700)
    os.chown(probe_data, 1000, 1000)
    env = {'HOME': '/home/alarm', 'PATH': '/usr/local/bin:/usr/bin',
           'XDG_RUNTIME_DIR': '/run/user/1000', 'LIBSEAT_BACKEND': 'seatd',
           'AQ_NO_MODIFIERS': '1', 'XDG_SESSION_TYPE': 'wayland', 'QT_QPA_PLATFORM': 'wayland'}
    if phase == 'green':
        env.update(POKELINUX_DISPLAY_PROFILE=str(ROOT / 'source/pixel10.json'),
                   POKELINUX_DISPLAY_PROFILE_RECEIPT=str(probe_data / 'profile-application.json'))
    identity = {'user': 1000, 'group': 1000,
                'extra_groups': tuple(grp.getgrnam(n).gr_gid for n in ('video', 'input'))}
    compositor = start(['/usr/bin/Hyprland', '--config', str(ROOT / 'monitor.lua')],
                       'hyprland.log', env=env, **identity)
    socket = wait(lambda: next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()), None), compositor)
    control = wait(lambda: next(Path('/run/user/1000/hypr').glob('*/.socket.sock'), None), compositor)
    env.update(WAYLAND_DISPLAY=socket.name, HYPRLAND_INSTANCE_SIGNATURE=control.parent.name)
    if phase == 'green':
        command(['hyprctl', 'plugin', 'load', '/tmp/pixel10-physical-size-adapter.so'], 'adapter-load.log', env=env, **identity)
        applied = json.loads((probe_data / 'profile-application.json').read_text())
        assert applied['profile_sha256'] == result['profile_sha256']
        assert applied['output'] == 'Virtual-1' and applied['selected_physical_mm'] == {'width': 65, 'height': 146}
        (ROOT / 'profile-application.json').write_text(json.dumps(applied, indent=2) + '\n')
        result['profile_application'] = applied
    probe = start(['/tmp/pixel10-output-probe', str(probe_data / 'observation.json')],
                  'qt-output-probe.log', env=env, **identity)
    probe.wait(timeout=180)
    assert probe.returncode == 0, 'actual Qt/output probe failed'
    observed = json.loads((probe_data / 'observation.json').read_text())
    for connector in observed['drm_connectors']:
        if connector.get('edid_base64'):
            connector['decoded_edid'] = edid(base64.b64decode(connector['edid_base64'], validate=True))
    monitor = subprocess.check_output(['hyprctl', '-j', 'monitors'], env=env, **identity, text=True, timeout=5)
    observed['compositor_monitors'] = json.loads(monitor)
    (ROOT / 'observation.json').write_text(json.dumps(observed, indent=2) + '\n')
    output = observed['wayland_outputs']
    assert len(output) == 1 and output[0]['current_mode']['width'] == 1080 and output[0]['current_mode']['height'] == 2424
    assert len(observed['qt_screens']) == 1 and observed['qt_window']['exposed']
    validation = qualify(profile, observed)
    result.update(prerequisites_qualified=True, observation=observed, qualification=validation,
                  user_ui_capture_approved=False, raw_png=None)
    if phase == 'red':
        assert not validation['qualified'], 'wrong-density baseline unexpectedly matches complete Pixel10 profile'
        assert any('physical' in failure or 'wl_output' in failure for failure in validation['failures']), 'red did not establish actual physical density mismatch'
        result.update(passed=True, desired_boundary_red=True,
                      groups=['actual1080x2424_default_density_rejected_by_strict_pixel10_profile'])
    else:
        assert validation['qualified'], 'actual physical display profile remains unqualified: ' + repr(validation['failures'])
        result['groups'] = ['actual_public_api_profile_reaches_wayland_and_qt_physical_density']
        # A second instance of the unchanged probe keeps its measurement surface
        # mapped during native capture. Metadata qualifies before any PNG request.
        capture_probe = start(['/tmp/pixel10-output-probe', str(probe_data / 'capture-observation.json')],
                              'capture-probe.log', env=env, **identity)
        def mapped_probe():
            rows = json.loads(subprocess.check_output(['hyprctl', '-j', 'clients'], env=env, **identity, text=True, timeout=5))
            return next((row for row in rows if row.get('pid') == capture_probe.pid and row.get('mapped') and not row.get('hidden')), None)
        mapped = wait(mapped_probe, capture_probe, timeout=30)
        result['native_capture_mapped_probe'] = mapped
        capture = start(['/tmp/pixel10-raw-output-capture', 'Virtual-1', str(probe_data / 'raw-native-output.png'),
                         str(probe_data / 'capture-receipt.json')], 'native-capture.log', env=env, **identity)
        capture.wait(timeout=30)
        assert capture.returncode == 0, 'raw native output capture failed'
        capture_probe.wait(timeout=180)
        assert capture_probe.returncode == 0
        capture_observed = json.loads((probe_data / 'capture-observation.json').read_text())
        assert qualify(profile, capture_observed)['qualified'], 'captured probe display metadata changed'
        receipt = json.loads((probe_data / 'capture-receipt.json').read_text())
        assert receipt['native_buffer_width'] == 1080 and receipt['native_buffer_height'] == 2424
        assert receipt['png_decode_pixel_hash_matches_native_buffer'] and not receipt['resize_or_export_scale_used']
        raw_png = probe_data / 'raw-native-output.png'
        assert png_pixels(raw_png) == profile['native_pixels'] and sha(raw_png) == receipt['png_sha256']
        (ROOT / 'raw-native-output.png').write_bytes(raw_png.read_bytes())
        (ROOT / 'capture-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        result.update(raw_png='raw-native-output.png', raw_png_sha256=sha(ROOT / 'raw-native-output.png'),
                      native_capture=receipt, capture_observation=capture_observed)
        result['groups'].append('raw_native_screencopy_buffer_matches1080x2424_png_without_resize')
        # Explicit normal unload qualifies restoration against the same output.
        command(['hyprctl', 'plugin', 'unload', '/tmp/pixel10-physical-size-adapter.so'], 'adapter-unload.log', env=env, **identity)
        restored = json.loads(subprocess.check_output(['hyprctl', '-j', 'monitors'], env=env, **identity, text=True, timeout=5))
        original = applied['original_physical_mm']
        assert len(restored) == 1 and restored[0]['name'] == 'Virtual-1'
        assert (restored[0]['physicalWidth'], restored[0]['physicalHeight']) == (original['width'], original['height'])
        result['restored_monitor'] = restored[0]
        result['groups'].append('normal_plugin_unload_restores_exact_owned_output_physical_extent')
        result.update(passed=True, physical_profile_qualified=True, raw_native_capture_qualified=True,
                      user_ui_capture_approved=False, phone_ui_selected=False)
except BaseException as error:
    result['error'] = str(error)
    result['traceback'] = traceback.format_exc()
finally:
    cleanup = []
    for child in reversed(children):
        if child.poll() is None:
            child.terminate()
            try: child.wait(timeout=5)
            except subprocess.TimeoutExpired: child.kill(); child.wait(timeout=5)
        cleanup.append({'pid': child.pid, 'returncode': child.returncode, 'direct_parent_wait': True})
    for log in logs: log.close()
    result['owned_child_waits'] = cleanup
    (ROOT / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    subprocess.run(['sync'], timeout=10)
    subprocess.run(['poweroff'], timeout=10)
