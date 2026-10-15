#!/usr/bin/env python3
"""Actual queued lock reply versus a later ordinary Grant; isolated host only.

Import --common from a separately frozen published storage-barrier.py. This
fixture requires x86_64 /proc syscall observations and never retries a bad stop.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import select
import shutil
import signal
import socket
import subprocess
import sys
import traceback


PEER = r'''import json,os,socket,sys,time
def emit(phase, **extra):
    status=open('/proc/self/status').read().splitlines()
    print(json.dumps(dict(phase=phase,pid=os.getpid(),uid=os.getuid(),gid=os.getgid(),
        groups=os.getgroups(),cap_eff=next(x.split()[1] for x in status if x.startswith('CapEff:')),
        at_ns=time.clock_gettime_ns(time.CLOCK_BOOTTIME),**extra)),flush=True)
with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as s:
    s.settimeout(.3);s.connect(sys.argv[1]);s.send(b'{"op":"unlock"}')
    denied=json.loads(s.recv(4096));assert denied=={'result':'denied'}
    emit('ready',handshake=denied)
    assert sys.stdin.readline()=='send\n'
    packet=b'{"op":"lock"}';assert s.send(packet)==len(packet)
    emit('sent',bytes_sent=len(packet))
    assert sys.stdin.readline()=='receive\n'
    emit('reply',reply=json.loads(s.recv(4096)))
'''


def peer_phase(peer, expected):
    if not select.select([peer.stdout], [], [], .3)[0]:
        raise TimeoutError('bounded owned UID1000 helper phase')
    value = json.loads(peer.stdout.readline())
    assert value['phase'] == expected and value['pid'] == peer.pid
    assert value['uid'] == value['gid'] == 1000 and value['groups'] == []
    assert int(value['cap_eff'], 16) == 0
    return value


def instruct(peer, value):
    peer.stdin.write(value + '\n')
    peer.stdin.flush()


def state(pid):
    return Path(f'/proc/{pid}/stat').read_text().split()[2]


def run(common, binary, results):
    runtime, policy = Path('/run/queued-lock'), Path('/var/lib/queued-lock')
    runtime.mkdir(mode=0o755)
    policy.mkdir(mode=0o700)
    seed = {'grants': [{'app': 'test', 'permission': 'capture', 'allowed': True}],
            'receipts': ['a' * 32 + '-' + '0' * 16], 'lock_pending': True}
    policy_file = policy / 'policy.json'
    policy_file.write_text(json.dumps(seed))
    policy_file.chmod(0o600)
    profile = binary.parent / 'queued-lock-profile.json'
    profile.write_text(json.dumps({'apps': [{'app': 'test', 'uid': 11010,
                                          'executable': str(binary), 'args': []}]}))
    output = (results / 'queued-lock-actor.log').open('xb')
    actor = subprocess.Popen([str(binary), str(runtime), str(policy), str(profile),
                              str(binary), '--storage-test=pause:1'],
                             stdout=output, stderr=subprocess.STDOUT)
    admin = runtime / 'admin.sock'
    record = {'passed': False, 'actor_pid': actor.pid, 'poll_phase_qualified': False,
              'authentication_clear_PAM_Source_Messages_executed': False}
    actor_fd = storage_fd = peer = waiting = None
    try:
        actor_fd = os.pidfd_open(actor.pid)
        common.wait(lambda: common.rpc(admin, {'op': 'snapshot'}).get('ready'))
        initial = common.rpc(admin, {'op': 'snapshot'})
        assert initial['ready'] and initial['locked'] and not initial['storage_busy']
        pid = common.storage_pid(actor)
        storage_fd = os.pidfd_open(pid)
        record.update(initial_snapshot=initial, storage_pid=pid)
        peer = subprocess.Popen([sys.executable, '-I', '-c', PEER, str(runtime / 'lock.sock')],
                                user=1000, group=1000, extra_groups=[], text=True,
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        record['peer_ready'] = peer_phase(peer, 'ready')
        # This same retained admin connection is registered after the UID peer.
        waiting = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        waiting.settimeout(.3)
        waiting.connect(str(admin))
        waiting.send(b'{"op":"snapshot"}')
        registered = json.loads(waiting.recv(4096))
        assert registered['ready'] and registered['locked'] and not registered['storage_busy']
        record['later_admin_registration_snapshot'] = registered
        wchan = common.wait(lambda: (value if 'poll' in value else None)
                            if (value := Path(f'/proc/{actor.pid}/wchan').read_text().strip()) else None)
        signal.pidfd_send_signal(actor_fd, signal.SIGSTOP)
        common.wait(lambda: state(actor.pid) in ('T', 't'))
        stopped = Path(f'/proc/{actor.pid}/syscall').read_text().strip()
        fields = stopped.split()
        record.update(pre_stop_wchan=wchan, stopped_syscall=stopped,
                      stopped_at_ns=common.now())
        # Local x86_64 unistd_64.h: poll=7, ppoll=271. Main actor poll has
        # three listeners and at least these two retained connections. One-FD
        # pidfd liveness polls cannot establish the required loop boundary.
        assert int(fields[0], 0) in (7, 271) and int(fields[2], 0) >= 5, 'failed setup: actor not in main poll'
        record['poll_phase_qualified'] = True
        instruct(peer, 'send')
        record['peer_sent'] = peer_phase(peer, 'sent')
        assert state(actor.pid) in ('T', 't')
        packet = json.dumps({'op': 'grant', 'app': 'test', 'permission': 'capture',
                             'allowed': False}).encode()
        assert waiting.send(packet) == len(packet)
        queued_at = common.now()
        assert state(actor.pid) in ('T', 't')
        signal.pidfd_send_signal(actor_fd, signal.SIGCONT)
        record['grant_queued_at_ns'] = queued_at
        common.wait(lambda: state(pid) in ('T', 't'))
        before = common.rpc(admin, {'op': 'snapshot'})
        assert before['ready'] and not before['degraded'] and before['locked']
        assert before['storage_busy'] and before['storage_occupied']
        assert not select.select([waiting], [], [], 0)[0]
        disk = json.loads(policy_file.read_text())
        assert disk == {**seed, 'grants': [{**seed['grants'][0], 'allowed': False}]}
        instruct(peer, 'receive')
        reply = peer_phase(peer, 'reply')
        tail, error = peer.communicate(timeout=2)
        assert peer.returncode == 0 and tail == '', error
        assert not Path(f'/proc/{peer.pid}').exists()
        after = common.rpc(admin, {'op': 'snapshot'})
        assert after['ready'] and not after['degraded'] and after['locked'] and after['storage_busy']
        assert state(pid) in ('T', 't') and common.storage_pid(actor) == pid
        assert not select.select([waiting], [], [], 0)[0]
        elapsed = common.now() - queued_at
        assert elapsed < 1_000_000_000, 'original one-second Grant bound exceeded'
        record.update(before_observed_reply=before, actual_uid1000_reply=reply,
                      after_observed_reply=after, held_disk_policy=disk,
                      original_grant_elapsed_ns=elapsed, peer_actual_parent_wait=peer.returncode,
                      peer_stdout_eof=True, peer_proc_absent=True,
                      held_through_reply=True, no_storage_successor=True,
                      desired_publication_barrier_passed=reply['reply'] == {'result': 'unavailable'})
        # Unexpected Ok is a behavioral RED only after normal exact closure.
        assert common.rpc(admin, {'op': 'stop'}) == {'result': 'ok'}
        assert json.loads(waiting.recv(4096)) == {'result': 'unavailable'}
        assert actor.wait(timeout=3) == 0
        assert select.select([storage_fd], [], [], 0)[0] and not Path(f'/proc/{pid}').exists()
        assert not Path(f'/proc/{actor.pid}').exists() and not (runtime / 'lock.sock').exists()
        assert json.loads(policy_file.read_text()) == disk
        record.update(actor_actual_parent_wait=actor.returncode, actor_proc_absent=True,
                      storage_pidfd_dead=True, storage_proc_absent=True,
                      storage_wait_owned_by_actor=True, normal_actor_shutdown_completed=True)
        assert record['desired_publication_barrier_passed'], 'queued idle Ok published across later busy storage'
        record['passed'] = True
    except BaseException:
        record['error'] = traceback.format_exc()
    finally:
        # These exception fallbacks preserve ownership but can never pass.
        for label, child in [('peer', peer), ('actor', actor)]:
            if child is None:
                continue
            try:
                if child.poll() is None:
                    record['passed'] = False
                    record[label + '_failure_only_forced_cleanup'] = True
                    child.kill()
                if label == 'peer':
                    child.communicate(timeout=3)
                else:
                    child.wait(timeout=4)
                record[label + '_final_actual_parent_wait'] = child.returncode
            except BaseException:
                record['passed'] = False
                record[label + '_cleanup_error'] = traceback.format_exc()
        if storage_fd is not None:
            record['final_storage_pidfd_dead'] = bool(select.select([storage_fd], [], [], 1)[0])
            record['final_storage_proc_absent'] = not Path(f"/proc/{record['storage_pid']}").exists()
            os.close(storage_fd)
        if actor_fd is not None:
            os.close(actor_fd)
        if waiting is not None:
            waiting.close()
        output.close()
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--results', type=Path, required=True)
    parser.add_argument('--common', type=Path, required=True)
    args = parser.parse_args()
    assert os.getuid() == os.geteuid() == 0 and os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1'
    assert platform.machine() == 'x86_64', 'host syscall qualification is x86_64 only'
    common_path = args.common.resolve(strict=True)
    assert common_path != Path(__file__).resolve()
    spec = importlib.util.spec_from_file_location('storage_barrier_common', common_path)
    common = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(common)
    install = Path('/opt/queued-lock-storage')
    install.mkdir(mode=0o755)
    binary = install / 'lifecycle-daemon'
    shutil.copyfile(args.binary, binary)
    binary.chmod(0o755)
    record = run(common, binary, args.results)
    result = {'passed': record['passed'], 'case': record,
              'common_sha256': hashlib.sha256(common_path.read_bytes()).hexdigest(),
              'limits': ['direct credential-qualified JSON lock IPC; no original action execution',
                         'headless x86_64 host; no Source, Messages, authentication, clear or PAM',
                         'one required poll-phase stop; failed setup is not retried or filtered',
                         'ordinary Grant fsync followed by SIGSTOP; not uninterruptible filesystem IO',
                         'storage Child wait belongs to daemon; forced fallbacks cannot pass',
                         'kernel/identity/timeout failure requires independently owned container closure']}
    (args.results / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
