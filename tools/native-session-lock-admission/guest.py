#!/usr/bin/env python3
"""Original installed Omarchy lock request against current owned native actor."""
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
import boundary
import realm

B = Path('/tmp/native-lock-entry')
B.mkdir()
O = B / 'evidence'
O.mkdir()
selection = json.loads((R / 'selection.json').read_text())
if selection.get('restart_boundary'):
    import restart_boundary as boundary
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
actor_log = O / 'actor.log'
actor_trace = O / 'actor-wire.trace'


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
    raise TimeoutError('bounded original lock-entry prerequisite')


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
    path = actor_log
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
                    'messages_writer' if b'--messages-worker' in args else
                    'policy_storage_worker' if b'--storage-worker' in args else 'installed_normal_Qt_app')
            owned[pid] = {'fd': os.pidfd_open(pid, os.O_NONBLOCK), 'role': role, 'actor_pid': actor.pid}
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


class Scope:
    result = result
    now = staticmethod(now)
    wait = staticmethod(wait)
    pump = staticmethod(pump)
    sleep = staticmethod(time.sleep)
    admin = staticmethod(admin)
    source_events = staticmethod(source_events)
    pidfd_pid = staticmethod(pidfd_pid)

    def shell_status(self, allow_starting=False):
        self.status_sequence += 1
        name = 'original-lock-public-status-' + str(self.status_sequence) + '.log'
        child = start([str(self.original / 'bin/omarchy-shell'), 'lock', 'status'], name, env, user=True)
        wait(lambda: child.poll() is not None, 30)
        finish(child, name, force=False)
        text = (O / name).read_text()
        if child.returncode:
            assert allow_starting and not self.source_started and shell.poll() is None, (child.returncode, text[-2000:])
            assert any(value in text for value in ['not running', 'not ready', 'not responding', 'Target not found.', 'Function not found.']), text[-2000:]
            return None
        return json.loads(text)

    def original_focus(self):
        return json.loads(ctl('nativebuildprobe'))

    def current_app_health(self, expected_locked=False):
        snapshot = admin({'op': 'snapshot'})
        focus = self.original_focus()
        events = source_events()
        context = next((row for row in reversed(events) if row['event'] == 'app_context' and row['listener'] == str(self.listener)), None)
        assert snapshot['ready'] and snapshot['locked'] == expected_locked and snapshot['principals'] == 1
        assert actor.poll() is None and not select.select([actor_fd], [], [], 0)[0]
        assert compositor.poll() is None and not select.select([source_fd], [], [], 0)[0]
        assert not any(row['event'] == 'source_failed' for row in events)
        assert focus['pid'] == self.app_pid and focus['uid'] == UID and focus['gid'] == UID and not focus['locked']
        assert live(self.app_pid) and live(self.writer_pid) and live(self.sdk_pid)
        assert context and context['state'] == 'Ready' and context['admission'] and context['occupied']
        assert context['metadata_scope'] == 'app_generation' and context['worker_pid'] == self.sdk_pid
        return {'actor_snapshot': snapshot, 'current_original_focus': focus, 'app_context': context,
                'actual_app_pid': self.app_pid, 'writer_pid': self.writer_pid, 'sdk_pid': self.sdk_pid,
                'observed_at_ns': now()}

    def start_original_lock_action(self):
        return start([str(self.original / 'bin/omarchy-system-lock')], 'original-installed-system-lock.log', env, user=True)

    def wire_pairs(self, offset=0):
        import re
        trace = actor_trace
        with trace.open('r') as source:
            source.seek(offset)
            text = source.read(2097153)
        assert len(text) <= 2097152, 'bounded owned actor trace overflow'
        pending, pairs = {}, []
        for line in text.splitlines():
            if line.split()[0] != str(actor.pid):
                continue
            received = re.search(r'recvmsg\((\d+), .*iov_base=("(?:[^"\\]|\\.)*")', line)
            if received and 'SCM_CREDENTIALS' in line:
                try:
                    payload = json.loads(json.loads(received.group(2)))
                except (ValueError, TypeError):
                    continue
                if isinstance(payload, dict) and payload.get('op') == 'messages_read':
                    assert f'pid={self.app_pid}, uid={UID}, gid={UID}' in line
                    pending[int(received.group(1))] = {'request': payload, 'request_line': line}
            sent = re.search(r'sendto\((\d+), ("(?:[^"\\]|\\.)*").* = (\d+)$', line)
            if sent and int(sent.group(1)) in pending:
                try:
                    payload = json.loads(json.loads(sent.group(2)))
                except (ValueError, TypeError):
                    continue
                pairs.append({**pending.pop(int(sent.group(1))), 'reply': payload, 'reply_line': line})
        return pairs

    def trace_offset(self):
        return actor_trace.stat().st_size

    def original_source_packets(self):
        import re
        # The retained trace may grow while the actor's nonblocking poll records
        # EAGAIN. Read a bounded snapshot of its tail for fresh Source witnesses;
        # the full immutable trace is preserved for independent verification.
        size = actor_trace.stat().st_size
        begin = max(0, size - 2097152)
        with actor_trace.open('rb') as source:
            source.seek(begin)
            data = source.read(size - begin)
        assert len(data) <= 2097152
        if begin:
            data = data.split(b'\n', 1)[1] if b'\n' in data else b''
        text = data.decode()
        packets = []
        for line in text.splitlines():
            if line.split()[0] != str(actor.pid):
                continue
            received = re.search(r'recvmsg\((\d+), .*iov_base=("(?:[^"\\]|\\.)*")', line)
            if not received or 'SCM_CREDENTIALS' not in line:
                continue
            try:
                payload = json.loads(json.loads(received.group(2)))
            except (ValueError, TypeError):
                continue
            if isinstance(payload, dict) and 'sampled_at_ns' in payload:
                assert f'pid={compositor.pid}, uid=1000, gid=1000' in line
                packets.append({'frame': payload, 'actual_receive_line': line})
        return packets

    def lock_rpc(self, value, uid):
        code = ('import socket,json,os; s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET); '
                's.settimeout(2); s.connect(' + repr(str(runtime / 'lock.sock')) + '); '
                's.send(' + repr(json.dumps(value).encode()) + '); '
                'print(json.dumps({"actual_uid":os.getuid(),"reply":json.loads(s.recv(4096))}))')
        name = 'lock-route-negative-' + str(self.negative_sequence) + '.log'
        self.negative_sequence += 1
        return json.loads(command(['python3', '-c', code], name, env, user=(UID if uid == UID else True if uid == 1000 else False)))

    def write_witness(self, name, witness):
        with (O / name).open('x') as output:
            json.dump(witness, output, indent=2)
            output.write('\n')

    def restart_display_epoch(self):
        """Actual clean actor replacement; fresh original compositor/Source.

        The original plugin consumes and unsets its launch-only channel. It has
        no fresh-endpoint reload API. This deliberately changes display epoch,
        retaining the SAME root policy directory and the held original QML task.
        """
        global actor, actor_fd, compositor, source_fd, receiver, receive, control
        global actor_log, actor_trace, actor_parent, incarnation, args, wayland
        track()
        old_actor, old_source = actor, compositor
        old_source_events = source_events()
        old_source_fd, old_actor_fd = source_fd, actor_fd
        stop_started = now()
        stop_ack = admin({'op': 'stop'})
        assert stop_ack == {'result': 'ok'}
        wait(lambda: old_actor.poll() is not None, 5)
        old_actor.wait(timeout=.25)
        assert old_actor.returncode == 0
        assert select.select([old_actor_fd], [], [], 0)[0]
        assert not Path(f'/proc/{old_actor.pid}').exists()
        live_policy = json.loads((state / 'policy/owner.live').read_text())
        clean_policy = json.loads((state / 'policy/owner.clean').read_text())
        assert live_policy['epoch'] == clean_policy['epoch']
        assert clean_policy['hash'] == sha(state / 'policy/policy.json')
        policy_before = {name: sha(state / 'policy' / name) for name in ['policy.json', 'owner.live', 'owner.clean']}
        for pid, row in owned.items():
            if row['actor_pid'] == old_actor.pid:
                assert select.select([row['fd']], [], [], 0)[0]
                assert not Path(f'/proc/{pid}').exists()
        finish(old_source, 'hyprland.log')
        assert select.select([old_source_fd], [], [], 0)[0]
        assert not Path(f'/proc/{old_source.pid}').exists()
        if receiver is not None:
            receiver.close()
        os.close(old_actor_fd)
        os.close(old_source_fd)
        actor_fd = source_fd = None
        receive, send = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        receive.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        receiver = receive
        incarnation = os.urandom(16).hex()
        env.update(POKELINUX_SESSION_FD=str(send.fileno()), POKELINUX_SESSION_SOURCE=incarnation,
                   POKELINUX_DISPLAY_PROFILE_RECEIPT='/run/user/1000/pixel10-profile-epoch2/applied.json')
        for name in ['WAYLAND_DISPLAY', 'HYPRLAND_INSTANCE_SIGNATURE']:
            env.pop(name, None)
        compositor = start(['/usr/bin/Hyprland', '--config', str(B / 'hypr.lua')], 'hyprland-epoch2.log', env, user=True, fds=(send.fileno(),))
        source_fd = os.pidfd_open(compositor.pid, os.O_NONBLOCK)
        send.close()
        def new_control():
            for path in Path('/run/user/1000/hypr').glob('*/.socket.sock'):
                try:
                    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
                        sock.connect(str(path))
                        if struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == (compositor.pid, 1000, 1000):
                            return path
                except (FileNotFoundError, ConnectionRefusedError):
                    pass
        control = wait(new_control, 120)
        wayland = wait(lambda: next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()), None), 120)
        env.update(WAYLAND_DISPLAY=wayland.name, HYPRLAND_INSTANCE_SIGNATURE=control.parent.name)
        assert ctl('plugin load ' + str(binaries / 'probe.so')).strip() == 'ok'
        profile_qualification = realm.apply_display(R / 'source', binaries, command, env, result, suffix='-epoch2')
        args[7:13] = [str(receive.fileno()), str(source_fd), str(compositor.pid), '1000', incarnation, str(wayland)]
        actor_log, actor_trace = O / 'actor-epoch2.log', O / 'actor-wire-epoch2.trace'
        actor_parent = start(['strace', '-qq', '-f', '--seccomp-bpf', '-ttt', '-s', '8192', '-e', 'trace=recvmsg,sendto',
                              '-o', str(actor_trace), *args], actor_log.name,
                             dict(os.environ, NATIVE_MESSAGES_ISOLATED='1'), fds=(receive.fileno(), source_fd))
        actor = NativeActor(actor_parent, wait(native_actor_pid))
        actor_fd = os.pidfd_open(actor.pid, os.O_NONBLOCK)
        receive.close()
        receiver = None
        self.actor, self.source_fd = actor, source_fd
        wait(lambda: admin({'op': 'snapshot'}).get('ready'))
        wait(lambda: admin({'op': 'messages_status'}).get('available'))
        assert sha(state / 'policy/policy.json') == policy_before['policy.json']
        launch = launch_rpc()
        window = wait(lambda: next((row for row in json.loads(ctl('j/clients')) if row.get('title') == 'Messages' and row.get('mapped')), None), 90)
        app_pid = window['pid']
        track()
        assert app_pid != self.app_pid and live(app_pid)
        assert int(Path(f'/proc/{app_pid}/stat').read_text().split()[3]) == actor.pid
        argv = [part.decode() for part in Path(f'/proc/{app_pid}/cmdline').read_bytes().split(b'\0')[:-1]]
        assert len(argv) == 7 and argv[0] == str(install / 'pokelinux-messages')
        assert argv[1:3] == profile_args[:2] and argv[4:] == profile_args[3:]
        listener = Path(argv[3])
        context = wait(lambda: next((row for row in reversed(source_events()) if row['event'] == 'app_context' and row['listener'] == str(listener)), None), 2)
        self.app_pid, self.listener, self.sdk_pid = app_pid, listener, context['worker_pid']
        self.writer_pid = next(pid for pid, row in owned.items() if row['role'] == 'messages_writer' and row['actor_pid'] == actor.pid)
        time.sleep(1)
        assert ctl('dispatch hl.dsp.focus({ window = ' + json.dumps('address:' + window['address']) + ' })').strip() == 'ok'
        assert json.loads(ctl('nativebuildprobe'))['pid'] == app_pid
        assert ctl('plugin load ' + str(binaries / 'adapter.so')).strip() == 'ok'
        wait(lambda: next((row for row in reversed(source_events()) if row['event'] == 'session' and (row.get('focus') or {}).get('pid') == app_pid), None), 3)
        witness = {'old_actor_pid': old_actor.pid, 'old_actor_actual_wait_returncode': old_actor.returncode,
            'stop_started_ns': stop_started, 'stop_ack': stop_ack, 'old_source_pid': old_source.pid,
            'old_source_actual_wait_returncode': old_source.returncode, 'new_actor_pid': actor.pid,
            'new_source_pid': compositor.pid, 'new_source_incarnation': incarnation,
            'same_root_policy_directory': str(state / 'policy'), 'policy_sha256_unchanged': policy_before['policy.json'],
            'old_clean_seal': clean_policy, 'old_source_events': old_source_events,
            'fresh_normal_app_pid': app_pid, 'fresh_normal_app_ELF_sha256': sha(install / 'pokelinux-messages'),
            'fresh_launch': launch, 'fresh_window': window, 'fresh_context': context,
            'fresh_Pixel10_profile_qualified': profile_qualification, 'changed_display_epoch': True,
            'same_Source_continuation_qualified': False, 'Source_sequence_checks_relaxed': False,
            'root_session_state_reset': False, 'original_QML_remains_T_held': Path(f'/proc/{shell.pid}/stat').read_text().split()[2] in ('T', 't')}
        assert witness['original_QML_remains_T_held']
        self.write_witness('actual-healthy-actor-and-display-restart.json', witness)
        return witness


try:
    assert os.getuid() == 0 and os.uname().machine == 'aarch64'
    assert os.uname().release == '7.1.6-1-aarch64-ARCH'
    assert sorted(path.name for path in Path('/sys/class/net').iterdir()) == ['lo']
    for name, expected in selection['files'].items():
        assert sha(R / name) == expected, name
    assert sha('/usr/bin/Hyprland') == selection['expected_hyprland_elf_sha256']
    result['kernel'] = {'release': os.uname().release, 'machine': os.uname().machine}
    command(['modprobe', 'virtio_gpu'], 'modprobe.log')
    command(['udevadm', 'settle'], 'udev.log')
    command(['usermod', '-a', '-G', 'video,input', 'alarm'], 'groups.log')
    command(['useradd', '-u', str(UID), '-M', '-s', '/usr/bin/nologin', 'messages-app'], 'app-user.log')
    command(['install', '-d', '-o', 'alarm', '-g', 'alarm', '-m', '700', '/run/user/1000'], 'runtime.log')
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
    for name in ['messages-app-context-fixture', 'lifecycle-launcher', 'pokelinux-messages']:
        shutil.copyfile(binaries / name, install / name)
        (install / name).chmod(0o755)
    appearance = Path('/usr/share/pokelinux/appearance')
    appearance.mkdir(parents=True)
    for name in ['PhoneTheme.qml', 'ThemePalette.js']:
        shutil.copyfile(R / 'source/native-app' / name, appearance / name)
    (B / 'hypr.lua').write_text(
        'hl.monitor({ output = "", mode = "1080x2424@60", position = "auto", scale = 2.625 })\n'
        'hl.permission(' + json.dumps(str(binaries / 'pixel10-raw-output-capture')) + ', "screencopy", "allow")\n'
        'hl.config({ debug = { disable_scale_checks = true }, animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
    receive, send = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    receive.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
    incarnation = os.urandom(16).hex()
    env = {'HOME': '/home/alarm', 'USER': 'alarm', 'LOGNAME': 'alarm', 'LANG': 'C.UTF-8', 'PATH': '/usr/local/bin:/usr/bin',
           'XDG_RUNTIME_DIR': '/run/user/1000', 'LIBSEAT_BACKEND': 'seatd', 'AQ_NO_MODIFIERS': '1',
           'XDG_SESSION_TYPE': 'wayland', 'QT_QPA_PLATFORM': 'wayland',
           'POKELINUX_SESSION_FD': str(send.fileno()), 'POKELINUX_SESSION_SOURCE': incarnation,
           'POKELINUX_DISPLAY_PROFILE': str(R / 'source/pixel10.json'),
           'POKELINUX_DISPLAY_PROFILE_RECEIPT': '/run/user/1000/pixel10-profile/applied.json'}
    compositor = start(['/usr/bin/Hyprland', '--config', str(B / 'hypr.lua')], 'hyprland.log', env, user=True, fds=(send.fileno(),))
    source_fd = os.pidfd_open(compositor.pid, os.O_NONBLOCK)
    send.close()
    receiver = receive
    wayland = wait(lambda: next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()), None), 120)
    control = wait(lambda: next(Path('/run/user/1000/hypr').glob('*/.socket.sock'), None), 120)
    env.update(WAYLAND_DISPLAY=wayland.name, HYPRLAND_INSTANCE_SIGNATURE=control.parent.name)
    assert ctl('plugin load ' + str(binaries / 'probe.so')).strip() == 'ok'
    profile_qualification = realm.apply_display(R / 'source', binaries, command, env, result)
    original, shell = realm.stage_omarchy(R / 'source', command, start, env, result)
    shell_fd = os.pidfd_open(shell.pid, os.O_NONBLOCK)
    scope = Scope()
    scope.original, scope.shell, scope.shell_fd = original, shell, shell_fd
    scope.shell_log, scope.status_sequence, scope.source_started = O / 'original-omarchy-shell.log', 0, False
    scope.negative_sequence = 0
    # The private pinned public archive preserves bytes. Normalize installer-owned
    # modes before selecting the fixed trusted action and command lookup paths.
    for directory, _, files in os.walk(original):
        Path(directory).chmod(Path(directory).stat().st_mode & ~0o022)
        for name in files:
            item = Path(directory) / name
            if not item.is_symlink():
                item.chmod(item.stat().st_mode & ~0o022)
    command(['python3', str(R / 'source/install.py'), '--binary', str(binaries / 'lifecycle-lock')], 'native-lock-install.log')
    result['installed_lock_original_sha256'] = sha('/usr/libexec/pokelinux/omarchy-system-lock-original')
    assert result['installed_lock_original_sha256'] == selection['original_action_sha256']
    result['installed_lock_wrapper_sha256'] = sha(original / 'bin/omarchy-system-lock')
    shutil.copyfile(binaries / 'strace', '/usr/bin/strace')
    Path('/usr/bin/strace').chmod(0o755)
    closure = command(['ldd', '/usr/bin/strace'], 'strace-runtime-closure.log')
    assert 'not found' not in closure
    result['strace_pre_Source_runtime_closure'] = closure
    result['strace_pre_Source_version'] = command(['strace', '--version'], 'strace-version.log')
    scope.profile_qualification, scope.source_fd = profile_qualification, source_fd
    def pre_Source_status():
        value = scope.shell_status(allow_starting=True)
        return value if value and value.get('passwordPam') and value.get('realScreens') == 1 else None
    status = wait(pre_Source_status, 60)
    assert not any(status[key] for key in ['requested', 'pending', 'sessionLocked', 'secure', 'authenticating'])
    result['original_public_pre_Source_lock_status'] = status
    result['original_shell_pidfd'] = {'pid': shell.pid, 'fdinfo': Path(f'/proc/self/fdinfo/{shell_fd}').read_text()}
    profile_args = ['/var/lib/pokelinux-messages-app', '/run/user/11010', str(context_parent / 'required-fresh-app-socket'),
                    '/home/alarm', str(runtime / 'app.sock'), str(appearance)]
    profile = B / 'app-profile.json'
    profile.write_text(json.dumps({'apps': [{'app': APP, 'uid': UID, 'executable': str(install / 'pokelinux-messages'), 'args': profile_args}]}))
    command([str(binaries / 'messages-seed-fixture'), str(state / 'raw'), '--renderer-text'], 'seed.log', dict(os.environ, NATIVE_MESSAGES_ISOLATED='1'))
    args = [str(install / 'messages-app-context-fixture'), str(runtime), str(state / 'policy'), str(profile),
            str(install / 'lifecycle-launcher'), str(state / 'raw'), str(state / 'inbox'), str(receive.fileno()),
            str(source_fd), str(compositor.pid), '1000', incarnation, str(wayland), str(context_parent)]
    actor_parent = start(['strace', '-qq', '-f', '--seccomp-bpf', '-ttt', '-s', '8192', '-e', 'trace=recvmsg,sendto',
                          '-o', str(O / 'actor-wire.trace'), *args], 'actor.log',
                         dict(os.environ, NATIVE_MESSAGES_ISOLATED='1'), fds=(receive.fileno(), source_fd))
    def native_actor_pid():
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
            sock.settimeout(2)
            sock.connect(str(runtime / 'admin.sock'))
            pid, uid, gid = struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            assert uid == gid == 0 and pid != actor_parent.pid
            return pid
    actor = NativeActor(actor_parent, wait(native_actor_pid))
    assert int(Path(f'/proc/{actor.pid}/stat').read_text().split()[3]) == actor_parent.pid
    result['owned_diagnostic_strace_parent'] = {'pid': actor_parent.pid, 'native_actor_pid': actor.pid,
        'seccomp_bpf_filter': 'recvmsg/sendto only', 'native_actor_source_or_configuration_changed': False,
        'native_actor_parent_wait_separate_from_Root_wait_of_tracer': True}
    actor_fd = os.pidfd_open(actor.pid, os.O_NONBLOCK)
    receive.close()
    receiver = None
    scope.actor = actor
    wait(lambda: admin({'op': 'snapshot'}).get('ready'))
    wait(lambda: admin({'op': 'messages_status'}).get('available'))
    assert admin({'op': 'grant', 'app': APP, 'permission': 'messages_read', 'allowed': True}) == {'result': 'ok'}
    launch = launch_rpc()
    window = wait(lambda: next((row for row in json.loads(ctl('j/clients')) if row.get('title') == 'Messages' and row.get('mapped')), None), 90)
    app_pid = window['pid']
    track()
    assert live(app_pid) and int(Path(f'/proc/{app_pid}/stat').read_text().split()[3]) == actor.pid
    argv = [part.decode() for part in Path(f'/proc/{app_pid}/cmdline').read_bytes().split(b'\0')[:-1]]
    assert len(argv) == 7 and argv[0] == str(install / 'pokelinux-messages')
    assert argv[1:3] == profile_args[:2] and argv[4:] == profile_args[3:]
    app_environment_names = {item.split(b'=', 1)[0].decode() for item in Path(f'/proc/{app_pid}/environ').read_bytes().split(b'\0') if item}
    forbidden_scale_names = {'QT_FONT_DPI', 'QT_SCALE_FACTOR', 'QT_SCREEN_SCALE_FACTORS', 'QT_DEVICE_PIXEL_RATIO', 'QT_USE_PHYSICAL_DPI'}
    assert not app_environment_names & forbidden_scale_names
    listener = Path(argv[3])
    assert listener.is_relative_to(context_parent) and listener != Path(profile_args[2])
    context = wait(lambda: next((row for row in reversed(source_events()) if row['event'] == 'app_context' and row['listener'] == str(listener)), None), 2)
    assert context['state'] == 'Ready' and context['admission'] and context['occupied'] and context['metadata_scope'] == 'app_generation'
    sdk_pid = context['worker_pid']
    assert live(sdk_pid)
    scope.app_pid, scope.listener, scope.sdk_pid = app_pid, listener, sdk_pid
    scope.writer_pid = next(pid for pid, row in owned.items() if row['role'] == 'messages_writer')
    time.sleep(1)
    result['all_original_setup_and_normal_render_prepared_before_Source_ns'] = now()
    assert ctl('dispatch hl.dsp.focus({ window = ' + json.dumps('address:' + window['address']) + ' })').strip() == 'ok'
    assert json.loads(ctl('nativebuildprobe'))['pid'] == app_pid
    assert 'TracerPid:\t' + str(actor_parent.pid) in Path(f'/proc/{actor.pid}/status').read_text()
    result['first_Source_adapter_load_invoked_ns'] = now()
    scope.source_started = True
    assert ctl('plugin load ' + str(binaries / 'adapter.so')).strip() == 'ok'
    wait(lambda: not admin({'op': 'snapshot'})['locked'], 3)
    focus_event = wait(lambda: next((row for row in reversed(source_events()) if row['event'] == 'session' and (row.get('focus') or {}).get('pid') == app_pid), None), 3)
    assert focus_event['focus'] == {'pid': app_pid, 'uid': UID, 'gid': UID}
    health = scope.current_app_health()
    result['normal_app'] = {'ELF_sha256': sha(install / 'pokelinux-messages'), 'pid': app_pid,
                            'actual_window': window, 'argv': argv, 'context': context, 'launch': launch,
                            'Qt_window_DPR': 'unobserved in unchanged production ELF',
                            'no_Qt_DPI_or_scale_environment_override': True,
                            'Source_authorization_proof_separate_from_cached_pixels': True}
    result['initial_current_focus_and_independent_grant'] = health
    time.sleep(1)
    scope.current_app_health()
    # A native capture already completed in its own immutable label. That label
    # retains its terminal Source failure. This measurement selects no heavy
    # screencopy while the unchanged Source lease protects the lock observation.
    preview = json.loads((R / 'source/independent-preview-proof.json').read_text())
    assert preview['native_capture_independently_qualified']
    assert preview['status'] == 'terminal_Source_timeout_after_native_capture_before_original_lock_action'
    assert preview['terminal_fixture_result_unchanged']
    assert preview['normal_app_selected']['ELF_sha256'] == selection['normal_qt_elf_sha256']
    assert preview['Pixel10_qualification']['qualified']
    assert preview['actual_native_capture']['sha256'] == selection['independent_preview']['PNG_sha256']
    result['independent_normal_app_preview'] = selection['independent_preview']
    result['native_capture_selected_in_this_lock_only_scope'] = False
    result['setup_complete'] = True
    boundary.run(scope)
    result['passed'] = True
except BaseException as error:
    result['error'] = str(error)
    result['traceback'] = traceback.format_exc()
    print(result['traceback'], flush=True)
finally:
    track()
    if actor is not None and actor.poll() is None:
        try:
            result['whole_Stop_ACK'] = admin({'op': 'stop'})
            assert result['whole_Stop_ACK'] == {'result': 'ok'}
            wait(lambda: actor.poll() is not None, 5)
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(str(error))
    for child, name in reversed(children):
        try:
            finish(child, name)
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(str(error))
    for pid, row in owned.items():
        dead = bool(select.select([row['fd']], [], [], 0)[0])
        absent = not Path(f'/proc/{pid}').exists()
        result['cleanup'].append({'role': row['role'], 'pid': pid, 'pidfd_death_observed': dead, 'proc_absent': absent,
                                  'service_owned_child_absence_qualified': dead and absent,
                                  'independent_parent_wait_syscall_traced': False})
        os.close(row['fd'])
    for fd in [shell_fd, actor_fd, source_fd]:
        if fd is not None:
            if fd == actor_fd:
                dead = bool(select.select([fd], [], [], 0)[0])
                absent = not Path(f'/proc/{actor.pid}').exists()
                result['cleanup'].append({'role': 'native_actor_owned_by_diagnostic_parent', 'pid': actor.pid,
                    'pidfd_death_observed': dead, 'proc_absent': absent,
                    'service_owned_child_absence_qualified': dead and absent,
                    'independent_parent_wait_syscall_traced': False})
            os.close(fd)
    if receiver is not None:
        receiver.close()
    for log in logs:
        log.close()
    result['source_events'] = source_events()
    result['all_tracked_processes_waited_or_finally_absent'] = not result.get('cleanup_errors') and all(
        row.get('direct_parent_wait') or row.get('service_owned_child_absence_qualified') for row in result['cleanup'])
    with (O / 'result.json').open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    shutil.copytree(O, R, dirs_exist_ok=True)
    subprocess.run(['sync'], timeout=10)
    subprocess.run(['systemctl', 'poweroff'], timeout=15)
