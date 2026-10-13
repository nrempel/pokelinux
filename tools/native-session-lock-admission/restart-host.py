#!/usr/bin/env python3
"""Actual owned-storage crash, uncertainty and clean-restart controls; no UI."""
import ctypes
import json
import os
from pathlib import Path
import select
import signal
import socket
import subprocess
import time
import traceback


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def main():
    assert os.getuid() == 0 and os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1'
    assert ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0
    runtime, install = Path('/run/pokelinux/messages'), Path('/opt/lock-restart')
    runtime.mkdir(mode=0o755, parents=True)
    install.mkdir(mode=0o755)
    for name in ['lifecycle-daemon', 'lifecycle-launcher', 'lifecycle-lock']:
        (install / name).write_bytes((Path('/bins') / name).read_bytes())
        (install / name).chmod(0o755)
    profile = install / 'profile.json'
    profile.write_text(json.dumps({'apps': [{'app': 'org.omarchy.StorageControl', 'uid': 11010,
        'executable': str(install / 'lifecycle-lock'), 'args': []}]}))
    retained = Path('/usr/libexec/pokelinux/omarchy-system-lock-original')
    retained.parent.mkdir(mode=0o755, parents=True)
    retained.write_text('#!/bin/sh\nprintf "authored-effect-control\\n"\n')
    retained.chmod(0o755)
    result = {'passed': False, 'groups': [], 'cleanup': [], 'phone_accessed': False,
              'actual_UI_or_Source_runtime': False, 'original_effect_is_authored_control': True}
    actors, clients, descriptors = [], [], []
    sequence = 0

    def wait(check, seconds=3):
        until = now() + int(seconds * 1e9)
        while now() < until:
            try:
                value = check()
                if value:
                    return value
            except (FileNotFoundError, ConnectionRefusedError):
                pass
            time.sleep(.001)
        raise TimeoutError('owned storage qualification deadline')

    def admin(value):
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as sock:
            sock.settimeout(1)
            sock.connect(str(runtime / 'admin.sock'))
            sock.send(json.dumps(value).encode())
            return json.loads(sock.recv(4096))

    def spawn(policy, mode=None):
        nonlocal sequence
        sequence += 1
        args = [str(install / 'lifecycle-daemon'), str(runtime), str(policy), str(profile), str(install / 'lifecycle-launcher')]
        if mode:
            args.append('--storage-test=' + mode)
        log = (Path('/evidence') / ('actor-' + str(sequence) + '.log')).open('x')
        descriptors.append(log)
        child = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
        actors.append(child)
        wait(lambda: (runtime / 'admin.sock').exists())
        return child

    def fresh(name, mode=None):
        policy = Path('/var/lib/lock-restart') / name
        policy.mkdir(mode=0o700, parents=True)
        child = spawn(policy, mode)
        if mode != 'pause:0':
            wait(lambda: admin({'op': 'snapshot'})['ready'])
            assert admin({'op': 'locked', 'locked': False}) == {'result': 'ok'}
            assert not admin({'op': 'snapshot'})['locked']
        return policy, child

    def client():
        child = subprocess.Popen([str(install / 'lifecycle-lock')], user=1000, group=1000,
            extra_groups=[], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        clients.append(child)
        return child

    def completed_client(child, success):
        stdout, stderr = child.communicate(timeout=3)
        assert (child.returncode == 0) == success
        if success:
            lines = stdout.decode().splitlines()
            ack = json.loads(lines[0])
            assert ack['event'] == 'native_lock_intent_ack' and lines[1:] == ['authored-effect-control']
        else:
            assert not stdout
            ack = None
        return {'returncode': child.returncode, 'stdout': stdout.decode(), 'stderr': stderr.decode(),
                'actual_native_ack': ack, 'authored_effect_executed': bool(stdout)}

    def worker(child):
        for entry in Path('/proc').iterdir():
            if entry.name.isdecimal():
                try:
                    if int((entry / 'stat').read_text().split()[3]) == child.pid and b'--storage-worker' in (entry / 'cmdline').read_bytes():
                        return int(entry.name)
                except FileNotFoundError:
                    pass

    def reap_orphans():
        until = now() + 2_000_000_000
        while now() < until:
            try:
                pid, status = os.waitpid(-1, os.WNOHANG)
            except ChildProcessError:
                return
            if pid:
                result['cleanup'].append({'adopted_pid': pid, 'wait_status': status, 'actual_subreaper_wait': True})
            else:
                time.sleep(.001)
        raise TimeoutError('owned orphan actual wait')

    def stopped(child):
        started = now()
        snapshot = admin({'op': 'snapshot'})
        snap_ns = now() - started
        started = now()
        ack = admin({'op': 'stop'})
        stop_ns = now() - started
        assert ack == {'result': 'ok'}
        child.wait(timeout=3)
        assert child.returncode == 0
        result['cleanup'].append({'actor_pid': child.pid, 'returncode': child.returncode, 'actual_parent_wait': True})
        return {'snapshot': snapshot, 'snapshot_ns': snap_ns, 'stop_ack': ack, 'stop_ns': stop_ns}

    def successor_refused(policy):
        successor = spawn(policy)
        successor.wait(timeout=3)
        assert successor.returncode != 0
        assert not (runtime / 'admin.sock').exists()
        result['cleanup'].append({'actor_pid': successor.pid, 'returncode': successor.returncode, 'actual_parent_wait': True})
        return {'actual_successor_pid': successor.pid, 'actual_wait_returncode': successor.returncode,
                'same_policy_directory': str(policy), 'root_state_removed_or_reset': False}

    try:
        policy, child = fresh('healthy')
        admitted = completed_client(client(), True)
        assert json.loads((policy / 'policy.json').read_text())['lock_pending']
        assert admin({'op': 'locked', 'locked': False}) == {'result': 'ok'}
        assert admin({'op': 'snapshot'})['locked']
        stop = stopped(child)
        seal = json.loads((policy / 'owner.clean').read_text())
        successor = spawn(policy)
        wait(lambda: admin({'op': 'snapshot'})['ready'])
        assert admin({'op': 'snapshot'})['locked']
        repeat = completed_client(client(), True)
        assert json.loads((policy / 'policy.json').read_text())['lock_pending']
        successor_stop = stopped(successor)
        result['groups'].append({'id': 'healthy_same_directory_restore_and_repeated_lock_ACK', 'passed': True,
            'admitted': admitted, 'clean_stop': stop, 'clean_seal': seal, 'repeat': repeat, 'successor_stop': successor_stop})

        policy, child = fresh('kill-before-ack', 'pause:1')
        pending = client()
        pid = wait(lambda: worker(child))
        wait(lambda: Path(f'/proc/{pid}/stat').read_text().split()[2] in ('T', 't'))
        persisted = json.loads((policy / 'policy.json').read_text())
        assert persisted['lock_pending'] and pending.poll() is None
        started = now()
        held = admin({'op': 'snapshot'})
        snap_ns = now() - started
        assert held['ready'] and held['locked'] and held['storage_busy']
        os.kill(child.pid, signal.SIGKILL)
        child.wait(timeout=2)
        assert child.returncode == -signal.SIGKILL
        result['cleanup'].append({'actor_pid': child.pid, 'returncode': child.returncode, 'actual_parent_wait': True})
        failed_client = completed_client(pending, False)
        reap_orphans()
        for path in runtime.iterdir():
            assert path.is_socket() and path.stat().st_uid == 0
            path.unlink()
        assert not (policy / 'owner.clean').exists()
        refused = successor_refused(policy)
        result['groups'].append({'id': 'actual_actor_SIGKILL_after_synced_lock_before_ACK_retains_uncertainty', 'passed': True,
            'held_storage_worker_pid': pid, 'snapshot': held, 'snapshot_ns': snap_ns,
            'persisted_before_actual_kill': persisted, 'client': failed_client, 'successor': refused})

        policy, child = fresh('worker-death', 'pause:1')
        pending = client()
        pid = wait(lambda: worker(child))
        wait(lambda: Path(f'/proc/{pid}/stat').read_text().split()[2] in ('T', 't'))
        assert json.loads((policy / 'policy.json').read_text())['lock_pending']
        os.kill(pid, signal.SIGKILL)
        failed_client = completed_client(pending, False)
        wait(lambda: admin({'op': 'snapshot'})['degraded'])
        stop = stopped(child)
        refused = successor_refused(policy)
        result['groups'].append({'id': 'actual_owned_storage_death_before_ACK_keeps_denial_and_responsive_Stop',
            'passed': True, 'storage_pid': pid, 'client': failed_client, 'stop': stop, 'successor': refused})

        policy, child = fresh('exclusive-write-failure')
        collision = policy / 'policy.pending'
        collision.write_bytes(b'explicit authored create_new collision')
        collision.chmod(0o600)
        failed_client = completed_client(client(), False)
        wait(lambda: admin({'op': 'snapshot'})['degraded'])
        stop = stopped(child)
        assert collision.read_bytes() == b'explicit authored create_new collision'
        refused = successor_refused(policy)
        result['groups'].append({'id': 'actual_exclusive_pending_file_collision_refuses_ACK_and_recovery',
            'passed': True, 'client': failed_client, 'stop': stop, 'successor': refused,
            'fault_is_explicit_fixture_input': True})

        policy, child = fresh('corrupt-recovery')
        admitted = completed_client(client(), True)
        stop = stopped(child)
        with (policy / 'policy.json').open('ab') as output:
            output.write(b'\nexplicit corrupt recovery input')
        refused = successor_refused(policy)
        result['groups'].append({'id': 'actual_corrupt_sealed_policy_recovery_refused', 'passed': True,
            'admitted': admitted, 'stop': stop, 'successor': refused, 'fault_is_explicit_fixture_input': True})

        policy, child = fresh('held-load', 'pause:0')
        pid = wait(lambda: worker(child))
        wait(lambda: Path(f'/proc/{pid}/stat').read_text().split()[2] in ('T', 't'))
        failed_client = completed_client(client(), False)
        held = admin({'op': 'snapshot'})
        assert not held['ready'] and held['locked']
        stop = stopped(child)
        result['groups'].append({'id': 'actual_held_initial_load_refuses_lock_ACK_and_keeps_Snapshot_Stop',
            'passed': True, 'client': failed_client, 'snapshot': held, 'stop': stop})
        result['passed'] = True
    except BaseException:
        result['error'] = traceback.format_exc()
    finally:
        for child in clients:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=3)
        for child in actors:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=3)
        reap_orphans()
        for log in descriptors:
            log.close()
        result['all_direct_children_actually_waited'] = all(child.poll() is not None for child in actors + clients)
        Path('/evidence/result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
