"""Only completed public Pixel profile/capture helpers; no theme/auth staging import."""
import hashlib,json,math,os,shutil
from pathlib import Path

def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()

def prepare_display_receipts():
    """Create the selected private receipt directory before owned compositor exec."""
    receipt_dir = Path('/run/user/1000/pixel10-profile')
    receipt_dir.mkdir(mode=0o700)
    os.chown(receipt_dir, 1000, 1000)

def discover_display(output, source, binaries, command, env, result, compositor_pid):
    """Phase one: preserve actual receipts and discover outputs without a window."""
    from measure import load_profile
    output, source, binaries = Path(output), Path(source), Path(binaries)
    receipt_dir = Path('/run/user/1000/pixel10-profile')
    assert env['POKELINUX_DISPLAY_PROFILE'] == str(source / 'pixel10.json')
    assert env['POKELINUX_DISPLAY_PROFILE_RECEIPT'] == str(receipt_dir / 'applied.json')
    record = {'profile_sha256': sha(source / 'pixel10.json'), 'qualification': None,
              'hardware_verified': False, 'discovery_has_no_measurement_surface': True}
    result['Pixel10_profile'] = record
    def receipt(name):
        with (receipt_dir / name).open('rb') as stream: raw = stream.read(65537)
        assert 0 < len(raw) <= 65536, 'bounded actual profile receipt'
        return raw, json.loads(raw)
    raw, ordering = receipt('ordered-load.json')
    assert ordering['schema'] == 1 and ordering['private_fixture_only'] and ordering['pid'] == compositor_pid and ordering['uid'] == 1000
    assert ordering['attempted_before_load'] and ordering['synchronous_original_config_loader']
    assert ordering['ready_callback_boottime_ns'] <= ordering['physical_load_completed_boottime_ns']
    assert ordering['fixed_plugin'] == str(binaries / 'pixel10-physical-size-adapter.so')
    assert ordering['actual_applied_receipt'] == str(receipt_dir / 'applied.json') and not ordering['Source_FD_or_facts_accessed']
    with (output / 'pixel10-ordered-load.json').open('xb') as saved: saved.write(raw)
    record.update(private_ready_ordering=ordering,
                  private_ready_ordering_receipt_sha256=hashlib.sha256(raw).hexdigest(),
                  private_ready_ordering_file='pixel10-ordered-load.json',
                  heavy_profile_initialization_before_first_loop_dispatch_source_bound=True)
    raw, applied = receipt('applied.json')
    assert applied['profile_sha256'] == record['profile_sha256']
    assert len(raw) == ordering['actual_applied_receipt_bytes']
    with (output / 'pixel10-applied.json').open('xb') as saved: saved.write(raw)
    record.update(public_profile_application=applied,
                  public_profile_application_receipt_sha256=hashlib.sha256(raw).hexdigest(),
                  public_profile_application_file='pixel10-applied.json')
    observed = receipt_dir / 'discovery.json'
    command([str(binaries / 'pixel10-output-discovery'), str(observed)],
            'pixel10-actual-discovery.log', env=env, user=1000, seconds=30)
    with observed.open('rb') as stream: raw = stream.read(131073)
    assert 0 < len(raw) <= 131072, 'bounded nonpresenting output discovery'
    observation = json.loads(raw)
    assert 'qt_window' not in observation and observation['platform'] == 'wayland'
    assert len(observation['qt_screens']) == len(observation['wayland_outputs']) == 1
    load_profile(source / 'pixel10.json')
    with (output / 'pixel10-discovery.json').open('xb') as saved: saved.write(raw)
    record.update(discovery=observation, discovery_sha256=hashlib.sha256(raw).hexdigest(),
                  discovery_file='pixel10-discovery.json')
    return record

def _rectangle(values):
    assert isinstance(values, list) and len(values) == 4 and all(type(v) is int for v in values)
    x, y, width, height = values
    assert width > 0 and height > 0
    return {'x': x, 'y': y, 'width': width, 'height': height}

def qualify_app(source, discovery, metadata, control, result, absolute_cutoff):
    """Phase two: actual normal Sounds frame metadata, never a probe window."""
    from measure import load_profile, qualify
    assert result['Pixel10_profile'] is discovery and discovery['qualification'] is None
    assert metadata['kind'] == 'sounds_window_metadata' and metadata['schema'] == 1 and metadata['title'] == 'Sounds'
    screen, window = metadata['screen'], metadata['window']
    assert window['visible'] is True and window['exposed'] is True
    assert len(screen['physical_mm']) == len(screen['physical_dpi']) == len(screen['logical_dpi']) == 2
    actual_screen = {'name': screen['name'], 'geometry': _rectangle(screen['geometry']),
        'available_geometry': _rectangle(screen['available_geometry']), 'device_pixel_ratio': screen['dpr'],
        'physical_size_mm': dict(zip(('width', 'height'), screen['physical_mm'])),
        'physical_dpi_x': screen['physical_dpi'][0], 'physical_dpi_y': screen['physical_dpi'][1],
        'logical_dpi_x': screen['logical_dpi'][0], 'logical_dpi_y': screen['logical_dpi'][1]}
    initial = discovery['discovery']
    assert actual_screen == initial['qt_screens'][0], 'initial discovery/app-time screen incompatibility or output change'
    geometry = _rectangle(window['geometry'])
    bounds = actual_screen['geometry']
    assert bounds['x'] <= geometry['x'] and bounds['y'] <= geometry['y']
    assert geometry['x'] + geometry['width'] <= bounds['x'] + bounds['width']
    assert geometry['y'] + geometry['height'] <= bounds['y'] + bounds['height']
    profile = load_profile(Path(source) / 'pixel10.json')
    assert type(window['dpr']) in (int, float) and math.isfinite(window['dpr'])
    assert abs(window['dpr'] - profile['native_ui']['requested_device_pixel_ratio']) <= profile['native_ui']['dpr_tolerance']
    current = json.loads(control('j/monitors', absolute_cutoff=absolute_cutoff))
    assert len(current) == 1, 'current compositor output count changed'
    monitor, output = current[0], initial['wayland_outputs'][0]
    expected = {'name': output['name'], 'width': output['current_mode']['width'],
        'height': output['current_mode']['height'], 'physicalWidth': output['physical_width_mm'],
        'physicalHeight': output['physical_height_mm'], 'x': output['position_x'],
        'y': output['position_y'], 'transform': output['transform']}
    assert {key: monitor[key] for key in expected} == expected, 'current compositor output differs from discovery'
    assert monitor['dpmsStatus'] is True and monitor['disabled'] is False
    assert type(monitor['scale']) in (int, float) and math.isfinite(monitor['scale'])
    assert abs(monitor['scale'] - window['dpr']) <= profile['native_ui']['dpr_tolerance']
    assert type(monitor['refreshRate']) in (int, float) and math.isfinite(monitor['refreshRate'])
    # Pinned HyprCtl emits five-decimal Hz; wl_output truncates Hz*1000 to int.
    # +/-0.005mHz formatting uncertainty plus <1mHz truncation, with .0001 margin.
    refresh_delta_millihz = monitor['refreshRate'] * 1000 - output['current_mode']['refresh_millihz']
    assert -0.0051 <= refresh_delta_millihz < 1.0051, 'current output refresh changed'
    observation = dict(initial)
    observation['qt_screens'] = [actual_screen]
    observation['qt_window'] = {'screen': screen['name'], 'device_pixel_ratio': window['dpr'],
                                'geometry': geometry, 'exposed': window['exposed']}
    qualification = qualify(profile, observation)
    assert qualification['qualified'], qualification['failures']
    discovery.update(observation=observation, qualification=qualification,
                     actual_normal_app_metadata=metadata, authenticated_current_compositor_output=monitor,
                     current_output_under_original_frame_cutoff_ns=absolute_cutoff,
                     observation_field_origins={'qt_screens': 'actual normal Sounds frameSwapped QScreen',
                        'qt_window': 'actual normal Sounds frameSwapped QWindow',
                        'wayland_outputs': 'initial nonpresenting discovery, checked against current compositor output',
                        'drm_connectors': 'initial nonpresenting DRM discovery'})
    return qualification

def capture_native(name, output, source, binaries, command, env, qualification):
    """No generic export scaling; exact native announced buffer and pixel proof."""
    from measure import load_profile, png_pixels
    assert qualification['qualified']
    output, source, binaries = Path(output), Path(source), Path(binaries)
    capture_dir = Path('/run/user/1000/native-audio-captures')
    capture_dir.mkdir(mode=0o700, exist_ok=True)
    os.chown(capture_dir, 1000, 1000)
    png, receipt = capture_dir / (name + '.png'), capture_dir / (name + '.json')
    command([str(binaries / 'pixel10-raw-output-capture'), 'Virtual-1', str(png), str(receipt)],
            name + '-native-capture.log', env=env, user=1000)
    proof = json.loads(receipt.read_text())
    assert png_pixels(png) == load_profile(source / 'pixel10.json')['native_pixels']
    assert proof['native_buffer_width'] == 1080 and proof['native_buffer_height'] == 2424
    assert proof['png_decode_pixel_hash_matches_native_buffer']
    assert not proof['resize_or_export_scale_used'] and sha(png) == proof['png_sha256']
    shutil.copyfile(png, output / png.name)
    shutil.copyfile(receipt, output / receipt.name)
    return {'file': png.name, 'sha256': sha(png), 'actual_native_capture': proof,
            'profile_qualification': qualification, 'phone_hardware_verified': False}
