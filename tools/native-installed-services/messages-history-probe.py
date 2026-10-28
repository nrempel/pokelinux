#!/usr/bin/env python3
"""Installed production history-only Messages, no app/receiving/Source/phone claim."""
import argparse
from contextlib import closing
import array
import json
import os
from pathlib import Path
import re
import select
import socket
import sqlite3
import struct
import sys
import time
import traceback

sys.dont_write_bytecode = True
from owned import Scope, now

UNIT = 'pokelinux-messages.service'
EXE = '/usr/libexec/pokelinux/messages-service'
RAW = Path('/var/lib/pokelinux/receipts/receipts.sqlite3')
INBOX = Path('/var/lib/pokelinux/inbox/inbox.sqlite3')
PROVIDER = '/run/pokelinux/messages/read-provider.sock'
PDU = '000008915155100000006201213141510005e8329bfd06'
MISSING = 'Installed production Messages unit must be present'
FIELDS = 'LoadState,UnitFileState,ActiveState,SubState,MainPID,ControlGroup,ExecMainCode,ExecMainStatus,Result,InvocationID,ExecStop'


def rows(path, query):
    with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True, timeout=0)) as db:
        return [list(row) for row in db.execute(query).fetchall()]


def opened_files(pid):
    result = set()
    for entry in Path(f'/proc/{pid}/fd').iterdir():
        try:
            result.add(os.readlink(entry))
        except FileNotFoundError:
            pass  # An unrelated transient IPC descriptor can close during listing.
    return result


def raw_rows():
    return rows(RAW, 'SELECT id,slot,origin_owner,source_incarnation,scope,kind,lower(hex(raw)),ack,cancelled,attempt FROM receipts ORDER BY seq LIMIT 3')


def inbox_rows():
    return rows(INBOX, 'SELECT id,slot,origin_owner,source_incarnation,state,body FROM records ORDER BY seq LIMIT 3')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--expected', required=True, type=Path)
    parser.add_argument('--until-ns', required=True, type=int)
    args = parser.parse_args()
    until = min(args.until_ns, now() + 90_000_000_000)
    logs = args.output.with_suffix('.logs')
    logs.mkdir(mode=0o700)
    scope = Scope(logs)
    serial = 0
    alive = []
    initial = None
    result = {'schema': 1, 'passed': False, 'original_until_ns': until,
              'scope': 'Production history-only cold startup, authored raw reconciliation and PID1 Stop',
              'app_read_or_incoming_or_Source_or_hardware_claimed': False}

    def command(argv, name):
        nonlocal serial
        serial += 1
        return scope.run(argv, f'{serial:03d}-{name}.log', until, limit=65536)

    def properties():
        code, body = command(['/usr/bin/systemctl', 'show', UNIT, '--property=' + FIELDS], 'unit-properties')
        value = dict(line.split('=', 1) for line in body.decode().splitlines() if '=' in line)
        return code, value

    def private_read(cutoff, query=None, token=1):
        assert now() < cutoff
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as peer:
            peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            peer.settimeout(max(.001, (cutoff - now()) / 1e9))
            peer.connect(PROVIDER)
            expected_peer = (int(initial['MainPID']), 0, 0)
            assert struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == expected_peer

            def send(value):
                assert now() < cutoff
                peer.settimeout(max(.001, (cutoff - now()) / 1e9))
                body = json.dumps(value, separators=(',', ':')).encode()
                assert peer.send(body) == len(body) and now() < cutoff

            def receive():
                assert now() < cutoff
                peer.settimeout(max(.001, (cutoff - now()) / 1e9))
                body, controls, flags, _ = peer.recvmsg(8192, socket.CMSG_SPACE(12) + socket.CMSG_SPACE(4))
                credentials = []
                unexpected = False
                for level, kind, value in controls:
                    if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                        fds = array.array('i')
                        fds.frombytes(value[:len(value) - len(value) % fds.itemsize])
                        for fd in fds:
                            os.close(fd)
                        unexpected = True
                    elif level == socket.SOL_SOCKET and kind == socket.SCM_CREDENTIALS:
                        credentials.append(struct.unpack('3i', value))
                    else:
                        unexpected = True
                assert body and not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC)
                assert not unexpected and credentials == [expected_peer]
                value = json.loads(body)
                assert now() < cutoff
                assert all(not select.select([fd], [], [], 0)[0] for child, fd in alive if child == expected_peer[0])
                return value

            send({'op': 'open', 'version': 1})
            while True:
                state = receive()
                assert set(state) == {'event', 'version', 'generation', 'available'}
                assert state['event'] == 'state' and state['version'] == 1
                if state['available'] is True:
                    break
            generation = state['generation']
            assert re.fullmatch('[0-9a-f]{32}', generation)
            if query is None:
                return state
            send({'op': 'read', 'version': 1, 'generation': generation,
                  'token': token, 'until_ns': cutoff, 'query': query})
            response = receive()
            assert set(response) == {'event', 'version', 'generation', 'token', 'until_ns', 'page'}
            assert (response['event'], response['version'], response['generation'], response['token'], response['until_ns']) == ('complete', 1, generation, token, cutoff)
            return response

    try:
        assert os.getresuid() == os.getresgid() == (0, 0, 0)
        assert Path('/proc/1/comm').read_text().strip() == 'systemd'
        code, initial = properties()
        result['initial_unit'] = initial
        # This fixed first requirement makes the original missing deployment a
        # real failed unit requirement, before any expected data or socket lookup.
        assert initial.get('LoadState') == 'loaded', MISSING
        assert code == 0
        assert initial['UnitFileState'] == 'enabled' and initial['ActiveState'] == 'active' and initial['SubState'] == 'running'
        assert re.fullmatch('[0-9a-f]{32}', initial['InvocationID'])
        pid = int(initial['MainPID'])
        assert pid > 1 and os.readlink(f'/proc/{pid}/exe') == EXE
        assert Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')[:-1] == [
            os.fsencode(EXE), b'/run/pokelinux/messages', os.fsencode(RAW.parent), os.fsencode(INBOX.parent)]
        result['normal_three_argument_history_owner'] = True
        metadata = Path(f'/proc/{pid}/status').read_text()
        assert int(re.search(r'^PPid:\s+(\d+)$', metadata, re.M)[1]) == 1
        for field in ['Uid', 'Gid']:
            assert re.search('^' + field + r':\s+(.+)$', metadata, re.M)[1].split() == ['0'] * 4
        cg = initial['ControlGroup']
        assert cg.startswith('/system.slice/') and Path(f'/proc/{pid}/cgroup').read_text().strip() == '0::' + cg
        for child in (Path('/sys/fs/cgroup' + cg) / 'cgroup.procs').read_text().split():
            child = int(child)
            alive.append((child, os.pidfd_open(child, os.O_NONBLOCK)))
        assert any(child == pid for child, _ in alive)
        assert all(not select.select([fd], [], [], 0)[0] for _, fd in alive)
        admission = min(until, now() + 10_000_000_000)
        state = private_read(admission)
        result['private_state'] = state
        assert not Path('/run/pokelinux/messages/app.sock').exists()
        result['standalone_app_policy_endpoint_absent'] = True
        # Check the actual sole writer and independent readonly process after
        # bootstrap, rather than inferring database ownership from final rows.
        writers = []
        readers = []
        roles = {}
        retained = {child for child, _ in alive}
        for child in (Path('/sys/fs/cgroup' + cg) / 'cgroup.procs').read_text().split():
            child = int(child)
            if child not in retained:
                alive.append((child, os.pidfd_open(child, os.O_NONBLOCK)))
            if child == pid:
                continue
            command_line = Path(f'/proc/{child}/cmdline').read_bytes().split(b'\0')
            if b'--messages-worker' in command_line:
                writers.append(child)
                roles['writer'] = child
            if b'--messages-read-worker' in command_line:
                readers.append(child)
                roles['readonly'] = child
        assert len(writers) == len(readers) == 1
        for role, child in roles.items():
            assert os.readlink(f'/proc/{child}/exe') == EXE
            status = Path(f'/proc/{child}/status').read_text()
            assert int(re.search(r'^PPid:\s+(\d+)$', status, re.M)[1]) == pid
            opened = opened_files(child)
            assert str(INBOX) in opened
            assert (str(RAW) in opened) == (role == 'writer')
        main_opened = opened_files(pid)
        assert str(RAW) not in main_opened and str(INBOX) not in main_opened
        result['actual_database_owners'] = roles
        expected = json.loads(args.expected.read_text())
        assert expected['schema'] == 1 and expected['authored_seed_only'] is True
        raw = raw_rows()
        assert len(raw) == 2 and raw == expected['raw_rows'] and len({r[0] for r in raw}) == 2
        for row in raw:
            assert all(re.fullmatch('[0-9a-f]{32}', row[i]) for i in [0, 2, 3])
            assert row[1] == 0 and row[4:] == ['unqualified', 'deliver', PDU, 'stored', 0, None]
        inbox = inbox_rows()
        assert len(inbox) == 2
        assert [r[:4] for r in inbox] == [r[:4] for r in raw]
        assert all(r[4:] == ['text', 'hello'] for r in inbox)
        assert not list(RAW.parent.glob('incoming-attempt-*'))
        conversations = private_read(min(until, now() + 1_000_000_000),
                                     {'query': 'conversations', 'after': None, 'limit': 8})
        assert conversations['generation'] == state['generation']
        page = conversations['page']
        assert page['page'] == 'conversations' and len(page['items']) == 1
        conversation = page['items'][0]['id']
        assert re.fullmatch('[0-9a-f]{32}', conversation)
        messages = private_read(min(until, now() + 1_000_000_000),
                                {'query': 'messages', 'conversation': conversation,
                                 'after_ordinal': 0, 'limit': 2}, token=2)
        assert messages['generation'] == state['generation']
        page = messages['page']
        assert page['page'] == 'messages' and len(page['items']) == 2
        assert [item['receipt'] for item in page['items']] == [row[0] for row in raw]
        assert all(item['text'] == 'hello' for item in page['items'])
        result['private_history_exact_ids_and_text'] = True
        result['private_reader_generation'] = state['generation']
        result['raw_rows'] = raw
        result['inbox_rows'] = inbox
        result['exact_raw_ids_and_provenance_reconciled'] = True
        code, body = command(['/usr/bin/systemctl', 'stop', UNIT], 'actual-PID1-Stop')
        assert code == 0, body[-2000:]
        code, final = properties()
        result['final_unit'] = final
        assert code == 0 and final['ActiveState'] == 'inactive' and final['SubState'] == 'dead'
        assert (final['MainPID'], final['Result'], final['ExecMainCode'], final['ExecMainStatus']) == ('0', 'success', '1', '0')
        assert 'code=exited ; status=0' in final['ExecStop'], 'real messages-stop helper must finish successfully'
        group = Path('/sys/fs/cgroup' + cg)
        assert not group.exists() or (group / 'cgroup.events').read_text().split('populated ')[1].splitlines()[0] == '0'
        for child, fd in alive:
            assert select.select([fd], [], [], 0)[0] and not Path(f'/proc/{child}').exists()
        assert raw_rows() == raw and inbox_rows() == inbox
        result['PID1_Main_and_Stop_helper_exit0'] = True
        result['cgroup_empty_and_retained_processes_dead'] = True
        result['guest_parent_wait_for_PID1_processes_claimed'] = False
        result['passed'] = True
    except AssertionError as error:
        result['failure_kind'] = 'assertion'
        result['failure'] = 'AssertionError: ' + str(error)
    except BaseException as error:
        result['failure_kind'] = 'unexpected'
        result['failure'] = type(error).__name__ + ': ' + str(error)
        result['traceback'] = traceback.format_exc()
    finally:
        if not result['passed'] and initial and initial.get('LoadState') == 'loaded':
            try:
                code, _ = command(['/usr/bin/systemctl', 'stop', UNIT], 'failure-Stop')
                result['failure_Stop_exit'] = code
            except BaseException as error:
                result['failure_cleanup_error'] = repr(error)
        for _, fd in alive:
            os.close(fd)
        result['cleanup_errors'] = scope.close(args.until_ns)
        result['owned_process_receipts'] = scope.receipts
        result['passed'] = result['passed'] and not result['cleanup_errors']
        args.output.write_text(json.dumps(result, indent=2) + '\n')
    print('PASS production Messages history and PID1 Stop' if result['passed'] else result.get('failure', 'FAIL'))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
