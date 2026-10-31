"""SETPCAP-only startup; actual cap0 Main reports first cause after owned closure."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import socket
import stat
import struct
import time

import owned
import closure
from owned import Scope, now
from closure import close_owned

OWNED = '5a738b9529a1a0e5d5b0bd20e27e2ec4244d4e61d7c8e0dad96fe1ffbef3c423'
CLOSURE = '646e9cfd5f755b626fe8babfa26637ddac328ede05cdab75aa77cc51332d70d6'
MARKER = 'MAIN_FIRST_RETIREMENT_MUST_RECORD_STARTUP_FAILURE'
MESSAGE = r'"(?:[^"\\\r\n]|\\.){0,300}"'
EARLY = re.compile(r'^connectivity_worker_failure_v1 Custom \{ kind: PermissionDenied, error: WorkerFailure \{ stage: ReduceCapabilities, original_until_ns: None, detected_at_ns: ([0-9]{1,20}), source: Os \{ code: 1, kind: PermissionDenied, message: ' + MESSAGE + r' \} \} \}\n', re.M)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def process_metadata(pid):
    wanted = ('Pid:', 'PPid:', 'Uid:', 'Gid:', 'Groups:', 'Threads:', 'CapInh:',
              'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:', 'NoNewPrivs:')
    return {line.split(':', 1)[0]: line.split(':', 1)[1].strip()
            for line in Path(f'/proc/{pid}/status').read_text().splitlines()
            if line.startswith(wanted)}


def cap0_root(value):
    return (all(int(value[key], 16) == 0 for key in
                ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb'])
            and value['NoNewPrivs'] == '1' and value['Groups'] == ''
            and value['Uid'].split() == value['Gid'].split() == ['0'] * 4)


def setpcap_only_root(value, allow_reduced=False):
    allowed = (0, 1 << 8) if allow_reduced else (1 << 8,)
    return (all(int(value[key], 16) in allowed for key in ['CapPrm', 'CapEff', 'CapBnd'])
            and all(int(value[key], 16) == 0 for key in ['CapInh', 'CapAmb'])
            and value['NoNewPrivs'] == '1' and value['Groups'] == ''
            and value['Uid'].split() == value['Gid'].split() == ['0'] * 4)


def network_inventory():
    ns = Path('/proc/self/ns/net').stat()
    return {'namespace': {'dev': ns.st_dev, 'ino': ns.st_ino},
            'links': sorted([index, name] for index, name in socket.if_nameindex())}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--main', required=True, type=Path)
    parser.add_argument('--main-sha256', required=True)
    parser.add_argument('--worker', required=True, type=Path)
    parser.add_argument('--worker-sha256', required=True)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--target-name', required=True)
    parser.add_argument('--target-ifindex', required=True, type=int)
    a = parser.parse_args()
    assert a.output.is_absolute() and not a.output.exists()
    a.output.mkdir(mode=0o700)
    scope = Scope(a.output)
    child = namespace = None
    natural = False
    started = now()
    W, C = started + 1_000_000_000, started + 3_000_000_000
    r = {'passed': False, 'behavior_RED_qualified': False, 'observation_qualified': False,
         'classification': 'SETUP_FAILURE_OR_UNCONFIRMED', 'started_at_ns': started,
         'original_work_until_ns': W, 'original_close_until_ns': C,
         'original_work_close_seconds': [1, 3], 'phone_accessed': False,
         'network_mutating_requests_admitted': 0, 'backend_or_interface_qualified': False,
         'selector_scope': 'Authored syntactic absent device; worker must fail before backend access',
         'Main_SIGTERM_commands_admitted': 0, 'readonly_Inspect_requests': 0,
         'capability_policy': {'parent_only_bit': 8, 'NET_ADMIN_bounding_absent': True,
             'Main_observation_requires_cap0': True},
         'stderr_mode': 'merged_stdout_PIPE', 'output_limit_bytes': 16384}

    def pump():
        scope.pump()
        scope.check()
        assert now() < W and scope.live(child), 'actual Main remains live under original work cutoff'

    def inspect(endpoint, request_id):
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_NONBLOCK | socket.SOCK_CLOEXEC) as peer:
            peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            peer.connect(str(endpoint))
            expected = (child['child'].pid, 0, 0)
            assert struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == expected
            body = json.dumps({'op': 'inspect', 'id': request_id, 'until_ns': str(W)}).encode()
            assert peer.send(body) == len(body)
            r['readonly_Inspect_requests'] += 1
            while now() < W:
                pump()
                try:
                    body, ancillary, flags, _ = peer.recvmsg(4096, socket.CMSG_SPACE(12))
                except BlockingIOError:
                    time.sleep(.001)
                    continue
                assert body and not flags and len(ancillary) == 1
                assert ancillary[0][:2] == (socket.SOL_SOCKET, socket.SCM_CREDENTIALS)
                assert struct.unpack('3i', ancillary[0][2]) == expected
                reply = json.loads(body)
                assert reply['schema'] == 1 and reply['id'] == request_id
                return reply
        raise TimeoutError('original readonly Inspect cutoff')

    try:
        assert digest(owned.__file__) == OWNED and digest(closure.__file__) == CLOSURE
        parent = process_metadata(os.getpid())
        assert setpcap_only_root(parent) and parent['Threads'] == '1'
        r['parent_metadata'] = parent
        libc = ctypes.CDLL(None, use_errno=True)
        assert libc.prctl(36, 1, 0, 0, 0) == 0
        subreaper = ctypes.c_int(-1)
        assert libc.prctl(37, ctypes.byref(subreaper), 0, 0, 0) == 0 and subreaper.value == 1
        r['actual_parent_subreaper'] = True
        for role, path, wanted in [('main', a.main, a.main_sha256), ('worker', a.worker, a.worker_sha256)]:
            assert path.is_absolute() and path.is_file() and not path.is_symlink()
            m = path.stat()
            assert stat.S_ISREG(m.st_mode) and m.st_uid == m.st_gid == 0
            assert m.st_mode & 0o111 and not m.st_mode & 0o6022 and m.st_nlink == 1
            assert 'security.capability' not in os.listxattr(path)
            assert re.fullmatch('[0-9a-f]{64}', wanted) and digest(path) == wanted
            header = path.read_bytes()[:20]
            assert header[:6] == b'\x7fELF\x02\x01' and int.from_bytes(header[18:20], 'little') == 62
            r[role] = {'path': str(path), 'bytes': m.st_size, 'sha256': wanted, 'ELF_machine': 62}
        before = network_inventory()
        assert a.target_name == 'pkl-diag-none' and a.target_ifindex == 2147483646
        assert all(index != a.target_ifindex and name != a.target_name for index, name in before['links'])
        r['network_before'] = before
        namespace = Path('/proc/self/ns/net').open('rb')
        n = os.fstat(namespace.fileno())
        assert before['namespace'] == {'dev': n.st_dev, 'ino': n.st_ino}
        profile = {'backend': 'wireless_open', 'target':
            {'namespace': before['namespace'], 'ifindex': a.target_ifindex, 'name': a.target_name},
            'frequency_mhz': 2412, 'bssid': [2, 0, 0, 0, 0, 1], 'ssid': list(b'pkl-diag')}
        profile_path = a.output / 'startup-profile.json'
        with profile_path.open('x') as stream:
            os.fchmod(stream.fileno(), 0o600)
            stream.write(json.dumps(profile) + '\n')
        m = profile_path.stat()
        assert m.st_uid == m.st_gid == 0 and stat.S_IMODE(m.st_mode) == 0o600 and m.st_nlink == 1
        endpoint = a.output / 'control.sock'
        argv = [str(a.main), str(endpoint), str(profile_path), str(a.output / 'admitted'), str(a.worker)]
        r['actual_argv'] = argv
        r['startup_profile'] = profile
        child = scope.start(argv, 'normal-main.log', 16384,
                            env=dict(os.environ, LANG='C', LC_ALL='C'), cutoff=W, wait_cutoff=C)
        metadata = process_metadata(child['child'].pid)
        assert setpcap_only_root(metadata, allow_reduced=True) and int(metadata['PPid']) == os.getpid()
        r['Main_initial_metadata'] = metadata
        while now() < W and not endpoint.is_socket():
            pump()
            time.sleep(.001)
        assert endpoint.is_socket()
        ready = False
        for request_id in range(1, 65):
            pump()
            view = inspect(endpoint, request_id)
            state = view['state']
            text = (a.output / 'normal-main.log').read_text()
            matches = list(EARLY.finditer(text))
            children = Path(f'/proc/{child["child"].pid}/task/{child["child"].pid}/children').read_text().split()
            if (len(matches) == 1 and not children and view['status'] == 'unavailable'
                    and state['available'] is False and state['configured'] is None
                    and state['pending'] is False and state['terminal_uncertain'] is True
                    and state['observed_at_ns'] == state['until_ns'] == '0'):
                detected = int(matches[0].group(1))
                assert child['spawn_started_at_ns'] <= detected <= now() < W
                assert re.fullmatch('[0-9a-f]{32}', state['worker'])
                metadata = process_metadata(child['child'].pid)
                assert cap0_root(metadata)
                r['Main_metadata_at_startup_failure'] = metadata
                r['startup_failure_before_external_Stop'] = {'worker_step': 'ReduceCapabilities',
                    'io_kind': 'PermissionDenied', 'raw_os_error': 1, 'detected_at_ns': detected,
                    'observed_at_ns': now(), 'Main_live': scope.live(child), 'Main_children': [],
                    'normal_readonly_reply': view}
                ready = True
                break
            time.sleep(.002)
        assert ready and scope.live(child), 'actual startup worker failure precedes later external Stop'
        r['external_Stop_sent_at_ns'] = now()
        r['Main_SIGTERM_commands_admitted'] = 1
        signal.pidfd_send_signal(child['pidfd'], signal.SIGTERM)
        r['actual_Main_exit'] = scope.finish(child, C)
        natural = True
        rows = []
        for line in (a.output / 'normal-main.log').read_text().splitlines():
            try:
                value = json.loads(line)
            except json.JSONDecodeError:
                continue
            if type(value) is dict and value.get('event') == 'connectivity_main_closure':
                rows.append(value)
        assert len(rows) == 1, 'one complete natural Main closure JSON line'
        terminal = rows[0]
        assert terminal['closed'] and terminal['reaped'] and terminal['output_eof']
        assert not terminal['occupied'] and terminal['namespace_handles_open']
        assert (terminal['worker_exit_code'], terminal['worker_exit_signal']) in [(1, None), (None, 9)]
        assert r['external_Stop_sent_at_ns'] <= terminal['completed_at_ns'] < child['wait']['waited_at_boottime_ns'] < C
        assert r['actual_Main_exit'] == 0
        r['Main_closure'] = terminal
        r['first_retirement'] = terminal.get('first_retirement')
        r['network_after'] = network_inventory()
        assert r['network_after'] == before
        assert digest(a.main) == a.main_sha256 and digest(a.worker) == a.worker_sha256
        r['actual_normal_Main_observation_qualified'] = True
    except BaseException as error:
        r['setup_or_observation_error'] = type(error).__name__ + ': ' + str(error)
    finally:
        r['closure'] = close_owned(scope, C, reap_adopted=True)
        if namespace is not None:
            namespace.close()
        r['namespace_handle_closed'] = namespace is None or namespace.closed
        r['owned_receipts'] = scope.receipts
        r['all_parent_handles_closed'] = all(value['wait'] is not None and value['eof'] and value['log'].closed
            and value['child'].stdout.closed and value['pidfd'] is None for value in scope.children)
        r['completed_at_ns'] = now()
    closed = (r['closure']['closed'] and r['closure']['wait_all_ECHILD']
        and not r['closure']['adopted_actual_waits'] and not r['closure']['adopted_pidfds_signaled']
        and r['all_parent_handles_closed'] and r['namespace_handle_closed'])
    observation = (natural and r.get('actual_normal_Main_observation_qualified') is True and closed
        and len(scope.receipts) == 1 and r['completed_at_ns'] < C
        and all(value['returncode'] == 0 and value['actual_direct_parent_wait'] and value['stdout_EOF']
                and value['wait_before_original_cutoff'] and value['EOF_before_original_cutoff']
                and not value['output_overflow'] for value in scope.receipts))
    cause = r.get('first_retirement')
    desired = (type(cause) is dict and cause.get('reason') == 'startup_failure'
        and type(cause.get('detected_at_ns')) is int
        and child['spawn_started_at_ns'] <= cause['detected_at_ns'] < r['external_Stop_sent_at_ns']
        and type(cause.get('original_until_ns')) is int
        and child['spawn_started_at_ns'] <= cause['original_until_ns'] - 3_000_000_000
        <= r['startup_failure_before_external_Stop']['observed_at_ns'])
    r['fixture_start_observation_until_ns'] = W
    r['Main_entry_startup_budget_ns'] = 3_000_000_000
    r['first_statement_placement_runtime_proved'] = False
    r['observation_qualified'] = observation
    r['passed'] = observation and desired
    r['behavior_RED_qualified'] = observation and not desired
    if r['passed']:
        r['classification'] = 'ACTUAL_NORMAL_MAIN_FIRST_STARTUP_RETIREMENT_GREEN'
    elif r['behavior_RED_qualified']:
        r['classification'] = 'ACTUAL_NORMAL_MAIN_MISSING_FIRST_STARTUP_RETIREMENT_RED'
    (a.output / 'result.json').write_text(json.dumps(r, indent=2) + '\n')
    if not observation:
        raise RuntimeError('UNCONFIRMED: setup, observation, timing or custody failed')
    assert r['passed'], MARKER


if __name__ == '__main__':
    main()
