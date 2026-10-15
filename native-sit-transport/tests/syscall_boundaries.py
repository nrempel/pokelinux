#!/usr/bin/env python3
"""Fixed Linux controls; argv after -- is the exact native or QEMU helper command."""

import os
from pathlib import Path
import select
import selectors
import socket
import subprocess
import sys
import tempfile
import time

REQUEST = bytes.fromhex((Path(__file__).parent / "fixtures/initial-v4.hex").read_text())
RESPONSE = bytes.fromhex((Path(__file__).parent / "fixtures/base-v4.hex").read_text())
assert len(REQUEST) == 983 and len(RESPONSE) == 144


def boot_ns():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def stop(child):
    if child.poll() is None:
        child.kill()
    # Both owned wait and stdout/stderr EOF have a second, finite bound.
    return child.communicate(timeout=2)


def finish(child, expected, acknowledgement=None):
    try:
        stdout, stderr = child.communicate(input=acknowledgement, timeout=7)
    except subprocess.TimeoutExpired:
        stop(child)
        raise AssertionError("owned child wait or stdout/stderr PIPE EOF exceeded 7s")
    assert child.returncode == 0, (child.returncode, stdout, stderr)
    assert stdout == expected, (stdout, stderr)
    assert stderr == b"", stderr
    return child.returncode


def spawn(command, mode, path):
    return subprocess.Popen(
        [*command, mode, str(path)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        close_fds=True,
    )


def read_header(child):
    data = bytearray()
    until = time.monotonic() + 5
    with selectors.DefaultSelector() as ready:
        ready.register(child.stdout, selectors.EVENT_READ)
        while len(data) < 20:
            remaining = until - time.monotonic()
            assert remaining > 0, "helper deadline header exceeded 5s"
            assert ready.select(remaining), "helper deadline header exceeded 5s"
            chunk = os.read(child.stdout.fileno(), 20 - len(data))
            assert chunk, "helper exited before its real-read deadline header"
            data.extend(chunk)
    return int.from_bytes(data[:16], "big"), int.from_bytes(data[16:], "big", signed=True)


def exact_request(peer):
    data = bytearray()
    until = time.monotonic() + 5
    while len(data) < len(REQUEST):
        remaining = until - time.monotonic()
        assert remaining > 0, "actual request read exceeded 5s"
        peer.settimeout(remaining)
        chunk = peer.recv(len(REQUEST) - len(data))
        assert chunk, "transport peer closed before all 983 request bytes"
        data.extend(chunk)
    assert data == REQUEST, "actual request bytes differ"
    peer.settimeout(5)


def blocked_read(child, descriptor):
    # x86_64 read=0 and aarch64 read=63. For QEMU this observes the host syscall.
    fields = Path(f"/proc/{child.pid}/syscall").read_text().split()
    return (
        len(fields) == 9
        and fields[0] in ("0", "63")
        and int(fields[1], 16) == descriptor
        and int(fields[3], 16) == 65_568
    )


def controls(command):
    with tempfile.TemporaryDirectory(prefix="sit-syscall-controls-") as directory:
        root = Path(directory)
        readonly = root / "readonly"
        unchanged = b"readonly-unchanged\n"
        readonly.write_bytes(unchanged)
        finish(spawn(command, "readonly", readonly), b"readonly-terminal-write-unknown\n")
        assert readonly.read_bytes() == unchanged
        print("PASS readonly: EBADF, terminal Write, OutcomeUnknown, redispatch/read denied")

        partial = root / "partial16"
        finish(spawn(command, "partial", partial), b"partial16-terminal-write-unknown\n")
        assert partial.read_bytes() == REQUEST[:16], "real file must contain exactly first 16 bytes"
        print("PASS partial16: exact persisted prefix, terminal Write, OutcomeUnknown, no retry")

        path = root / "late.sock"
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            listener.bind(str(path))
            listener.listen(1)
            listener.settimeout(5)
            child = spawn(command, "late-read", path)
            try:
                peer, _ = listener.accept()
                with peer:
                    peer.settimeout(5)
                    exact_request(peer)
                    deadline, descriptor = read_header(child)
                    assert boot_ns() < deadline, "original BOOTTIME deadline already expired"
                    while not blocked_read(child, descriptor):
                        assert child.poll() is None, "helper exited before actual blocked read"
                        assert boot_ns() < deadline, "actual read was not witnessed before expiry"
                        select.select([], [], [], 0.002)
                    witnessed = boot_ns()
                    assert witnessed < deadline
                    # No response or shutdown is sent before the original deadline.
                    while boot_ns() <= deadline + 50_000_000:
                        assert child.poll() is None, "helper returned before delayed release"
                        select.select([], [], [], 0.002)
                    assert blocked_read(child, descriptor), "read stopped before delayed release"
                    released = boot_ns()
                    assert released > deadline
                    peer.sendall(RESPONSE)
                    # Requires closure while transport is still alive, before helper exit.
                    assert peer.recv(1) == b"", "terminal transport did not close its descriptor"
                    assert child.poll() is None, "EOF came only from helper exit"
                    finish(child, b"late-read-deadline-unknown\n", b"\x7f")
                    print(
                        "PASS late-read: actual blocked read before original BOOTTIME deadline; "
                        f"release {released - deadline}ns after deadline; ErrorDeadline, "
                        "OutcomeUnknown, no chunk, peer EOF, owned wait0 and both PIPE EOF"
                    )
            except BaseException:
                stop(child)
                raise


if __name__ == "__main__":
    assert len(sys.argv) >= 3 and sys.argv[1] == "--", "usage: syscall_boundaries.py -- HELPER [RUNNER-ARGS...]"
    controls(sys.argv[2:])
