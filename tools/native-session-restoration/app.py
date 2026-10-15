#!/usr/bin/env python3
"""Private headless app/locker/lock-client fixture; no authentication producer."""
import json
import os
from pathlib import Path
import socket
import struct
import sys


def identity():
    status = Path('/proc/self/status').read_text().splitlines()
    return {'pid': os.getpid(), 'uid': os.getuid(), 'gid': os.getgid(),
            'groups': os.getgroups(),
            'cap_eff': next(line.split()[1] for line in status if line.startswith('CapEff:'))}


def drop(uid):
    assert os.getuid() == os.geteuid() == 0
    os.setgroups([])
    os.setgid(uid)
    os.setuid(uid)
    actual = identity()
    assert actual['uid'] == actual['gid'] == uid and actual['groups'] == []
    assert int(actual['cap_eff'], 16) == 0


mode, *args = sys.argv[1:]
if mode == '--locker':
    drop(1000)
    assert sys.stdin.readline() == 'stop\n'
elif mode == '--lock':
    drop(1000)
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as peer:
        peer.settimeout(.5)
        peer.connect(args[0])
        peer.send(b'{"op":"lock"}')
        reply = json.loads(peer.recv(4096))
    print(json.dumps({'identity': identity(), 'reply': reply}), flush=True)
elif mode == '--app':
    actual = identity()
    assert actual['uid'] == actual['gid'] == 11010 and actual['groups'] == []
    assert int(actual['cap_eff'], 16) == 0
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as service:
        service.settimeout(.5)
        service.connect(args[0])
        assert struct.unpack('3i', service.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[1] == 0
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as driver:
            driver.bind(args[1])
            os.chmod(args[1], 0o600)
            driver.listen(4)
            while True:
                connection, _ = driver.accept()
                with connection:
                    connection.settimeout(.5)
                    assert struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[1] == 0
                    packet = connection.recv(4096)
                    if not packet:
                        continue
                    request = json.loads(packet)
                    if request == {'op': 'identity'}:
                        reply = actual
                    elif request == {'op': 'read'}:
                        service.send(b'{"op":"messages_read","request":{"query":{"fixture":"history"},"ttl_ms":500}}')
                        reply = json.loads(service.recv(4096))
                    else:
                        raise ValueError('unexpected private fixture command')
                    connection.send(json.dumps(reply).encode())
else:
    raise ValueError('unknown private fixture role')
