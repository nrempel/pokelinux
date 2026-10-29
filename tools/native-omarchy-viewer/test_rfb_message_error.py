"""Actual local socket transport with authored unsupported RFB message bytes."""
import argparse
import importlib.util
import json
import socket

parser = argparse.ArgumentParser()
parser.add_argument('--module', required=True)
args = parser.parse_args()
spec = importlib.util.spec_from_file_location('codec', args.module)
codec = importlib.util.module_from_spec(spec)
spec.loader.exec_module(codec)


class Wire:
    def __init__(self, sock):
        self.sock = sock
        self.received = 0

    def read(self, count):
        data = self.sock.recv(count)
        assert len(data) == count, 'short authored socket read'
        self.received += len(data)
        return data


rows = []
for kind, bells in [(1, 0), (255, 0), (17, 2)]:
    client, peer = socket.socketpair()
    with client, peer:
        client.settimeout(1)
        peer.settimeout(1)
        payload = b'authored-unread-payload'
        peer.sendall(bytes([2]) * bells + bytes([kind]) + payload)
        wire = Wire(client)
        error = None
        try:
            codec.receive_raw_update(wire, 2, 2, 'a' * 64)
        except AssertionError as caught:
            error = str(caught)
        passed = (error == f'unexpected server message: type={kind}'
                  and wire.received == bells + 1
                  and client.recv(len(payload), socket.MSG_PEEK) == payload)
        rows.append({'authored_type': kind, 'preceding_bells': bells,
                     'passed': passed, 'error': error,
                     'received_bytes': wire.received})
result = {'scope': 'Actual socketpair; authored message types; no claim about the failed VM byte',
          'cases': rows, 'passed': all(row['passed'] for row in rows)}
print(json.dumps(result, sort_keys=True))
raise SystemExit(0 if result['passed'] else 1)
