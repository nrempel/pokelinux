#!/usr/bin/env python3
"""One owned no-workload setup-error case, with a real held child and pipes."""
import ctypes
import fcntl
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
import termios
import time

R = Path('/results')
I = Path('/run/pokelinux-auth-scope-test')
end = time.monotonic() + 30
workload = None
child = None
supervisor = None
handles = {}
frames = bytearray()
stdout_eof = False
out = {'schema': 1, 'passed': False, 'qualified_setup_RED': False,
       'qualified_setup_GREEN': False,
       'case': 'injected_missing_pidfd_held_supervisor', 'records': [],
       'independent_teardown': [], 'adopted_waits': [],
       'no_worker_or_PAM': True, 'injected_absent_handle': True,
       'actual_syscall_denial_or_resource_failure': False,
       'ptrace_used': False, 'seccomp_injection_used': False}


def require(value, label):
    if not value:
        raise RuntimeError(label)


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write(name, value):
    with (R / name).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def live(fd):
    return not select.select([fd], [], [], 0)[0]


def membership(pid):
    rows = Path(f'/proc/{pid}/cgroup').read_text().splitlines()
    require(len(rows) == 1 and rows[0].startswith('0::/'), 'unified_cgroup_membership')
    return rows[0][3:]


def process(pid):
    root = Path('/proc') / str(pid)
    text = (root / 'stat').read_text()
    fields = text[text.rfind(')') + 2:].split()
    status = dict(line.split(':', 1) for line in (root / 'status').read_text().splitlines() if ':' in line)
    return {'pid': pid, 'ppid': int(fields[1]), 'state': fields[0],
            'pgid': int(fields[2]), 'sid': int(fields[3]), 'start_ticks': int(fields[19]),
            'TracerPid': int(status['TracerPid']), 'uids': list(map(int, status['Uid'].split())),
            'gids': list(map(int, status['Gid'].split())), 'membership': membership(pid),
            'exe_sha256': sha(root / 'exe'), 'argv': (root / 'cmdline').read_bytes().split(b'\0')[:-1]}


def serial_process(pid):
    row = process(pid)
    row['argv'] = [word.decode('ascii') for word in row['argv']]
    return row


def own(pid, label):
    require(pid not in handles, 'duplicate_owned_pid')
    before = process(pid)
    fd = os.pidfd_open(pid, 0)
    after = process(pid)
    require(before['start_ticks'] == after['start_ticks'] and before['ppid'] == after['ppid']
            and live(fd), 'exact_live_pidfd_capture')
    handles[pid] = (fd, label)


def children(pid):
    return list(map(int, Path(f'/proc/{pid}/task/{pid}/children').read_text().split()))


def leaf_sample():
    values = dict(line.split() for line in (workload / 'cgroup.events').read_text().splitlines())
    metadata = workload.stat()
    return {'at_BOOTTIME_ns': now(), 'populated': int(values['populated']),
            'procs': sorted(map(int, (workload / 'cgroup.procs').read_text().split())),
            'directory_identity': {'dev': metadata.st_dev, 'ino': metadata.st_ino}}


def empty_leaf(label):
    row = leaf_sample()
    require(row['populated'] == 0 and row['procs'] == [], label)
    return row


def provision():
    global workload
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    libc = ctypes.CDLL(None, use_errno=True)
    require(os.getuid() == os.geteuid() == 0 and libc.prctl(36, 1, 0, 0, 0) == 0,
            'root_driver_subreaper')
    member = membership(os.getpid())
    require(member.endswith('/pokelinux-auth-scope-test.service'), 'fixed_private_service')
    require(any(' /sys/fs/cgroup ' in row and ' - cgroup2 ' in row
                for row in Path('/proc/self/mountinfo').read_text().splitlines()), 'actual_cgroup2')
    observed = subprocess.run(['/usr/bin/systemctl', 'show', 'pokelinux-auth-scope-test.service',
        '--property=Delegate,ControlGroup,ProtectControlGroups,KillMode,NoNewPrivileges,MainPID'],
        capture_output=True, text=True, timeout=3, check=True)
    props = dict(line.split('=', 1) for line in observed.stdout.splitlines())
    out['actual_unit_properties'] = props
    require(props['Delegate'] == 'yes' and props['ControlGroup'] == member
            and props['ProtectControlGroups'] == 'no' and props['NoNewPrivileges'] == 'no'
            and props['KillMode'] == 'control-group' and int(props['MainPID']) == os.getpid(),
            'actual_private_delegation')
    root = Path('/sys/fs/cgroup' + member)
    require(list(map(int, (root / 'cgroup.procs').read_text().split())) == [os.getpid()],
            'only_owned_driver_in_delegated_root')
    (root / 'owner').mkdir(mode=0o755)
    (root / 'workload').mkdir(mode=0o755)
    (root / 'owner/cgroup.procs').write_text(str(os.getpid()) + '\n')
    require(membership(os.getpid()) == member + '/owner'
            and not (root / 'cgroup.procs').read_text().strip(), 'move_only_owned_driver')
    workload = root / 'workload'
    for path in [root, root / 'owner', workload, workload / 'cgroup.procs', workload / 'cgroup.events']:
        metadata = path.lstat()
        require(metadata.st_uid == 0 and not metadata.st_mode & 0o022
                and not stat.S_ISLNK(metadata.st_mode), 'root_owned_private_controls')
    out['initial_leaf_sample'] = empty_leaf('initial_empty_workload')


def pump(seconds=0):
    global stdout_eof
    require(time.monotonic() < end, 'driver_bound')
    if not stdout_eof and select.select([child.stdout], [], [], seconds)[0]:
        value = os.read(child.stdout.fileno(), 4096)
        stdout_eof |= not value
        frames.extend(value)
    while len(frames) >= 48:
        magic, event, flags, pid, status, adopted, retired, observed, reserved = struct.unpack(
            '<8sIIIIIIQQ', bytes(frames[:48]))
        del frames[:48]
        require(magic == b'PKSCOPE2' and reserved == 0 and event == 5,
                'only_parent_records_in_no_workload_case')
        require(flags & ~0x79 == 0 and 2 <= pid <= 2147483647 and adopted == 0,
                'current_parent_record_encoding')
        require(flags == 0x41 and retired == 6 and status == 0,
                'sticky_occupied_Setup_fault_never_closed')
        require(supervisor is None or pid == supervisor, 'exact_parent_record_supervisor')
        out['records'].append({'event': event, 'flags': flags, 'pid': pid,
            'raw_wait_status': status, 'adopted': adopted, 'retired': retired,
            'at_BOOTTIME_ns': observed, 'received_BOOTTIME_ns': now()})
        require(len(out['records']) <= 32, 'record_bound')
    require(not stdout_eof or not frames, 'no_truncated_parent_frame')


def wait_for(predicate, seconds, label):
    until = min(time.monotonic() + seconds, end)
    while not predicate():
        require(time.monotonic() < until, label)
        pump(.002)


def blocked_sample():
    parent = serial_process(child.pid)
    held = serial_process(supervisor)
    require(parent['state'] != 'T' and parent['TracerPid'] == held['TracerPid'] == 0,
            'native_parent_never_stopped_or_traced')
    pipe = os.readlink(f'/proc/{supervisor}/fd/1')
    require(pipe.startswith('pipe:[') and pipe.endswith(']'), 'held_stdout_is_pipe')
    targets = []
    for path in Path(f'/proc/{child.pid}/fd').iterdir():
        if os.readlink(path) == pipe:
            values = dict(line.split(':', 1) for line in
                          Path(f'/proc/{child.pid}/fdinfo/{path.name}').read_text().splitlines())
            flags = int(values['flags'].strip(), 8)
            if flags & os.O_ACCMODE == os.O_RDONLY:
                targets.append({'fd': int(path.name), 'pipe': pipe,
                    'inode': int(values['ino']), 'flags_octal': values['flags'].strip(),
                    'O_NONBLOCK': bool(flags & os.O_NONBLOCK)})
    require(len(targets) <= 1, 'unique_supervisor_stdout_parent_read_fd')
    target = targets[0] if targets else None
    wchan = Path(f'/proc/{child.pid}/wchan').read_text().strip()
    raw = Path(f'/proc/{child.pid}/syscall').read_text().split()
    syscall = {'number': None, 'fd': None}
    if raw and raw[0] != 'running':
        syscall['number'] = int(raw[0])
        if syscall['number'] == 63:
            syscall['fd'] = int(raw[1], 0)
    causal = bool(target and parent['state'] == 'S' and held['state'] == 'T'
        and not target['O_NONBLOCK'] and (wchan == 'pipe_read'
        or syscall == {'number': 63, 'fd': target['fd']}))
    return {'at_BOOTTIME_ns': now(), 'parent': parent, 'held_supervisor': held,
            'parent_wchan': wchan, 'parent_syscall': syscall, 'target_pipe_read_fd': target,
            'causal_blocked_read': causal, 'leaf': empty_leaf('empty_leaf_during_setup_fault')}


def command_pipe_sample():
    # FIONREAD observes queued input through our still-owned writer. The native
    # parent must retain the matching nonblocking read end at its real stdin.
    writer = child.stdin.fileno()
    pipe = os.readlink(f'/proc/self/fd/{writer}')
    require(pipe.startswith('pipe:[') and pipe.endswith(']'), 'owned_command_writer_is_pipe')
    parent_pipe = os.readlink(f'/proc/{child.pid}/fd/0')
    values = dict(line.split(':', 1) for line in
                  Path(f'/proc/{child.pid}/fdinfo/0').read_text().splitlines())
    flags = int(values['flags'].strip(), 8)
    pending = struct.unpack('i', fcntl.ioctl(writer, termios.FIONREAD, struct.pack('i', 0)))[0]
    sample = {'at_BOOTTIME_ns': now(), 'owned_writer_fd': writer, 'owned_writer_pipe': pipe,
        'owned_writer_inode': os.fstat(writer).st_ino, 'native_parent_stdin_fd': 0,
        'native_parent_stdin_pipe': parent_pipe, 'native_parent_stdin_inode': int(values['ino']),
        'native_parent_stdin_flags_octal': values['flags'].strip(),
        'native_parent_stdin_read_only': flags & os.O_ACCMODE == os.O_RDONLY,
        'native_parent_stdin_O_NONBLOCK': bool(flags & os.O_NONBLOCK), 'FIONREAD_queued_bytes': pending}
    require(parent_pipe == pipe and sample['native_parent_stdin_inode'] == sample['owned_writer_inode']
            and sample['native_parent_stdin_read_only'] and sample['native_parent_stdin_O_NONBLOCK'],
            'actual_retained_matching_nonblocking_native_command_read_end')
    require(pending == 0, 'actual_queued_Stop_snapshot_consumed')
    return sample


def run_case():
    global child, supervisor
    selected = json.loads((R / 'selection.json').read_text())
    out['original_until_BOOTTIME_ns'] = now() + 15_000_000_000
    argv = [str(I / 'auth-scope-fixture'), '--missing-pidfd-parent', '--parent', str(os.getpid()),
            '--until', str(out['original_until_BOOTTIME_ns'])]
    child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.DEVNULL, env={}, bufsize=0)
    own(child.pid, 'native_parent')
    os.set_blocking(child.stdin.fileno(), False)
    os.set_blocking(child.stdout.fileno(), False)
    wait_for(lambda: bool(children(child.pid)), 3, 'actual_supervisor_spawn')
    direct = children(child.pid)
    require(len(direct) == 1, 'exactly_one_no_workload_supervisor')
    supervisor = direct[0]
    own(supervisor, 'held_supervisor')
    wait_for(lambda: process(supervisor)['state'] == 'T', 3, 'actual_self_stopped_supervisor')
    for pid, mode, parent in [(child.pid, '--missing-pidfd-parent', os.getpid()),
                              (supervisor, '--held-supervisor-no-workload', child.pid)]:
        row = process(pid)
        require(row['ppid'] == parent and row['uids'] == row['gids'] == [0] * 4
                and row['TracerPid'] == 0, 'actual_owned_root_process')
        require(row['exe_sha256'] == selected['workspace_files']['auth-scope-fixture'], 'exact_fixture_ELF')
        require(row['argv'] == [str(I / 'auth-scope-fixture').encode(), mode.encode(), b'--parent',
            str(parent).encode(), b'--until', str(out['original_until_BOOTTIME_ns']).encode()],
            'fixed_test_mode_parent_and_original_deadline')
        require(row['membership'] == membership(os.getpid()), 'both_owned_processes_outside_leaf')
    # GREEN requires the unsolicited initial Setup snapshot before the command.
    # Use the existing bounded pump; never renew the original attempt deadline.
    wait_for(lambda: any(row['pid'] == supervisor for row in out['records']), 3,
             'initial_exact_Setup_report_before_command')
    # Drain observations from before the command. They cannot answer the fresh
    # Stop/snapshot oracle, including an initial already-retired Setup snapshot.
    while select.select([child.stdout], [], [], 0)[0] and not stdout_eof:
        pump(0)
    out['pre_command_records'] = list(out['records'])
    require(out['pre_command_records'], 'nonempty_initial_Setup_before_command')
    out['pre_command_discarded_residue_bytes'] = len(frames)
    require(not frames, 'zero_pre_command_residue')
    frames.clear()
    require(not stdout_eof, 'parent_still_open_before_probe')
    out['causal_before_probe'] = blocked_sample()
    out['setup_prerequisites_complete'] = True
    before = len(out['records'])
    at = now()
    out['initial_exact_Setup_report_before_command'] = bool(out['pre_command_records']
        and all(row['event'] == 5 and row['flags'] == 0x41 and row['retired'] == 6
                and row['pid'] == supervisor and row['raw_wait_status'] == 0 and row['adopted'] == 0
                and row['at_BOOTTIME_ns'] < at and row['received_BOOTTIME_ns'] < at
                for row in out['pre_command_records']))
    require(out['initial_exact_Setup_report_before_command'], 'initial_exact_Setup_fenced_before_command')
    require(os.write(child.stdin.fileno(), b'S?') == 2, 'actual_queued_Stop_snapshot')
    out['Stop_snapshot_queued_BOOTTIME_ns'] = at
    out['Stop_snapshot_actual_write_bytes'] = 2
    while now() < at + 250_000_000:
        pump(min(.01, max(0, at + 250_000_000 - now()) / 1_000_000_000))
        row = process(child.pid)
        require(row['state'] != 'T' and row['TracerPid'] == 0, 'parent_unstopped_untraced_during_probe')
    pump(0)
    out['Stop_snapshot_probe_elapsed_ns'] = now() - at
    require(out['Stop_snapshot_probe_elapsed_ns'] >= 250_000_000, 'full_original_250ms_probe_window')
    fresh = [row for row in out['records'][before:]
             if row['at_BOOTTIME_ns'] >= at and row['received_BOOTTIME_ns'] <= at + 250_000_000]
    matches = [row for row in fresh if row['flags'] == 0x41 and row['retired'] == 6
               and row['pid'] == supervisor and row['raw_wait_status'] == 0]
    out['Stop_snapshot_fresh_records'] = fresh
    out['response_oracle_passed'] = bool(out['initial_exact_Setup_report_before_command']
        and out['pre_command_records'] and matches and not frames and not stdout_eof
        and all(row['flags'] == 0x41 and row['retired'] == 6 for row in out['records'][before:]))
    if matches:
        out['Stop_snapshot_response_ns'] = matches[0]['received_BOOTTIME_ns'] - at
        require(0 <= out['Stop_snapshot_response_ns'] <= 250_000_000, 'strict_original_response_bound')
    out['causal_after_probe'] = blocked_sample()
    require(live(handles[child.pid][0]) and live(handles[supervisor][0]), 'both_owned_processes_still_live')
    out['response_oracle'] = {'limit_ns': 250_000_000, 'parent_report_received': False,
        'required_GREEN_parent_event': 5, 'required_GREEN_flags': 0x41,
        'required_GREEN_retired': 6, 'closed_or_unoccupied_is_failure': True}
    out['response_oracle']['parent_report_received'] = bool(fresh)
    require(out['causal_after_probe']['held_supervisor']['state'] == 'T', 'supervisor_remains_held')
    if out['response_oracle_passed']:
        require(out['initial_exact_Setup_report_before_command'] and out['pre_command_records'],
                'GREEN_requires_initial_exact_Setup_before_command')
        out['postwindow_command_pipe_sample'] = command_pipe_sample()
        require(live(handles[child.pid][0]) and live(handles[supervisor][0]), 'live_during_consumption_guard')
        out['qualification_strengthening'] = 'initial exact Setup report before command plus postwindow owned writer FIONREAD0 and matching retained native stdin'
        out['qualified_setup_GREEN_before_teardown'] = True
        out['feature_proof_frozen_BOOTTIME_ns'] = now()
        write('feature-green-before-cleanup.json', out)
    else:
        require(out['causal_before_probe']['causal_blocked_read']
                and out['causal_after_probe']['causal_blocked_read'], 'actual_setup_pipe_block_cause_across_window')
        require(not out['records'][before:] and not frames and not stdout_eof,
                'actual_250ms_no_parent_report_RED')
        out['qualified_setup_RED_before_teardown'] = True
        out['feature_failure_frozen_BOOTTIME_ns'] = now()
        out['feature_proof_frozen_BOOTTIME_ns'] = out['feature_failure_frozen_BOOTTIME_ns']
        write('feature-red-before-cleanup.json', out)


def teardown():
    # Driver teardown is independent of native cleanup and never qualifies GREEN.
    if child is None:
        return
    if supervisor is None and Path(f'/proc/{child.pid}').exists():
        for pid in children(child.pid):
            row = process(pid)
            require(row['ppid'] == child.pid and len(handles) < 2, 'bounded_owned_cleanup_child')
            if pid not in handles:
                own(pid, 'held_supervisor')
    if live(handles[child.pid][0]):
        signal.pidfd_send_signal(handles[child.pid][0], signal.SIGKILL)
        out['independent_teardown'].append({'pid': child.pid, 'signal': 'SIGKILL',
            'label': 'driver_only_parent_teardown', 'at_BOOTTIME_ns': now(), 'exact_owned_pidfd': True})
    status = child.wait(timeout=2)
    out['actual_parent_direct_wait'] = {'pid': child.pid, 'exit': status, 'at_BOOTTIME_ns': now()}
    require(status == -signal.SIGKILL, 'actual_parent_direct_wait9')
    for pid, (fd, label) in handles.items():
        if pid == child.pid:
            continue
        row = serial_process(pid)
        require(row['ppid'] == os.getpid() and row['TracerPid'] == 0, 'actual_owned_supervisor_adoption')
        out['adopted_supervisor_before_teardown'] = row
        if live(fd):
            signal.pidfd_send_signal(fd, signal.SIGKILL)
            out['independent_teardown'].append({'pid': pid, 'signal': 'SIGKILL',
                'label': 'driver_only_adopted_supervisor_teardown', 'at_BOOTTIME_ns': now(), 'exact_owned_pidfd': True})
        until = time.monotonic() + 2
        waited = (0, 0)
        while waited[0] == 0 and time.monotonic() < until:
            waited = os.waitpid(pid, os.WNOHANG)
            if waited[0] == 0:
                time.sleep(.002)
        require(waited == (pid, signal.SIGKILL), 'actual_adopted_supervisor_wait9')
        out['adopted_waits'].append({'pid': waited[0], 'raw_wait_status': waited[1], 'at_BOOTTIME_ns': now()})
    wait_for(lambda: stdout_eof, .5, 'actual_parent_stdout_EOF_after_teardown')
    require(not frames, 'no_parent_fragment_after_teardown')
    out['final_parent_stdout_EOF'] = True
    out['final_leaf_sample'] = empty_leaf('final_empty_no_workload_leaf')
    out['final_owned_process_samples'] = []
    for pid, (fd, label) in handles.items():
        row = {'pid': pid, 'label': label, 'pidfd_dead': not live(fd),
               'proc_absent': not Path(f'/proc/{pid}').exists(), 'at_BOOTTIME_ns': now()}
        require(row['pidfd_dead'] and row['proc_absent'], 'actual_final_PID_closure')
        out['final_owned_process_samples'].append(row)
    try:
        os.waitpid(-1, os.WNOHANG)
        raise RuntimeError('expected_final_driver_ECHILD')
    except ChildProcessError:
        out['final_global_ECHILD'] = True
    if 'feature_proof_frozen_BOOTTIME_ns' in out:
        require(all(row['at_BOOTTIME_ns'] > out['feature_proof_frozen_BOOTTIME_ns']
                    for row in out['independent_teardown']), 'RED_frozen_before_teardown')


try:
    provision()
    run_case()
except BaseException as error:
    out['error_type'] = type(error).__name__
    if isinstance(error, RuntimeError):
        out['error_code'] = str(error)
finally:
    try:
        teardown()
    except BaseException as error:
        out['cleanup_error_type'] = type(error).__name__
        if isinstance(error, RuntimeError):
            out['cleanup_error_code'] = str(error)
    out['qualified_setup_RED'] = bool(out.get('qualified_setup_RED_before_teardown')
        and out.get('final_global_ECHILD') and not out.get('error_type') and not out.get('cleanup_error_type'))
    out['qualified_setup_GREEN'] = bool(out.get('qualified_setup_GREEN_before_teardown')
        and out.get('final_global_ECHILD') and not out.get('error_type') and not out.get('cleanup_error_type'))
    out['passed'] = out['qualified_setup_GREEN']
    out['classification'] = ('QUALIFIED_INJECTED_MISSING_PIDFD_SETUP_PIPE_BLOCK_RED' if out['qualified_setup_RED']
        else 'QUALIFIED_SETUP_FAULT_RESPONSIVE_QUARANTINE_GREEN' if out['qualified_setup_GREEN']
        else 'UNQUALIFIED_SETUP_FAULT_ATTEMPT')
    out['completed_BOOTTIME_ns'] = now()
    write('delegated-result.json', out)
    for fd, label in handles.values():
        os.close(fd)
