#!/usr/bin/env python3
"""Separate actual-credential process: display metadata grants no history access."""
import argparse
import json
import os
import socket
from pathlib import Path


def call(path, request):
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as connection:
        connection.settimeout(2)
        try:
            connection.connect(str(path))
            connection.send(json.dumps(request).encode())
            return {'reply': json.loads(connection.recv(4096))}
        except OSError as error:
            return {'errno': error.errno, 'error': type(error).__name__}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime', required=True, type=Path)
    parser.add_argument('--state', required=True, type=Path)
    args = parser.parse_args()
    result = {'pid': os.getpid(), 'uid': os.getuid(), 'gid': os.getgid()}
    result['app_read'] = call(args.runtime / 'app.sock', {
        'op': 'messages_read', 'request': {'ttl_ms': 100,
        'query': {'query': 'conversations', 'after': None, 'limit': 1}}})
    result['admin_snapshot'] = call(args.runtime / 'admin.sock', {'op': 'snapshot'})
    result['raw_history'] = {}
    for name in ('raw/receipts.sqlite3', 'inbox/inbox.sqlite3'):
        try:
            with (args.state / name).open('rb') as stream:
                data = stream.read(16)
            result['raw_history'][name] = {'readable': True, 'bytes': len(data)}
        except OSError as error:
            result['raw_history'][name] = {'readable': False, 'errno': error.errno}
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
