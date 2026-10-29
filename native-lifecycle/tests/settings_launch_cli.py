"""Actual fixed Settings launcher packets; isolated root fixture only."""
import json
import os
from pathlib import Path
import selectors
import socket
import struct
import subprocess
import tempfile
import time
import unittest

NOW = lambda: time.clock_gettime_ns(time.CLOCK_BOOTTIME)


class Launcher(unittest.TestCase):
    role = "settings"

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix=self.role+"-launch-", dir="/run")
        self.root = Path(self.temp.name)
        self.root.chmod(0o755)
        self.path = self.root / ("launch-"+self.role+".sock")
        self.binary = os.environ["NATIVE_"+self.role.upper()+"_LAUNCH_BIN"]
        self.expected_packet = ('{"op":"launch_'+self.role+'"}').encode()

    def tearDown(self):
        self.temp.cleanup()

    def invoke(self, args, response=None):
        start = NOW()
        until = start + 3_000_000_000
        close = start + 5_000_000_000
        server = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        peer = None
        server.setblocking(False)
        server.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        if response is not None:
            server.bind(str(self.path))
            self.path.chmod(0o666)
            server.listen(1)
        child = subprocess.Popen([self.binary, *args], user=1000, group=1000,
                                 extra_groups=[], stdin=subprocess.DEVNULL,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                 env={})
        packet = None
        try:
            while NOW() < until:
                if response is not None and peer is None:
                    try:
                        peer, _ = server.accept()
                        peer.setblocking(False)
                        peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
                        cred = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                        self.assertEqual(cred, (child.pid, 1000, 1000))
                    except BlockingIOError:
                        pass
                if peer is not None and packet is None:
                    try:
                        data, ancillary, flags, _ = peer.recvmsg(512, socket.CMSG_SPACE(12))
                        self.assertFalse(flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC))
                        self.assertEqual(len(ancillary), 1)
                        self.assertEqual(ancillary[0][:2], (socket.SOL_SOCKET, socket.SCM_CREDENTIALS))
                        self.assertEqual(struct.unpack("3i", ancillary[0][2]), (child.pid, 1000, 1000))
                        packet = data
                        self.assertEqual(data, self.expected_packet)
                        body = json.dumps({"result": response}).encode()
                        self.assertEqual(peer.send(body), len(body))
                    except BlockingIOError:
                        pass
                if child.poll() is not None:
                    break
                time.sleep(0.001)
        finally:
            if child.poll() is None:
                child.kill()
            stdout, stderr = child.communicate(timeout=max(0.001, (close-NOW())/1e9))
            actual = NOW()
            print(json.dumps({"actual_cli_wait": child.returncode, "both_PIPE_EOF": True,
                              "completed_ns": actual, "original_until_ns": until,
                              "original_close_ns": close, "packet": None if packet is None else packet.decode()}), flush=True)
            self.assertLess(actual, until)
            self.assertLess(len(stdout) + len(stderr), 8192)
            if peer is not None:
                peer.close()
            server.close()
        return child.returncode, packet

    def test_actual_fixed_packet_and_ok(self):
        code, packet = self.invoke([str(self.path)], "ok")
        self.assertEqual(packet, self.expected_packet, "fixed CLI must actually send its single request")
        self.assertEqual(code, 0)

    def test_actual_denied_reply_is_failure(self):
        code, packet = self.invoke([str(self.path)], "denied")
        self.assertEqual(packet, self.expected_packet)
        self.assertNotEqual(code, 0)

    def test_missing_relative_extra_and_absent_endpoint_refused(self):
        for args in [[], ["relative.sock"], [str(self.path), "extra"], [str(self.path)]]:
            with self.subTest(args=args):
                code, packet = self.invoke(args)
                self.assertNotEqual(code, 0)
                self.assertIsNone(packet)


if __name__ == "__main__":
    unittest.main(verbosity=2)
