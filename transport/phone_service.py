#!/usr/bin/env python3
"""Background Android observation for the native shell client.

An optional dispatcher admits commands independently of the panel connection.
Unix peer credentials restrict access to the same UID, not to a specific app.
"""
import argparse
import copy
from contextlib import ExitStack
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import socket
import socketserver
import stat
import struct
import threading
import uuid

from omarchy_bridge import Client, validate_controls, validate_dial_status
from phone_dispatch import Dispatcher
from phone_ledger import Ledger
from phone_ipc import receive, send
from phone_model import elapsed_time


class Observation:
    def __init__(self, client_factory, clock=elapsed_time, interval=.5):
        self.client_factory = client_factory
        self.clock = clock
        self.interval = interval
        self.epoch = uuid.uuid4().hex
        self.sequence = 0
        self.lock = threading.Lock()
        self.stopped = threading.Event()
        self.threads = []
        self.barriers = {'controls': 0, 'dial_status': 0}
        self.values = {name: {'value': None, 'started': None, 'valid': False}
                       for name in ('controls', 'dial_status')}

    def invalidate(self, name=None):
        with self.lock:
            for lane in self.values if name is None else (name,):
                self.barriers[lane] += 1
                self.values[lane]['valid'] = False
            self.sequence += 1

    def poll_once(self, name):
        with self.lock:
            barrier = self.barriers[name]
        started = self.clock()
        value = None
        valid = False
        try:
            value = getattr(self.client_factory(), name)()
            validator = validate_controls if name == 'controls' else validate_dial_status
            validator(value, value.get('id'))
            valid = True
        except (OSError, ValueError):
            pass
        with self.lock:
            if barrier != self.barriers[name]:
                return
            previous = self.values[name]
            self.values[name] = {'value': value if valid else previous['value'],
                                 'started': started if valid else previous['started'],
                                 'valid': valid}
            self.sequence += 1

    def snapshot(self):
        with self.lock:
            now = self.clock()
            result = {'epoch': self.epoch, 'sequence': self.sequence}
            for name, value in self.values.items():
                started = value['started']
                result[name] = {'value': copy.deepcopy(value['value']),
                                'fresh': bool(value['valid'] and started is not None and 0 <= now - started <= 3)}
            return result

    def start(self):
        if self.threads:
            raise ValueError('Observation already started')
        def poll(name):
            while not self.stopped.is_set():
                self.poll_once(name)
                self.stopped.wait(self.interval)
        for name in self.values:
            thread = threading.Thread(target=poll, args=(name,), name='phone-' + name)
            self.threads.append(thread)
            thread.start()

    def close(self):
        self.stopped.set()
        for thread in self.threads:
            thread.join()  # Production Client bounds each network exchange.


class _Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = False
    block_on_close = True
    request_queue_size = 8

    def process_request(self, request, address):
        if not self.slots.acquire(blocking=False):
            self.shutdown_request(request)
            return
        try:
            super().process_request(request, address)
        except BaseException:
            self.slots.release()
            raise

    def process_request_thread(self, request, address):
        try:
            super().process_request_thread(request, address)
        finally:
            self.slots.release()


class _Handler(socketserver.BaseRequestHandler):
    def handle(self):
        self.request.settimeout(2)
        try:
            _, uid, _ = struct.unpack('3i', self.request.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            if uid != os.getuid():
                return
            request = receive(self.request)
            request_id = request.get('id')
            if (type(request.get('v')) is not int
                    or request['v'] != 1
                    or not isinstance(request_id, str) or not re.fullmatch('[0-9a-f]{32}', request_id)):
                send(self.request, {'v': 1, 'error': 'invalid_request'})
                return
            operation = request.get('op')
            if operation == 'snapshot' and set(request) == {'v', 'id', 'op'}:
                response = {'v': 1, 'id': request_id, 'snapshot': self.server.observation.snapshot()}
            elif self.server.dispatcher and operation == 'session' and set(request) == {'v','id','op'}:
                response = {'v':1, 'id':request_id, 'session':self.server.dispatcher.begin_session()}
            elif self.server.dispatcher and operation == 'acknowledge':
                try:
                    response = {'v':1, 'id':request_id,
                                'acknowledged':self.server.dispatcher.acknowledge(request)}
                except ValueError:
                    response = {'v':1, 'id':request_id, 'error':'acknowledgment_rejected'}
            elif self.server.dispatcher and operation == 'recovery' and set(request) == {'v','id','op'}:
                ledger = self.server.dispatcher.ledger
                response = {'v': 1, 'id': request_id,
                            'recovery': dict(epoch=ledger.epoch, **ledger.unresolved())}
            elif self.server.dispatcher and operation in ('control', 'dial', 'result'):
                try:
                    if operation in ('control','dial'):
                        result = self.server.dispatcher.execute(request)
                    else:
                        if set(request) != {'v', 'id', 'epoch', 'op'}:
                            raise ValueError('Invalid result envelope')
                        result = self.server.dispatcher.ledger.result(request['epoch'], request_id)
                    response = {'v': 1, 'id': request_id, 'epoch': request['epoch'], 'operation': result}
                except ValueError:
                    response = {'v': 1, 'id': request_id, 'error': 'command_rejected'}
            else:
                response = {'v': 1, 'error': 'invalid_request'}
            send(self.request, response)
        except (OSError, ValueError):
            return


class LocalService:
    def __init__(self, directory, observation, dispatcher=None):
        if not hasattr(socket, 'SO_PEERCRED'):
            raise ValueError('Linux peer credentials required')
        directory = Path(directory)
        directory.mkdir(mode=0o700, parents=True, exist_ok=True)
        info = directory.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError('Private runtime directory required')
        self.path = directory / 'phone.sock'
        self.fd = os.open(directory / 'owner.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        self.server = None
        self.thread = None
        self.observation = observation
        try:
            fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            os.close(self.fd)
            self.fd = None
            raise ValueError('Phone service already running') from error
        try:
            if self.path.exists() or self.path.is_symlink():
                info = self.path.lstat()
                if not stat.S_ISSOCK(info.st_mode) or info.st_uid != os.getuid():
                    raise ValueError('Unexpected runtime socket path')
                self.path.unlink()
            self.server = _Server(str(self.path), _Handler)
            os.chmod(self.path, 0o600)
            self.server.observation = observation
            self.server.dispatcher = dispatcher
            self.server.slots = threading.BoundedSemaphore(8)
            self.socket_inode = self.path.stat().st_ino
        except BaseException:
            if self.server:
                self.server.server_close()
            os.close(self.fd)
            self.fd = None
            raise

    def start(self):
        if self.thread is not None:
            raise ValueError('Service already started')
        self.observation.start()
        self.thread = threading.Thread(target=self.server.serve_forever, kwargs={'poll_interval': .05})
        self.thread.start()

    def close(self):
        if self.fd is None:
            return
        if self.thread:
            self.server.shutdown()
            self.thread.join()
        self.server.server_close()
        self.observation.close()
        if self.path.exists() and self.path.lstat().st_ino == self.socket_inode:
            self.path.unlink()
        os.close(self.fd)
        self.fd = None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', required=True, type=Path)
    parser.add_argument('--runtime', type=Path)
    parser.add_argument('--state', type=Path)
    args = parser.parse_args()
    runtime = args.runtime
    if runtime is None:
        if not os.environ.get('XDG_RUNTIME_DIR'):
            parser.error('XDG_RUNTIME_DIR or --runtime required')
        runtime = Path(os.environ['XDG_RUNTIME_DIR']) / 'omarchy-phone'
    state = args.state
    if state is None:
        base = Path(os.environ.get('XDG_STATE_HOME') or Path.home() / '.local/state')
        if not base.is_absolute():
            parser.error('XDG_STATE_HOME must be absolute')
        state = base / 'omarchy-phone'
    def client():
        if args.config.stat().st_mode & 0o022:
            raise ValueError('Pairing file is writable by others')
        with args.config.open() as source:
            data = source.read(65537)
        if len(data) > 65536:
            raise ValueError('Pairing file too large')
        return Client(json.loads(data))
    stopped = threading.Event()
    signal.signal(signal.SIGTERM, lambda *_: stopped.set())
    signal.signal(signal.SIGINT, lambda *_: stopped.set())
    with ExitStack() as resources:
        ledger = Ledger(state)
        resources.callback(ledger.close)
        observation = Observation(client)
        dispatcher = Dispatcher(observation, ledger, client)
        service = LocalService(runtime, observation, dispatcher)
        # Stop request workers before closing their ledger.
        resources.callback(service.close)
        service.start()
        stopped.wait()


if __name__ == '__main__':
    main()
