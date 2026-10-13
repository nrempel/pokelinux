#!/usr/bin/env python3
"""Own installed UID fixture: retain a real app socket in an unregistered child."""
import json, os, socket, sys, time
from pathlib import Path
assert os.getuid() == 11010
fd = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC)
fd.connect(sys.argv[1]); fd.setblocking(False)
path = Path(sys.argv[2])
listener = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
listener.bind(str(path)); listener.listen(4)
while True:
    client, _ = listener.accept()
    request = json.loads(client.recv(4096))
    operation = request['driver']
    if operation == 'send':
        fd.send(json.dumps(request['request'], separators=(',', ':')).encode())
        response = {'driver_result': 'sent'}
    elif operation == 'receive':
        try:
            packet = fd.recv(4096)
            if not packet: os._exit(0)
            response = json.loads(packet)
        except BlockingIOError:
            response = {'driver_result': 'pending'}
    elif operation == 'exit':
        os._exit(0)
    elif operation == 'fork_holder':
        child = os.fork()
        if child == 0:
            client.close(); listener.close()
            # fork keeps this real socket descriptor; this process is never
            # registered and sends no read request or claimed credentials.
            while True: time.sleep(60)
        response = {'driver_result': 'held', 'pid': child,
                    'socket_inode': os.fstat(fd.fileno()).st_ino}
    else:
        raise RuntimeError('unsupported own driver command')
    client.send(json.dumps(response, separators=(',', ':')).encode()); client.close()
