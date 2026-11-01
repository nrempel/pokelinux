"""Real host socket/peer/input boundary; authored RFB server and pixels, no GUI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import threading
import time
import traceback

p = argparse.ArgumentParser()
p.add_argument('--host', type=Path, required=True)
p.add_argument('--guest', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--abort', action='store_true')
a = p.parse_args()
assert os.getuid() == 1000
assert not a.output.resolve().is_relative_to(a.host.resolve()), 'output must be outside source tree'
a.output.mkdir()
source = a.output / 'source'
shutil.copytree(a.host, source)
shutil.copyfile(a.guest / 'permissions_acceptance_input.py', source / 'permissions_acceptance_input.py')
sys.path.insert(0, str(source))
sys.dont_write_bytecode = True
from permissions_acceptance_input import AcceptanceInput
from viewer_protocol import publish, digest, read
from owned import now

selection = a.output / 'selection.json'
selection.write_text('{"schema":1,"scope":"authored host RFB boundary"}\n')
sha = hashlib.sha256(selection.read_bytes()).hexdigest()
d = a.output / 'viewer-input'
d.mkdir()
until = now() + 20_000_000_000
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(str(a.output / 'viewer.sock'))
server.listen(1)
server.settimeout(15)
events = []
state = {'socket_EOF': False, 'authored_frames_sent': 0}
down_seen = threading.Event()
connection = []
pixel_format = struct.pack('>BBBBHHHBBB3x', 32, 24, 0, 1, 255, 255, 255, 16, 8, 0)

def serve():
    try:
        conn, _ = server.accept()
        connection.append(conn)
        with conn:
            conn.settimeout(15)
            def exact(n):
                data = bytearray()
                while len(data) < n:
                    chunk = conn.recv(n - len(data))
                    assert chunk, 'early peer EOF'
                    data.extend(chunk)
                return bytes(data)
            def frame():
                conn.sendall(struct.pack('>BBH', 0, 0, 1) + struct.pack('>HHHHi', 0, 0, 1080, 2424, 0))
                conn.sendall(bytes(1080 * 2424 * 4))
                state['authored_frames_sent'] += 1
            banner = b'RFB 003.008\n'
            conn.sendall(banner)
            assert exact(12) == banner
            conn.sendall(b'\x01\x01')
            assert exact(1) == b'\x01'
            conn.sendall(bytes(4))
            assert exact(1) == b'\x00'
            name = b'authored-boundary-control'
            conn.sendall(struct.pack('>HH', 1080, 2424) + pixel_format + struct.pack('>I', len(name)) + name)
            assert exact(20) == bytes(4) + pixel_format
            assert exact(8) == struct.pack('>BBHi', 2, 0, 1, 0)
            assert exact(10) == struct.pack('>BBHHHH', 3, 0, 0, 0, 1080, 2424)
            frame()
            while True:
                first = conn.recv(1)
                if not first:
                    state['socket_EOF'] = True
                    conn.shutdown(socket.SHUT_WR)
                    if not a.abort:
                        publish(d / 'retired.json', {'schema': 1, 'selection_sha256': sha,
                            'ending': 'client-disconnected', 'operational_success': True})
                    break
                if first == b'\x04':
                    packet = first + exact(7)
                    _, pressed, padding, key = struct.unpack('>BBHI', packet)
                    assert pressed in (0, 1) and padding == 0
                    events.append(['key', pressed, key])
                    if pressed:
                        down_seen.set()
                elif first == b'\x05':
                    _, mask, x, y = struct.unpack('>BBHH', first + exact(5))
                    events.append(['pointer', mask, x, y])
                elif first == b'\x03':
                    assert not a.abort
                    assert first + exact(9) == struct.pack('>BBHHHH', 3, 0, 0, 0, 1080, 2424)
                    frame()
                else:
                    raise AssertionError('unexpected RFB message')
    except BaseException as error:
        state['server_error'] = repr(error)

t = threading.Thread(target=serve)
t.start()
proc = subprocess.Popen([sys.executable, '-B', str(source / 'permissions_acceptance_viewer.py'), str(a.output)],
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
report = {'passed': False, 'actual_UNIX_socket': True, 'authored_server_and_pixels': True,
          'GUI_or_ARM_executed': False, 'abort_control': a.abort}
def tick():
    assert proc.poll() is None or (proc.returncode == 0 and report.get('phase') == 'retirement'), 'actual peer ended during input handoff'
    assert now() < until

try:
    publish(d / 'human-ready.json', {'schema': 1, 'selection_sha256': sha, 'host_dwell_until_ns': until})
    guest = AcceptanceInput(d, sha, until, until, tick, lambda: None)
    guest.session_until = until
    guest.wait(d / 'acceptance-connected.json', until)
    if a.abort:
        request = {'schema': 1, 'selection_sha256': sha, 'seq': 1, 'action': 'page-down',
                   'point': None, 'guest_until_ns': until}
        publish(d / 'accept-01.request.json', request)
        ready = guest.wait(d / 'accept-01.ready.json', until)
        sampled = now()
        publish(d / 'accept-01.commit.json', {'schema': 1, 'selection_sha256': sha, 'seq': 1,
            'request_sha256': digest(request), 'host_ready_anchor_ns': ready['host_ready_anchor_ns'],
            'guest_sample_ns': sampled, 'guest_until_ns': until, 'remaining_ns': until - sampled})
        assert down_seen.wait(3), 'no actual key-down reached server'
        publish(d / 'acceptance-abort.json', {'schema': 1, 'selection_sha256': sha, 'failed_seq': 1})
        expected = [['key', 1, 0xff56], ['key', 0, 0xff56]]
    else:
        for action in ('page-down', 'down', 'return', 'alt-tab', 'close', 'click'):
            report['phase'] = 'perform:' + action
            guest.perform([540, 646] if action == 'click' else None, until, action)
        report['phase'] = 'retirement'
        guest.finish_acceptance()
        expected = [['key', pressed, key] for key in (0xff56, 0xff54, 0xff0d) for pressed in (1, 0)]
        expected += [['key', pressed, key] for pressed, key in
                     ((1, 0xffe9), (1, 0xff09), (0, 0xff09), (0, 0xffe9),
                      (1, 0xffeb), (1, 0x77), (0, 0x77), (0, 0xffeb))]
        expected += [['pointer', mask, 540, 646] for mask in (0, 1, 0)]
    stdout, stderr = proc.communicate(timeout=3)
    t.join(3)
    assert not t.is_alive() and 'server_error' not in state
    assert proc.returncode == (1 if a.abort else 0)
    assert events == expected, {'actual_events': events, 'expected': expected}
    assert state['socket_EOF'] and state['authored_frames_sent'] == (1 if a.abort else 2)
    peer = json.loads((a.output / 'permissions-viewer-result.json').read_text())
    assert peer['passed'] is (not a.abort)
    if a.abort:
        assert len(peer['actions']) == 1 and peer['actions'][0]['outcome'] == 'uncertain'
        assert not (d / 'accept-01.done.json').exists()
    else:
        assert len(peer['actions']) == 6 and all(x['outcome'] == 'sent_once' for x in peer['actions'])
    report.update(passed=True, peer_actual_wait=proc.returncode, actions=len(peer['actions']))
except BaseException as error:
    report['error'] = repr(error)
    report['traceback'] = traceback.format_exc()
finally:
    if proc.poll() is None:
        proc.terminate()
    try:
        stdout, stderr = proc.communicate(timeout=2)
    except subprocess.TimeoutExpired:
        proc.kill()
        stdout, stderr = proc.communicate(timeout=2)
    for conn in connection:
        try: conn.shutdown(socket.SHUT_RDWR)
        except OSError: pass
    server.close()
    t.join(16)
    assert not t.is_alive(), 'server thread must be joined'
    report.update(peer_actual_wait=proc.returncode, server=state, actual_events=events)
    (a.output / 'peer.stdout').write_bytes(stdout)
    (a.output / 'peer.stderr').write_bytes(stderr)
    (a.output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
raise SystemExit(0 if report['passed'] else 1)
