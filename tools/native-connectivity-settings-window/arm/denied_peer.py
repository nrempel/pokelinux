"""One actual UID11050 private-socket denial; fixture only, no controller imports."""
import sys
import time
sys.dont_write_bytecode = True
now = lambda: time.clock_gettime_ns(time.CLOCK_BOOTTIME)
print('DENIAL_STAGE entry ' + str(now()), flush=True)
import ctypes
import errno
import json
import os
import socket

assert len(sys.argv) == 3
endpoint, deadline = sys.argv[1], int(sys.argv[2])
assert now() < deadline and os.getuid() == os.geteuid() == 0
libc = ctypes.CDLL(None, use_errno=True)
if libc.prctl(38, 1, 0, 0, 0) != 0:
    raise OSError(ctypes.get_errno(), 'prctl')
os.setgroups([])
os.setresgid(11050, 11050, 11050)
os.setresuid(11050, 11050, 11050)
assert os.getuid() == os.geteuid() == os.getgid() == os.getegid() == 11050 and os.getgroups() == []
with open('/proc/self/status') as stream:
    cap = {v.split(':')[0]: v.split(':')[1].strip() for v in stream.read().splitlines()
        if v.startswith(('CapInh:', 'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:', 'NoNewPrivs:'))}
assert int(cap['CapEff'], 16) == int(cap['CapPrm'], 16) == 0 and cap['NoNewPrivs'] == '1'
print('DENIAL_STAGE credentials ' + str(now()), flush=True)
denied = None
with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC) as connection:
    try:
        connection.settimeout(max(0, deadline-now()) / 1e9)
        connection.connect(endpoint)
    except OSError as error:
        denied = error.errno
print('DENIAL_STAGE connectreturn ' + str(now()) + ' errno=' + str(denied), flush=True)
assert denied == errno.EACCES and now() < deadline
print(json.dumps({'passed': True, 'actual_uid': os.getuid(), 'actual_gid': os.getgid(),
    'groups': os.getgroups(), 'caps': cap, 'connect_errno': denied,
    'private_access_denied': True, 'application_permission_decision': False,
    'observed_at_ns': now(), 'original_until_ns': deadline}), flush=True)
