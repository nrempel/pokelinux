"""Installed systemd service lifecycle with disposable offline credentials.

No Android peer is present. This verifies startup, permissions, local RPC,
journal identity and shutdown, not authenticated Android behavior.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import stat
import subprocess
import uuid


def exercise(user, run, wait, compositor):
    enrollment = Path('/home/alarm/.config/omarchy-phone/enrollment')
    runtime = Path('/run/user/1000/omarchy-phone')
    state = Path('/home/alarm/.local/state/omarchy-phone')
    run(['install', '-d', '-o', 'alarm', '-g', 'alarm', '-m', '700', str(enrollment)])
    run(user(['openssl', 'req', '-x509', '-newkey', 'ec', '-pkeyopt',
              'ec_paramgen_curve:P-256', '-noenc', '-days', '1',
              '-subj', '/CN=offline.test', '-addext', 'subjectAltName=DNS:offline.test',
              '-keyout', str(enrollment / 'client.key'),
              '-out', str(enrollment / 'client.pem')]))
    shutil.copyfile(enrollment / 'client.pem', enrollment / 'ca.pem')
    run(user(['openssl', 'x509', '-in', str(enrollment / 'ca.pem'), '-outform', 'DER',
              '-out', str(enrollment / 'ca.der')]))
    pin = hashlib.sha256((enrollment / 'ca.der').read_bytes()).hexdigest()
    (enrollment / 'ca.der').unlink()
    # Reserve a loopback port without listening, so no unrelated process can
    # become the endpoint during this test. There is deliberately no server.
    with socket.socket() as unavailable:
        unavailable.bind(('127.0.0.1', 0))
        endpoint = f'127.0.0.1:{unavailable.getsockname()[1]}'
        (enrollment / 'endpoint.json').write_text(json.dumps({
            'endpoint': endpoint, 'server_name': 'offline.test', 'server_sha256': pin}))
        for path in enrollment.iterdir():
            os.chown(path, 1000, 1000)
            path.chmod(0o600)

        def ctl(*args):
            return run(user(['systemctl', '--user', *args, 'omarchy-phone.service']))

        def hello():
            request = json.dumps({'v': 1, 'id': uuid.uuid4().hex, 'op': 'hello'}) + '\n'
            output = subprocess.check_output(user([
                'omarchy-phone', '--socket', str(runtime / 'bridge.sock'), 'rpc']),
                input=request, text=True, stderr=subprocess.PIPE, timeout=10)
            reply = json.loads(output)['result']
            assert reply['kind'] == 'hello'
            return reply

        def permissions(path, mode):
            info = path.stat()
            assert info.st_uid == 1000 and stat.S_IMODE(info.st_mode) == mode
            return {'uid': info.st_uid, 'mode': oct(stat.S_IMODE(info.st_mode))}

        try:
            ctl('start')
            wait((runtime / 'bridge.sock').is_socket, compositor, 20)
            before = hello()
            modes = {str(p): permissions(p, mode) for p, mode in (
                (runtime, 0o700), (runtime / 'bridge.sock', 0o600),
                (state, 0o700), (state / 'journal.sqlite3', 0o600))}
            unit = ctl('show', '-p', 'ActiveState', '-p', 'NoNewPrivileges', '-p', 'MainPID')
            assert 'ActiveState=active' in unit and 'NoNewPrivileges=yes' in unit
            ctl('restart')
            wait((runtime / 'bridge.sock').is_socket, compositor, 20)
            after = hello()
            assert before['epoch'] == after['epoch'], 'Persistent journal epoch changed'
            assert before['session'] != after['session'], 'Restart retained old session authority'
        finally:
            ctl('stop')
        assert not (runtime / 'bridge.sock').exists()
        assert 'ActiveState=inactive' in ctl('show', '-p', 'ActiveState')
    return {'started': True, 'local_rpc': True, 'permissions': modes,
            'unit': unit, 'restart_preserved_epoch': True,
            'restart_changed_session': True, 'stopped_socket_removed': True,
            'android_endpoint_present': False, 'authenticated_android_verified': False}
