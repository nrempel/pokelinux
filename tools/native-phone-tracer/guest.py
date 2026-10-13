#!/usr/bin/env python3
"""Actual synthetic incoming SMS to genuine Omarchy and normal Messages; no phone."""
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import struct
import subprocess
import sys
import time
import traceback

R = Path('/results')
sys.dont_write_bytecode = True
sys.path.insert(0, str(R / 'source'))
import realm

B = Path('/tmp/native-phone-tracer')
B.mkdir()
O = B / 'evidence'
O.mkdir()
selection = json.loads((R / 'selection.json').read_text())
APP, UID = 'org.omarchy.Messages', 11010
runtime = Path('/run/pokelinux/messages')
state = Path('/var/lib/pokelinux')
install = Path('/usr/libexec/pokelinux')
context_parent = Path('/run/pokelinux-display')
result = {'passed': False, 'groups': [], 'cleanup': [], 'desired_boundary_red': False, 'native_lock_admission_qualified': False,
          'setup_complete': False, 'source_revived': False, 'sample_budget_ns': 100_000_000,
          'phone_accessed': False, 'hardware_verified': False, 'PAM_authentication_executed': False,
          'trusted_unlock_qualified': False, 'secure_presentation_qualified': False}
children, logs, owned = [], [], {}
actor = compositor = shell = None
source_fd = actor_fd = shell_fd = None
receiver = None
control = None
env = None


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def pump():
    # The native actor exclusively owns its authenticated Source channel.
    # No Python observer competes for or synthesizes those packets.
    return


def wait(check, seconds=20, optional=False):
    until = now() + int(seconds * 1e9)
    while now() < until:
        try:
            value = check()
            if value:
                return value
        except (FileNotFoundError, ConnectionRefusedError):
            pass
        time.sleep(.004)
    if optional:
        return None
    raise TimeoutError('bounded native phone tracer prerequisite')


def start(args, name, env=None, user=False, fds=(), stdin=None):
    log = (O / name).open('x')
    logs.append(log)
    identity = ({'user': UID, 'group': UID, 'extra_groups': []} if user == UID else
                {'user': 1000, 'group': 1000, 'extra_groups': os.getgrouplist('alarm', 1000)} if user else {})
    child = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, env=env,
                             pass_fds=fds, stdin=stdin, start_new_session=True, **identity)
    children.append((child, name))
    return child


def finish(child, name, force=True):
    if any(row.get('pid') == child.pid and row.get('direct_parent_wait') for row in result['cleanup']):
        return
    signals = []
    if child.poll() is None and force:
        for sig, budget in ((signal.SIGCONT, .01), (signal.SIGTERM, 3), (signal.SIGKILL, 2)):
            try:
                os.killpg(child.pid, sig)
                signals.append(sig.name)
            except ProcessLookupError:
                pass
            if sig == signal.SIGCONT:
                continue
            if wait(lambda: child.poll() is not None, budget, optional=True):
                break
    child.wait(timeout=1)
    result['cleanup'].append({'role': name, 'pid': child.pid, 'signals': signals,
                              'returncode': child.returncode, 'direct_parent_wait': True})


def command(args, name, env=None, user=False, seconds=30):
    child = start(args, name, env, user)
    wait(lambda: child.poll() is not None, seconds)
    finish(child, name, force=False)
    text = (O / name).read_text()
    assert child.returncode == 0, (args, child.returncode, text[-2000:])
    return text


def ctl(request):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.settimeout(.05)
        sock.connect(str(control))
        assert struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == (compositor.pid, 1000, 1000)
        sock.setblocking(False)
        packet = (request if request.startswith('j/') else '/' + request).encode()
        sent, reply = 0, b''
        until = now() + 15_000_000_000
        while sent < len(packet):
            try:
                sent += sock.send(packet[sent:])
            except BlockingIOError:
                time.sleep(.004)
            assert now() < until
        while now() < until:
            try:
                chunk = sock.recv(65536)
            except BlockingIOError:
                time.sleep(.004)
                continue
            if not chunk:
                return reply.decode()
            reply += chunk
            assert len(reply) < 131072
    raise TimeoutError('original compositor control')


def admin(request):
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
        sock.settimeout(2)
        sock.connect(str(runtime / 'admin.sock'))
        assert struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == (actor.pid, 0, 0)
        sock.send(json.dumps(request).encode())
        return json.loads(sock.recv(4096))


def source_events():
    path = O / 'actor.log'
    if not path.exists():
        return []
    return [value for line in path.read_text().splitlines()
            if line.startswith('{') and (value := json.loads(line)).get('event')]


def track():
    if actor is None:
        return
    for path in Path('/proc').iterdir():
        if not path.name.isdecimal():
            continue
        try:
            if int((path / 'stat').read_text().split()[3]) != actor.pid:
                continue
            pid = int(path.name)
            if pid in owned:
                continue
            args = (path / 'cmdline').read_bytes().split(b'\0')
            role = ('display_context_worker' if b'--display-context-worker' in args else
                    'messages_read_worker' if b'--messages-read-worker' in args else
                    'messages_writer' if b'--messages-worker' in args or b'--messages-incoming-worker' in args else
                    'policy_storage_worker' if b'--storage-worker' in args else 'installed_normal_Qt_app')
            owned[pid] = {'fd': os.pidfd_open(pid, os.O_NONBLOCK), 'role': role}
        except FileNotFoundError:
            pass


def live(pid):
    return Path(f'/proc/{pid}').exists() and not select.select([owned[pid]['fd']], [], [], 0)[0]


def pidfd_pid(fd):
    return int(next(line.split(':', 1)[1] for line in Path(f'/proc/self/fdinfo/{fd}').read_text().splitlines() if line.startswith('Pid:')))


def launch_rpc():
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
        sock.settimeout(.05)
        sock.connect(str(runtime / 'admin.sock'))
        assert struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == (actor.pid, 0, 0)
        sock.setblocking(False)
        started = now()
        until = started + 2_000_000_000
        packet = json.dumps({'op': 'launch', 'app': APP}).encode()
        while True:
            try:
                assert sock.send(packet) == len(packet)
                break
            except BlockingIOError:
                assert now() < until
                time.sleep(.0005)
        while now() < until:
            try:
                reply = sock.recv(4096)
            except BlockingIOError:
                time.sleep(.0005)
                continue
            assert json.loads(reply) == {'result': 'ok'}
            return {'invoked_at_ns': started, 'reply_at_ns': now(), 'reply': 'ok'}
    raise TimeoutError('unchanged two-second launch RPC budget')


class NativeActor:
    """Exact native task, with its owned diagnostic parent retaining actual wait."""
    def __init__(self, parent, pid):
        self.parent, self.pid = parent, pid

    def poll(self):
        return self.parent.poll()

    def wait(self, timeout):
        return self.parent.wait(timeout=timeout)

    @property
    def returncode(self):
        return self.parent.returncode


def record(name, **facts):
    result['groups'].append({'id': name, 'passed': True, **facts})
    print('PASS ' + name, flush=True)


def wire_pairs(app_pid, offset=0):
    import re
    trace = O / 'actor-wire.trace'
    budget = 2 * 1024 * 1024
    with trace.open('rb') as stream:
        # Bound this observation to the size of this opened trace. The writer
        # can append between fstat and read; newly appended bytes belong to
        # the next observation, not to an overflow of this one.
        size = os.fstat(stream.fileno()).st_size
        start_at = max(offset, size - budget)
        stream.seek(start_at)
        observed = stream.read(max(0, size - start_at))
    assert len(observed) <= budget
    if start_at:
        observed = observed.partition(b'\n')[2]  # Discard a truncated first record.
    if observed and not observed.endswith(b'\n'):
        observed = observed.rpartition(b'\n')[0]  # Discard an unfinished final record.
    text = observed.decode('utf-8')
    pending, pairs, unfinished = {}, [], None
    for raw in text.splitlines():
        header = re.match(r'^(\d+)\s+([0-9.]+)\s+(.*)$', raw)
        if not header or header.group(1) != str(actor.pid):
            continue
        body = header.group(3)
        if body.endswith(' <unfinished ...>'):
            assert unfinished is None, 'one syscall per observed actor thread'
            unfinished = (body.removesuffix(' <unfinished ...>'), raw)
            continue
        resumed = re.match(r'^<\.\.\. (recvmsg|sendto) resumed>(.*)$', body)
        if resumed:
            if unfinished is None:
                continue  # The start of this syscall preceded the bounded slice.
            first, first_raw = unfinished
            unfinished = None
            assert first.startswith(resumed.group(1) + '(')
            body = first + resumed.group(2)
            actual_lines = [first_raw, raw]
        else:
            actual_lines = [raw]
        line = header.group(1) + ' ' + header.group(2) + ' ' + body
        received = re.search(r'recvmsg\((\d+), .*iov_base=("(?:[^"\\]|\\.)*")', line)
        if received and 'SCM_CREDENTIALS' in line:
            try:
                payload = json.loads(json.loads(received.group(2)))
            except (ValueError, TypeError):
                continue
            if isinstance(payload, dict) and payload.get('op') == 'messages_read':
                if f'pid={app_pid}, uid={UID}, gid={UID}' in line:
                    channel = int(received.group(1))
                    assert channel not in pending, 'normal app has one outstanding exchange'
                    pending[channel] = {'request': payload, 'request_line': line,
                                        'request_actual_lines': actual_lines}
                else:
                    pending.pop(int(received.group(1)), None)
        sent = re.search(r'sendto\((\d+), ("(?:[^"\\]|\\.)*").* = (\d+)$', line)
        if sent and int(sent.group(1)) in pending:
            try:
                payload = json.loads(json.loads(sent.group(2)))
            except (ValueError, TypeError):
                continue
            assert int(sent.group(3)) == len(json.loads(sent.group(2)).encode()), 'complete actual publication'
            pairs.append({**pending.pop(int(sent.group(1))), 'reply': payload, 'reply_line': line,
                          'reply_actual_lines': actual_lines})
    return pairs


def focus_app(pid):
    window = wait(lambda: next((row for row in json.loads(ctl('j/clients'))
                               if row.get('pid') == pid and row.get('mapped')), None), 10)
    assert ctl('dispatch hl.dsp.focus({ window = ' + json.dumps('address:' + window['address']) + ' })').strip() == 'ok'
    wait(lambda: json.loads(ctl('nativebuildprobe'))['pid'] == pid, 3)
    return window


def current_health(pid, expected_locked=False):
    snapshot = admin({'op': 'snapshot'})
    focus = json.loads(ctl('nativebuildprobe'))
    events = source_events()
    assert snapshot['ready'] and snapshot['locked'] == expected_locked and snapshot['principals'] == 1
    assert actor.poll() is None and compositor.poll() is None
    assert not any(row['event'] == 'source_failed' for row in events)
    assert focus['pid'] == pid and focus['uid'] == UID and focus['gid'] == UID and not focus['locked']
    assert Path(f'/proc/{pid}').exists()
    return {'snapshot': snapshot, 'focus': focus, 'observed_at_ns': now()}


def message_page(pid, ids, offset=0):
    for pair in reversed(wire_pairs(pid, offset)):
        reply = pair['reply']
        if isinstance(reply, dict) and reply.get('page') == 'messages':
            items = reply.get('items', [])
            if ({row.get('receipt') for row in items} == set(ids)
                    and all(row.get('text') == 'hello' and row.get('sender') == '+15550100' for row in items)):
                return pair
    return None


input_sequence = 0
capture_sequence = 0


def capture_frontend(name):
    # Observe the emulator display without asking the guest compositor to
    # perform a synchronous Wayland framebuffer readback. This is a different
    # evidence class from the retained direct native-buffer captures.
    global capture_sequence
    from measure import png_pixels, load_profile
    capture_sequence += 1
    directory = R / 'capture'
    directory.mkdir(exist_ok=True)
    request = directory / f'{capture_sequence:04}.json'
    temporary = request.with_suffix('.pending')
    temporary.write_text(json.dumps({'sequence': capture_sequence, 'name': name}) + '\n')
    temporary.replace(request)
    reply = request.with_suffix('.reply.json')
    value = wait(lambda: json.loads(reply.read_text()) if reply.exists() else None, 20)
    assert value['ok'] and value['sequence'] == capture_sequence and value['name'] == name
    assert not value['resize_or_export_scale_used'] and not value['Wayland_native_buffer_pixel_equality_qualified']
    png = directory / value['png']
    assert png_pixels(png) == load_profile(R / 'source/pixel10.json')['native_pixels']
    assert sha(png) == value['png_sha256']
    shutil.copyfile(png, O / (name + '.png'))
    return {'file': name + '.png', 'sha256': value['png_sha256'], 'frontend_capture': value,
            'profile_qualification': qualification, 'phone_hardware_verified': False,
            'scope': 'actual unscaled SDL/X11 frontend; direct Wayland native-buffer equality unobserved'}


def normal_keyboard_action():
    global input_sequence
    input_sequence += 1
    directory = R / 'input'
    directory.mkdir(exist_ok=True)
    request = directory / f'{input_sequence:04d}.json'
    temporary = request.with_suffix('.pending')
    temporary.write_text(json.dumps({'sequence': input_sequence, 'keys': ['tab', 'ret']}) + '\n')
    temporary.replace(request)
    response = request.with_suffix('.reply.json')
    value = wait(lambda: json.loads(response.read_text()) if response.exists() else None, 10)
    assert value['ok'] and value['sequence'] == input_sequence and value['keys'] == ['tab', 'ret']
    return value


def launch_installed():
    # Same fixed Exec as the installed desktop entry, called by the real session UID.
    command([str(install / 'messages-launch'), str(runtime / 'launch-messages.sock')],
            'desktop-launch-' + str(now()) + '.log', env=env, user=True)
    def current_window():
        track()
        # Compositor enumeration can briefly retain the previously closed
        # window. Select only a currently owned, live normal app generation.
        return next((row for row in json.loads(ctl('j/clients'))
                     if row.get('title') == 'Messages' and row.get('mapped')
                     and row.get('pid') in owned
                     and owned[row['pid']]['role'] == 'installed_normal_Qt_app'
                     and live(row['pid'])), None)
    window = wait(current_window, 60)
    pid = window['pid']
    track()
    assert not window['xwayland'] and live(pid)
    assert int(Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]) == UID
    assert int(Path(f'/proc/{pid}/stat').read_text().split()[3]) == actor.pid
    argv = [part.decode() for part in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')[:-1]]
    assert len(argv) == 7 and argv[0] == str(install / 'pokelinux-messages')
    listener = Path(argv[3])
    assert listener.is_relative_to(context_parent)
    context = wait(lambda: next((row for row in reversed(source_events())
        if row['event'] == 'app_context' and row['listener'] == str(listener)), None), 3)
    assert context['state'] == 'Ready' and context['admission'] and context['occupied']
    assert context['metadata_scope'] == 'app_generation'
    assert live(context['worker_pid'])
    env_names = {part.split(b'=', 1)[0] for part in Path(f'/proc/{pid}/environ').read_bytes().split(b'\0') if part}
    assert not env_names & {b'QT_FONT_DPI', b'QT_SCALE_FACTOR', b'QT_SCREEN_SCALE_FACTORS', b'QT_DEVICE_PIXEL_RATIO', b'QT_USE_PHYSICAL_DPI'}
    return pid, window, context


def rows():
    import sqlite3
    with sqlite3.connect('file:' + str(state / 'raw/receipts.sqlite3') + '?mode=ro', uri=True, timeout=0) as db:
        raw = db.execute('SELECT id,slot,source_incarnation,scope,ack,hex(raw) FROM receipts ORDER BY seq').fetchall()
    with sqlite3.connect('file:' + str(state / 'inbox/inbox.sqlite3') + '?mode=ro', uri=True, timeout=0) as db:
        inbox = db.execute('SELECT id,slot,source_incarnation,state,body FROM records ORDER BY seq').fetchall()
    assert [row[0] for row in raw] == [row[0] for row in inbox]
    return {'raw': raw, 'inbox': inbox}


peer = None
try:
    assert os.getuid() == 0 and os.uname().machine == 'aarch64'
    assert os.uname().release == '7.1.6-1-aarch64-ARCH'
    assert sorted(path.name for path in Path('/sys/class/net').iterdir()) == ['lo']
    for name, expected in selection['files'].items():
        assert sha(R / name) == expected, name
    assert sha('/usr/bin/Hyprland') == selection['expected_hyprland_elf_sha256']
    result.update(scope='booted Arch ARM kernel/systemd to synthetic Binder incoming and current native services to genuine Omarchy/normal Qt',
                  production_modem_backend=False, Google_kernel_booted=False,
                  normal_app_replaced_or_instrumented=False, source_budget_ns=100_000_000)
    command(['modprobe', 'virtio_gpu'], 'modprobe.log')
    command(['udevadm', 'settle'], 'udev.log')
    command(['usermod', '-a', '-G', 'video,input', 'alarm'], 'groups.log')
    command(['useradd', '-u', str(UID), '-M', '-s', '/usr/bin/nologin', 'messages-app'], 'app-user.log')
    command(['install', '-d', '-o', 'alarm', '-g', 'alarm', '-m', '700', '/run/user/1000'], 'runtime.log')
    # Establish systemd's user runtime mount before the compositor creates its
    # live sockets. Starting it later can replace the directory underneath them.
    command(['systemctl', 'start', 'user@1000.service'], 'user-service.log')
    wait(lambda: Path('/run/user/1000/bus').is_socket(), 30)
    start(['seatd', '-g', 'video'], 'seatd.log', dict(os.environ, SEATD_VTBOUND='0'))
    for path, mode in [(runtime, 0o755), (state, 0o700), (state / 'policy', 0o700), (install, 0o755),
                       (Path('/run/user/11010'), 0o700), (Path('/var/lib/pokelinux-messages-app'), 0o700),
                       (context_parent, 0o711)]:
        path.mkdir(parents=True, exist_ok=True)
        path.chmod(mode)
    for path in [Path('/run/user/11010'), Path('/var/lib/pokelinux-messages-app')]:
        os.chown(path, UID, UID)
    binaries = B / 'bins'
    binaries.mkdir()
    for path in (R / 'bins').iterdir():
        shutil.copyfile(path, binaries / path.name)
        (binaries / path.name).chmod(0o755)
    for name in ['messages-phone-tracer', 'lifecycle-launcher', 'messages-launch', 'pokelinux-messages']:
        shutil.copyfile(binaries / name, install / name)
        (install / name).chmod(0o755)
    desktop = Path('/usr/share/applications/org.omarchy.Messages.desktop')
    desktop.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(R / 'source/org.omarchy.Messages.desktop', desktop)
    assert 'Exec=/usr/libexec/pokelinux/messages-launch /run/pokelinux/messages/launch-messages.sock' in desktop.read_text()
    appearance = Path('/usr/share/pokelinux/appearance')
    appearance.mkdir(parents=True)
    for name in ['PhoneTheme.qml', 'ThemePalette.js']:
        shutil.copyfile(R / 'source/native-app' / name, appearance / name)
    (B / 'hypr.lua').write_text(
        'hl.monitor({ output = "", mode = "1080x2424@60", position = "auto", scale = 2.625 })\n'
        'hl.permission(' + json.dumps(str(binaries / 'pixel10-raw-output-capture')) + ', "screencopy", "allow")\n'
        'hl.config({ debug = { disable_scale_checks = true }, animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })\n')
    receive, send = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    receive.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
    incarnation = os.urandom(16).hex()
    env = {'HOME': '/home/alarm', 'USER': 'alarm', 'LOGNAME': 'alarm', 'LANG': 'C.UTF-8', 'PATH': '/usr/local/bin:/usr/bin',
           'XDG_RUNTIME_DIR': '/run/user/1000', 'LIBSEAT_BACKEND': 'seatd', 'AQ_NO_MODIFIERS': '1',
           'XDG_SESSION_TYPE': 'wayland', 'QT_QPA_PLATFORM': 'wayland',
           'POKELINUX_SESSION_FD': str(send.fileno()), 'POKELINUX_SESSION_SOURCE': incarnation,
           'POKELINUX_DISPLAY_PROFILE': str(R / 'source/pixel10.json'),
           'POKELINUX_DISPLAY_PROFILE_RECEIPT': '/run/user/1000/pixel10-profile/applied.json',
           'OMARCHY_THEME_SKIP_BACKGROUND': '1',
           'DBUS_SESSION_BUS_ADDRESS': 'unix:path=/run/user/1000/bus'}
    compositor = start(['/usr/bin/Hyprland', '--config', str(B / 'hypr.lua')], 'hyprland.log', env, user=True, fds=(send.fileno(),))
    source_fd = os.pidfd_open(compositor.pid, os.O_NONBLOCK)
    send.close()
    receiver = receive
    wayland = wait(lambda: next((path for path in Path('/run/user/1000').glob('wayland-*') if path.is_socket()), None), 120)
    control = wait(lambda: next(Path('/run/user/1000/hypr').glob('*/.socket.sock'), None), 120)
    env.update(WAYLAND_DISPLAY=wayland.name, HYPRLAND_INSTANCE_SIGNATURE=control.parent.name)
    assert ctl('plugin load ' + str(binaries / 'probe.so')).strip() == 'ok'
    qualification = realm.apply_display(R / 'source', binaries, command, env, result)
    original, shell = realm.stage_omarchy(R / 'source', command, start, env, result)
    def shell_ready():
        assert shell.poll() is None, 'original Omarchy shell exited during setup'
        name = 'original-shell-ready-' + str(now()) + '.log'
        child = start(['quickshell', 'ipc', '-n', '-p', str(original / 'shell'), 'call', 'shell', 'ping'],
                      name, env=env, user=True)
        wait(lambda: child.poll() is not None, 5)
        finish(child, 'original-shell-ready', force=False)
        return child.returncode == 0 and (O / name).read_text().strip() == 'ok'
    wait(shell_ready, 30)
    command(['systemctl', '--user', 'import-environment', *env.keys()], 'user-env.log', env=env, user=True)
    Path('/home/alarm/.bash_profile').write_text('export OMARCHY_PATH=/usr/share/omarchy\nexport PATH="$OMARCHY_PATH/bin:/usr/local/bin:/usr/bin"\n')
    command([str(original / 'bin/omarchy-theme-set'), 'tokyo-night'], 'original-theme-tokyo-night.log', env=env, user=True, seconds=90)
    wait(shell_ready, 30)
    plugins = json.loads(command(['quickshell', 'ipc', '-n', '-p', str(original / 'shell'),
        'call', 'shell', 'listPlugins'], 'original-active-plugins.log', env=env, user=True))
    assert any(plugin['id'] == 'omarchy.bar' for plugin in plugins)
    result['actual_original_Omarchy_plugins'] = plugins
    current_theme = Path('/home/alarm/.local/state/omarchy/current/theme')
    assert (current_theme / 'colors.toml').is_file()
    result['actual_generated_Omarchy_theme'] = {'name': 'tokyo-night', 'colors_sha256': sha(current_theme / 'colors.toml'),
                                             'supported_original_skip_background': True}
    # Publish only selected appearance files. The session's private home stays
    # mode0700; the unchanged distinct-UID app receives an appearance-only view.
    theme_home = Path('/usr/share/pokelinux/theme-home')
    published_theme = theme_home / '.local/state/omarchy/current/theme'
    original_selection = current_theme.resolve(strict=True)
    selected_appearance = [(current_theme / 'colors.toml', published_theme / 'colors.toml')]
    for source_path, relative in [(current_theme / 'shell.toml', '.local/state/omarchy/current/theme/shell.toml'),
                                  (Path('/home/alarm/.config/omarchy/shell.toml'), '.config/omarchy/shell.toml'),
                                  (Path('/home/alarm/.config/fontconfig/fonts.conf'), '.config/fontconfig/fonts.conf')]:
        if source_path.exists():
            selected_appearance.append((source_path, theme_home / relative))
    published = []
    for source_path, destination in selected_appearance:
        assert source_path.is_file() and source_path.stat().st_size <= 256 * 1024
        digest = sha(source_path)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source_path, destination)
        destination.chmod(0o444)
        assert sha(source_path) == sha(destination) == digest
        published.append({'source': str(source_path), 'published': str(destination), 'sha256': digest})
    assert current_theme.resolve(strict=True) == original_selection
    reader = '''import hashlib,json,os,sys
from pathlib import Path
assert os.getuid()==os.geteuid()==11010 and os.getgid()==11010 and os.getgroups()==[]
p=Path(sys.argv[1])/'.local/state/omarchy/current/theme/colors.toml'
assert p.resolve(strict=True).is_relative_to(Path(sys.argv[1]))
data=p.read_bytes(); assert 0<len(data)<=256*1024
print(json.dumps({'uid':os.getuid(),'gid':os.getgid(),'groups':os.getgroups(),'sha256':hashlib.sha256(data).hexdigest()}))
'''
    appearance_readback = json.loads(command(['/usr/bin/python3', '-c', reader, str(theme_home)],
                                             'normal-UID-appearance-readback.log', user=UID))
    assert appearance_readback['sha256'] == sha(current_theme / 'colors.toml') == sha(published_theme / 'colors.toml')
    assert Path('/home/alarm').stat().st_mode & 0o777 == 0o700
    result['appearance_only_view'] = {'files': published, 'readback': appearance_readback,
                                     'private_home_mode': '0700', 'live_publication_service_qualified': False}
    shutil.copyfile(binaries / 'strace', '/usr/bin/strace')
    Path('/usr/bin/strace').chmod(0o755)
    assert 'not found' not in command(['ldd', '/usr/bin/strace'], 'strace-closure.log')
    runtime_mount = Path('/private/artifacts/runtime')
    runtime_mount.mkdir(parents=True, exist_ok=True)
    command(['mount', '-t', '9p', '-o', 'trans=virtio,version=9p2000.L,ro', 'android-runtime', str(runtime_mount)], 'original-reference-runtime-mount.log')
    Path('/dev/binderfs').mkdir(exist_ok=True)
    command(['mount', '-t', 'binder', '-o', 'max=8,stats=global', 'binder', '/dev/binderfs'], 'binderfs.log')
    sys.path.insert(0, str(R / 'source'))
    import peer_session
    import pwd
    peer_session.BASE = Path('/run/tracer-peer')
    peer_session.BASE.mkdir()
    peer_session.LOADER = '/private/artifacts/runtime/apex/com.android.runtime/bin/linker64'
    peer_session.LIBRARIES = '/private/artifacts/runtime/system/lib64:/private/artifacts/runtime/apex/com.android.runtime/lib64/bionic'
    for name in ['incoming-peer', 'property-info', 'property-init']:
        shutil.copyfile(binaries / name, peer_session.BASE / name)
        (peer_session.BASE / name).chmod(0o755)
    peerbins = peer_session.BASE / 'target/aarch64-unknown-linux-gnu/debug'
    peerbins.mkdir(parents=True)
    for name in ['incoming-manager', 'incoming-probe']:
        shutil.copyfile(binaries / name, peerbins / name)
        (peerbins / name).chmod(0o755)
    peer = peer_session.PeerSession(pwd.getpwnam('alarm'), O, 'one-tracer-generation').start()
    result['synthetic_original_library_peer'] = peer.evidence
    profile_args = ['/var/lib/pokelinux-messages-app', '/run/user/11010', str(context_parent / 'required-fresh-app-socket'),
                    str(theme_home), str(runtime / 'app.sock'), str(appearance)]
    profile = B / 'app-profile.json'
    profile.write_text(json.dumps({'apps': [{'app': APP, 'uid': UID, 'executable': str(install / 'pokelinux-messages'), 'args': profile_args}]}))
    args = [str(install / 'messages-phone-tracer'), str(runtime), str(state / 'policy'), str(profile),
            str(install / 'lifecycle-launcher'), str(state / 'raw'), str(state / 'inbox'), str(receive.fileno()),
            str(source_fd), str(compositor.pid), '1000', incarnation, str(wayland), str(context_parent), peer.generation, 'none']
    actor_parent = start(['strace', '-qq', '-f', '--seccomp-bpf', '-ttt', '-s', '8192', '-e', 'trace=recvmsg,sendto', '-e', 'status=successful',
                          '-o', str(O / 'actor-wire.trace'), *args], 'actor.log',
                         dict(os.environ, NATIVE_MESSAGES_ISOLATED='1'), fds=(receive.fileno(), source_fd))

    def native_pid():
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
            sock.settimeout(2)
            sock.connect(str(runtime / 'admin.sock'))
            pid, uid, gid = struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            assert uid == gid == 0 and pid != actor_parent.pid
            return pid

    actor = NativeActor(actor_parent, wait(native_pid))
    actor_fd = os.pidfd_open(actor.pid, os.O_NONBLOCK)
    owned[actor.pid] = {'fd': os.pidfd_open(actor.pid, os.O_NONBLOCK), 'role': 'native_actor'}
    receive.close()
    receiver = None
    wait(lambda: admin({'op': 'snapshot'}).get('ready'))
    wait(lambda: admin({'op': 'messages_status'}).get('receiving'))
    wait(lambda: admin({'op': 'messages_status'}).get('available'))
    track()
    writer = next(pid for pid, row in owned.items() if row['role'] == 'messages_writer')
    writer_argv = Path(f'/proc/{writer}/cmdline').read_bytes().split(b'\0')[:-1]
    assert writer_argv[-2].decode() == peer.generation
    writer_incarnation = writer_argv[-3].decode()

    def probe(*arguments):
        return json.loads(command([str(peerbins / 'incoming-probe'), *map(str, arguments)],
                          'probe-' + str(now()) + '.log'))

    def counters():
        counts = [probe('counters', slot) for slot in range(2)]
        assert all(row[:2] == [1, 1] and row[3:] == [0, 0, 0, 0] for row in counts)
        return counts

    def arrive(count):
        # Literal authored test PDU decoded by the actual native incoming path.
        command([str(peerbins / 'incoming-probe'), 'incoming', '0', '0',
                 '000008915155100000006201213141510005e8329bfd06', '0'],
                'incoming-' + str(count) + '.log')
        wait(lambda: admin({'op': 'messages_status'})['incoming_completed'] == count, 4)
        records = rows()
        assert len(records['raw']) == count and all(row[3:5] == ('test_backend', 'stored') for row in records['raw'])
        assert all(row[3:] == ('text', 'hello') for row in records['inbox'])
        assert live(writer) and Path(f'/proc/{writer}/cmdline').read_bytes().split(b'\0')[:-1][-3].decode() == writer_incarnation
        return records

    first = arrive(1)
    assert admin({'op': 'snapshot'})['principals'] == 0
    record('synthetic_Binder_SMS_commits_native_raw_and_inbox_with_UI_closed', exact=first,
           writer_pid=writer, writer_incarnation=writer_incarnation, counters=counters())
    app_pid, window, context = launch_installed()
    wait(lambda: next((pair for pair in wire_pairs(app_pid) if pair['reply'] == {'result': 'denied'}), None), 3)
    # Finish the first real normal-app render before Source starts. The native
    # source's25ms timer and100ms lease are unchanged throughout the live slice.
    time.sleep(1)
    result['pre_Source_render'] = realm.capture_native('pre-source-normal-app-denied', O, R / 'source', binaries,
                                                      command, env, qualification)
    # The host SDL viewport reaches native dimensions only after actual app
    # rendering. Its origin is configured before the first Source adapter load,
    # while the actor still denies this normal app. No viewport resize occurs.
    assert actor.poll() is None and admin({"op": "snapshot"})["locked"]
    placement = R / 'placement'
    placement_request = placement / '0001.json'
    placement_pending = placement_request.with_suffix('.pending')
    placement_pending.write_text(json.dumps({'op': 'place_owned_native_frontend_at_origin',
                                             'before_Source_adapter_load': True}) + '\n')
    placement_pending.replace(placement_request)
    placed = wait(lambda: json.loads((placement / '0001.reply.json').read_text())
                  if (placement / '0001.reply.json').exists() else None, 20)
    assert placed['ok'] and not placed['resize_or_export_scale_used']
    result['frontend_constructor_configuration'] = placed
    focus_app(app_pid)
    assert ctl('plugin load ' + str(binaries / 'adapter.so')).strip() == 'ok'
    wait(lambda: not admin({'op': 'snapshot'})['locked'], 3)
    wait(lambda: next((row for row in reversed(source_events())
        if row['event'] == 'session' and (row.get('focus') or {}).get('pid') == app_pid), None), 3)
    assert admin({'op': 'grant', 'app': APP, 'permission': 'messages_read', 'allowed': True}) == {'result': 'ok'}
    ids = [row[0] for row in first['raw']]
    page = wait(lambda: message_page(app_pid, ids), 4)
    record('real_desktop_launcher_current_focus_and_independent_permission_deliver_exact_SMS_to_normal_app',
           app_pid=app_pid, context=context, actual_wire=page, health=current_health(app_pid))
    offset = (O / 'actor-wire.trace').stat().st_size
    keyboard = normal_keyboard_action()
    activated = wait(lambda: message_page(app_pid, ids, offset), 4)
    record('actual_QEMU_keyboard_submission_and_subsequent_normal_app_history_read', input=keyboard,
           actual_wire=activated, health=current_health(app_pid))
    initial_capture_before = current_health(app_pid)
    initial_capture = capture_frontend('native-phone-tracer-first-message')
    initial_capture_after = current_health(app_pid)
    record('first_exact_native_SMS_frontend_capture_at_Pixel10_profile', capture=initial_capture,
           health_before=initial_capture_before, health_after=initial_capture_after,
           literal_body_visual_review_required=True, normal_QWindow_DPR='unobserved; whole output independently qualified')
    assert ctl('dispatch hl.dsp.window.close({ window = ' + json.dumps('address:' + window['address']) + ' })').strip() == 'ok'
    wait(lambda: not live(app_pid), 3)
    wait(lambda: not live(context['worker_pid']), 3)
    assert admin({'op': 'snapshot'})['principals'] == 0
    second = arrive(2)
    assert second['raw'][0][2] == second['raw'][1][2] and second['raw'][0][0] != second['raw'][1][0]
    record('normal_app_close_reaps_app_context_and_next_closed_UI_SMS_keeps_same_writer_and_generation',
           actual_closed_app_pid=app_pid, retired_context=context, exact=second, counters=counters())
    old_pid, old_context = app_pid, context
    app_pid, window, context = launch_installed()
    focus_app(app_pid)
    assert app_pid != old_pid and context['instance'] != old_context['instance']
    ids = [row[0] for row in second['raw']]
    page = wait(lambda: message_page(app_pid, ids), 4)
    record('fresh_normal_app_context_reopens_same_native_incoming_history', app_pid=app_pid,
           context=context, actual_wire=page, health=current_health(app_pid), counters=counters())
    offset = (O / 'actor-wire.trace').stat().st_size
    assert admin({'op': 'grant', 'app': APP, 'permission': 'messages_read', 'allowed': False}) == {'result': 'ok'}
    denied = wait(lambda: next((pair for pair in wire_pairs(app_pid, offset)
                               if pair['reply'] == {'result': 'denied'}), None), 3)
    restore_offset = (O / 'actor-wire.trace').stat().st_size
    assert admin({'op': 'grant', 'app': APP, 'permission': 'messages_read', 'allowed': True}) == {'result': 'ok'}
    restored = wait(lambda: message_page(app_pid, ids, restore_offset), 4)
    record('revocation_denies_fresh_normal_app_read_and_explicit_grant_restores_current_history',
           denied=denied, restored=restored, health=current_health(app_pid))
    before_capture = current_health(app_pid)
    time.sleep(.2)
    capture = capture_frontend('native-phone-tracer-messages')
    after_capture = current_health(app_pid)
    record('actual_unscaled_Pixel10_frontend_capture_with_live_current_app_and_genuine_Omarchy',
           capture=capture, health_before=before_capture, health_after=after_capture,
           literal_body_visual_review_required=True, normal_QWindow_DPR='unobserved; whole output independently qualified')
    result['setup_complete'] = True
    result['passed'] = True
except BaseException as error:
    result['error'] = str(error)
    result['traceback'] = traceback.format_exc()
    print(result['traceback'], flush=True)
finally:
    if actor is not None and actor.poll() is None:
        try:
            result['owned_Stop_requested_at_ns'] = now()
            admin({'op': 'stop'})
            wait(lambda: actor.poll() is not None, 5)
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(str(error))
    for child, name in reversed(children):
        try:
            finish(child, name)
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(str(error))
    for pid, owner in owned.items():
        dead = bool(select.select([owner['fd']], [], [], 0)[0])
        absent = not Path(f'/proc/{pid}').exists()
        result['cleanup'].append({'role': owner['role'], 'pid': pid, 'owned_pidfd_death_observed': dead,
                                  'proc_absent': absent, 'reaped': dead and absent})
        os.close(owner['fd'])
    if peer is not None:
        try:
            peer.close()
            for name, child in peer.owned:
                result['cleanup'].append({'role': 'synthetic_' + name, 'pid': child.pid,
                                          'direct_parent_wait': True, 'returncode': child.returncode})
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(str(error))
    if receiver is not None:
        receiver.close()
    for fd in (source_fd, actor_fd, shell_fd):
        if fd is not None:
            os.close(fd)
    for stream in logs:
        stream.close()
    result['all_owned_processes_waited_or_finally_absent'] = not result.get('cleanup_errors') and all(
        row.get('direct_parent_wait') or row.get('reaped') for row in result['cleanup'])
    result['passed'] = result['passed'] and result['all_owned_processes_waited_or_finally_absent']
    result['source_events'] = source_events()
    stop_at = result.get('owned_Stop_requested_at_ns')
    result['no_Source_failure_before_owned_Stop_request'] = bool(stop_at) and all(
        row['observed_at_ns'] >= stop_at for row in result['source_events'] if row['event'] == 'source_failed')
    result['passed'] = result['passed'] and result['no_Source_failure_before_owned_Stop_request']
    (O / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    shutil.copytree(O, R, dirs_exist_ok=True)
    subprocess.run(['sync'], timeout=10)
    subprocess.run(['systemctl', 'poweroff'], timeout=15)
