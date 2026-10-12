#!/usr/bin/env python3
"""Full-boot ARM64 integration test. This file runs only inside the QEMU guest."""
import json
import hashlib
import os
import platform
import shutil
import subprocess
import time
import traceback
from pathlib import Path

from evdev import UInput, AbsInfo, ecodes as E
from PIL import Image

PROJECT = Path('/workspace')
RESULTS = Path('/results')
checks = {}
result = {'checks': checks, 'hardware_verified': False,
          'graphics': 'Mesa virgl -> QEMU virtio GPU -> host llvmpipe software rendering',
          'kernel': platform.release(), 'architecture': platform.machine()}
result['tested_files'] = {name: hashlib.sha256((PROJECT / name).read_bytes()).hexdigest()
    for name in ('sim/guest/test.py', 'sim/guest/touch_client.py', 'tools/with-runtime.sh',
                 'artifacts/linux-aarch64/bin/runtime-probe',
                 'artifacts/linux-aarch64/lib/libpixel-runtime.so',
                 'artifacts/linux-aarch64/lib/libhybris/eglplatform_pixel.so',
                 'artifacts/runtime/runtime-manifest.json')}
processes = []
logs = []


def run(command, *, env=None, timeout=90):
    completed = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, timeout=timeout)
    print('$', ' '.join(map(str, command)), '\n', completed.stdout, flush=True)
    completed.check_returncode()
    return completed.stdout


def start(command, name, env=None):
    log = (RESULTS / name).open('w')
    logs.append(log)
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, env=env)
    processes.append(process)
    return process


def wait_for(predicate, timeout=120):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.25)
    raise TimeoutError('Guest condition did not become true')


def events():
    path = RESULTS / 'touch-events.jsonl'
    return [json.loads(line)['event'] for line in path.read_text().splitlines()] if path.exists() else []


def user(command, env):
    return ['runuser', '-u', 'alarm', '--', 'env', *[f'{k}={v}' for k, v in env.items()], *command]


def screenshot(name, channel, env):
    path = RESULTS / name
    run(user(['grim', str(path)], env))
    with Image.open(path) as image:
        rgb = image.convert('RGB')
        width, height = rgb.size
        # The native pointer starts at the centre, which is also our touch point.
        # Check an interior patch away from pointer and notification overlays.
        samples = [rgb.getpixel((width // 4 + x, height // 4 + y))
                   for x in (-20, 0, 20) for y in (-20, 0, 20)]
        if not all(p[channel] > 240 and all(v < 20 for i, v in enumerate(p) if i != channel)
                   for p in samples):
            raise AssertionError(f'Unexpected {name} pixels: {samples}')


def test():
    if 'pixel.mode=test' not in Path('/proc/cmdline').read_text():
        raise RuntimeError('Refusing to run outside the designated simulation guest')
    # Guest ownership must not require changing the real host user's UID.
    os.chown(RESULTS, 12345, 12345)
    if RESULTS.stat().st_uid != 12345:
        raise AssertionError('9p output ownership was not mapped')
    os.chown(RESULTS, 1000, 1000)
    checks['shared_output_ownership'] = RESULTS.stat().st_uid == 1000
    checks['boot'] = Path('/proc/1/comm').read_text().strip() == 'systemd'
    checks['arch_arm64'] = Path('/etc/arch-release').exists() and platform.machine() == 'aarch64'
    result['packages'] = run(['pacman', '-Q', 'hyprland', 'aquamarine', 'mesa', 'linux-aarch64']).strip()
    output = run(['sh', '/workspace/tools/with-runtime.sh',
                  '/workspace/artifacts/linux-aarch64/bin/runtime-probe',
                  '/workspace/artifacts/runtime', '--smoke',
                  '/workspace/build/bridge/stock-android-fixture.so'])
    checks['stock_abi'] = 'STOCK_ABI_SMOKE=0' in output
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1')
    run(['/workspace/build/bridge/test-software-egl',
         '/workspace/artifacts/linux-aarch64/lib/libhybris/eglplatform_pixel.so'], env=env)
    checks['software_egl'] = True
    run(['modprobe', 'virtio_gpu'])
    run(['modprobe', 'uinput'])
    run(['install', '-d', '-o', 'alarm', '-g', 'alarm', '-m', '700', '/run/user/1000'])
    run(['udevadm', 'settle'])
    start(['seatd', '-g', 'video'], 'seatd.log', dict(os.environ, SEATD_VTBOUND='0'))
    config = Path('/tmp/pixel-hyprland.lua')
    config.write_text('''hl.monitor({ output = "", mode = "800x600@60", position = "auto", scale = 1 })
hl.config({
    general = { gaps_in = 0, gaps_out = 0, border_size = 0 },
    animations = { enabled = false },
    decoration = { rounding = 0, blur = { enabled = false }, shadow = { enabled = false } },
    misc = { disable_hyprland_logo = true, disable_splash_rendering = true },
    debug = { disable_logs = false, enable_stdout_logs = true },
})
hl.permission("/usr/bin/grim", "screencopy", "allow")
''')
    user_env = {'XDG_RUNTIME_DIR': '/run/user/1000', 'HOME': '/home/alarm',
                'LIBSEAT_BACKEND': 'seatd', 'AQ_NO_MODIFIERS': '1',
                'XDG_SESSION_TYPE': 'wayland', 'GDK_BACKEND': 'wayland',
                'PIXEL_TOUCH_EVENTS': str(RESULTS / 'touch-events.jsonl')}
    axes = lambda low, high: AbsInfo(value=0, min=low, max=high, fuzz=0, flat=0, resolution=0)
    capabilities = {
        E.EV_KEY: [E.BTN_TOUCH],
        E.EV_ABS: [(E.ABS_X, axes(0, 32767)), (E.ABS_Y, axes(0, 32767)),
                  (E.ABS_MT_SLOT, axes(0, 9)), (E.ABS_MT_TRACKING_ID, axes(0, 65535)),
                  (E.ABS_MT_POSITION_X, axes(0, 32767)), (E.ABS_MT_POSITION_Y, axes(0, 32767))]}
    with UInput(capabilities, name='Pixel simulation touchscreen',
                input_props=[E.INPUT_PROP_DIRECT]) as touch:
        run(['udevadm', 'settle'])
        compositor = start(user(['Hyprland', '--config', str(config)], user_env), 'hyprland.log')
        socket = wait_for(lambda: next((p for p in Path('/run/user/1000').glob('wayland-*')
                                       if p.is_socket()), None), 180)
        user_env['WAYLAND_DISPLAY'] = socket.name
        # A Wayland socket is created before Hyprland's control interface is ready.
        # Wait for the control socket instead of racing `hyprctl instances`.
        control = wait_for(lambda: next((p for p in Path('/run/user/1000/hypr').glob('*/.socket.sock')
                                        if p.is_socket()), None), 180)
        user_env['HYPRLAND_INSTANCE_SIGNATURE'] = control.parent.name
        def control_ready():
            if compositor.poll() is not None:
                raise RuntimeError('Hyprland exited during startup')
            probe = subprocess.run(user(['hyprctl', '-j', 'monitors'], user_env),
                                   text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   timeout=15)
            if probe.returncode:
                return False
            try:
                return bool(json.loads(probe.stdout))
            except json.JSONDecodeError:
                return False
        # hyprctl has a fixed five-second receive timeout; cold shader setup
        # under CPU emulation can take longer even after the socket is open.
        wait_for(control_ready, 180)
        config_errors = run(user(['hyprctl', 'configerrors'], user_env)).strip()
        if config_errors:
            raise AssertionError('Hyprland configuration errors: ' + config_errors)
        monitors = run(user(['hyprctl', '-j', 'monitors'], user_env))
        (RESULTS / 'monitors.json').write_text(monitors)
        (RESULTS / 'devices.json').write_text(run(user(['hyprctl', '-j', 'devices'], user_env)))
        checks['hyprland'] = compositor.poll() is None and bool(json.loads(monitors))
        start(user(['python3', '/workspace/sim/guest/touch_client.py'], user_env), 'touch-client.log')
        wait_for(lambda: 'ready' in events())
        time.sleep(2)
        screenshot('before-touch.png', 0, user_env)
        checks['wayland_render'] = True
        for code, value in ((E.ABS_MT_SLOT, 0), (E.ABS_MT_TRACKING_ID, 1),
                            (E.ABS_MT_POSITION_X, 16384), (E.ABS_MT_POSITION_Y, 16384),
                            (E.ABS_X, 16384), (E.ABS_Y, 16384)):
            touch.write(E.EV_ABS, code, value)
        touch.write(E.EV_KEY, E.BTN_TOUCH, 1)
        touch.syn()
        wait_for(lambda: 'touch_down' in events(), 20)
        time.sleep(1)
        screenshot('touch-down.png', 1, user_env)
        checks['touch'] = True
        touch.write(E.EV_ABS, E.ABS_MT_TRACKING_ID, -1)
        touch.write(E.EV_KEY, E.BTN_TOUCH, 0)
        touch.syn()
        wait_for(lambda: 'touch_up' in events(), 20)
        time.sleep(1)
        screenshot('touch-up.png', 2, user_env)
        checks['touch_release'] = True
        run(user(['hyprctl', 'dispatch', 'hl.dsp.exit()'], user_env))
        if compositor.wait(timeout=20) != 0:
            raise AssertionError('Hyprland did not shut down cleanly')
        checks['compositor_shutdown'] = True


try:
    test()
except Exception:
    result['error'] = traceback.format_exc()
    print(result['error'], flush=True)
finally:
    for index, path in enumerate(Path('/run/user/1000/hypr').glob('*/hyprland.log')):
        shutil.copyfile(path, RESULTS / f'hyprland-internal-{index}.log')
    for process in reversed(processes):
        if process.poll() is None:
            process.terminate()
    for log in logs:
        log.close()
    (RESULTS / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2), flush=True)
