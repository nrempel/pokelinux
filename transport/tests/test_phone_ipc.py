import socket
import sys
import threading
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from phone_ipc import FrameError, MAX_FRAME, receive, send


class LocalProtocolTests(unittest.TestCase):
    def setUp(self):
        self.reader, self.writer = socket.socketpair()
        self.reader.settimeout(.1)
        self.writer.settimeout(1)
        self.addCleanup(self.reader.close)
        self.addCleanup(self.writer.close)

    def test_round_trip_and_connection_contains_one_request(self):
        value = {'v': 1, 'op': 'snapshot', 'id': 'a' * 32}
        send(self.writer, value)
        self.assertEqual(receive(self.reader), value)

    def test_malformed_and_ambiguous_json_rejected(self):
        for data in (b'{"v":1,"v":2}\n', b'{"v":NaN}\n', b'{"v":1e999}\n',
                     b'[]\n', b'{"text":"\xff"}\n'):
            with self.subTest(data=data):
                self.writer.sendall(data)
                with self.assertRaises(FrameError):
                    receive(self.reader)

    def test_oversize_is_rejected_before_unbounded_read(self):
        writer = threading.Thread(target=self.writer.sendall,
                                  args=(b'x' * (MAX_FRAME + 1),))
        writer.start()
        with self.assertRaises(FrameError):
            receive(self.reader)
        writer.join(2)
        self.assertFalse(writer.is_alive())

    def test_partial_eof_is_not_a_command(self):
        self.writer.sendall(b'{"v":1}')
        self.writer.shutdown(socket.SHUT_WR)
        with self.assertRaises(FrameError):
            receive(self.reader)

    def test_read_deadline_is_preserved(self):
        self.writer.sendall(b'{')
        with self.assertRaises(TimeoutError):
            receive(self.reader)

    def test_trickled_bytes_do_not_extend_the_deadline(self):
        def trickle():
            for byte in b'{"v":1}\n':
                self.writer.sendall(bytes([byte]))
                time.sleep(.03)
        writer = threading.Thread(target=trickle)
        writer.start()
        started = time.monotonic()
        with self.assertRaises(TimeoutError):
            receive(self.reader)
        self.assertLess(time.monotonic() - started, .2)
        self.assertEqual(self.reader.gettimeout(), .1)
        writer.join(2)
        self.assertFalse(writer.is_alive())

    def test_two_coalesced_frames_are_not_one_request(self):
        self.writer.sendall(b'{"v":1}\n{"v":1}\n')
        with self.assertRaises(FrameError):
            receive(self.reader)

    def test_invalid_output_never_writes_a_partial_frame(self):
        for value in ({'n': float('nan')}, {'text': 'x' * MAX_FRAME}, []):
            with self.subTest(value_type=type(value).__name__):
                with self.assertRaises(FrameError):
                    send(self.writer, value)
        with self.assertRaises(TimeoutError):
            self.reader.recv(1)


if __name__ == '__main__':
    unittest.main()
