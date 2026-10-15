#!/usr/bin/env python3
"""Actual lock.sock admission while an owned storage child is SIGSTOP-held.

Run only in a disposable network-free root container. The ordinary grant write
and existing isolated pause hook are real; no authentication or clear is faked.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import subprocess
import sys
import time
import traceback


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def rpc(path, value):
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
        sock.settimeout(.3)
        sock.connect(str(path))
        sock.send(json.dumps(value).encode())
        return json.loads(sock.recv(4096))


def wait(check, seconds=2):
    until = now() + int(seconds * 1e9)
    while now() < until:
        try:
            value = check()
            if value:
                return value
        except (FileNotFoundError, ConnectionRefusedError):
            pass
        time.sleep(.001)
    raise TimeoutError('original bounded storage barrier fixture deadline')


UID_RPC = '''import json,os,socket,sys,time
s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.settimeout(.3)
s.connect(sys.argv[1]);s.send(sys.argv[2].encode());reply=json.loads(s.recv(4096))
status=open('/proc/self/status').read().splitlines()
print(json.dumps({'reply':reply,'uid':os.getuid(),'gid':os.getgid(),'groups':os.getgroups(),
 'cap_eff':next(x.split()[1] for x in status if x.startswith('CapEff:')),
 'pid':os.getpid(),'at_ns':time.clock_gettime_ns(time.CLOCK_BOOTTIME)}))
'''


def user_rpc(path, value):
    child = subprocess.Popen([sys.executable, '-c', UID_RPC, str(path), json.dumps(value)],
                             user=1000, group=1000, extra_groups=[],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        out, error = child.communicate(timeout=2)
    except BaseException:
        if child.poll() is None:
            child.kill()
        child.communicate(timeout=2)
        raise
    assert child.returncode == 0, error.decode()
    record = json.loads(out)
    assert record['uid'] == record['gid'] == 1000 and record['groups'] == []
    assert int(record['cap_eff'], 16) == 0
    assert not Path(f'/proc/{child.pid}').exists()
    record.update(actual_parent_wait=child.returncode, stdout_eof=True, proc_absent=True)
    return record


def storage_pid(actor):
    children = Path(f'/proc/{actor.pid}/task/{actor.pid}/children').read_text().split()
    matches = [int(pid) for pid in children
               if b'--storage-worker' in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')]
    assert len(matches) == 1, matches
    return matches[0]


def case(binary, results, pending):
    label = 'pending-true' if pending else 'pending-false'
    runtime = Path('/run') / label
    policy = Path('/var/lib') / label
    runtime.mkdir(mode=0o755)
    policy.mkdir(mode=0o700)
    history = ['a' * 32 + '-' + '0' * 16]
    seed = {'grants': [{'app': 'test', 'permission': 'capture', 'allowed': True}],
            'receipts': history, 'lock_pending': pending}
    policy_file = policy / 'policy.json'
    policy_file.write_text(json.dumps(seed))
    policy_file.chmod(0o600)
    profile = binary.parent / (label + '.json')
    profile.write_text(json.dumps({'apps': [{'app': 'test', 'uid': 11010,
                                          'executable': str(binary), 'args': []}]}))
    output = (results / (label + '-actor.log')).open('xb')
    actor = subprocess.Popen([str(binary), str(runtime), str(policy), str(profile),
                              str(binary), '--storage-test=pause:1'],
                             stdout=output, stderr=subprocess.STDOUT)
    admin = runtime / 'admin.sock'
    lock = runtime / 'lock.sock'
    record = {'case': label, 'passed': False, 'actor_pid': actor.pid,
              'auth_clear_or_PAM_executed': False, 'Source_provider_present': False,
              'hold_kind': 'actual owned storage child SIGSTOP after ordinary grant fsync; not a filesystem syscall stall'}
    alive = None
    waiting = None
    failure = None
    try:
        wait(lambda: rpc(admin, {'op': 'snapshot'}).get('ready'))
        assert rpc(admin, {'op': 'locked', 'locked': False}) == {'result': 'ok'}
        initial = rpc(admin, {'op': 'snapshot'})
        assert initial['locked'] == pending and not initial['storage_busy']
        record['initial_snapshot'] = initial
        if pending:
            before = hashlib.sha256(policy_file.read_bytes()).hexdigest()
            idle = user_rpc(lock, {'op': 'lock'})
            assert idle['reply'] == {'result': 'ok'}
            assert hashlib.sha256(policy_file.read_bytes()).hexdigest() == before
            record['idle_already_durable_lock'] = idle
        assert user_rpc(lock, {'op': 'unlock'})['reply'] == {'result': 'denied'}
        assert rpc(lock, {'op': 'lock'}) == {'result': 'denied'}
        pid = storage_pid(actor)
        alive = os.pidfd_open(pid)
        record['storage_pid'] = pid
        waiting = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        waiting.settimeout(.3)
        waiting.connect(str(admin))
        waiting.send(json.dumps({'op': 'grant', 'app': 'test', 'permission': 'capture',
                                 'allowed': False}).encode())
        wait(lambda: Path(f'/proc/{pid}/stat').read_text().split()[2] in ('T', 't'))
        held_at = now()
        before = rpc(admin, {'op': 'snapshot'})
        assert before['ready'] and not before['degraded'] and before['storage_busy'] and before['storage_occupied']
        assert not select.select([waiting], [], [], 0)[0]
        disk_while_held = json.loads(policy_file.read_text())
        assert disk_while_held == {**seed, 'grants': [{**seed['grants'][0], 'allowed': False}]}
        reply = user_rpc(lock, {'op': 'lock'})
        after = rpc(admin, {'op': 'snapshot'})
        assert after['ready'] and not after['degraded'] and after['locked'] and after['storage_busy']
        assert Path(f'/proc/{pid}/stat').read_text().split()[2] in ('T', 't')
        assert not select.select([waiting], [], [], 0)[0]
        assert record['storage_pid'] == storage_pid(actor)
        record.update(held_at_ns=held_at, before_lock_snapshot=before,
                      actual_uid1000_lock=reply, after_lock_snapshot=after,
                      held_disk_policy=disk_while_held,
                      held_through_lock_reply=True, no_storage_successor=True,
                      desired_storage_barrier_passed=reply['reply'] == {'result': 'unavailable'})
        # Always close the actual ownership boundary, including the behavioral
        # RED. A failed contract assertion cannot strand a stopped child.
        started = now()
        assert rpc(admin, {'op': 'stop'}) == {'result': 'ok'}
        record['stop_ack_elapsed_ns'] = now() - started
        assert json.loads(waiting.recv(4096)) == {'result': 'unavailable'}
        assert actor.wait(timeout=3) == 0
        record['actor_actual_parent_wait'] = actor.returncode
        assert select.select([alive], [], [], 0)[0] and not Path(f'/proc/{pid}').exists()
        assert not Path(f'/proc/{actor.pid}').exists()
        assert not lock.exists()
        record.update(storage_pidfd_dead=True, storage_proc_absent=True,
                      actor_proc_absent=True, storage_wait_owned_by_actor=True,
                      actor_shutdown_completed=True)
        assert json.loads(policy_file.read_text()) == disk_while_held
        assert record['desired_storage_barrier_passed'], 'busy storage acknowledged cached lock_pending as durable admission'
        record['passed'] = True
    except BaseException:
        failure = traceback.format_exc()
    finally:
        if actor.poll() is None:
            record['failure_only_forced_actor_cleanup'] = True
            actor.kill()
        record['final_actual_actor_parent_wait'] = actor.wait(timeout=4)
        if alive is not None:
            record['final_storage_pidfd_dead'] = bool(select.select([alive], [], [], 1)[0])
            record['final_storage_proc_absent'] = not Path(f"/proc/{record['storage_pid']}").exists()
            os.close(alive)
        if waiting is not None:
            waiting.close()
        output.close()
        if failure:
            record['error'] = failure
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--results', type=Path, required=True)
    args = parser.parse_args()
    assert os.getuid() == os.geteuid() == 0
    assert os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1'
    install = Path('/opt/lock-storage-barrier')
    install.mkdir(mode=0o755)
    binary = install / 'lifecycle-daemon'
    shutil.copyfile(args.binary, binary)
    binary.chmod(0o755)
    records = [case(binary, args.results, pending) for pending in [True, False]]
    result = {'passed': all(r['passed'] for r in records), 'cases': records,
              'actual_lock_IPC': True, 'actual_ordinary_storage_write': True,
              'authentication_or_clear_executed': False, 'phone_accessed': False,
              'limits': ['headless host daemon; no actual Source or Messages app',
                         'grant write is not an authenticated unlock clear',
                         'actual SIGSTOP-held child, not uninterruptible filesystem IO',
                         'fixture observes storage death; storage Child wait belongs to daemon']}
    (args.results / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
