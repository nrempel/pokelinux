"""Real virtual-GPU compositor test; never evidence for Pixel hardware."""
import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import traceback

out = Path('/results')
processes = []
logs = []
result = {'passed': False, 'hardware_verified': False}


def run(args):
    # Desktop launchers can exit while their child keeps stdout open. Waiting
    # for pipe EOF would wait for the app instead of the launcher.
    with tempfile.TemporaryFile(mode='w+') as capture:
        try:
            subprocess.run(args, stdout=capture, stderr=subprocess.STDOUT,
                           timeout=60, check=True)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            capture.seek(0)
            (out / 'failed-command.log').write_text(str(args) + '\n' + capture.read())
            raise
        capture.seek(0)
        return capture.read()


def start(args, name, env=None):
    log = (out / name).open('w')
    logs.append(log)
    process = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, env=env)
    processes.append(process)
    return process


def wait(check, process, seconds=180):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if process.poll() is not None:
            raise RuntimeError(f'Process exited: {process.returncode}')
        value = check()
        if value:
            return value
        time.sleep(.5)
    raise TimeoutError('Guest condition')


try:
    result['kernel'] = run(['uname', '-a'])
    run(['modprobe', 'virtio_gpu'])
    run(['udevadm', 'settle'])
    result['drm_devices'] = sorted(str(p) for p in Path('/dev/dri').glob('*'))
    assert any('renderD' in p for p in result['drm_devices'])
    run(['usermod', '-a', '-G', 'video,input', 'alarm'])
    run(['install', '-d', '-o', 'alarm', '-g', 'alarm', '-m', '700', '/run/user/1000'])
    os.chown(out, 1000, 1000)
    package = '/workspace/artifacts/client/omarchy-phone-0.1.4-1-aarch64.pkg.tar'
    result['installation'] = run(['pacman', '-U', '--noconfirm', package])
    result['integrity'] = run(['pacman', '-Qkk', 'omarchy-phone'])
    start(['seatd', '-g', 'video'], 'seatd.log', dict(os.environ, SEATD_VTBOUND='0'))
    config = Path('/tmp/native-graphics.lua')
    config.write_text('hl.monitor({ output = "", mode = "800x600@60", position = "auto", scale = 1 })\n'
                      'hl.permission("/usr/bin/grim", "screencopy", "allow")\n')
    env = {'HOME': '/home/alarm', 'XDG_RUNTIME_DIR': '/run/user/1000',
           'LIBSEAT_BACKEND': 'seatd', 'AQ_NO_MODIFIERS': '1',
           'XDG_SESSION_TYPE': 'wayland', 'QT_QPA_PLATFORM': 'wayland'}

    def user(args):
        return ['runuser', '-u', 'alarm', '--', 'env',
                *[f'{k}={v}' for k, v in env.items()], *args]

    compositor = start(user(['Hyprland', '--config', str(config)]), 'hyprland.log')
    socket = wait(lambda: next((p for p in Path('/run/user/1000').glob('wayland-*')
                              if p.is_socket()), None), compositor)
    env['WAYLAND_DISPLAY'] = socket.name
    control = wait(lambda: next(Path('/run/user/1000/hypr').glob('*/.socket.sock'), None), compositor)
    env['HYPRLAND_INSTANCE_SIGNATURE'] = control.parent.name
    result['monitors'] = json.loads(run(user(['hyprctl', '-j', 'monitors'])))
    assert result['monitors']
    run(['systemctl', 'start', 'user@1000.service'])
    bus_path = Path('/run/user/1000/bus')
    env['DBUS_SESSION_BUS_ADDRESS'] = 'unix:path=' + str(bus_path)
    wait(bus_path.is_socket, compositor)
    run(user(['systemctl', '--user', 'import-environment', *env.keys()]))
    result['desktop_services'] = run(user(['systemctl', '--user', 'start',
                                         'pipewire.service', 'wireplumber.service']))
    # Exercise the installed unit, not a handwritten daemon invocation.
    assert not Path('/home/alarm/.config/omarchy-phone/enrollment').exists()
    run(user(['systemctl', '--user', 'start', 'omarchy-phone.service']))

    def enrollment_rejected():
        return run(user(['systemctl', '--user', 'show', 'omarchy-phone.service',
                         '-p', 'ExecMainStatus', '--value'])).strip() == '1'

    wait(enrollment_rejected, compositor, 15)
    assert not list(Path('/run/user/1000/omarchy-phone').glob('*.sock'))
    result['unenrolled_service_failure'] = run(user([
        'systemctl', '--user', 'show', 'omarchy-phone.service',
        '-p', 'ExecMainStatus', '-p', 'ActiveState', '-p', 'SubState']))
    run(user(['systemctl', '--user', 'stop', 'omarchy-phone.service']))
    assert run(user(['systemctl', '--user', 'show', 'omarchy-phone.service',
                     '-p', 'ActiveState', '--value'])).strip() == 'inactive'
    result['unenrolled_service_stopped'] = True
    archive = Path('/workspace/sim/cache/omarchy-c668141e-full.tar.gz')
    result['omarchy_archive_sha256'] = hashlib.sha256(archive.read_bytes()).hexdigest()
    assert result['omarchy_archive_sha256'] == '8cc6b1d9d903c606600395e3b3d80ad9f15bd3ab78f0f19b0f49041b1df82013'
    source_parent = Path('/tmp/omarchy-full')
    source_parent.mkdir()
    with tarfile.open(archive) as source:
        source.extractall(source_parent, filter='data')
    manifest = json.loads(Path('/workspace/sim/omarchy-source/source-manifest.json').read_text())
    root = source_parent / ('omarchy-' + manifest['commit'])
    result['omarchy_commit'] = manifest['commit']
    sys.path.insert(0, '/workspace/tools')
    from omarchy_theme_patch import apply
    result['omarchy_qt_namespace_patch'] = apply(root, manifest)
    sudo_package = '/workspace/sim/cache/session-packages/sudo-1.9.17.p2-6-aarch64.pkg.tar.xz'
    result['sudo_installation'] = run(['pacman', '-U', '--noconfirm', sudo_package])
    helper = '/usr/bin/omarchy-theme-set-browser-policy'
    rule = '/etc/sudoers.d/omarchy-theme-browser'
    run(['install', '-o', 'root', '-g', 'root', '-m', '755',
         str(root / 'bin/omarchy-theme-set-browser-policy'), helper])
    run(['install', '-d', '-m', '750', '/etc/sudoers.d'])
    run(['install', '-o', 'root', '-g', 'root', '-m', '440',
         str(root / 'etc/sudoers.d/omarchy-theme-browser'), rule])
    result['browser_theme_rule_validation'] = run(['visudo', '-c', '-f', rule])
    result['browser_theme_grant'] = run(user(['sudo', '-n', helper, '123abc']))
    denied = subprocess.run(user(['sudo', '-n', helper, 'not-a-color']),
                            capture_output=True, text=True, timeout=10)
    assert denied.returncode != 0
    result['browser_theme_invalid_argument_rejected'] = True
    result['browser_theme_installed_sha256'] = {
        p: hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in (helper, rule)}
    env['OMARCHY_PATH'] = str(root)
    env['PATH'] = str(root / 'bin') + ':/usr/bin'
    # Upstream theme hooks use login shells. Give the disposable user the
    # same Omarchy bin search path as the graphical session.
    Path('/home/alarm/.bash_profile').write_text(
        f'export OMARCHY_PATH="{root}"\nexport PATH="$OMARCHY_PATH/bin:/usr/local/bin:/usr/bin"\n')
    run(user(['systemctl', '--user', 'import-environment', 'OMARCHY_PATH', 'PATH']))
    shell = start(user(['quickshell', '-p', str(root / 'shell')]), 'omarchy-shell.log')

    def shell_ready():
        try:
            return run(user(['quickshell', 'ipc', '-n', '-p', str(root / 'shell'),
                             'call', 'shell', 'ping'])).strip() == 'ok'
        except subprocess.CalledProcessError:
            return False

    wait(shell_ready, shell)
    def plugins_ready():
        plugins = json.loads(run(user([
            'quickshell', 'ipc', '-n', '-p', str(root / 'shell'),
            'call', 'shell', 'listPlugins'])))
        return plugins if any(p.get('id') == 'omarchy.bar' for p in plugins) else None

    result['omarchy_plugins'] = wait(plugins_ready, shell)
    result['desktop_launch'] = run(user(['uwsm-app', '--', 'gtk-launch', 'org.omarchy.Phone.desktop']))

    def phone_window():
        clients = json.loads(run(user(['hyprctl', '-j', 'clients'])))
        return [c for c in clients if c.get('class') == 'org.omarchy.Phone']

    result['phone_windows'] = wait(phone_window, compositor)
    assert all(c['mapped'] and c['visible'] and not c['xwayland']
               for c in result['phone_windows'])
    result['phone_cgroup'] = Path('/proc', str(result['phone_windows'][0]['pid']), 'cgroup').read_text()
    assert '/app-graphical.slice/' in result['phone_cgroup']
    run(user(['grim', '/results/phone-wayland.png']))
    assert (out / 'phone-wayland.png').stat().st_size > 1000
    from native_theme_probe import exercise
    result['theme_changes'] = exercise(out, user, run, phone_window)
    assert shell_ready()
    from native_service_probe import exercise as exercise_service
    result['installed_service_lifecycle'] = exercise_service(user, run, wait, compositor)
    result['passed'] = True
except Exception:
    result['error'] = traceback.format_exc()
finally:
    with (out / 'guest-journal.log').open('w') as journal:
        try:
            subprocess.run(['journalctl', '-b', '-n', '250', '--no-pager'],
                           stdout=journal, stderr=subprocess.STDOUT, timeout=10)
        except subprocess.TimeoutExpired:
            pass
    for process in reversed(processes):
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    for log in logs:
        log.close()
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
