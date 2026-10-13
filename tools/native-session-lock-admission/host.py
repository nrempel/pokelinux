#!/usr/bin/env python3
"""Actual UID1000 lock-only IPC in a disposable network-free container."""
import argparse
import array
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import struct
import subprocess
import time
import traceback


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--phase', choices=['red', 'green'], required=True)
    a = p.parse_args()
    assert os.getuid() == os.geteuid() == 0
    assert os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1'
    install, runtime, policy = Path('/opt/lock-test'), Path('/run/pokelinux/messages'), Path('/var/lib/lock-test')
    for path in [install, runtime, policy]:
        path.mkdir(mode=0o755, parents=True)
    policy.chmod(0o700)
    for path in Path('/bins').glob('lifecycle-*'):
        if path.suffix != '.d':
            shutil.copyfile(path, install / path.name)
            (install / path.name).chmod(0o755)
    app = runtime / 'app'
    app.mkdir(mode=0o700)
    os.chown(app, 11010, 11010)
    profile = install / 'profile.json'
    profile.write_text(json.dumps({'apps': [{'app': 'test', 'uid': 11010,
        'executable': str(install / 'lifecycle-client'), 'args': [str(runtime / 'app.sock'), str(app / 'driver.sock')]}]}))
    result = {'phase': a.phase, 'passed': False, 'groups': [], 'phone_accessed': False,
              'actor_has_messages_provider': False, 'source_revival': False}
    child = subprocess.Popen([str(install / 'lifecycle-daemon'), str(runtime), str(policy), str(profile),
                              str(install / 'lifecycle-launcher')], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)

    def call(path, value):
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
            sock.settimeout(2)
            sock.connect(str(path))
            sock.send(json.dumps(value).encode())
            return json.loads(sock.recv(4096))

    def admin(value):
        return call(runtime / 'admin.sock', value)

    def wait(check, seconds=4):
        until = now() + int(seconds * 1e9)
        while now() < until:
            try:
                value = check()
                if value:
                    return value
            except (FileNotFoundError, ConnectionRefusedError):
                pass
            time.sleep(.002)
        raise TimeoutError('owned host fixture deadline')

    def as_uid(uid, action):
        read, write = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(read)
            try:
                os.setgroups([])
                os.setgid(uid)
                os.setuid(uid)
                outcome = action()
            except BaseException as error:
                outcome = {'exception': type(error).__name__, 'errno': getattr(error, 'errno', None)}
            os.write(write, json.dumps({'uid': os.getuid(), 'pid': os.getpid(), 'at_ns': now(), 'outcome': outcome}).encode())
            os.close(write)
            os._exit(0)
        os.close(write)
        wire = os.read(read, 65536)
        os.close(read)
        assert os.waitpid(pid, 0)[1] == 0
        return json.loads(wire)

    try:
        wait(lambda: admin({'op': 'snapshot'}).get('ready'))
        assert admin({'op': 'locked', 'locked': False}) == {'result': 'ok'}
        before = admin({'op': 'snapshot'})
        assert before['ready'] and not before['locked'] and child.poll() is None
        attempted = as_uid(1000, lambda: call(runtime / 'lock.sock', {'op': 'lock'}))
        after = admin({'op': 'snapshot'})
        result['groups'].append({'id': 'actual_uid1000_missing_ingress_red', 'before': before,
                                 'actual_child': attempted, 'after': after,
                                 'desired_boundary_passed': attempted['outcome'] == {'result': 'ok'} and after['locked']})
        if a.phase == 'red':
            assert attempted['uid'] == 1000 and attempted['outcome'] == {'exception': 'FileNotFoundError', 'errno': 2}
            assert after['ready'] and not after['locked']
            result['expected_red_qualified'] = True
        else:
            assert attempted['outcome'] == {'result': 'ok'} and after['locked']
            forbidden = [{'op': 'unlock'}, {'op': 'locked', 'locked': False}, {'op': 'lock', 'locked': False},
                         {'op': 'grant', 'app': 'test', 'permission': 'capture', 'allowed': True},
                         {'op': 'launch', 'app': 'test'}, {'op': 'stop'}, {'op': 'lock', 'params': {}}]
            for value in forbidden:
                reply = as_uid(1000, lambda: call(runtime / 'lock.sock', value))
                assert reply['outcome'] == {'result': 'denied'}, reply
            for uid in [0, 11010, 1001]:
                reply = as_uid(uid, lambda: call(runtime / 'lock.sock', {'op': 'lock'}))
                assert reply['outcome'] == {'result': 'denied'}, reply
            assert admin({'op': 'snapshot'})['locked']
            result['groups'].append({'id': 'unauthorized_and_all_non_lock_commands_denied', 'passed': True})
            # The kernel's packet credentials, rather than connection origin, own admission.
            pair, other = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
            pid = os.fork()
            if pid == 0:
                pair.close()
                os.setgroups([]); os.setgid(1000); os.setuid(1000)
                with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
                    sock.connect(str(runtime / 'lock.sock'))
                    other.sendmsg([b'fd'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [sock.fileno()]))])
                os._exit(0)
            other.close()
            _, ancillary, _, _ = pair.recvmsg(16, socket.CMSG_SPACE(4))
            fds = array.array('i'); fds.frombytes(ancillary[0][2][:4]); pair.close()
            assert os.waitpid(pid, 0)[1] == 0
            assert admin({'op': 'locked', 'locked': False}) == {'result': 'ok'}
            with socket.socket(fileno=fds[0]) as delegated:
                delegated.settimeout(2); delegated.send(b'{"op":"lock"}')
                assert json.loads(delegated.recv(4096)) == {'result': 'denied'}
            assert not admin({'op': 'snapshot'})['locked']
            result['groups'].append({'id': 'inherited_uid1000_connection_root_packet_denied', 'passed': True})
            assert admin({'op': 'grant', 'app': 'test', 'permission': 'capture', 'allowed': True}) == {'result': 'ok'}
            assert admin({'op': 'launch', 'app': 'test'}) == {'result': 'ok'}
            wait(lambda: (app / 'driver.sock').exists())
            permit = call(app / 'driver.sock', {'op': 'prepare', 'permission': 'capture', 'ttl_ms': 5000})
            assert permit['result'] == 'prepared'
            assert as_uid(1000, lambda: call(runtime / 'lock.sock', {'op': 'lock'}))['outcome'] == {'result': 'ok'}
            assert call(app / 'driver.sock', {'op': 'admit', 'permit': permit['permit']}) == {'result': 'denied'}
            assert call(app / 'driver.sock', {'op': 'prepare', 'permission': 'capture', 'ttl_ms': 5000}) == {'result': 'denied'}
            result['groups'].append({'id': 'lock_ack_revokes_prepared_and_new_capture_authority', 'passed': True})
            retained = Path('/usr/libexec/pokelinux/omarchy-system-lock-original')
            retained.parent.mkdir(mode=0o755, parents=True)
            retained.write_text('#!/bin/sh\nprintf "synthetic_original_action_uid="\nid -u\n')
            retained.chmod(0o755)
            # This authored action is a client-effect control only. Actual
            # unchanged Omarchy execution is qualified by the separate ARM run.
            client = install / 'lifecycle-lock'
            execution = subprocess.run([str(client)], user=1000, group=1000, extra_groups=[],
                                       capture_output=True, timeout=3)
            assert execution.returncode == 0
            lines = execution.stdout.decode().splitlines()
            ack = json.loads(lines[0])
            assert ack['event'] == 'native_lock_intent_ack' and lines[1:] == ['synthetic_original_action_uid=1000']
            assert ack['acknowledged_at_ns'] < ack['until_ns']
            result['groups'].append({'id': 'fixed_unprivileged_client_actual_ack_before_authored_effect',
                                     'passed': True, 'actual_ack': ack, 'effect_is_authored_control': True})
            for uid, args in [(0, []), (11010, []), (1000, ['--unlock']), (1000, ['/bin/true'])]:
                execution = subprocess.run([str(client), *args], user=uid, group=uid, extra_groups=[],
                                           capture_output=True, timeout=3)
                assert execution.returncode != 0 and not execution.stdout
            result['groups'].append({'id': 'native_client_denies_wrong_uid_and_effect_arguments', 'passed': True})
            os.kill(child.pid, signal.SIGSTOP)
            wait(lambda: Path(f'/proc/{child.pid}/stat').read_text().split()[2] in ('T', 't'))
            began = now()
            try:
                execution = subprocess.run([str(client)], user=1000, group=1000, extra_groups=[],
                                           capture_output=True, timeout=3)
                elapsed = now() - began
                assert execution.returncode != 0 and b'TimedOut' in execution.stderr and not execution.stdout
            finally:
                os.kill(child.pid, signal.SIGCONT)
            assert admin({'op': 'snapshot'})['locked']
            result['groups'].append({'id': 'held_actual_actor_ack_cannot_execute_effect_after_absolute_deadline',
                                     'passed': True, 'elapsed_ns': elapsed, 'budget_ns': 2_000_000_000,
                                     'native_client_returncode': execution.returncode})
        started = now()
        snapshot = admin({'op': 'snapshot'})
        snap_ns = now() - started
        started = now()
        assert admin({'op': 'stop'}) == {'result': 'ok'}
        stop_ns = now() - started
        child.wait(timeout=4)
        assert child.returncode == 0
        assert not (runtime / 'lock.sock').exists()
        result['groups'].append({'id': 'responsive_snapshot_stop_and_actual_actor_wait', 'passed': True,
                                 'snapshot': snapshot, 'snapshot_ns': snap_ns, 'stop_ns': stop_ns, 'returncode': child.returncode})
        result['passed'] = True
    except BaseException:
        result['error'] = traceback.format_exc()
    finally:
        if child.poll() is None:
            child.kill()
        child.wait(timeout=4)
        result['actual_actor_parent_wait'] = True
        result['actor_output'] = child.stdout.read().decode()
        Path('/evidence/result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
