"""Real socket transport, authored ServerCutText/frame bytes; no VM payload claim."""
import argparse
import importlib.util
import json
import socket
import struct
import time

parser = argparse.ArgumentParser()
parser.add_argument('--module', required=True)
args = parser.parse_args()
spec = importlib.util.spec_from_file_location('codec', args.module)
codec = importlib.util.module_from_spec(spec)
spec.loader.exec_module(codec)


class Wire:
    def __init__(self, sock):
        self.sock = sock
        self.until = time.monotonic() + 1
        self.received = 0

    def read(self, count):
        data = bytearray()
        while len(data) < count:
            remaining = self.until - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('original socket cutoff')
            self.sock.settimeout(remaining)
            part = self.sock.recv(count - len(data))
            if not part:
                raise EOFError('truncated authored message')
            data.extend(part)
            self.received += len(part)
        return bytes(data)


def cut(text, padding=b'\xa5\x5a\xff'):
    return b'\x03' + padding + struct.pack('>I', len(text)) + text


pixels = bytes([16, 32, 48, 0]) * 4
frame = struct.pack('>BBH', 0, 0, 1) + struct.pack('>HHHHi', 0, 0, 2, 2, 0) + pixels
cases = [
    ('text_then_frame', cut(b'authored-unused-text') + frame, 1, len(b'authored-unused-text'), None),
    ('empty_text_then_frame', cut(b'') + frame, 1, 0, None),
    ('maximum_bounded_text', cut(b'Q' * 4096) * 8 + frame, 8, 32768, None),
    ('oversized_header', b'\x03\0\0\0' + struct.pack('>I', 4097), 0, 0, 'bounded ServerCutText bytes'),
    ('extension_length_refused', b'\x03\0\0\0' + struct.pack('>I', 0xffffffff), 0, 0, 'bounded ServerCutText bytes'),
    ('ninth_message_refused', cut(b'') * 9 + frame, 0, 0, 'bounded ServerCutText messages'),
    ('truncated_header', b'\x03\0\0', 0, 0, 'truncated authored message'),
    ('truncated_payload', cut(b'abc')[:-1], 0, 0, 'truncated authored message'),
]
rows = []
for name, data, messages, text_bytes, expected_error in cases:
    client, peer = socket.socketpair()
    with client, peer:
        peer.settimeout(1)
        peer.sendall(data)
        peer.shutdown(socket.SHUT_WR)
        wire = Wire(client)
        row = {'name': name, 'passed': False}
        try:
            actual, meta = codec.receive_raw_update(wire, 2, 2, 'a' * 64)
            assert expected_error is None and actual == pixels
            assert meta['discarded_server_cut_text_messages'] == messages
            assert meta['discarded_server_cut_text_bytes'] == text_bytes
            assert 'authored-unused-text' not in json.dumps(meta)
            assert wire.received == len(data)
            row['passed'] = True
        except (AssertionError, EOFError, TimeoutError) as error:
            row['error'] = str(error)
            row['passed'] = expected_error is not None and expected_error in str(error)
        row['received_bytes'] = wire.received
        rows.append(row)
result = {'scope': 'Actual socketpair with authored protocol bytes; payload discarded, no clipboard integration',
          'cases': rows, 'passed': all(row['passed'] for row in rows)}
print(json.dumps(result, sort_keys=True))
raise SystemExit(0 if result['passed'] else 1)
