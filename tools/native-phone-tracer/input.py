#!/usr/bin/env python3
"""Real keyboard input and unscaled frontend captures of the owned emulator."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import socket
import stat
import struct
import subprocess
import time


def atomic(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    with temporary.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    assert not path.exists(), 'preserve input replies'
    os.rename(temporary, path)


def process(pid):
    path = Path('/proc') / str(pid)
    fields = (path / 'stat').read_text().rsplit(')', 1)[1].split()
    return {'pid': pid, 'ppid': int(fields[1]), 'start_ticks': int(fields[19])}


class Keyboard:
    def __init__(self, root, owned):
        self.root, self.owned, self.sequence = Path(root), owned, 1
        self.socket = self.stream = self.proc_fd = None
        self.identity = None
        self.counter = 0
        self.capture_sequence = 1

    def close(self):
        if self.stream is not None:
            self.stream.close()
        if self.socket is not None:
            self.socket.close()
        if self.proc_fd is not None:
            os.close(self.proc_fd)

    def read(self):
        line = self.stream.readline(65537)
        if not line or len(line) > 65536 or not line.endswith(b'\n'):
            raise ValueError('bounded complete QMP reply required')
        return json.loads(line)

    def command(self, execute, arguments=None):
        assert execute in ['qmp_capabilities', 'send-key', 'screendump']
        assert self.owned()['running'], 'owned container exited before keyboard action'
        self.counter += 1
        packet = {'execute': execute, 'id': self.counter}
        if arguments is not None:
            packet['arguments'] = arguments
        self.socket.sendall((json.dumps(packet) + '\n').encode())
        for _ in range(128):
            reply = self.read()
            if reply.get('id') == self.counter:
                if 'error' in reply or 'return' not in reply:
                    # Preserve the actual owned frontend diagnostic. This is
                    # not a retry or an alternative command path.
                    raise ValueError('QMP rejected ' + execute + ': ' + json.dumps(reply, sort_keys=True))
                return reply
        raise ValueError('bounded QMP response count exceeded')

    def connect(self):
        state = self.owned()
        assert state['running']
        path = self.root / 'qmp.sock'
        assert stat.S_ISSOCK(path.lstat().st_mode)
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(3)
        self.socket.connect(str(path))
        pid, uid, gid = struct.unpack('3i', self.socket.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        assert uid == os.getuid() and gid == os.getgid(), 'QMP peer task ownership differs'
        peer = process(pid)
        argv = (Path('/proc') / str(pid) / 'cmdline').read_bytes().split(b'\0')
        assert Path(os.fsdecode(argv[0])).name == 'qemu-system-aarch64'
        assert ('unix:/results/qmp.sock,server=on,wait=off').encode() in argv
        ancestry, current = [], pid
        for _ in range(32):
            row = process(current)
            ancestry.append(row)
            if current == state['init_pid']:
                break
            current = row['ppid']
            assert current > 1, 'QMP peer is not descended from owned Docker init'
        assert ancestry[-1]['pid'] == state['init_pid']
        self.proc_fd = os.open(str(Path('/proc') / str(pid)), os.O_RDONLY | os.O_DIRECTORY)
        assert process(pid) == peer
        self.identity = {'peer': peer, 'uid': uid, 'gid': gid, 'container_id': state['id'],
                         'owned_container_init_pid': state['init_pid'], 'ancestry': ancestry,
                         'proc_directory_fd_retained': True, 'actual_socket_peer_qualified': True}
        self.stream = self.socket.makefile('rwb', buffering=0)
        greeting = self.read()
        assert 'QMP' in greeting
        self.identity['greeting'] = greeting['QMP']
        self.command('qmp_capabilities')
        atomic(self.root / 'input' / 'ownership.json', self.identity)

    def poll(self):
        path = self.root / 'input' / f'{self.sequence:04}.json'
        if not path.exists():
            return False
        assert self.sequence <= 16, 'bounded input request count'
        assert path.lstat().st_size <= 4096 and stat.S_ISREG(path.lstat().st_mode)
        request = json.loads(path.read_text())
        assert set(request) == {'sequence', 'keys'} and type(request['sequence']) is int
        assert request['sequence'] == self.sequence
        keys = request['keys']
        assert isinstance(keys, list) and 1 <= len(keys) <= 16
        assert all(type(key) is str and key in ['tab', 'ret'] for key in keys), 'only tab/return qcodes'
        record = {'sequence': self.sequence, 'keys': keys, 'ok': False,
                  'started_monotonic_ns': time.monotonic_ns(), 'hold_time_ms': 50, 'gap_ms': 200,
                  'QML_model_injection': False, 'actual_QMP_return': False}
        try:
            if self.socket is None:
                self.connect()
            assert process(self.identity['peer']['pid']) == self.identity['peer']
            replies = []
            for index, key in enumerate(keys):
                if index:
                    time.sleep(.2)
                reply = self.command('send-key', {'keys': [{'type': 'qcode', 'data': key}], 'hold-time': 50})
                replies.append({'qcode': key, 'reply': reply, 'returned_monotonic_ns': time.monotonic_ns()})
            record.update(ok=True, actual_QMP_return=True, replies=replies)
        except Exception as error:
            record['error'] = type(error).__name__ + ': ' + str(error)
            raise
        finally:
            record['completed_monotonic_ns'] = time.monotonic_ns()
            atomic(self.root / 'input' / f'{self.sequence:04}.reply.json', record)
        self.sequence += 1
        return True

    def execute_frontend(self, prefix, record, place_only=False):
        if self.socket is None:
            self.connect()
        peer = self.identity['peer']
        assert process(peer['pid']) == peer
        host_proc = Path('/proc') / str(peer['pid'])
        namespaces = [row.split()[1:] for row in (host_proc / 'status').read_text().splitlines()
                      if row.startswith('NSpid:')]
        assert len(namespaces) == 1 and len(namespaces[0]) == 2
        assert int(namespaces[0][0]) == peer['pid']
        qemu_pid = int(namespaces[0][-1])
        cmdline_hash = hashlib.sha256((host_proc / 'cmdline').read_bytes()).hexdigest()
        docker = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
                  '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH',
                  'docker', '--host=unix:///var/run/docker.sock']
        command = docker + ['exec', '--user', '1000:1000', self.identity['container_id'],
            'python3', '/results/source/frontend-capture.py',
            '--container-id', self.identity['container_id'], '--qemu-pid', str(qemu_pid),
            '--qemu-start-ticks', str(peer['start_ticks']),
            '--qemu-cmdline-sha256', cmdline_hash,
            '--qmp-argument', 'unix:/results/qmp.sock,server=on,wait=off',
            '--output-prefix', '/results/' + prefix.relative_to(self.root).as_posix()]
        if place_only:
            command.append('--place-only')
        record['owned_Docker_exec_command'] = command
        # A timeout propagates to the retained whole-container supervisor;
        # it must close or report unresolved ownership before any successor.
        completed = subprocess.run(command, capture_output=True, timeout=15)
        record.update(direct_owned_Docker_exec_CLI_wait=True, exec_returncode=completed.returncode,
                      exec_stdout=completed.stdout.decode(errors='replace')[:8192],
                      exec_stderr=completed.stderr.decode(errors='replace')[:8192])
        helper_reply = Path(str(prefix) + '.reply.json')
        assert helper_reply.is_file() and helper_reply.stat().st_size <= 64 * 1024
        helper = json.loads(helper_reply.read_text())
        record['frontend'] = helper
        assert completed.returncode == 0 and helper['ok'], 'owned frontend capture helper failed'
        assert not helper['resize_used'] and not helper['crop_used'] and not helper['guest_UI_injection_used']
        assert not helper['Wayland_native_buffer_pixel_equality_qualified']
        assert helper['container_id'] == self.identity['container_id']
        assert helper['qemu']['pid'] == qemu_pid and helper['qemu']['start_ticks'] == peer['start_ticks']
        assert helper['qemu']['cmdline_sha256'] == cmdline_hash
        assert process(peer['pid']) == peer
        helper_pid = helper['helper_process']['pid']
        assert helper_pid > 1 and helper['helper_process']['uid'] == 1000
        helper_proc = Path('/proc') / str(self.identity['owned_container_init_pid']) / 'root/proc' / str(helper_pid)
        assert not helper_proc.exists(), 'exact reported helper PID must be absent after exec completion'
        record['helper_proc_absent_after_owned_exec_completion'] = True
        return helper

    def poll_placement(self):
        directory = self.root / 'placement'
        path = directory / '0001.json'
        if not path.exists() or (directory / '0001.reply.json').exists():
            return False
        assert stat.S_ISREG(path.lstat().st_mode) and path.stat().st_size <= 4096
        request = json.loads(path.read_text())
        assert request == {'op': 'place_owned_native_frontend_at_origin', 'before_Source_adapter_load': True}
        prefix = directory / '0001.frontend'
        for suffix in ['.claim', '.before.json', '.after.json', '.png', '.rgb', '.reply.json']:
            assert not Path(str(prefix) + suffix).exists()
        record = {'ok': False, 'started_monotonic_ns': time.monotonic_ns(),
                  'scope': 'fixed host frontend origin after denied app render, before first native Source adapter load',
                  'resize_or_export_scale_used': False, 'guest_UI_or_model_injection_used': False,
                  'direct_owned_Docker_exec_CLI_wait': False}
        try:
            helper = self.execute_frontend(prefix, record, place_only=True)
            assert helper['host_frontend_origin_placement_used'] and not helper['actual_pixels_captured']
            after = helper['geometry_after_placement']
            assert (after['root_position']['x'], after['root_position']['y']) == (0, 0)
            assert (after['width'], after['height']) == (1080, 2424)
            record['ok'] = True
        except Exception as error:
            record['error'] = type(error).__name__ + ': ' + str(error)
            raise
        finally:
            record['completed_monotonic_ns'] = time.monotonic_ns()
            atomic(directory / '0001.reply.json', record)
        return True

    def poll_capture(self):
        directory = self.root / 'capture'
        path = directory / f'{self.capture_sequence:04}.json'
        if not path.exists():
            return False
        assert self.capture_sequence <= 4
        assert stat.S_ISREG(path.lstat().st_mode) and path.stat().st_size <= 4096
        request = json.loads(path.read_text())
        assert set(request) == {'sequence', 'name'} and type(request['sequence']) is int
        assert request['sequence'] == self.capture_sequence
        assert request['name'] in ['native-phone-tracer-first-message', 'native-phone-tracer-messages']
        prefix = directory / f'{self.capture_sequence:04}.frontend'
        for suffix in ['.claim', '.before.json', '.after.json', '.png', '.rgb', '.reply.json']:
            assert not Path(str(prefix) + suffix).exists()
        record = {'sequence': self.capture_sequence, 'name': request['name'], 'ok': False,
                  'method': 'owned SDL/X11 whole-client-window capture',
                  'started_monotonic_ns': time.monotonic_ns(), 'resize_or_export_scale_used': False,
                  'Wayland_native_buffer_pixel_equality_qualified': False, 'hardware_verified': False,
                  'direct_owned_Docker_exec_CLI_wait': False}
        try:
            helper = self.execute_frontend(prefix, record)
            png, raw = Path(str(prefix) + '.png'), Path(str(prefix) + '.rgb')
            assert stat.S_ISREG(png.lstat().st_mode) and 0 < png.stat().st_size <= 16 * 1024 * 1024
            assert stat.S_ISREG(raw.lstat().st_mode) and raw.stat().st_size == 1080 * 2424 * 3
            spec = importlib.util.spec_from_file_location('independent_capture_decode', self.root / 'source/inspect-png.py')
            decoder = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(decoder)
            data, rgb, nonopaque, _ = decoder.decode(png)
            assert nonopaque == 0 and rgb == raw.read_bytes()
            assert hashlib.sha256(rgb).hexdigest() == helper['rgb_sha256']
            assert hashlib.sha256(data).hexdigest() == helper['png_sha256']
            record.update(ok=True, png=png.name, png_sha256=helper['png_sha256'],
                          decoded_RGB_sha256=helper['rgb_sha256'],
                          actual_native_frontend_pixels={'width': 1080, 'height': 2424},
                          frontend_XImage_PNG_decoded_RGB_equality_qualified=True,
                          literal_text_visual_review_pending=True)
        except Exception as error:
            record['error'] = type(error).__name__ + ': ' + str(error)
            raise
        finally:
            record['completed_monotonic_ns'] = time.monotonic_ns()
            atomic(directory / f'{self.capture_sequence:04}.reply.json', record)
        self.capture_sequence += 1
        return True
