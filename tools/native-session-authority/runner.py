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
BASELINE = 'f733d3b937ab13fa9eb8d81bcf76ab72d0dcf184'
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
        observed = os.fstat(raw)
        owned = os.pidfd_open(pid, os.O_NONBLOCK)
        try:
            exact = os.fstat(owned)
            assert (exact.st_dev, exact.st_ino) == (observed.st_dev, observed.st_ino)
        finally:
            os.close(owned)
        return {'pid': pid, 'uid': uid, 'gid': gid,
                'pidfs_device': observed.st_dev, 'pidfs_inode': observed.st_ino}
    finally:
        os.close(raw)


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
    next_heartbeat = 0
    heartbeat = False

    def publish(state):
        nonlocal sequence
        sequence += 1
        frame = {'source': source_name, 'sequence': sequence,
                 'sampled_at_ns': state.get('sampled_at_ns', boot()),
                 'locked': state.get('locked', False),
                 'lock_completed': state.get('lock_completed', False),
                 'focus': state.get('focus')}
        output.send(wire(frame))
        return frame

    try:
        while True:
            delay = max(0, (next_heartbeat - boot()) / 1e9) if heartbeat else 1
            readable, _, _ = select.select([control], [], [], min(delay, 1))
            if readable:
                packet = control.recv(4096)
                if not packet:
                    return
                command = json.loads(packet)
                operation = command['source_op']
                if operation == 'state':
                    current = command['state']
                    heartbeat = command.get('heartbeat', True)
                    frame = publish(current)
                    next_heartbeat = boot() + 20_000_000
                    control.send(wire({'sequence': sequence, 'frame': frame}))
                elif operation == 'burst':
                    heartbeat = False
                    frames = [publish(state) for state in command['states']]
                    current = command['states'][-1]
                    heartbeat = command.get('heartbeat', True)
                    next_heartbeat = boot() + 20_000_000
                    control.send(wire({'sequence': sequence, 'frames': frames}))
                elif operation == 'idle':
                    heartbeat = False
                    control.send(wire({'sequence': sequence}))
                elif operation == 'close':
                    heartbeat = False
                    output.close()
                    control.send(wire({'sequence': sequence, 'closed': True}))
                elif operation == 'quit':
                    control.send(wire({'sequence': sequence, 'quit': True}))
                    return
                else:
                    raise ValueError('unsupported isolated source command')
            elif heartbeat:
                publish(current)
                next_heartbeat = boot() + 20_000_000
    except (BrokenPipeError, ConnectionResetError):
        # Owned actor shutdown or a fenced source closes the receiver.
        return
    finally:
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
            identity = peer_identity(client)
        assert identity['uid'] == identity['gid'] == APP_UID
        self.app = identity
        self.track_child(identity['pid'], 'app')
        self.observe('actual_app_connection_peerpidfd_matches_independent_owned_pidfd', identity)

    def grant(self, allowed=True):
        assert self.admin({'op': 'grant', 'app': APP, 'permission': 'messages_read',
                           'allowed': allowed}) == {'result': 'ok'}

    def facts(self, focus='app', locked=False, heartbeat=True):
        if focus == 'app':
            focus = self.app
        state = {'locked': locked, 'lock_completed': False, 'focus': focus}
        reply = self.command({'source_op': 'state', 'state': state, 'heartbeat': heartbeat})
        self.observe('source_packet', reply)
        # Actor normally polls source once per 20ms turn. A snapshot reply proves
        # a later actor turn, but is not an atomic cross-process publication barrier.
        if locked:
            wait(lambda: self.admin({'op': 'snapshot'}).get('locked'))
        else:
            self.admin({'op': 'snapshot'})
        return reply

    def command(self, value):
        self.control.send(wire(value))
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
        if self.app:
            live = Path('/proc') / str(self.app['pid'])
            assert not live.exists(), 'service abandoned its actual launched app'
            self.details['reaps'].append({'role': 'app_service_owned', 'pid': self.app['pid'],
                                         'proc_absent': True})
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


def unsupported_pidfs_denies(scope):
    assert not pidfs_supported(scope.alive), 'unsupported-kernel case requires actual anon_inode pidfds'
    scope.grant()
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'unsupported pidfs platform published exact-focus data'
    scope.observe('actual_anon_inode_pidfds_never_grant', {'source_pidfs': scope.pidfs(scope.alive), 'app_identity': scope.app})


def source_only_read(scope):
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'source facts granted MessagesRead permission'
    scope.grant()
    scope.facts()
    page = scope.read()
    assert page.get('page') == 'conversations' and len(page['items']) == 1, (
        'native source ingress did not authorize without manual root unlock/focus', page)
    scope.observe('authorized_page_kind_and_count', {'page': page['page'], 'count': len(page['items'])})


def focus_and_pidfs_denied(scope):
    scope.authorized()
    source_pidfs = scope.pidfs(scope.alive)
    assert pidfs_supported(scope.alive), 'this kernel exposes anon_inode pidfds; exact pidfs boundary is unsupported'
    assert source_pidfs['inode'] != scope.app['pidfs_inode'], 'distinct processes aliased pidfs inode'
    for label, focus in [('synthetic_null_or_ineligible_layer_fact', None),
                         ('wrong_launched_pid', {**scope.app, 'pid': scope.source.pid}),
                         ('wrong_native_uid', {**scope.app, 'uid': SOURCE_UID}),
                         ('wrong_native_gid', {**scope.app, 'gid': SOURCE_UID}),
                         ('same_pid_metadata_other_actual_pidfs', {
                             **scope.app, 'pidfs_device': source_pidfs['device'],
                             'pidfs_inode': source_pidfs['inode']})]:
        scope.facts(focus=focus)
        assert scope.read() == {'result': 'denied'}, label
        scope.observe(label, {'denied': True, 'focus': focus})
        scope.facts()
        assert scope.read().get('page') == 'conversations'


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
    assert scope.app['pidfs_inode'] != old['pidfs_inode']
    scope.facts(focus=old)
    assert scope.read() == {'result': 'denied'}, 'old focus identified fresh app generation'
    scope.facts()
    assert scope.read() == {'result': 'denied'}, 'fresh focus granted fresh app permission'
    scope.grant()
    scope.facts()
    assert scope.read().get('page') == 'conversations'


CASES = [unknown_denied, root_controls_unknown_denied, source_only_read,
         focus_and_pidfs_denied, held_semantic_burst, heartbeat_preserves_read,
         heartbeat_cannot_extend_read, stopped_source_denies, source_eof_denies,
         source_death_denies, eagain_rechecks_source, source_false_cannot_clear_intent,
         new_app_identity_and_grant]


def inside(args):
    assert os.getuid() == 0 and os.environ.get('NATIVE_MESSAGES_ISOLATED') == '1'
    bins, output = Path(args.bins), Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    results = []
    choices = CASES + [unsupported_pidfs_denies]
    selected = [case for case in choices if (not args.case and case in CASES) or (args.case and case.__name__ in args.case)]
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
        'pidfs_identity_qualified': all(case['setup_complete'] for case in results) and self_pidfs_supported(),
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
        if args.phase == 'red':
            selection = subprocess.check_output(['git', 'rev-parse', BASELINE], cwd=repo).decode().strip()
            archive = output / 'baseline.tar'
            with archive.open('wb') as destination:
                subprocess.run(['git', 'archive', selection, *[name for name in DEPENDENCIES if name != 'native-session']],
                               cwd=repo, stdout=destination, check=True)
            with tarfile.open(archive) as selected:
                selected.extractall(source, filter='data')
            archive.unlink()
            shutil.copytree(repo / 'native-session', source / 'native-session', ignore=shutil.ignore_patterns('target', '__pycache__'))
            # Declarations only let the current Observer compile against frozen
            # f733. Old authority/service behavior remains byte-identical.
            shared = repo / 'native-lifecycle/src/session.rs'
            if shared.exists():
                shutil.copyfile(shared, source / 'native-lifecycle/src/session.rs')
                lib = source / 'native-lifecycle/src/lib.rs'
                lib.write_text(lib.read_text().replace('pub mod service;', 'pub mod service;\npub mod session;'))
        else:
            selection = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo).decode().strip()
            for name in DEPENDENCIES:
                selected = repo / name
                if args.store_selection and name not in ('native-lifecycle', 'native-session'):
                    selected = Path(args.store_selection) / name
                    assert selected.is_dir(), 'frozen history dependency missing: ' + name
                shutil.copytree(selected, source / name, ignore=shutil.ignore_patterns('target', '__pycache__', '._*'))
        fixture = source / 'tools/native-session-authority'
        shutil.copytree(Path(__file__).parent, fixture, ignore=shutil.ignore_patterns('target', '__pycache__'))
        metadata = {'phase': args.phase, 'head': selection,
            'selected_entry': 'f733 original run_with_messages; real Observer constructed but no source ingress' if args.phase == 'red'
                              else 'run_with_messages_and_session; real Observer Provider ingress',
            'red_declaration_only_overlay': ['native-lifecycle/src/session.rs', 'native-lifecycle/src/lib.rs: pub mod session'] if args.phase == 'red' else [],
            'target': args.target,
            'frozen_history_store_selection': args.store_selection,
            'features': 'isolated-fixture,baseline-ignored' if args.phase == 'red' else 'isolated-fixture',
            'sources': hashes(source), 'container_image': IMAGE,
            'runtime_image': RUNTIME_IMAGE,
            'runtime_image_id': subprocess.check_output(DOCKER + ['image', 'inspect', RUNTIME_IMAGE, '--format', '{{.Id}}']).decode().strip(),
            'container_image_id': subprocess.check_output(DOCKER + ['image', 'inspect', IMAGE, '--format', '{{.Id}}']).decode().strip()}
        (output / 'selection.json').write_text(json.dumps(metadata, indent=2) + '\n')
        common = ['--network', 'none', '--cpus', '2', '--memory', '3g', '--pids-limit', '128',
                  '--security-opt', 'no-new-privileges', '--cap-drop', 'ALL',
                  '-v', str(source) + ':/source:ro', '-v', str(output) + ':/results:rw']
        build = 'cargo generate-lockfile --offline --manifest-path /source/tools/native-session-authority/fixture/Cargo.toml'
        # Generate the fixture lockfile once in a writable private copy, then copy
        # it back into the selected source and require --locked for every build.
        temp = output / 'lock-source'
        shutil.copytree(source, temp)
        with (output / 'build.log').open('wb') as log:
            generated = subprocess.run(DOCKER + ['run', '--rm', '--pull=never',
                '--network', 'none', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
                '--user', f'{os.getuid()}:{os.getgid()}',
                '-v', str(temp) + ':/source:rw', '-v', str(repo / 'sim/rust-cargo') + ':/cargo:rw',
                '-e', 'CARGO_HOME=/cargo', IMAGE, 'bash', '-c', build], stdout=log, stderr=subprocess.STDOUT)
            assert generated.returncode == 0, 'offline fixture lock generation failed; inspect build.log'
            shutil.copyfile(temp / 'tools/native-session-authority/fixture/Cargo.lock', fixture / 'fixture/Cargo.lock')
            shutil.copyfile(fixture / 'fixture/Cargo.lock', Path(__file__).parent / 'fixture/Cargo.lock')
            shutil.rmtree(temp)
            metadata['sources'] = hashes(source)
            (output / 'selection.json').write_text(json.dumps(metadata, indent=2) + '\n')
            feature = metadata['features']
            commands = [
                ['cargo', 'build', '--offline', '--locked', '--manifest-path', '/source/tools/native-session-authority/fixture/Cargo.toml', '--features', feature],
                ['cargo', 'build', '--offline', '--locked', '--manifest-path', '/source/native-lifecycle/Cargo.toml', '--bin', 'lifecycle-launcher'],
                ['cargo', 'build', '--offline', '--locked', '--manifest-path', '/source/native-messages/Cargo.toml', '--features', 'isolated-fixture', '--bin', 'messages-seed-fixture']]
            for command in commands:
                command.extend(['--target', args.target])
                result = subprocess.run(DOCKER + ['run', '--rm', '--pull=never', *common,
                    '--user', f'{os.getuid()}:{os.getgid()}',
                    '-v', str(repo / 'sim/rust-cargo') + ':/cargo:rw', '-e', 'CARGO_HOME=/cargo',
                    '-e', 'CARGO_TARGET_DIR=/results/target', IMAGE, *command], stdout=log, stderr=subprocess.STDOUT)
                assert result.returncode == 0, 'isolated build failed; inspect build.log'
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
            failures = [case['case'] for case in details['cases'] if not case['passed']]
            expected = ('source_only_read', 'root_controls_unknown_denied')
            assert all(case['setup_complete'] for case in details['cases']), 'red setup failures are not authority evidence'
            assert next(case['passed'] for case in details['cases'] if case['case'] == 'unknown_denied'), 'red default unknown denial control failed'
            assert all(case in failures for case in expected), 'red missed actual missing-ingress/control-bypass failures'
            assert all(not case.get('cleanup_error') for case in details['cases']), 'red cleanup not qualified'
            print('Qualified RED: frozen old entry exposes ' + ', '.join(failures))
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
    parser.add_argument('--store-selection', help='isolated frozen history dependencies; actor/observer still use current sources')
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
