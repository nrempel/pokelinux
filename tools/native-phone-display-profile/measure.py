#!/usr/bin/env python3
"""Strict observed display qualification; never fixes DPI or resizes a capture."""
import argparse
import hashlib
import json
import math
import struct
import zlib
from pathlib import Path

MEASUREMENT_CONTRACT_VERSION = 2


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_profile(path):
    profile = json.loads(Path(path).read_text())
    if profile['schema_version'] != 1 or profile['id'] != 'pixel10':
        raise ValueError('unsupported display profile')
    pixels, physical, ui = profile['native_pixels'], profile['physical'], profile['native_ui']
    if (pixels['width'], pixels['height']) != (1080, 2424):
        raise ValueError('Pixel 10 portrait pixels must be1080x2424')
    if (physical['nominal_mm']['width'], physical['nominal_mm']['height']) != (65, 146):
        raise ValueError('reviewed nominal physical profile must be65x146mm')
    if physical['published_ppi'] != 422 or physical['ppi_tolerance'] != 1.0:
        raise ValueError('physical density guard changed')
    if ui['requested_device_pixel_ratio'] != 2.625 or ui['dpr_tolerance'] != 0.001:
        raise ValueError('selected native surface scale guard changed')
    return profile


def edid(data):
    if len(data) < 128 or len(data) % 128 or data[:8] != bytes.fromhex('00ffffffffffff00'):
        raise ValueError('missing or malformed actual EDID')
    if len(data) != 128 * (1 + data[126]) or any(sum(data[i:i + 128]) % 256 for i in range(0, len(data), 128)):
        raise ValueError('EDID extent/checksum mismatch')
    dtd = data[54:72]
    mode = None
    if int.from_bytes(dtd[:2], 'little'):
        mode = {
            'width': dtd[2] | ((dtd[4] & 0xf0) << 4),
            'height': dtd[5] | ((dtd[7] & 0xf0) << 4),
            'physical_width_mm': dtd[12] | ((dtd[14] & 0xf0) << 4),
            'physical_height_mm': dtd[13] | ((dtd[14] & 0x0f) << 8)
        }
    return {'base_physical_mm': {'width': data[21] * 10, 'height': data[22] * 10},
            'preferred_detailed_timing': mode, 'blocks': len(data) // 128,
            'sha256': hashlib.sha256(data).hexdigest()}


def png_pixels(path):
    data = Path(path).read_bytes()
    if len(data) < 33 or data[:8] != b'\x89PNG\r\n\x1a\n' or data[8:16] != b'\0\0\0\rIHDR':
        raise ValueError('capture lacks a PNG IHDR')
    if zlib.crc32(data[12:29]) & 0xffffffff != struct.unpack('>I', data[29:33])[0]:
        raise ValueError('capture IHDR checksum mismatch')
    width, height = struct.unpack('>II', data[16:24])
    return {'width': width, 'height': height}


def finite_positive(value):
    return isinstance(value, (float, int)) and not isinstance(value, bool) and math.isfinite(value) and value > 0


def qualify(profile, observation, screenshot=None):
    """All required values must come from actual probe outputs, not the profile."""
    failures = []
    target = profile['native_pixels']
    nominal = profile['physical']['nominal_mm']
    ppi = profile['physical']['published_ppi']
    tolerance = profile['physical']['ppi_tolerance']
    wl = observation.get('wayland_outputs', [])
    screens = observation.get('qt_screens', [])
    if len(wl) != 1 or len(screens) != 1:
        return {'qualified': False, 'failures': ['require exactly one actual Wayland output and Qt screen']}
    output, screen = wl[0], screens[0]
    mode = output.get('current_mode', {})
    if (mode.get('width'), mode.get('height')) != (target['width'], target['height']):
        failures.append('actual Wayland current mode differs from1080x2424')
    if (output.get('physical_width_mm'), output.get('physical_height_mm')) != (nominal['width'], nominal['height']):
        failures.append('actual wl_output geometry does not report65x146mm')
    if screen.get('name') != output.get('name'):
        failures.append('Qt screen and measured Wayland output names differ')
    size = screen.get('physical_size_mm', {})
    if (size.get('width'), size.get('height')) != (nominal['width'], nominal['height']):
        failures.append('actual Qt physicalSize differs from nominal65x146mm')
    window = observation.get('qt_window', {})
    if window.get('screen') != screen.get('name') or not window.get('exposed'):
        failures.append('probe surface is not exposed on the measured Qt screen')
    dpr = window.get('device_pixel_ratio')
    if not finite_positive(dpr) or abs(dpr - profile['native_ui']['requested_device_pixel_ratio']) > profile['native_ui']['dpr_tolerance']:
        failures.append('actual Qt surface DPR differs from selected2.625')
    densities = {}
    for axis, dimension in (('x', 'width'), ('y', 'height')):
        physical_mm = output.get('physical_' + dimension + '_mm')
        native = mode.get(dimension)
        native_ppi = native * 25.4 / physical_mm if finite_positive(native) and finite_positive(physical_mm) else None
        qt_dpi = screen.get('physical_dpi_' + axis)
        screen_dpr = screen.get('device_pixel_ratio')
        qt_screen_product = qt_dpi * screen_dpr if finite_positive(qt_dpi) and finite_positive(screen_dpr) else None
        qt_ppi = qt_dpi * dpr if finite_positive(qt_dpi) and finite_positive(dpr) else None
        densities[axis] = {'native_pixels_over_wl_mm_ppi': native_ppi,
                           'qt_physical_dpi_times_screen_dpr_diagnostic': qt_screen_product,
                           'qt_physical_dpi_times_target_window_dpr_ppi': qt_ppi}
        if native_ppi is None or abs(native_ppi - ppi) > tolerance:
            failures.append('actual ' + axis + ' native physical PPI differs from422')
        if qt_ppi is None or abs(qt_ppi - ppi) > tolerance:
            failures.append('actual Qt ' + axis + ' physical DPI times target window DPR differs from422')
        logical = screen.get('geometry', {}).get(dimension)
        if not finite_positive(native) or not finite_positive(dpr) or not finite_positive(logical) or abs(logical - native / dpr) >= 1:
            failures.append('actual Qt ' + axis + ' logical extent differs from native pixels/window DPR by at least one pixel')
    connectors = [c for c in observation.get('drm_connectors', []) if c.get('connected') and c.get('name') == output.get('name')]
    if len(connectors) != 1:
        failures.append('require one actual connected DRM connector matching wl_output')
    else:
        active = connectors[0].get('active_mode', {})
        if not active.get('valid') or (active.get('width'), active.get('height')) != (target['width'], target['height']):
            failures.append('actual DRM active mode differs from1080x2424')
    if screenshot is not None and screenshot != target:
        failures.append('raw PNG pixels differ from1080x2424')
    return {'measurement_contract_version': MEASUREMENT_CONTRACT_VERSION,
            'qualified': not failures, 'failures': failures, 'densities': densities,
            'pixel_dimensions': mode, 'native_surface_dpr': dpr,
            'qt_screen_dpr_diagnostic': screen.get('device_pixel_ratio'),
            'physical_qt_coordinate_basis': 'actual target QWindow DPR with less than one logical pixel rounding per axis; integer QScreen DPR diagnostic retained',
            'logical_geometry': screen.get('geometry'),
            'physical_size_scope': 'actual Wayland/Qt metadata of selected simulation profile; DRM connector/base EDID precision separately recorded'}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--profile', type=Path, required=True)
    p.add_argument('--observation', type=Path, required=True)
    p.add_argument('--raw-png', type=Path)
    p.add_argument('--sidecar', type=Path, required=True)
    a = p.parse_args()
    profile = load_profile(a.profile)
    observation = json.loads(a.observation.read_text())
    image_pixels = png_pixels(a.raw_png) if a.raw_png else None
    result = qualify(profile, observation, image_pixels)
    sidecar = {'measurement_contract_version': MEASUREMENT_CONTRACT_VERSION,
               'consumer_sha256': sha(__file__),
               'profile_sha256': sha(a.profile), 'observation_sha256': sha(a.observation),
               'raw_png_sha256': sha(a.raw_png) if a.raw_png else None,
               'raw_png_pixels': image_pixels, 'observation': observation,
               'qualification': result, 'hardware_verified': False,
               'resize_or_font_dpi_override_used': False,
               'raw_capture_dimension_qualified': image_pixels == profile['native_pixels'] if image_pixels else False}
    with a.sidecar.open('x') as out:
        json.dump(sidecar, out, indent=2)
        out.write('\n')
    print(json.dumps(result, indent=2))
    return 0 if result['qualified'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
