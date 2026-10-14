#!/usr/bin/env python3
"""Owned Linux process/socket evidence; synthetic compositor facts, no hardware."""
import argparse
import ctypes
import array
import fcntl
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
import tarfile
import time
import traceback

SOURCE_UID = 1000
APP_UID = 11010
APP = 'org.omarchy.Messages'
PROTOCOL_BASELINE = '84ac3f856d9b71aeb7bccbfd33eb5a8a828b079f'
SERVICE = 'messages-session-authority-fixture-service'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH',
          'docker', '--host=unix:///var/run/docker.sock']
IMAGE = 'pixel-phone-rust:20261012-b472'
RUNTIME_IMAGE = 'pixel-graphics-build:20261011-b472'
DEPENDENCIES = ('native-lifecycle', 'native-messages', 'native-session',
                'native-kernel-identity', 'native-inbox', 'native-messaging')
Q = {'query': 'conversations', 'after': None, 'limit': 8}


def boot():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def wire(value):
    return json.dumps(value, separators=(',', ':')).encode()


def wait(test, timeout=3):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        try:
            value = test()
            if value:
                return value
        except (FileNotFoundError, ConnectionRefusedError):
            pass
        time.sleep(.002)
    raise AssertionError('bounded fixture wait expired')


def call(path, value, timeout=2):
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as client:
        client.settimeout(timeout)
        client.connect(str(path))
        client.send(wire(value))
        packet = client.recv(65536)
        assert packet, 'real peer closed without reply'
        return json.loads(packet)


def pidfs_supported(fd):
    libc = ctypes.CDLL(None, use_errno=True)
    data = ctypes.create_string_buffer(256)
    assert libc.fstatfs(fd, ctypes.byref(data)) == 0, 'fstatfs failed'
    return struct.unpack('l', data.raw[:struct.calcsize('l')])[0] == 0x50494446


def self_pidfs_supported():
    fd = os.pidfd_open(os.getpid(), os.O_NONBLOCK)
    try:
        return pidfs_supported(fd)
    finally:
        os.close(fd)


def peer_identity(client):
    pid, uid, gid = struct.unpack('3i', client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    raw = struct.unpack('i', client.getsockopt(socket.SOL_SOCKET, 77, 4))[0]  # Linux SO_PEERPIDFD
    try:
        os.set_inheritable(raw, False)
        assert os.readlink(f'/proc/self/fd/{raw}') == 'anon_inode:[pidfd]'
        assert fdinfo_pid(raw) == pid and not select.select([raw], [], [], 0)[0]
        return {'pid': pid, 'uid': uid, 'gid': gid}, raw
    except BaseException:
        os.close(raw)
        raise


def fdinfo_pid(fd):
    values = [line.split('\t', 1)[1] for line in Path(f'/proc/self/fdinfo/{fd}').read_text().splitlines()
              if line.startswith('Pid:\t')]
    assert len(values) == 1
    return int(values[0])


def receive_rights(control):
    packet, ancillary, flags, _ = control.recvmsg(65536, socket.CMSG_SPACE(64 * array.array('i').itemsize),
                                                 socket.MSG_CMSG_CLOEXEC)
    descriptors = []
    try:
        for level, kind, data in ancillary:
            assert level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS, 'unexpected control ancillary'
            assert len(data) % array.array('i').itemsize == 0, 'malformed control rights'
            received = array.array('i')
            received.frombytes(data)
            descriptors.extend(received)
        assert not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC), 'truncated control packet'
        assert all(not os.get_inheritable(fd) for fd in descriptors), 'received control descriptor lacks CLOEXEC'
        return packet, descriptors
    except BaseException:
        for fd in descriptors:
            os.close(fd)
        raise


def app_entry(service, driver):
    assert os.getuid() == APP_UID and os.getgid() == APP_UID
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as fd:
        fd.connect(service)
        fd.setblocking(False)
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as listener:
            listener.bind(driver)
            listener.listen(4)
            while True:
                client, _ = listener.accept()
                with client:
                    _, uid, _ = struct.unpack('3i', client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                    assert uid == 0
                    packet = client.recv(4096)
                    if not packet:
                        continue
                    request = json.loads(packet)
                    operation = request['driver']
                    if operation == 'send':
                        fd.send(wire(request['request']))
                        reply = {'driver_result': 'sent'}
                    elif operation == 'receive':
                        try:
                            packet = fd.recv(65536)
                            reply = json.loads(packet) if packet else {'driver_result': 'eof'}
                        except BlockingIOError:
                            reply = {'driver_result': 'pending'}
                    elif operation == 'exit':
                        client.send(wire({'driver_result': 'exiting'}))
                        return
                    else:
                        raise ValueError('unsupported isolated app command')
                    client.send(wire(reply))


def source_entry(output_raw, control_raw, source_name):
    assert os.getuid() == SOURCE_UID and os.getgid() == SOURCE_UID
    output = socket.socket(fileno=int(output_raw))
    control = socket.socket(fileno=int(control_raw))
    output.setblocking(False)
    sequence = 0
    current = None
    focus_fd = None
    next_heartbeat = 0
    heartbeat = False

    def retain(state, received):
        nonlocal current, focus_fd
        if state.get('focus') is None:
            assert received is None, 'null control focus carried rights'
        else:
            assert received is not None, 'focused control state lacks owned pidfd'
            assert os.readlink(f'/proc/self/fd/{received}') == 'anon_inode:[pidfd]'
        if focus_fd is not None:
            os.close(focus_fd)
        focus_fd = received
        current = state

    def publish(state):
        nonlocal sequence
        sequence += 1
        frame = {'source': source_name, 'sequence': sequence,
                 'sampled_at_ns': state.get('sampled_at_ns', boot()),
                 'locked': state.get('locked', False),
                 'lock_completed': state.get('lock_completed', False),
                 'focus': state.get('focus'), 'locker': {'state': 'unavailable'}}
        rights = [] if focus_fd is None else [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [focus_fd]))]
        assert (frame['focus'] is not None) == (focus_fd is not None)
        assert output.sendmsg([wire(frame)], rights) == len(wire(frame))
        return frame

    def ownership():
        return {'source_owned_focus_handles': int(focus_fd is not None),
                'source_fd_count': len(list(Path('/proc/self/fd').iterdir())),
                'source_focus_cloexec': focus_fd is None or not os.get_inheritable(focus_fd),
                'source_focus_kernel_pid': None if focus_fd is None else fdinfo_pid(focus_fd)}

    try:
        while True:
            delay = max(0, (next_heartbeat - boot()) / 1e9) if heartbeat else 1
            readable, _, _ = select.select([control], [], [], min(delay, 1))
            if readable:
                packet, received = receive_rights(control)
                try:
                    if not packet:
                        assert not received
                        return
                    command = json.loads(packet)
                    operation = command['source_op']
                    states = [command['state']] if operation == 'state' else command['states'] if operation == 'burst' else []
                    assert len(received) == sum(state.get('focus') is not None for state in states), 'control focus/rights count mismatch'
                    if operation in ('state', 'burst'):
                        heartbeat = False
                        frames = []
                        for state in states:
                            descriptor = received[0] if state.get('focus') is not None else None
                            retain(state, descriptor)
                            if descriptor is not None:
                                received.pop(0)
                            frames.append(publish(current))
                        heartbeat = command.get('heartbeat', True)
                        next_heartbeat = boot() + 20_000_000
                        reply = {'sequence': sequence, **ownership()}
                        reply['frame' if operation == 'state' else 'frames'] = frames[0] if operation == 'state' else frames
                        control.send(wire(reply))
                    elif operation == 'idle':
                        heartbeat = False
                        control.send(wire({'sequence': sequence, **ownership()}))
                    elif operation == 'inspect':
                        control.send(wire({'sequence': sequence, **ownership()}))
                    elif operation == 'close':
                        heartbeat = False
                        retain({'focus': None}, None)
                        output.close()
                        control.send(wire({'sequence': sequence, 'closed': True, **ownership()}))
                    elif operation == 'quit':
                        retain({'focus': None}, None)
                        control.send(wire({'sequence': sequence, 'quit': True, **ownership()}))
                        return
                    else:
                        raise ValueError('unsupported isolated source command')
                finally:
                    for fd in received:
                        os.close(fd)
            elif heartbeat:
                publish(current)
                next_heartbeat = boot() + 20_000_000
    except (BrokenPipeError, ConnectionResetError):
        # Owned actor shutdown or a fenced source closes the receiver.
        return
    finally:
        if focus_fd is not None:
            os.close(focus_fd)
        output.close()
        control.close()


class Scope:
    def __init__(self, bins, output, name):
        self.bins, self.output, self.name = bins, output, name
        self.directory = Path('/run/session-authority') / name
        self.state = Path('/var/lib/session-authority') / name
        self.install = Path('/opt/session-authority') / name
        self.source = self.service = self.control = self.alive = None
        self.app = None
        self.app_fd = None
        self.focus_handles = {}
        self.source_name = os.urandom(16).hex()
        self.tracked_children = {}
        self.details = {'name': name, 'observations': [], 'reaps': []}
        self.log = None
        self.stopped = set()

    def setup(self):
        for directory, mode in ((self.directory, 0o755), (self.state, 0o700),
                                (self.install, 0o755), (self.state / 'policy', 0o700),
                                (self.directory / 'app', 0o700)):
            directory.mkdir(parents=True)
            directory.chmod(mode)
        os.chown(self.directory / 'app', APP_UID, APP_UID)
        for binary in (SERVICE, 'lifecycle-launcher'):
            shutil.copyfile(self.bins / binary, self.install / binary)
            (self.install / binary).chmod(0o755)
        shutil.copyfile(Path(__file__), self.install / 'fixture.py')
        (self.install / 'fixture.py').chmod(0o644)
        profile = {'apps': [{'app': APP, 'uid': APP_UID, 'executable': str(Path(sys.executable).resolve()),
                            'args': [str(self.install / 'fixture.py'), '--app',
                                     str(self.directory / 'app.sock'),
                                     str(self.directory / 'app' / 'driver.sock')]}]}
        (self.install / 'profile.json').write_text(json.dumps(profile))
        subprocess.run([str(self.bins / 'messages-seed-fixture'), str(self.state / 'raw')],
                       check=True, timeout=3)
        receive, send = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        receive.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        receive.setblocking(False)
        self.details['socketpair_creation_credentials'] = list(struct.unpack(
            '3i', receive.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)))
        control, child_control = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        self.source = subprocess.Popen([sys.executable, str(self.install / 'fixture.py'),
            '--source', str(send.fileno()), str(child_control.fileno()), self.source_name],
            pass_fds=(send.fileno(), child_control.fileno()), user=SOURCE_UID,
            group=SOURCE_UID, extra_groups=())
        send.close()
        child_control.close()
        self.control = control
        self.control.settimeout(2)
        self.alive = os.pidfd_open(self.source.pid, os.O_NONBLOCK)
        self.details['source_identity'] = {'pid': self.source.pid, 'uid': SOURCE_UID,
            'gid': SOURCE_UID, 'source': self.source_name, 'owned_pidfd': self.pidfs(self.alive), 'pidfs_supported': pidfs_supported(self.alive)}
        self.log = (self.output / (self.name + '.service.log')).open('wb')
        self.service = subprocess.Popen([str(self.install / SERVICE), str(self.directory),
            str(self.state / 'policy'), str(self.install / 'profile.json'),
            str(self.install / 'lifecycle-launcher'), str(self.state / 'raw'),
            str(self.state / 'inbox'), str(receive.fileno()), str(self.alive),
            str(self.source.pid), str(SOURCE_UID), self.source_name],
            pass_fds=(receive.fileno(), self.alive), stdout=self.log, stderr=self.log)
        receive.close()
        wait(lambda: self.admin({'op': 'snapshot'}).get('ready'))
        wait(lambda: self.admin({'op': 'messages_status'}).get('available'))
        self.launch()
        self.track_children()
        return self

    @staticmethod
    def pidfs(fd):
        stat = os.fstat(fd)
        return {'device': stat.st_dev, 'inode': stat.st_ino}

    def admin(self, value):
        return call(self.directory / 'admin.sock', value)

    def driver(self, value):
        return call(self.directory / 'app' / 'driver.sock', value)

    def observe(self, label, value):
        self.details['observations'].append({'label': label, 'boottime_ns': boot(), 'value': value})
        return value

    def launch(self):
        (self.directory / 'app' / 'driver.sock').unlink(missing_ok=True)
        pid = os.fork()
        if pid == 0:
            try:
                os.setgroups([])
                os.setgid(SOURCE_UID)
                os.setuid(SOURCE_UID)
                assert call(self.directory / 'launch-messages.sock', {'op': 'launch_messages'}) == {'result': 'ok'}
                os._exit(0)
            except BaseException:
                os._exit(4)
        actual, status = os.waitpid(pid, 0)
        self.details['reaps'].append({'role': 'desktop_launcher', 'pid': actual, 'wait_status': status})
        assert status == 0, 'UID1000 launch-only ingress failed'
        wait(lambda: (self.directory / 'app' / 'driver.sock').exists())
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as client:
            client.settimeout(2)
            client.connect(str(self.directory / 'app' / 'driver.sock'))
            identity, retained = peer_identity(client)
        assert identity['uid'] == identity['gid'] == APP_UID
        self.app = identity
        self.app_fd = retained
        self.focus_handles[identity['pid']] = retained
        self.track_child(identity['pid'], 'app')
        self.observe('actual_app_socket_peer_pidfd_retained', {**identity, 'kernel_pid': fdinfo_pid(retained),
            'cloexec': not os.get_inheritable(retained), 'pidfd_metadata': self.pidfs(retained),
            'pidfs_supported': pidfs_supported(retained)})

    def grant(self, allowed=True):
        assert self.admin({'op': 'grant', 'app': APP, 'permission': 'messages_read',
                           'allowed': allowed}) == {'result': 'ok'}

    def facts(self, focus='app', locked=False, heartbeat=True, focus_fd=None):
        if focus == 'app':
            focus = self.app
        state = {'locked': locked, 'lock_completed': False, 'focus': focus}
        reply = self.command({'source_op': 'state', 'state': state, 'heartbeat': heartbeat},
                             handles=None if focus_fd is None else [focus_fd])
        self.observe('source_packet', reply)
        # Actor normally polls source once per 20ms turn. A snapshot reply proves
        # a later actor turn, but is not an atomic cross-process publication barrier.
        if locked:
            wait(lambda: self.admin({'op': 'snapshot'}).get('locked'))
        else:
            self.admin({'op': 'snapshot'})
        return reply

    def command(self, value, handles=None):
        states = [value['state']] if value['source_op'] == 'state' else value['states'] if value['source_op'] == 'burst' else []
        if handles is None:
            handles = [self.focus_handles[state['focus']['pid']] if state['focus']['pid'] in self.focus_handles
                       else self.app_fd for state in states if state.get('focus') is not None]
        assert len(handles) == sum(state.get('focus') is not None for state in states)
        ancillary = [] if not handles else [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', handles))]
        assert self.control.sendmsg([wire(value)], ancillary) == len(wire(value))
        return json.loads(self.control.recv(65536))

    def send(self, query=Q, ttl=1000):
        assert self.driver({'driver': 'send', 'request': {'op': 'messages_read',
            'request': {'query': query, 'ttl_ms': ttl}}}) == {'driver_result': 'sent'}

    def receive(self):
        return wait(lambda: (value if (value := self.driver({'driver': 'receive'})).get(
            'driver_result') != 'pending' else None))

    def read(self, query=Q):
        self.send(query)
        return self.receive()

    def authorized(self):
        self.grant()
        self.facts()
        page = self.read()
        assert page.get('page') == 'conversations' and len(page['items']) == 1, (
            'fresh exact native source + separate grant failed to authorize without manual unlock/focus', page)
        self.query = {'query': 'messages', 'conversation': page['items'][0]['id'],
                      'after_ordinal': 0, 'limit': 2}
        return page

    def track_child(self, pid, role):
        if pid not in self.tracked_children:
            fd = os.pidfd_open(pid, os.O_NONBLOCK)
            self.tracked_children[pid] = {'fd': fd, 'role': role}
            self.observe('actual_service_owned_child', {'pid': pid, 'role': role, 'pidfs': self.pidfs(fd), 'pidfs_supported': pidfs_supported(fd)})

    def track_children(self):
        for candidate in Path('/proc').iterdir():
            if not candidate.name.isdecimal():
                continue
            try:
                if int((candidate / 'stat').read_text().split()[3]) != self.service.pid:
                    continue
                arguments = (candidate / 'cmdline').read_bytes().split(b'\0')
                role = 'messages_worker' if b'--messages-worker' in arguments else 'policy_storage_worker' if b'--storage-worker' in arguments else 'app'
                self.track_child(int(candidate.name), role)
            except (FileNotFoundError, ProcessLookupError):
                pass

    def worker(self):
        for candidate in Path('/proc').iterdir():
            if candidate.name.isdecimal():
                try:
                    if int((candidate / 'stat').read_text().split()[3]) != self.service.pid:
                        continue
                    if b'--messages-worker' in (candidate / 'cmdline').read_bytes().split(b'\0'):
                        pid = int(candidate.name)
                        self.track_child(pid, 'messages_worker')
                        return pid
                except (OSError, ValueError):
                    pass
        raise AssertionError('actual owned Messages worker missing')

    def halt(self, pid):
        os.kill(pid, signal.SIGSTOP)
        self.stopped.add(pid)
        stopped = wait(lambda: (state if (state := Path(f'/proc/{pid}/stat').read_text().split()[2]) in ('T', 't') else None))
        self.observe('actual_SIGSTOP_observed_in_proc', {'pid': pid, 'state': stopped})

    def resume(self, pid):
        try:
            os.kill(pid, signal.SIGCONT)
        except ProcessLookupError:
            pass
        self.stopped.discard(pid)
        self.observe('actual_SIGCONT_requested', {'pid': pid})

    def hold_read(self):
        worker = self.worker()
        self.halt(worker)
        self.held_read_started = boot()
        self.send(self.query)
        wait(lambda: self.admin({'op': 'messages_status'}).get('busy'))
        return worker

    def responsiveness(self):
        started = boot()
        reply = self.admin({'op': 'snapshot'})
        elapsed = boot() - started
        assert reply.get('ready') and elapsed < 500_000_000
        self.observe('responsive_actor_snapshot', {'elapsed_ns': elapsed, 'reply': reply})

    def clean_stop(self):
        started = boot()
        assert self.admin({'op': 'stop'}) == {'result': 'ok'}
        self.service.wait(timeout=4)
        elapsed = boot() - started
        assert self.service.returncode == 0 and elapsed < 3_000_000_000
        self.observe('responsive_actor_stop', {'elapsed_ns': elapsed, 'returncode': 0})
        assert not (self.directory / 'messages-storage.sock').exists()
        assert not (self.directory / 'app.sock').exists()

    def cleanup(self):
        errors = []
        for pid in list(self.stopped):
            self.resume(pid)
        if self.service:
            if self.service.poll() is None:
                try:
                    self.admin({'op': 'stop'})
                    self.service.wait(timeout=4)
                except BaseException as error:
                    errors.append('graceful service cleanup: ' + str(error))
                    self.service.kill()
                    self.service.wait(timeout=4)
            self.details['reaps'].append({'role': 'service', 'pid': self.service.pid,
                                         'returncode': self.service.returncode})
        if self.source:
            if self.source.poll() is None:
                try:
                    self.command({'source_op': 'quit'})
                    self.source.wait(timeout=2)
                except BaseException:
                    self.source.kill()
                    self.source.wait(timeout=2)
            self.details['reaps'].append({'role': 'source', 'pid': self.source.pid,
                                         'returncode': self.source.returncode})
        if self.control:
            self.control.close()
        if self.alive is not None:
            os.close(self.alive)
        if self.log:
            self.log.close()
        for fd in self.focus_handles.values():
            os.close(fd)
        self.focus_handles.clear()
        if self.app:
            live = Path('/proc') / str(self.app['pid'])
            if live.exists():
                errors.append('service abandoned its actual launched app')
            self.details['reaps'].append({'role': 'app_service_owned', 'pid': self.app['pid'],
                                         'proc_absent': not live.exists()})
        for pid, child in self.tracked_children.items():
            dead = bool(select.select([child['fd']], [], [], 0)[0])
            absent = not Path(f'/proc/{pid}').exists()
            self.details['reaps'].append({'role': child['role'] + '_service_owned', 'pid': pid, 'owned_pidfd_death_observed': dead, 'proc_absent': absent})
            os.close(child['fd'])
            if not dead or not absent:
                errors.append('service-owned child death/reap not observed: ' + str(pid))
        self.details['cleanup_errors'] = errors
        (self.output / (self.name + '.json')).write_text(json.dumps(self.details, indent=2) + '\n')
        assert not errors, '; '.join(errors)


def unknown_denied(scope):
    assert scope.read() == {'result': 'denied'}


def root_controls_unknown_denied(scope):
    scope.grant()
    assert scope.admin({'op': 'locked', 'locked': False}) == {'result': 'ok'}
    assert scope.admin({'op': 'background', 'app': APP, 'background': False}) == {'result': 'ok'}
    assert scope.read() == {'result': 'denied'}, 'manual root controls substituted for unknown trusted source'


def anon_inode_owned_focus_read(scope):
    assert not pidfs_supported(scope.alive), 'host case requires actual anon_inode pidfds'
    page = scope.authorized()
    assert scope.pidfs(scope.alive) == scope.pidfs(scope.app_fd), 'host pidfds unexpectedly have distinct metadata'
    scope.observe('actual_shared_anon_inode_owned_focus_authorizes', {
        'source_pidfd_metadata': scope.pidfs(scope.alive), 'app_pidfd_metadata': scope.pidfs(scope.app_fd),
        'actual_source_pid': scope.source.pid, 'actual_app_pid': scope.app['pid'],
        'page': page['page'], 'count': len(page['items'])})


def source_only_read(scope):
    scope.grant()
    scope.facts()
    page = scope.read()
    assert page.get('page') == 'conversations' and len(page['items']) == 1, (
        'native source ingress did not authorize without manual root unlock/focus', page)
    scope.observe('authorized_page_kind_and_count', {'page': page['page'], 'count': len(page['items'])})
    scope.grant(False)
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'source facts granted MessagesRead permission'
    scope.observe('focus_facts_cannot_supply_revoked_policy_grant', {'denied': True})


def focus_and_handle_denied(scope):
    scope.authorized()
    for label, focus, handle in [('synthetic_null_or_ineligible_layer_fact', None, None),
                                 ('wrong_launched_pid', {**scope.app, 'pid': scope.source.pid}, scope.app_fd),
                                 ('wrong_native_uid', {**scope.app, 'uid': SOURCE_UID}, scope.app_fd),
                                 ('wrong_native_gid', {**scope.app, 'gid': SOURCE_UID}, scope.app_fd),
                                 ('same_pid_uid_gid_metadata_other_live_handle', scope.app, scope.alive)]:
        scope.facts(focus=focus, focus_fd=handle)
        assert scope.read() == {'result': 'denied'}, label
        scope.observe(label, {'denied': True, 'focus': focus,
                             'actual_handle_pid': None if handle is None else fdinfo_pid(handle)})
        scope.facts()
        assert scope.read().get('page') == 'conversations'


def dead_retained_handle_denied(scope):
    scope.authorized()
    old = dict(scope.app)
    dead = scope.app_fd
    scope.grant(False)
    assert scope.admin({'op': 'terminate', 'app': APP}) == {'result': 'ok'}
    wait(lambda: scope.admin({'op': 'snapshot'}).get('principals') == 0)
    wait(lambda: not Path(f'/proc/{old["pid"]}').exists())
    assert select.select([dead], [], [], 0)[0] and fdinfo_pid(dead) == -1
    scope.observe('old_app_actual_reap_retained_dead_socket_handle', {'pid': old['pid'],
        'proc_absent': True, 'owned_pidfd_death_observed': True, 'kernel_pid': fdinfo_pid(dead)})
    scope.launch()
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'fresh focus supplied a revoked policy grant'
    scope.grant()
    scope.facts()
    assert scope.read().get('page') == 'conversations'
    scope.facts(focus=scope.app, focus_fd=dead)
    assert scope.read() == {'result': 'denied'}, 'dead retained handle with fresh exact metadata authorized'
    scope.observe('fresh_exact_metadata_dead_retained_handle_denied', {'focus': scope.app,
        'handle_kernel_pid': fdinfo_pid(dead), 'source_alive': scope.source.poll() is None})
    scope.facts()
    assert scope.read().get('page') == 'conversations', 'dead focus closed healthy source'
    scope.facts(focus=None)
    original = scope.command({'source_op': 'inspect'})
    for _ in range(24):
        scope.facts()
        scope.facts(focus=None)
    final = scope.command({'source_op': 'inspect'})
    assert original['source_owned_focus_handles'] == final['source_owned_focus_handles'] == 0
    assert original['source_fd_count'] == final['source_fd_count']
    assert final['source_focus_cloexec']
    scope.observe('source_focus_replacement_and_null_close_no_fd_growth', {
        'replacements': 24, 'before': original, 'after': final})


def held_semantic_burst(scope):
    scope.authorized()
    worker = scope.hold_read()
    scope.command({'source_op': 'idle'})
    scope.halt(scope.service.pid)
    burst = scope.command({'source_op': 'burst', 'states': [
        {'locked': True, 'focus': None}, {'locked': False, 'focus': None},
        {'locked': False, 'focus': scope.app}], 'heartbeat': True})
    scope.resume(scope.service.pid)
    scope.responsiveness()
    scope.resume(worker)
    assert scope.receive() == {'result': 'denied'}, 'held read survived lock/unlock/refocus within one drain'
    scope.observe('queued_while_actor_stopped_single_drain_burst', burst)


def heartbeat_preserves_read(scope):
    scope.authorized()
    worker = scope.hold_read()
    original = scope.command({'source_op': 'state', 'state': {'locked': False, 'focus': scope.app}, 'heartbeat': True})['frame']
    scope.resume(worker)
    page = scope.receive()
    assert page.get('page') == 'messages', 'identical heartbeat canceled valid held read'
    assert boot() - scope.held_read_started < 100_000_000, 'positive read did not finish inside source lease'
    scope.observe('heartbeat_held_read_completed_inside_original_lease', {'page': page['page'], 'heartbeat': original, 'request_sent_at_ns': scope.held_read_started, 'observed_at_ns': boot()})


def heartbeat_cannot_extend_read(scope):
    scope.authorized()
    worker = scope.hold_read()
    original = scope.facts()['frame']
    time.sleep(.13)
    scope.responsiveness()
    scope.resume(worker)
    reply = scope.receive()
    assert reply == {'result': 'denied'}, 'new heartbeats extended old current-read source lease'
    scope.observe('heartbeat_did_not_extend_original_read_lease', {
        'first_sample_ns': original['sampled_at_ns'], 'completed_at_ns': boot(), 'reply': reply})


def stopped_source_denies(scope):
    scope.authorized()
    scope.halt(scope.source.pid)
    time.sleep(.12)
    scope.responsiveness()
    assert scope.read() == {'result': 'denied'}
    scope.clean_stop()


def source_eof_denies(scope):
    scope.authorized()
    scope.command({'source_op': 'close'})
    scope.responsiveness()
    assert scope.read() == {'result': 'denied'}
    assert scope.source.poll() is None, 'EOF case accidentally relied on source death'
    scope.observe('actual_source_EOF_while_child_alive', {'source_pid': scope.source.pid, 'child_alive': True})
    scope.clean_stop()


def source_death_denies(scope):
    scope.authorized()
    scope.source.kill()
    scope.source.wait(timeout=2)
    assert scope.source.returncode == -signal.SIGKILL
    scope.responsiveness()
    assert scope.read() == {'result': 'denied'}
    scope.clean_stop()


def eagain_rechecks_source(scope):
    scope.authorized()
    initial = scope.admin({'op': 'messages_status'})['backpressure']
    count = 0
    for _ in range(180):
        scope.send(scope.query)
        count += 1
        status = wait(lambda: (status if not (status := scope.admin({'op': 'messages_status'})).get('busy') else None))
        if status['backpressure'] > initial:
            assert status['pending_publications'] == 1
            blocked_at = boot()
            break
    else:
        raise AssertionError('real Linux seqpacket reply queue never reached EAGAIN')
    changed = scope.facts(focus=None)
    changed_at = changed['frame']['sampled_at_ns']
    assert changed_at - blocked_at < 80_000_000, 'transition arrived only after old stamp expiry'
    packets = [scope.receive() for _ in range(count)]
    assert all(packet.get('page') == 'messages' for packet in packets[:-1]), 'earlier successfully queued pages differ'
    assert packets[-1] == {'result': 'denied'}, 'EAGAIN retry published stale page after source transition'
    scope.observe('actual_reply_EAGAIN_then_source_fence', {'requests': count,
        'already_queued_pages': count - 1, 'backpressure': status['backpressure'],
        'blocked_at_ns': blocked_at, 'transition_at_ns': changed_at,
        'held_reply': packets[-1]})


def source_false_cannot_clear_intent(scope):
    scope.authorized()
    assert scope.admin({'op': 'locked', 'locked': True}) == {'result': 'ok'}
    scope.facts(locked=False)
    assert scope.admin({'op': 'snapshot'})['locked']
    assert scope.read() == {'result': 'denied'}, 'source false cleared outstanding root lock intent'
    assert scope.admin({'op': 'locked', 'locked': False}) == {'result': 'ok'}
    scope.facts()
    assert scope.read().get('page') == 'conversations'
    assert scope.admin({'op': 'background', 'app': APP, 'background': True}) == {'result': 'ok'}
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'source focus cleared outstanding root background intent'
    assert scope.admin({'op': 'background', 'app': APP, 'background': False}) == {'result': 'ok'}
    scope.facts()
    assert scope.read().get('page') == 'conversations'
    scope.observe('explicit_root_fixture_intent_clear_requires_source', {'cleared_by_root_fixture': True,
        'production_PAM_unlock_qualified': False})


def new_app_identity_and_grant(scope):
    scope.authorized()
    old = dict(scope.app)
    scope.grant(False)
    assert scope.admin({'op': 'terminate', 'app': APP}) == {'result': 'ok'}
    wait(lambda: scope.admin({'op': 'snapshot'}).get('principals') == 0)
    wait(lambda: not Path(f'/proc/{old["pid"]}').exists())
    scope.observe('old_app_actually_reaped', {'pid': old['pid']})
    scope.launch()
    assert scope.app['pid'] != old['pid']
    assert select.select([scope.focus_handles[old['pid']]], [], [], 0)[0]
    scope.facts(focus=old)
    assert scope.read() == {'result': 'denied'}, 'old focus identified fresh app generation'
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'fresh focus granted fresh app permission'
    scope.grant()
    scope.facts()
    assert scope.read().get('page') == 'conversations'


CASES = [unknown_denied, root_controls_unknown_denied, source_only_read,
         focus_and_handle_denied, dead_retained_handle_denied, held_semantic_burst, heartbeat_preserves_read,
         heartbeat_cannot_extend_read, stopped_source_denies, source_eof_denies,
         source_death_denies, eagain_rechecks_source, source_false_cannot_clear_intent,
         new_app_identity_and_grant]


def inside(args):
    assert os.getuid() == 0 and os.environ.get('NATIVE_MESSAGES_ISOLATED') == '1'
    bins, output = Path(args.bins), Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    results = []
    choices = CASES + [anon_inode_owned_focus_read]
    selected = [case for case in choices if (not args.case and (case in CASES or not self_pidfs_supported()))
                or (args.case and case.__name__ in args.case)]
    assert selected and (not args.case or len(selected) == len(args.case))
    for case in selected:
        scope = Scope(bins, output, case.__name__)
        entry = {'case': case.__name__, 'passed': False, 'setup_complete': False}
        try:
            scope.setup()
            entry['setup_complete'] = True
            case(scope)
            entry['passed'] = True
            print('PASS ' + case.__name__, flush=True)
        except BaseException as error:
            entry['error'] = str(error)
            entry['traceback'] = traceback.format_exc()
            print('FAIL ' + case.__name__ + ': ' + str(error), flush=True)
        finally:
            try:
                scope.cleanup()
            except BaseException as error:
                entry['passed'] = False
                entry['cleanup_error'] = str(error)
        results.append(entry)
    passed = all(case['passed'] for case in results)
    (output / 'result.json').write_text(json.dumps({'passed': passed, 'cases': results,
        'kernel': {field: getattr(os.uname(), field) for field in ('sysname', 'nodename', 'release', 'version', 'machine')}, 'evidence': 'actual Linux process/credential/pidfd/seqpacket + real SQLite',
        'source': 'synthetic separately launched UID1000 facts source',
        'compositor_original_protocol_qualified': False, 'phone_hardware_qualified': False,
        'pidfs_metadata_distinct': self_pidfs_supported(),
        'owned_focus_protocol_exercised': all(case['setup_complete'] for case in results),
        'positive_authority_qualified': any(case['passed'] and case['case'] in ('source_only_read', 'anon_inode_owned_focus_read') for case in results),
        'suspend_qualified': False, 'zero_external_transition_publication_race_qualified': False}, indent=2) + '\n')
    return 0 if passed else 1


def hashes(directory):
    return {str(path.relative_to(directory)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(directory.rglob('*')) if path.is_file()}


def orchestrate(args):
    repo = Path(__file__).resolve().parents[2]
    label = args.label or args.phase
    assert label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in label)
    root = repo / 'artifacts/native-session-authority'
    root.mkdir(parents=True, exist_ok=True)
    with (root / 'runner.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        output = root / label
        assert not output.exists(), 'preserve existing proof ' + str(output)
        output.mkdir()
        source = output / 'source'
        source.mkdir()
        assert (args.phase == 'red') == bool(args.protocol_baseline), 'red requires explicit --protocol-baseline; green selects current sources'
        origins = {}
        if args.phase == 'red':
            selection = subprocess.check_output(['git', 'rev-parse', args.protocol_baseline], cwd=repo).decode().strip()
            assert selection == PROTOCOL_BASELINE, 'select the documented immutable scalar-protocol baseline'
            assert not args.store_selection, 'protocol baseline freezes all production dependencies'
            archive = output / 'baseline.tar'
            with archive.open('wb') as destination:
                subprocess.run(['git', 'archive', selection, *DEPENDENCIES],
                               cwd=repo, stdout=destination, check=True)
            with tarfile.open(archive) as selected:
                selected.extractall(source, filter='data')
            archive.unlink()
            origins = {name: {'kind': 'immutable_commit', 'commit': selection} for name in DEPENDENCIES}
        else:
            selection = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo).decode().strip()
            for name in DEPENDENCIES:
                selected = repo / name
                if args.store_selection and name not in ('native-lifecycle', 'native-session', 'native-kernel-identity'):
                    selected = Path(args.store_selection) / name
                    assert selected.is_dir(), 'frozen history dependency missing: ' + name
                shutil.copytree(selected, source / name, ignore=shutil.ignore_patterns('target', '__pycache__', '._*'))
                origins[name] = {'kind': 'working_tree' if selected == repo / name else 'explicit_frozen_store',
                                 'path': str(selected)}
        fixture = source / 'tools/native-session-authority'
        shutil.copytree(Path(__file__).parent, fixture, ignore=shutil.ignore_patterns('target', '__pycache__'))
        metadata = {'phase': args.phase, 'head': selection,
            'selected_entry': 'run_with_messages_and_session; real Observer Provider ingress',
            'protocol_baseline': selection if args.phase == 'red' else None,
            'baseline_timing': 'immutable previous production tested after current code was authored' if args.phase == 'red' else None,
            'source_origins': origins,
            'wire_protocol': 'NativeFocus pid/uid/gid plus exactly one actual focused socket-derived SCM_RIGHTS pidfd; null focus has no FD',
            'production_source_overlay': [],
            'target': args.target,
            'frozen_history_store_selection': args.store_selection,
            'features': 'isolated-fixture',
            'sources': hashes(source), 'container_image': IMAGE,
            'runtime_image': RUNTIME_IMAGE,
            'runtime_image_id': subprocess.check_output(DOCKER + ['image', 'inspect', RUNTIME_IMAGE, '--format', '{{.Id}}']).decode().strip(),
            'container_image_id': subprocess.check_output(DOCKER + ['image', 'inspect', IMAGE, '--format', '{{.Id}}']).decode().strip()}
        (output / 'selection.json').write_text(json.dumps(metadata, indent=2) + '\n')
        common = ['--network', 'none', '--cpus', '12', '--memory', '12g', '--pids-limit', '128',
                  '--security-opt', 'no-new-privileges', '--cap-drop', 'ALL',
                  '-v', str(source) + ':/source:ro', '-v', str(output) + ':/results:rw']
        # Every affected crate lock is generated offline in a private writable
        # copy before freezing. Production source bytes are never overlaid.
        manifests = ['tools/native-session-authority/fixture', *DEPENDENCIES]
        temp = output / 'lock-source'
        shutil.copytree(source, temp)
        lock_container = None
        with (output / 'build.log').open('wb') as log:
            lock_container = subprocess.check_output(DOCKER + ['create', '--name', 'pokelinux-session-authority-lock-' + label,
                '--pull=never', '--network', 'none', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
                '--user', f'{os.getuid()}:{os.getgid()}', '-v', str(temp) + ':/source:rw',
                '-v', str(repo / 'sim/rust-cargo') + ':/cargo:rw', '-e', 'CARGO_HOME=/cargo', IMAGE,
                'bash', '-c', 'for manifest in "$@"; do cargo generate-lockfile --offline --manifest-path "$manifest" || exit; done',
                'lock-generation', *['/source/' + name + '/Cargo.toml' for name in manifests]]).decode().strip()
            try:
                generated = subprocess.run(DOCKER + ['start', '-a', lock_container], stdout=log, stderr=subprocess.STDOUT)
                assert generated.returncode == 0, 'offline selected lock generation failed; inspect build.log'
            finally:
                subprocess.run(DOCKER + ['rm', '-f', lock_container], stdout=subprocess.DEVNULL, check=True)
            for name in manifests:
                shutil.copyfile(temp / name / 'Cargo.lock', source / name / 'Cargo.lock')
            if args.phase == 'green':
                shutil.copyfile(fixture / 'fixture/Cargo.lock', Path(__file__).parent / 'fixture/Cargo.lock')
            shutil.rmtree(temp)
            metadata['offline_regenerated_locks'] = [name + '/Cargo.lock' for name in manifests]
            metadata['sources'] = hashes(source)
            (output / 'selection.json').write_text(json.dumps(metadata, indent=2) + '\n')
            feature = metadata['features']
            commands = [
                ['cargo', 'build', '--offline', '--locked', '--manifest-path', '/source/tools/native-session-authority/fixture/Cargo.toml', '--features', feature],
                ['cargo', 'build', '--offline', '--locked', '--manifest-path', '/source/native-lifecycle/Cargo.toml', '--bin', 'lifecycle-launcher'],
                ['cargo', 'build', '--offline', '--locked', '--manifest-path', '/source/native-messages/Cargo.toml', '--features', 'isolated-fixture', '--bin', 'messages-seed-fixture']]
            for index, command in enumerate(commands):
                command.extend(['--target', args.target])
                build_container = subprocess.check_output(DOCKER + ['create', '--name',
                    'pokelinux-session-authority-build-' + label + '-' + str(index), '--pull=never', *common,
                    '--user', f'{os.getuid()}:{os.getgid()}',
                    '-v', str(repo / 'sim/rust-cargo') + ':/cargo:rw', '-e', 'CARGO_HOME=/cargo',
                    '-e', 'CARGO_TARGET_DIR=/results/target', IMAGE, *command]).decode().strip()
                try:
                    result = subprocess.run(DOCKER + ['start', '-a', build_container], stdout=log, stderr=subprocess.STDOUT)
                    assert result.returncode == 0, 'isolated build failed; inspect build.log'
                finally:
                    subprocess.run(DOCKER + ['rm', '-f', build_container], stdout=subprocess.DEVNULL, check=True)
        bins = output / 'bins'
        bins.mkdir()
        for name in (SERVICE, 'lifecycle-launcher', 'messages-seed-fixture'):
            shutil.copyfile(output / 'target' / args.target / 'debug' / name, bins / name)
            (bins / name).chmod(0o755)
        metadata['elf_sha256'] = hashes(bins)
        (output / 'selection.json').write_text(json.dumps(metadata, indent=2) + '\n')
        if args.build_only:
            (output / 'proof-sha256.json').write_text(json.dumps({key: value for key, value in hashes(output).items() if not key.startswith(('source/', 'target/'))}, indent=2) + '\n')
            print('Built frozen ' + args.phase + ' ' + args.target + ' selection at ' + str(output))
            return 0
        assert args.target == 'x86_64-unknown-linux-gnu', 'ARM targets require actual QEMU system guest through vm.py'
        name = 'pokelinux-session-authority-' + label
        arguments = ['python3', '/source/tools/native-session-authority/runner.py', '--inside',
                     '--bins', '/bins', '--output', '/results/execution']
        for case in args.case or []:
            arguments += ['--case', case]
        create = DOCKER + ['create', '--name', name, '--pull=never', *common, '--user', '0:0',
            '--cap-add', 'SETUID', '--cap-add', 'SETGID', '--cap-add', 'CHOWN', '--cap-add', 'KILL',
            '--cap-add', 'DAC_OVERRIDE', '--read-only', '-e', 'NATIVE_MESSAGES_ISOLATED=1',
            '--tmpfs', '/opt:rw,exec,nosuid,nodev', '--tmpfs', '/run:rw,nosuid,nodev',
            '--tmpfs', '/var/lib:rw,nosuid,nodev', '--tmpfs', '/tmp:rw,nosuid,nodev',
            '-v', str(bins) + ':/bins:ro', RUNTIME_IMAGE, *arguments]
        container = subprocess.check_output(create).decode().strip()
        metadata['container_id'] = container
        (output / 'selection.json').write_text(json.dumps(metadata, indent=2) + '\n')
        try:
            with (output / 'execution.log').open('wb') as log:
                result = subprocess.run(DOCKER + ['start', '-a', container], stdout=log, stderr=subprocess.STDOUT)
            (output / 'exit-code').write_text(str(result.returncode) + '\n')
            print((output / 'execution.log').read_text(), end='')
        finally:
            subprocess.run(DOCKER + ['rm', '-f', container], stdout=subprocess.DEVNULL, check=True)
        proof = {key: value for key, value in hashes(output).items()
                 if not key.startswith(('source/', 'target/')) and key != 'proof-sha256.json'}
        (output / 'proof-sha256.json').write_text(json.dumps(proof, indent=2) + '\n')
        details = json.loads((output / 'execution/result.json').read_text())
        if args.phase == 'red':
            cases = details['cases']
            failures = [case['case'] for case in cases if not case['passed']]
            assert all(case['setup_complete'] for case in cases), 'red setup failures are not authority evidence'
            assert all(not case.get('cleanup_error') for case in cases), 'red cleanup not qualified'
            assert next(case['passed'] for case in cases if case['case'] == 'unknown_denied'), 'red default unknown denial failed'
            assert next(case['passed'] for case in cases if case['case'] == 'root_controls_unknown_denied'), 'red unknown source root fencing failed'
            assert 'source_only_read' in failures, 'immutable scalar baseline unexpectedly admitted owned-focus protocol'
            print('Qualified RED: immutable scalar protocol rejects desired owned-focus positive authority: ' + ', '.join(failures))
            return 0
        return result.returncode


def main():
    if '--app' in sys.argv:
        index = sys.argv.index('--app')
        app_entry(*sys.argv[index + 1:])
        return 0
    if '--source' in sys.argv:
        index = sys.argv.index('--source')
        source_entry(*sys.argv[index + 1:])
        return 0
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--phase', choices=('red', 'green'))
    parser.add_argument('--label')
    parser.add_argument('--target', default='x86_64-unknown-linux-gnu', choices=('x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'))
    parser.add_argument('--build-only', action='store_true')
    parser.add_argument('--protocol-baseline', help='explicit immutable scalar-protocol production commit for desired behavioral red')
    parser.add_argument('--store-selection', help='explicit frozen history dependencies; lifecycle/session/kernel identity always use current sources')
    parser.add_argument('--case', action='append')
    parser.add_argument('--inside', action='store_true')
    parser.add_argument('--bins', default='/bins')
    parser.add_argument('--output', default='/results/execution')
    args = parser.parse_args()
    if args.inside:
        return inside(args)
    assert args.phase, '--phase required for isolated build/execution'
    return orchestrate(args)


if __name__ == '__main__':
    raise SystemExit(main())
