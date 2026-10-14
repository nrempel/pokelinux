#!/usr/bin/env python3
"""Private two-group actual ARM recursive-scope observer; no authentication."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import select
import signal
import stat
import struct
import subprocess
import time

R = Path('/results')
I = Path('/run/pokelinux-auth-scope-test')
workload = None
# This internal driver bound fits the unchanged 35s service-CLI/80s guest bound.
end = time.monotonic() + 30
out = {'schema': 2, 'passed': False, 'feature_GREEN': False, 'groups': [],
       'outer_pidfd_cleanup_used': False, 'migration_containment_qualified': False,
       'original_helper_scope_composition_qualified': False}

def require(value, label):
    if not value:
        raise RuntimeError(label)


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def live(fd):
    return not select.select([fd], [], [], 0)[0]


def membership(pid):
    lines = Path(f'/proc/{pid}/cgroup').read_text().splitlines()
    require(len(lines) == 1 and lines[0].startswith('0::/'), 'unified_membership')
    return lines[0][3:]


def populated():
    values = dict(line.split() for line in (workload / 'cgroup.events').read_text().splitlines())
    require(values['populated'] in ('0', '1'), 'populated_encoding')
    return int(values['populated'])


def procs():
    return sorted(map(int, (workload / 'cgroup.procs').read_text().split()))


def process(pid):
    base = Path('/proc') / str(pid)
    text = (base / 'stat').read_text()
    fields = text[text.rfind(')') + 2:].split()
    values = dict(line.split(':', 1) for line in (base / 'status').read_text().splitlines()
                  if ':' in line)
    return {'pid': pid, 'ppid': int(fields[1]), 'pgid': int(fields[2]),
            'sid': int(fields[3]), 'start_ticks': int(fields[19]), 'state': fields[0],
            'uids': list(map(int, values['Uid'].split())),
            'gids': list(map(int, values['Gid'].split())),
            'groups': list(map(int, values.get('Groups', '').split())),
            'caps': {key: values[key].strip() for key in
                     ('CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb')},
            'NNP': int(values['NoNewPrivs']), 'membership': membership(pid)}


def provision():
    global workload
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    libc = ctypes.CDLL(None, use_errno=True)
    require(os.getuid() == os.geteuid() == 0 and libc.prctl(36, 1, 0, 0, 0) == 0,
            'dedicated_driver_root_subreaper')
    member = membership(os.getpid())
    require(member.endswith('/pokelinux-auth-scope-test.service'), 'fixed_delegated_service')
    root = Path('/sys/fs/cgroup' + member)
    require(any(' /sys/fs/cgroup ' in row and ' - cgroup2 ' in row
                for row in Path('/proc/self/mountinfo').read_text().splitlines()), 'actual_cgroup2_mount')
    command = ['/usr/bin/systemctl', 'show', 'pokelinux-auth-scope-test.service',
               '--property=Delegate,ControlGroup,ProtectControlGroups,KillMode,NoNewPrivileges,MainPID']
    show = subprocess.run(command, capture_output=True, text=True, timeout=3, check=True)
    props = dict(line.split('=', 1) for line in show.stdout.splitlines())
    out['actual_unit_properties'] = props
    require(props['Delegate'] == 'yes' and props['ControlGroup'] == member
            and props['ProtectControlGroups'] == 'no' and props['NoNewPrivileges'] == 'no'
            and int(props['MainPID']) == os.getpid(), 'actual_private_delegation')
    require(list(map(int, (root / 'cgroup.procs').read_text().split())) == [os.getpid()],
            'only_exact_driver_in_delegated_root')
    (root / 'owner').mkdir(mode=0o755)
    (root / 'workload').mkdir(mode=0o755)
    (root / 'owner/cgroup.procs').write_text(str(os.getpid()) + '\n')
    require(membership(os.getpid()) == member + '/owner'
            and not (root / 'cgroup.procs').read_text().strip(), 'move_only_exact_owned_driver')
    available = (root / 'cgroup.controllers').read_text().split()
    enabled = [name for name in ('memory', 'pids') if name in available]
    if enabled:
        (root / 'cgroup.subtree_control').write_text(' '.join('+' + name for name in enabled) + '\n')
    workload = root / 'workload'
    out['controllers'] = {'available': available,
                          'enabled': (root / 'cgroup.subtree_control').read_text().split(),
                          'cgroup_kill_depends_on_pids_controller': False}
    limits = {'cgroup.max.depth': '0', 'cgroup.max.descendants': '0'}
    if 'pids' in enabled:
        limits['pids.max'] = '8'
    if 'memory' in enabled:
        limits.update({'memory.max': '134217728', 'memory.swap.max': '0'})
    out['limits'] = {}
    for name, value in limits.items():
        (workload / name).write_text(value + '\n')
        actual = (workload / name).read_text().strip()
        require(actual == value, 'limit_readback')
        out['limits'][name] = actual
    for path in (root, root / 'owner', workload, workload / 'cgroup.procs',
                 workload / 'cgroup.kill', workload / 'cgroup.events'):
        metadata = path.lstat()
        require(metadata.st_uid == 0 and not metadata.st_mode & 0o022
                and not stat.S_ISLNK(metadata.st_mode), 'cgroup_controls_root_owned')
    require(populated() == 0 and procs() == [], 'initial_empty_private_leaf')


def leaf_sample():
    value = {'at_BOOTTIME_ns': now(), 'populated': populated(), 'procs': procs()}
    metadata = workload.stat()
    value['directory_identity'] = {'dev': metadata.st_dev, 'ino': metadata.st_ino}
    pids = workload / 'pids.current'
    value['pids_current'] = int(pids.read_text()) if pids.exists() else None
    return value


class Attempt:
    def __init__(self, name, prior):
        self.data = {'case': name, 'passed': False, 'closed': False, 'records': [],
                     'readiness': [], 'hold_signals': [], 'independent_SIGKILL': [],
                     'outer_adopted_waits': [], 'prior_case_fully_closed': prior}
        self.child = None
        self.handles = {}
        self.scope_buffer = bytearray()
        self.ready_buffer = bytearray()
        self.stdout_eof = False
        self.ready_eof = False
        self.ready_read = None
        self.supervisor = None

    def own(self, pid, label):
        require(pid not in self.handles, 'duplicate_owned_pid')
        fd = os.pidfd_open(pid, 0)
        require(live(fd), 'live_pidfd_at_capture')
        self.handles[pid] = (fd, label)
        return fd

    def latest(self, event):
        return next((row for row in reversed(self.data['records']) if row['event'] == event), None)

    def pump(self, seconds=0.01):
        require(time.monotonic() < end, 'delegated_outer_bound')
        fds = []
        if not self.stdout_eof:
            fds.append(self.child.stdout.fileno())
        if not self.ready_eof:
            fds.append(self.ready_read)
        for fd in select.select(fds, [], [], seconds)[0]:
            data = os.read(fd, 4096)
            if fd == self.child.stdout.fileno():
                self.stdout_eof |= not data
                self.scope_buffer.extend(data)
            else:
                self.ready_eof |= not data
                self.ready_buffer.extend(data)
        while len(self.scope_buffer) >= 48:
            data = bytes(self.scope_buffer[:48])
            del self.scope_buffer[:48]
            magic, event, flags, pid, status, adopted, retired, observed, reserved = struct.unpack('<8sIIIIIIQQ', data)
            require(magic == b'PKSCOPE2' and reserved == 0 and event in (1, 2, 3, 4, 5), 'current_scope_frame')
            require(flags & ~(0x1fff if event != 5 else 0x79) == 0, 'current_scope_flags')
            require(2 <= pid <= 2147483647, 'scope_pid')
            if event in (2, 3):
                require(flags == adopted == retired == 0, 'wait_frame_metadata')
            if event in (1, 4):
                require(not flags & 2, 'no_typed_candidate_in_canceled_synthetic_workload')
                require(not flags & 4096 or flags & 2048, 'scope_kill_return_requires_attempt')
                require(not flags & 2048 or retired != 0, 'scope_kill_requires_retirement')
            self.data['records'].append({'event': event, 'flags': flags, 'pid': pid,
                                         'raw_wait_status': status, 'adopted': adopted,
                                         'retired': retired, 'at_BOOTTIME_ns': observed})
            require(len(self.data['records']) <= 128, 'record_bound')
        while len(self.ready_buffer) >= 40:
            data = bytes(self.ready_buffer[:40])
            del self.ready_buffer[:40]
            magic, version, role, pid, ppid, sid, pgid, observed = struct.unpack('<8sIIIIIIQ', data)
            require(magic == b'PAMOWN1\0' and version == 1 and role in (1, 2, 3), 'module_frame')
            require(not any(row['role'] == role for row in self.data['readiness']), 'duplicate_module_role')
            self.own(pid, f'module_role_{role}')
            self.data['readiness'].append({'role': role, 'pid': pid, 'ppid': ppid,
                                           'sid': sid, 'pgid': pgid, 'at_BOOTTIME_ns': observed})
        require(not self.stdout_eof or not self.scope_buffer, 'truncated_scope_frame')
        require(not self.ready_eof or not self.ready_buffer, 'truncated_module_frame')

    def wait_for(self, condition, seconds, label):
        until = min(time.monotonic() + seconds, end)
        while not condition():
            require(time.monotonic() < until, label)
            self.pump()

    def send(self, data):
        require(os.write(self.child.stdin.fileno(), data) == len(data), 'private_parent_command_write')

    def start(self):
        initial = leaf_sample()
        require(initial['populated'] == 0 and initial['procs'] == []
                and initial['pids_current'] in (0, None), 'closed_leaf_before_fresh_attempt')
        self.data['admission_leaf_sample'] = initial
        self.data['original_until_BOOTTIME_ns'] = now() + 15_000_000_000
        self.ready_read, ready_write = os.pipe2(os.O_CLOEXEC | os.O_NONBLOCK)
        try:
            self.child = subprocess.Popen([str(I / 'auth-scope-fixture'), '--isolated-parent',
                '--parent', str(os.getpid()), '--until', str(self.data['original_until_BOOTTIME_ns'])],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=ready_write, env={}, bufsize=0)
        finally:
            os.close(ready_write)
        self.own(self.child.pid, 'native_parent')
        os.set_blocking(self.child.stdout.fileno(), False)
        os.set_blocking(self.child.stdin.fileno(), False)
        self.wait_for(lambda: len(self.data['readiness']) == 3 and self.latest(5) is not None
                      and self.latest(1) is not None and self.latest(1)['flags'] & 0x84 == 0x84,
                      5, 'actual_three_live_ready_roles')
        roles = {row['role']: row for row in self.data['readiness']}
        self.worker, self.descendant, self.grandchild = (roles[n]['pid'] for n in (1, 2, 3))
        tasks = (self.worker, self.descendant, self.grandchild)
        require(len(set(tasks)) == 3 and self.latest(1)['pid'] == self.worker, 'exact_worker_binding')
        self.supervisor = self.latest(5)['pid']
        self.own(self.supervisor, 'native_supervisor')
        actual = {pid: process(pid) for pid in tasks}
        supervisor = process(self.supervisor)
        require(actual[self.worker]['ppid'] == self.supervisor
                and supervisor['ppid'] == self.child.pid, 'owned_native_parent_chain')
        for role, pid in enumerate(tasks, 1):
            row = actual[pid]
            require(all(row[key] == roles[role][key] for key in ('pid', 'ppid', 'sid', 'pgid')),
                    'independent_proc_ready_identity')
            require(row['membership'] == str(workload).removeprefix('/sys/fs/cgroup')
                    and live(self.handles[pid][0]), 'all_tasks_in_exact_leaf')
            require(row['uids'] == [1000] * 4 and row['gids'] == [1000] * 4
                    and row['groups'] == [] and row['NNP'] == 0
                    and all(int(value, 16) == (1 << 29 if key == 'CapBnd' else 0)
                            for key, value in row['caps'].items()), 'fixed_synthetic_worker_roles')
        require(actual[self.descendant]['ppid'] == self.worker
                and actual[self.descendant]['sid'] == actual[self.descendant]['pgid'] == self.descendant
                and actual[self.grandchild]['ppid'] == self.descendant
                and actual[self.grandchild]['sid'] == actual[self.grandchild]['pgid'] == self.descendant,
                'actual_fork_setsid_grandchild_chain')
        require(supervisor['membership'] == membership(os.getpid())
                and supervisor['membership'].endswith('/owner')
                and procs() == sorted(tasks) and populated() == 1, 'outside_supervisor_private_leaf')
        self.data['live_process_qualification'] = list(actual.values())
        self.data['live_supervisor_qualification'] = supervisor
        self.data['setup_prerequisites_complete'] = True

    def hold(self):
        fd = self.handles[self.supervisor][0]
        signal.pidfd_send_signal(fd, signal.SIGSTOP)
        self.data['hold_signals'].append({'pid': self.supervisor, 'signal': 'SIGSTOP',
                                          'at_BOOTTIME_ns': now(), 'exact_owned_pidfd': True})
        self.wait_for(lambda: process(self.supervisor)['state'] == 'T', 1, 'actual_supervisor_stopped_T')
        self.data['stopped_supervisor_sample'] = process(self.supervisor)
        require(not self.latest(1)['flags'] & 0x1800, 'no_scope_kill_before_hold')

    def held_sample(self):
        tasks = (self.worker, self.descendant, self.grandchild)
        processes = [process(pid) for pid in tasks]
        supervisor = process(self.supervisor)
        leaf = leaf_sample()
        require(supervisor['state'] == 'T' and live(self.handles[self.supervisor][0])
                and all(live(self.handles[pid][0]) for pid in tasks)
                and leaf['populated'] == 1 and leaf['procs'] == sorted(tasks), 'all_three_live_while_executor_stopped')
        require(self.latest(5)['flags'] == 1 and not self.latest(1)['flags'] & 0x1800,
                'parent_occupied_and_scope_kill_unexecuted_while_held')
        return {'at_BOOTTIME_ns': now(), 'processes': processes, 'supervisor': supervisor,
                'leaf': leaf, 'parent_state': self.latest(5), 'scope_state': self.latest(1)}

    def exercise(self, deadline):
        self.hold()
        if deadline:
            original = self.data['original_until_BOOTTIME_ns']
            remaining = max(0, original - now()) / 1e9
            self.wait_for(lambda: self.latest(5)['retired'] == 3, remaining + .25,
                          'native_parent_original_deadline_retirement')
            first = next(row for row in self.data['records'] if row['event'] == 5 and row['retired'] == 3)
            require(first['at_BOOTTIME_ns'] >= original and first['flags'] == 1,
                    'deadline_retired_locally_while_occupied')
            self.data['first_parent_deadline_retirement'] = first
            while now() < original + 100_000_000:
                self.pump()
            before = len(self.data['records'])
            at = now()
            self.send(b'?')
            self.wait_for(lambda: any(row['event'] == 5 for row in self.data['records'][before:]),
                          .25, 'deadline_held_parent_snapshot_responsive')
            self.data['snapshot_response_ns'] = now() - at
            require(0 <= self.data['snapshot_response_ns'] <= 250_000_000, 'deadline_snapshot_within_original_250ms')
            require(self.latest(5)['retired'] == 3, 'local_deadline_reason_preserved')
        else:
            before = len(self.data['records'])
            at = now()
            self.send(b'C')
            self.wait_for(lambda: any(row['event'] == 5 and row['retired'] == 1
                                      for row in self.data['records'][before:]),
                          .25, 'Cancel_responsive_while_executor_stopped')
            self.data['cancel_response_ns'] = now() - at
            require(0 <= self.data['cancel_response_ns'] <= 250_000_000, 'Cancel_response_within_original_250ms')
            before = len(self.data['records'])
            at = now()
            self.send(b'S?')
            self.wait_for(lambda: any(row['event'] == 5 for row in self.data['records'][before:]),
                          .25, 'Stop_snapshot_responsive_while_executor_stopped')
            self.data['Stop_snapshot_response_ns'] = now() - at
            require(0 <= self.data['Stop_snapshot_response_ns'] <= 250_000_000, 'Stop_snapshot_within_original_250ms')
            require(self.latest(5)['retired'] == 1, 'Cancel_reason_preserved_after_Stop')
            until = time.monotonic() + .1
            while time.monotonic() < until:
                self.pump()
            require(now() < self.data['original_until_BOOTTIME_ns'], 'Cancel_within_original_deadline')
        self.data['held_retired_sample'] = self.held_sample()
        self.data['resume_BOOTTIME_ns'] = now()
        signal.pidfd_send_signal(self.handles[self.supervisor][0], signal.SIGCONT)
        self.data['hold_signals'].append({'pid': self.supervisor, 'signal': 'SIGCONT',
                                          'at_BOOTTIME_ns': now(), 'exact_owned_pidfd': True})
        self.wait_for(lambda: self.stdout_eof and self.ready_eof and self.latest(5)['flags'] & 1 == 0,
                      3, 'actual_recursive_native_closure_before_outer_kill')
        self.finish(deadline)

    def finish(self, deadline):
        code = self.child.wait(timeout=2)
        self.data['native_parent_actual_direct_wait'] = {'pid': self.child.pid, 'exit': code,
                                                       'at_BOOTTIME_ns': now()}
        require(code == 0 and not self.scope_buffer and not self.ready_buffer, 'actual_parent_wait0_and_complete_framing')
        records = self.data['records']
        worker_waits = [row for row in records if row['event'] == 2]
        adopted = [row for row in records if row['event'] == 3]
        closed = [row for row in records if row['event'] == 4]
        require(len(worker_waits) == 1 and worker_waits[0]['pid'] == self.worker
                and worker_waits[0]['raw_wait_status'] == 9, 'native_worker_actual_wait9')
        require(sorted(row['pid'] for row in adopted) == sorted((self.descendant, self.grandchild))
                and all(row['raw_wait_status'] == 9 for row in adopted), 'native_two_adopted_actual_waits9')
        require(len(closed) == 1 and closed[0]['flags'] == 0x1f9c and closed[0]['adopted'] == 2
                and closed[0]['retired'] in ((1, 3) if deadline else (1,)),
                'recursive_kill_return_global_ECHILD_populated0_worker_EOF')
        require(all(row['pid'] == self.worker for row in records if row['event'] in (1, 4))
                and all(row['pid'] == self.supervisor for row in records if row['event'] == 5),
                'one_exact_native_attempt')
        terminal_index = records.index(closed[0])
        require(all(records.index(row) < terminal_index for row in worker_waits + adopted)
                and not any(row['event'] in (1, 2, 3, 4) for row in records[terminal_index + 1:]),
                'all_actual_receipts_before_single_terminal')
        require(self.latest(5)['flags'] == 0x38 and self.latest(5)['retired'] == (3 if deadline else 1),
                'native_parent_actual_supervisor_wait0_terminal_EOF')
        require(all(row['at_BOOTTIME_ns'] >= self.data['resume_BOOTTIME_ns']
                    for row in worker_waits + adopted), 'actual_deaths_after_resume_not_outer_kill')
        self.data['final_leaf_sample'] = leaf_sample()
        require(self.data['final_leaf_sample']['populated'] == 0
                and self.data['final_leaf_sample']['procs'] == []
                and self.data['final_leaf_sample']['pids_current'] in (0, None), 'independent_final_leaf_empty')
        self.data['final_owned_process_samples'] = [
            {'pid': pid, 'label': label, 'pidfd_dead': not live(fd),
             'proc_absent': not Path(f'/proc/{pid}').exists(), 'at_BOOTTIME_ns': now()}
            for pid, (fd, label) in self.handles.items()]
        require(all(row['pidfd_dead'] and row['proc_absent'] for row in self.data['final_owned_process_samples']),
                'all_five_owned_processes_dead_and_proc_absent')
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
            if pid:
                self.data['outer_adopted_waits'].append({'pid': pid, 'status': status, 'actual_waitpid': True})
            raise RuntimeError('driver_expected_true_global_ECHILD')
        except ChildProcessError:
            self.data['driver_final_ECHILD'] = True
        require(not self.data['independent_SIGKILL'], 'outer_fallback_cannot_qualify_GREEN')
        self.data['closed'] = self.data['passed'] = True
        self.data['completed_BOOTTIME_ns'] = now()

    def kill_failure(self, pid):
        fd, label = self.handles[pid]
        if live(fd):
            signal.pidfd_send_signal(fd, signal.SIGKILL)
            self.data['independent_SIGKILL'].append({'pid': pid, 'label': label,
                'reason': 'failed_fixture_cleanup_not_GREEN', 'at_BOOTTIME_ns': now()})
            out['outer_pidfd_cleanup_used'] = True

    def cleanup_failure(self):
        if self.supervisor is not None and live(self.handles[self.supervisor][0]):
            signal.pidfd_send_signal(self.handles[self.supervisor][0], signal.SIGCONT)
            self.data['hold_signals'].append({'pid': self.supervisor, 'signal': 'SIGCONT',
                'reason': 'failed_fixture_unblock_not_GREEN', 'at_BOOTTIME_ns': now()})
        if workload is not None:
            for pid in procs():
                if pid not in self.handles:
                    self.own(pid, 'private_leaf_failed_fixture_cleanup')
        for pid, (_, label) in list(self.handles.items()):
            if label not in ('native_parent', 'native_supervisor'):
                self.kill_failure(pid)
        until = time.monotonic() + 2
        while self.child is not None and self.child.poll() is None and time.monotonic() < until:
            try:
                self.pump()
            except BaseException:
                break
        for pid, (_, label) in list(self.handles.items()):
            if label in ('native_parent', 'native_supervisor'):
                self.kill_failure(pid)
        if self.child is not None:
            self.data['failed_native_parent_actual_wait'] = self.child.wait(timeout=2)
        until = time.monotonic() + 2
        while True:
            try:
                pid, status = os.waitpid(-1, os.WNOHANG)
            except ChildProcessError:
                self.data['failure_cleanup_driver_global_ECHILD'] = True
                break
            if pid:
                self.data['outer_adopted_waits'].append({'pid': pid, 'status': status, 'actual_waitpid': True})
            else:
                require(time.monotonic() < until, 'failed_cleanup_wait_bound')
                time.sleep(.005)

    def close_handles(self):
        for fd, _ in self.handles.values():
            os.close(fd)
        if self.ready_read is not None:
            os.close(self.ready_read)
        if self.child is not None:
            self.child.stdin.close()
            self.child.stdout.close()


attempt = None
try:
    provision()
    prior = None
    for name, deadline in [('cancel_held_executor', False), ('deadline_held_executor', True)]:
        attempt = Attempt(name, prior)
        out['groups'].append(attempt.data)
        attempt.start()
        attempt.exercise(deadline)
        prior = {'case': name, 'closed': True, 'completed_BOOTTIME_ns': attempt.data['completed_BOOTTIME_ns'],
                 'native_parent_wait0': True, 'native_adopted_waits': 2, 'driver_global_ECHILD': True}
        with (R / (name + '-result.json')).open('x') as output:
            json.dump(attempt.data, output, indent=2)
            output.write('\n')
        attempt.close_handles()
        attempt = None
    out['passed'] = out['feature_GREEN'] = (len(out['groups']) == 2
        and all(group['passed'] and group['closed'] for group in out['groups'])
        and not out['outer_pidfd_cleanup_used'])
except BaseException as error:
    out['error_type'] = type(error).__name__
    if isinstance(error, RuntimeError):
        out['error_code'] = str(error)
    if attempt is not None:
        attempt.data['error_type'] = type(error).__name__
        if isinstance(error, RuntimeError):
            attempt.data['error_code'] = str(error)
finally:
    if attempt is not None:
        try:
            attempt.cleanup_failure()
        except BaseException as error:
            out['cleanup_error_type'] = type(error).__name__
        attempt.close_handles()
    with (R / 'delegated-result.json').open('x') as output:
        json.dump(out, output, indent=2)
        output.write('\n')
