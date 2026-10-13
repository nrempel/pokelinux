#!/usr/bin/env python3
"""Real retained desktop transport order; no actor/provider or credential mocks."""
import json, os, re, signal, socket, time
from pathlib import Path

def wait(function, seconds=3):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = function()
        if value: return value
        time.sleep(.001)
    raise AssertionError('bounded retained desktop fixture wait')

def state(pid):
    try: return Path(f'/proc/{pid}/status').read_text().split('State:\t')[1].split()[0]
    except FileNotFoundError: return None

def pause_in_real_poll(pid):
    # Observe the actual kernel poll syscall before stopping the actor. Check it
    # again after SIGSTOP; retry rather than inventing a mocked retain order.
    end = time.monotonic() + .6
    observed = []
    while time.monotonic() < end:
        before = Path(f'/proc/{pid}/syscall').read_text().split()
        if not before or before[0] not in ('7', '271', '73'):
            time.sleep(.001); continue
        os.kill(pid, signal.SIGSTOP)
        wait(lambda: state(pid) == 'T', .1)
        actual = Path(f'/proc/{pid}/syscall').read_text().split()
        observed.append({'before':before[0], 'stopped':actual[0] if actual else None,
            'wchan':Path(f'/proc/{pid}/wchan').read_text().strip()})
        # ARM64 ppoll exits on SIGSTOP into do_signal_stop before the actor
        # resumes userspace. The real actor accepts EINTR in this same pass.
        if actual and (actual[0] in ('7', '271', '73') or
                (before[0] == '73' and actual[0] == '-1' and observed[-1]['wchan'] == 'do_signal_stop')):
            return {'observed_poll_syscall':int(before[0]), 'stopped_syscall':int(actual[0]),
                'stopped_wchan':observed[-1]['wchan']}
        os.kill(pid, signal.SIGCONT)
    raise AssertionError('actor was not observed stopped in actual poll: '+json.dumps(observed[-4:]))

def app_pid(actor):
    found = []
    for item in Path('/proc').iterdir():
        if not item.name.isdecimal(): continue
        try:
            text = (item/'status').read_text()
            if int(text.split('PPid:\t')[1].split()[0]) == actor and int(text.split('Uid:\t')[1].split()[0]) == 11010:
                found.append(int(item.name))
        except (FileNotFoundError, ProcessLookupError): pass
    assert len(found) == 1, 'exactly one actual registered UID11010 child required'
    return found[0]

def worker_source(pid, actor):
    command = [part for part in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0') if part]
    assert len(command) == 7 and command[1] == b'--messages-worker'
    assert command[5] == str(actor).encode(), 'actual worker must belong to this actor'
    source = command[6].decode('ascii')
    assert re.fullmatch('[0-9a-f]{32}', source), 'actual authored worker incarnation required'
    return source

class RetainedDesktop:
    def __init__(self, path):
        root, child = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        root.settimeout(3)
        self.control = root; self.holders = []; self.pid = os.fork()
        if self.pid == 0:
            root.close()
            try:
                os.setgroups([]); os.setgid(1000); os.setuid(1000)
                desktop = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
                desktop.settimeout(3); desktop.connect(str(path))
                desktop.send(b'{"op":"query"}')
                assert json.loads(desktop.recv(4096)) == {'result':'denied'}
                child.send(json.dumps({'accepted_uid':os.getuid(),'desktop_pid':os.getpid()}).encode())
                while True:
                    command = child.recv(64)
                    if command != b'launch': break
                    desktop.send(b'{"op":"launch_messages"}')
                    child.send(b'{"actual_launch_sent":true}')
                    child.send(json.dumps({'launch_reply':json.loads(desktop.recv(4096))}).encode())
                os._exit(0)
            except BaseException:
                os._exit(9)
        child.close()
        first = json.loads(root.recv(4096))
        assert first == {'accepted_uid':1000,'desktop_pid':self.pid}
        self.accepted = first

    def verify_reopen(self, actor, runtime, worker_pid, send, receive, admin, driver, query, ids, foreign_fd=False):
        old_app = app_pid(actor.pid); old_worker = worker_pid()
        old_source = worker_source(old_worker, actor.pid)
        held = driver({'driver':'fork_holder'})
        assert held['driver_result'] == 'held'
        self.holders.append((held['pid'], os.pidfd_open(held['pid'])))
        os.kill(old_worker, signal.SIGSTOP); send(query)
        wait(lambda: admin({'op':'messages_status'}).get('busy'))
        os.kill(old_app, signal.SIGKILL)
        wait(lambda: not Path(f'/proc/{old_app}').exists() and admin({'op':'snapshot'}).get('principals') == 0)
        before = admin({'op':'messages_status'})
        assert before['available'] and before['busy'] and before['occupied'], 'real old query must still be ready/busy after actual principal reaping'
        syscall = pause_in_real_poll(actor.pid)
        try:
            (runtime/'app'/'driver.sock').unlink()
            self.control.send(b'launch')
            assert json.loads(self.control.recv(4096)) == {'actual_launch_sent':True}
            if not foreign_fd:
                # The duplicate descriptor existed only to schedule actual app
                # reaping before EOF. Release it while the actor is stopped, so
                # both the retained launch packet and old read EOF are ready.
                self.release_holders()
        finally:
            os.kill(actor.pid, signal.SIGCONT)
        reply = json.loads(self.control.recv(4096))
        assert reply == {'launch_reply':{'result':'ok'}}, 'old app must be actually reaped before new registration'
        wait(lambda: not Path(f'/proc/{old_app}').exists())
        wait(lambda: (runtime/'app'/'driver.sock').exists())
        def ready_successor():
            if not admin({'op':'messages_status'}).get('available'): return False
            try: candidate = worker_pid()
            except AssertionError as error:
                if str(error) != 'owned Messages child missing': raise
                # A status reply and /proc scan are separate real observations.
                # The old child may disappear between them before restart.
                return False
            return candidate if candidate != old_worker else False
        try:
            new_worker = wait(ready_successor)
        except AssertionError:
            snapshot = admin({'op':'messages_status'})
            print(json.dumps({'retained_connection_precedes_old_read':True,'actual_desktop_uid':1000,
                'actual_poll_pause':syscall,'old_app_reaped':not Path(f'/proc/{old_app}').exists(),
                'foreign_fd_retained':foreign_fd,'storage':snapshot}),flush=True)
            raise AssertionError('successful retained desktop launch left local Messages storage unavailable')
        assert new_worker != old_worker and not Path(f'/proc/{old_worker}').exists()
        new_source = worker_source(new_worker, actor.pid)
        assert new_source != old_source, 'actual successor must have a fresh incarnation'
        send(query); assert receive() == {'result':'denied'}, 'desktop launch must not grant the new app reads'
        assert admin({'op':'background','app':'org.omarchy.Messages','background':False}) == {'result':'ok'}
        send(query); page = receive()
        assert [value['receipt'] for value in page['items']] == ids
        new_app = app_pid(actor.pid)
        detail = {'actual_desktop_uid':1000,'desktop_pid':self.pid,'preconnected_before_old_app_read':True,
            'actual_poll_pause':syscall,'old_app':old_app,'new_app':new_app,
            'old_app_actually_reaped':not Path(f'/proc/{old_app}').exists(),'old_worker':old_worker,
            'new_worker':new_worker,'old_worker_source':old_source,'new_worker_source':new_source,'old_worker_actually_reaped':not Path(f'/proc/{old_worker}').exists(),
            'old_query_replayed':False,'new_app_denied_before_current_authorization':True,
            'exact_local_occurrences_preserved':True,'foreign_fd_retained':foreign_fd,
            'foreign_fd_socket_inode':held['socket_inode'],
            'old_worker_ready_busy_after_actual_principal_reap':True}
        print(json.dumps(detail),flush=True)
        return detail

    def release_holders(self):
        for pid, fd in self.holders:
            try: signal.pidfd_send_signal(fd, signal.SIGKILL)
            except ProcessLookupError: pass
            os.close(fd); wait(lambda: not Path(f'/proc/{pid}').exists())
        self.holders.clear()

    def close(self):
        self.release_holders()
        if self.pid is not None:
            try: self.control.send(b'quit')
            except OSError: pass
            os.waitpid(self.pid,0); self.pid = None; self.control.close()
