"""Fixed metadata-only Main receive trace; existing WorkerTrace owns all stops."""
import os
from pathlib import Path
import select
import signal

from owned import now
from worker_trace import (WorkerTrace, pc, DETACH, EVENT_EXIT, GETTID, RECVMSG,
                          SENDTO, SENDMSG, SLEEPS)

POLL, PPOLL = 7, 271  # Native x86_64 only, already checked by WorkerTrace.info.


def main_channel(pid):
    rows = {}
    for line in Path(f'/proc/{pid}/net/unix').read_text().splitlines()[1:]:
        fields = line.split()
        private_address = len(fields) == 7 or (len(fields) == 8 and fields[7].startswith('@'))
        if private_address and int(fields[4], 16) == 5 and int(fields[5], 16) == 3:
            rows[int(fields[6])] = True
    found = []
    for path in Path(f'/proc/{pid}/fd').iterdir():
        try:
            value = os.readlink(path)
        except FileNotFoundError:
            continue
        if value.startswith('socket:[') and int(value[8:-1]) in rows:
            found.append({'fd': int(path.name), 'inode': int(value[8:-1])})
    assert len(found) == 1, 'sole anonymous/abstract connected Main SEQPACKET, excluding filesystem ingress'
    return found[0]


def park_worker(trace):
    """Stop at normal loop sleep after completed observation send/post-checks."""
    pid, pidfd, scope, until = trace.pid, trace.pidfd, trace.scope, trace.until
    trace.attach()
    sent = None
    try:
        while now() < until:
            status = trace.wait()
            event, sig = status >> 16, os.WSTOPSIG(status)
            assert event != EVENT_EXIT, 'worker exited before safe silence boundary'
            if sig != signal.SIGTRAP | 0x80:
                assert event == 0
                trace.resume(0 if sig == signal.SIGTRAP else sig)
                continue
            op, value, args = trace.info()
            if op == 1:
                trace.pending = {'nr': value, 'args': args, 'entry_ns': now()}
                if sent is not None and value in SLEEPS:
                    stopped = now()
                    # Signal delivery on detach creates a real group stop; no
                    # normal worker operation/envelope is held open here.
                    signal.pidfd_send_signal(pidfd, signal.SIGSTOP)
                    trace.detach()
                    trace.record.update(observation_send=sent,
                        safe_boundary='loop_sleep_entry_after_completed_observation_send_and_postchecks',
                        safe_boundary_at_ns=stopped, detached_at_actual_nonterminal_stop=True,
                        normal_Main_retains_terminal_wait=True, SIGSTOP_via_retained_pidfd=True,
                        detach_signal=0,
                        detached_at_ns=now(), syscall_stops=trace.steps)
                    while now() < until:
                        scope.pump(); scope.check()
                        text = Path(f'/proc/{pid}/status').read_text()
                        state = next(line.split()[1] for line in text.splitlines() if line.startswith('State:'))
                        tracer = int(next(line.split()[1] for line in text.splitlines() if line.startswith('TracerPid:')))
                        if state == 'T' and tracer == 0:
                            assert not select.select([pidfd], [], [], 0)[0]
                            trace.record.update(group_stop_confirmed=True, TracerPid=0,
                                                group_stop_confirmed_at_ns=now())
                            return trace.record
                        select.select([], [], [], .0001)
                    raise TimeoutError('original safe worker group-stop cutoff')
            else:
                entry = trace.pending; trace.pending = None
                if (entry is not None and entry['nr'] in [SENDTO, SENDMSG]
                        and entry['args'][0] == trace.channel['fd'] and value in [48, 61]):
                    sent = {'fd': entry['args'][0], 'bytes': value,
                            'entry_ns': entry['entry_ns'], 'exit_ns': now(), 'contents_inspected': False}
            trace.resume()
        raise TimeoutError('original safe worker boundary cutoff')
    except BaseException:
        trace.close()
        raise


class MainTrace(WorkerTrace):
    def __init__(self, pid, pidfd, worker_pid, worker_pidfd, scope, until, close_until, source_proof):
        # Reuse exact syscall wait/read/hold helpers without their worker-only
        # parent and one-socket assumptions. Scope must not poll this direct child.
        self.pid = pid; self.pidfd = pidfd; self.main_pid = os.getpid()
        self.worker_pid = worker_pid; self.worker_pidfd = worker_pidfd; self.scope = scope
        self.until = until; self.close_until = close_until
        self.active = self.stopped = False; self.pending = None; self.steps = 0
        assert not select.select([pidfd], [], [], 0)[0]
        assert Path(f'/proc/{pid}/task/{pid}/children').read_text().split() == [str(worker_pid)]
        self.channel = main_channel(pid)
        self.record = {'traced_pid': pid, 'one_fixture_owned_Main_only': True,
            'channel': self.channel, 'source_role_proof': source_proof,
            'packet_payload_or_process_memory_read': False, 'syscall_results_changed': False,
            'effect_resubmission': False, 'original_until_ns': until,
            'original_close_until_ns': close_until, 'trace_stops': []}

    def detach(self):
        super().detach()
        self.record.pop('normal_Main_retains_terminal_wait', None)
        self.record['Scope_retains_Main_terminal_wait'] = True

    def worker_silence(self):
        assert not select.select([self.worker_pidfd], [], [], 0)[0]
        text = Path(f'/proc/{self.worker_pid}/status').read_text()
        assert next(line.split()[1] for line in text.splitlines() if line.startswith('State:')) == 'T'
        assert int(next(line.split()[1] for line in text.splitlines() if line.startswith('TracerPid:'))) == 0
        return {'group_stop_T': True, 'TracerPid': 0, 'original_pidfd_nonterminal': True,
                'observed_at_ns': now()}

    def run(self, mode, admit_apply=None):
        assert mode in ['idle', 'consumed'] and admit_apply is None
        self.attach(); phase = 'no-data'; held = False; after = []
        while now() < self.until:
            status = self.wait(); sig, event = os.WSTOPSIG(status), status >> 16
            if event == EVENT_EXIT:
                self.record['Main_exit_event_before_terminal_wait'] = True
                self.detach(); return self.record
            if sig != signal.SIGTRAP | 0x80:
                assert event == 0
                self.resume(0 if sig == signal.SIGTRAP else sig); continue
            op, value, args = self.info()
            if op == 1:
                self.pending = {'nr': value, 'args': args, 'entry_ns': now()}
                if value == RECVMSG and args[0] == self.channel['fd'] and held:
                    after.append({'entry_ns': now(), 'fd': args[0]})
                if mode == 'idle' and not held and phase == 'next-thread-check' and value == GETTID:
                    assert main_channel(self.pid) == self.channel
                    self.record['idle_gate_attribution'] = {
                        'prior_actual_no_data': self.record['no_data'],
                        'loop_sleep_exit_ns': self.record['loop_sleep_exit_ns'],
                        'stopped_syscall': 'gettid entry',
                        'source_role': 'baseline receive pre-auth or candidate readiness local check'}
                    self.record['worker_silence_before_hold'] = self.worker_silence()
                    self.hold(80_000_000, 'silent-worker-before-Main-auth')
                    self.record['worker_silence_after_hold_before_Main_resume'] = self.worker_silence()
                    held = True
                elif held and value in SLEEPS:
                    self.record['post_hold_private_receives'] = after
                    self.record['returned_to_Main_loop_sleep_entry'] = True
                    try:
                        channel = os.readlink(f'/proc/{self.pid}/fd/{self.channel["fd"]}')
                    except FileNotFoundError:
                        channel = None  # A baseline retirement closes this channel.
                    assert channel is None or channel == f'socket:[{self.channel["inode"]}]'
                    self.record['private_channel_closed_after_hold'] = channel is None
                    self.detach(); return self.record
            else:
                entry = self.pending; self.pending = None
                if entry is not None:
                    nr, args, at = entry['nr'], entry['args'], now()
                    if not held and mode == 'idle':
                        empty_receive = nr == RECVMSG and args[0] == self.channel['fd'] and value == -11
                        empty_hint = nr in [POLL, PPOLL] and args[1] == 3 and value == 0
                        if empty_receive or empty_hint:
                            self.record['no_data'] = {'kind': 'private_recvmsg_EAGAIN' if empty_receive else 'source_qualified_three_fd_readiness_zero',
                                'syscall': nr, 'return_value': value, 'entry_ns': entry['entry_ns'],
                                'exit_ns': at, 'nfds': args[1] if empty_hint else None,
                                'contents_inspected': False}
                            phase = 'loop-sleep'
                        elif nr == RECVMSG and args[0] == self.channel['fd'] and value > 0:
                            phase = 'no-data'  # A success is never an idle anchor.
                    if nr in SLEEPS and value == 0 and phase == 'loop-sleep':
                        self.record['loop_sleep_exit_ns'] = at; phase = 'next-thread-check'
                    if nr == RECVMSG and args[0] == self.channel['fd']:
                        if held and after:
                            after[-1].update(return_value=value, exit_ns=at)
                        if mode == 'consumed' and not held and value in [48, 61]:
                            self.record['actual_consumed_frame'] = {'fd': args[0], 'bytes': value,
                                'entry_ns': entry['entry_ns'], 'exit_stop_at_ns': at, 'contents_inspected': False}
                            self.hold(80_000_000, 'consumed-Main-receive-envelope-expiry'); held = True
            self.resume()
        raise TimeoutError('original bounded Main trace exhausted')
