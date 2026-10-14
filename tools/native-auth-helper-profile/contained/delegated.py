#!/usr/bin/env python3
"""One self-only credential-profile control in the explicitly owned ARM guest."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import resource
import select
import signal
import stat
import time

R = Path('/results')
I = Path('/run/pokelinux-auth-containment-test')
PRIVATE = Path('/run/pokelinux-auth-worker-root')
MEMBER = '/system.slice/pokelinux-auth-containment-test.service'
CG = Path('/sys/fs/cgroup' + MEMBER)
end = time.monotonic() + 30
out = {'schema': 1, 'passed': False, 'qualified_GREEN': False,
       'scope': 'authored helper-profile fixed direct-path containment control in an owned disposable guest',
       'credentials_used': False, 'PAM_executed': False, 'phone_accessed': False,
       'teardown': [], 'direct_wait': None}
pid = None
pidfd = None
reader = None
gate = None
waited = None
until = None
libc = ctypes.CDLL(None, use_errno=True)

def require(ok, code):
    if not ok:
        raise RuntimeError(code)

def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)

def write(name, value):
    with (R / name).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')

def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def live():
    return not select.select([pidfd], [], [], 0)[0]

def membership(value):
    text = Path(f'/proc/{value}/cgroup').read_text()
    require(text.startswith('0::') and text.count('\n') == 1, 'one_unified_membership')
    return text[3:].strip()

def leaf(name):
    root = CG / name
    text = dict(line.split() for line in (root / 'cgroup.events').read_text().splitlines())
    return {'populated': int(text['populated']), 'procs': sorted(map(int, (root / 'cgroup.procs').read_text().split())),
            'identity': [root.stat().st_dev, root.stat().st_ino], 'at_BOOTTIME_ns': now()}

def process(value):
    data = dict(line.split(':', 1) for line in Path(f'/proc/{value}/status').read_text().splitlines())
    text = Path(f'/proc/{value}/stat').read_text()
    fields = text[text.rindex(')') + 2:].split()
    return {'pid': value, 'ppid': int(fields[1]), 'state': fields[0], 'start_ticks': int(fields[19]),
            'uids': list(map(int, data['Uid'].split())), 'gids': list(map(int, data['Gid'].split())),
            'groups': list(map(int, data['Groups'].split())),
            'caps': {key: int(data[key], 16) for key in ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb']},
            'NNP': int(data['NoNewPrivs']), 'seccomp': int(data['Seccomp']), 'seccomp_filters': int(data['Seccomp_filters']), 'threads': int(data['Threads']), 'tracer': int(data['TracerPid']),
            'membership': membership(value), 'at_BOOTTIME_ns': now()}

def held():
    row = process(pid)
    require(live() and row['ppid'] == os.getpid() and row['start_ticks'] == out['admitted']['start_ticks'], 'same_live_owned_generation')
    require(row['uids'] == row['gids'] == [1000, 0, 0, 0] and not row['groups'], 'actual_original_helper_ID_profile')
    require(row['caps'] == {'CapInh': 0, 'CapPrm': 1 << 29, 'CapEff': 1 << 29, 'CapBnd': 1 << 29, 'CapAmb': 0}, 'actual_audit_only_capability_profile')
    require(row['NNP'] == 0 and row['threads'] == 1 and row['tracer'] == 0, 'actual_untraced_singlethread_NNP0')
    require(row['seccomp'] == 2 and row['seccomp_filters'] == 1, 'actual_inherited_filter_and_NNP0')
    require(sorted(int(p.name) for p in Path(f'/proc/{pid}/fd').iterdir()) == [0, 1, 2], 'only_selected_private_stdio_inherited')
    for fd in [0, 2]:
        require(stat.S_ISCHR(Path(f'/proc/{pid}/fd/{fd}').stat().st_mode), 'no_inherited_outside_directory_handle')
    exe = Path(f'/proc/{pid}/exe')
    actual, installed = exe.stat(), (PRIVATE / str(I).lstrip('/') / 'helper-profile-probe').stat()
    require((actual.st_dev, actual.st_ino) == (installed.st_dev, installed.st_ino), 'actual_selected_privileged_exec')
    require(sha(exe) == selected['workspace_files']['helper-profile-probe'], 'actual_selected_probe_hash')
    require(Path(f'/proc/{pid}/cmdline').read_bytes() == os.fsencode(I / 'helper-profile-probe') + b'\0', 'fixed_probe_argv')
    line = next(line for line in Path(f'/proc/{pid}/limits').read_text().splitlines() if line.startswith('Max core file size'))
    require(line.split()[4:6] == ['0', '0'], 'actual_CORE_soft_hard0')
    info = dict(line.split(':', 1) for line in Path(f'/proc/{pid}/fdinfo/1').read_text().splitlines())
    flags = int(info['flags'], 8)
    require(int(info['ino']) == os.fstat(reader).st_ino and flags & os.O_ACCMODE == os.O_WRONLY and flags & os.O_NONBLOCK, 'actual_private_nonblocking_report_pipe')
    require(row['membership'] == MEMBER + '/workload', 'actual_owned_workload_membership')
    require(leaf('workload')['procs'] == [pid] and leaf('escaped')['procs'] == [], 'probe_contained_in_workload')
    require(Path(f'/proc/{pid}/ns/mnt').stat().st_ino != Path('/proc/self/ns/mnt').stat().st_ino, 'actual_private_mount_namespace')
    require(Path(f'/proc/{pid}/ns/pid').stat().st_ino == Path('/proc/self/ns/pid').stat().st_ino, 'same_real_PID_namespace')
    expected_root = PRIVATE.stat()
    for name in ['root', 'cwd']:
        actual_root = Path(f'/proc/{pid}/{name}').stat()
        require((actual_root.st_dev, actual_root.st_ino) == (expected_root.st_dev, expected_root.st_ino), 'actual_pivoted_root_and_cwd_inode')
    require(not Path(f'/proc/{pid}/root/sys/fs/cgroup').exists(), 'no_cgroup_view_in_actual_root')
    row['mount_namespace_inode'] = Path(f'/proc/{pid}/ns/mnt').stat().st_ino
    row['root_link_text'] = os.readlink(f'/proc/{pid}/root')
    row['cwd_link_text'] = os.readlink(f'/proc/{pid}/cwd')
    row['exe_link_text'] = os.readlink(exe)
    row['root_identity'] = [expected_root.st_dev, expected_root.st_ino]
    row['exe_identity'] = [actual.st_dev, actual.st_ino]
    row['report_pipe_inode'] = int(info['ino'])
    row['CORE_soft_hard'] = [0, 0]
    return row

def wait_for(predicate, seconds, code):
    deadline = min(time.monotonic() + seconds, end)
    while not predicate():
        require(time.monotonic() < deadline, code)
        require(until is None or now() < until, 'original_attempt_deadline')
        time.sleep(.002)

def provision():
    require(os.getuid() == os.geteuid() == 0 and platform.machine() == 'aarch64' and platform.release() == '7.1.6-1-aarch64-ARCH', 'owned_selected_ARM_guest_only')
    require(membership(os.getpid()) == MEMBER, 'fixed_private_delegated_service')
    require(libc.prctl(36, 1, 0, 0, 0) == 0 and libc.prctl(4, 0, 0, 0, 0) == 0, 'dedicated_nondumpable_subreaper')
    require(sorted(map(int, (CG / 'cgroup.procs').read_text().split())) == [os.getpid()], 'only_driver_at_delegated_root')
    for name in ['owner', 'workload', 'escaped']:
        (CG / name).mkdir(mode=0o755)
    (CG / 'owner/cgroup.procs').write_text(str(os.getpid()) + '\n')
    require(membership(os.getpid()) == MEMBER + '/owner' and not (CG / 'cgroup.procs').read_text().strip(), 'only_owned_driver_moved_to_owner')
    for base in [CG, CG / 'owner', CG / 'workload', CG / 'escaped']:
        for path, mode in [(base, 0o755), (base / 'cgroup.procs', 0o644)]:
            m = path.lstat()
            require(m.st_uid == m.st_gid == 0 and stat.S_IMODE(m.st_mode) == mode and not stat.S_ISLNK(m.st_mode), 'fixed_root_owned_controls')
    require(not leaf('workload')['populated'] and not leaf('escaped')['populated'], 'initial_empty_private_leaves')
    out['initial_leaves'] = {name: leaf(name) for name in ['owner', 'workload', 'escaped']}

def run_case():
    global pid, pidfd, reader, gate, until
    require(len(list(Path('/proc/self/task').iterdir())) == 1, 'singlethread_before_fork')
    reader, writer = os.pipe2(os.O_CLOEXEC | os.O_NONBLOCK)
    gate_read, gate = os.pipe2(os.O_CLOEXEC)
    parent = os.getpid()
    until = now() + 15_000_000_000
    out['original_until_BOOTTIME_ns'] = until
    pid = os.fork()
    if pid == 0:
        try:
            os.close(gate)
            os.close(reader)
            require(libc.prctl(1, signal.SIGKILL, 0, 0, 0) == 0 and os.getppid() == parent, 'gated_child_parent')
            require(os.read(gate_read, 1) == b'G', 'fixed_admission_gate')
            devnull = os.open('/dev/null', os.O_RDWR | os.O_CLOEXEC)
            os.dup2(devnull, 0, inheritable=True)
            os.dup2(devnull, 2, inheritable=True)
            os.dup2(writer, 1, inheritable=True)
            require(libc.close_range(3, ctypes.c_uint(0xffffffff), 0) == 0, 'child_descriptor_closure')
            os.execve(I / 'helper-profile-launcher', [str(I / 'helper-profile-launcher')], {})
        except BaseException:
            os._exit(111)
    os.close(gate_read)
    os.close(writer)
    pidfd = os.pidfd_open(pid, 0)
    before = process(pid)
    require(before['ppid'] == parent and before['membership'] == MEMBER + '/owner' and live(), 'exact_unreaped_gated_child')
    out['gated_child'] = before
    control = os.open(CG / 'workload/cgroup.procs', os.O_WRONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        body = str(pid).encode() + b'\n'
        require(os.write(control, body) == len(body), 'full_exact_child_admission')
    finally:
        os.close(control)
    out['admitted'] = process(pid)
    require(out['admitted']['start_ticks'] == before['start_ticks'] and membership(pid) == MEMBER + '/workload' and leaf('workload')['procs'] == [pid] and live(), 'actual_admission_before_exec')
    require(os.write(gate, b'G') == 1, 'actual_admission_gate_release')
    os.close(gate)
    gate = None
    out['gate_released_BOOTTIME_ns'] = now()
    packet = bytearray()
    deadline = min(time.monotonic() + 4, end)
    while len(packet) < 16:
        require(time.monotonic() < deadline and now() < until, 'bounded_complete_probe_report')
        if select.select([reader], [], [], .002)[0]:
            chunk = os.read(reader, 17 - len(packet))
            require(chunk, 'probe_report_no_early_EOF')
            packet.extend(chunk)
    require(len(packet) == 16 and packet[:8] == b'PKHCPR01', 'current_complete_report_encoding')
    out['typed_report'] = {'outcome': packet[8], 'stage': packet[9], 'errno_class': packet[10],
                           'reserved': packet[11], 'flags': int.from_bytes(packet[12:], 'little')}
    out['observed_process_after_packet'] = process(pid)
    require(out['typed_report'] == {'outcome': 1, 'stage': 10, 'errno_class': 5, 'reserved': 0, 'flags': 135}, 'complete_direct_open_ENOENT_and_real_profile_packet')
    out['held_before_owned_workload_kill'] = held()
    control = os.open(CG / 'workload/cgroup.kill', os.O_WRONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        m = os.fstat(control)
        require(m.st_uid == 0 and not m.st_mode & 0o022, 'anchored_owned_workload_kill')
        require(os.write(control, b'1\n') == 2, 'full_owned_workload_kill_write')
        out['owned_workload_kill'] = {'identity': [m.st_dev, m.st_ino], 'bytes': 2, 'at_BOOTTIME_ns': now()}
    finally:
        os.close(control)
    require(select.select([pidfd], [], [], .5)[0], 'retained_helper_dies_after_owned_workload_kill')
    out['required_remain_in_workload'] = True
    out['containment_observed_before_teardown'] = True
    out['feature_observed_BOOTTIME_ns'] = now()
    write('feature-green-before-cleanup.json', out)

def teardown():
    global waited
    if gate is not None:
        os.close(gate)
    if pid is None:
        return
    if pidfd is None:
        # The unreaped exact fork child still belongs to this dedicated process.
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    elif live():
        signal.pidfd_send_signal(pidfd, signal.SIGKILL)
        out['teardown'].append({'pid': pid, 'exact_owned_pidfd': True, 'signal': 9, 'at_BOOTTIME_ns': now()})
    deadline = min(time.monotonic() + 2, end)
    while waited is None:
        value = os.waitpid(pid, os.WNOHANG)
        if value[0] == pid:
            waited = value
            break
        require(time.monotonic() < deadline, 'actual_exact_child_wait_bound')
        time.sleep(.002)
    out['direct_wait'] = {'pid': pid, 'raw_status': waited[1], 'at_BOOTTIME_ns': now()}
    eof = False
    extra = bytearray()
    deadline = min(time.monotonic() + 1, end)
    while not eof:
        require(time.monotonic() < deadline, 'actual_report_EOF_bound')
        if select.select([reader], [], [], .002)[0]:
            chunk = os.read(reader, 17)
            eof = not chunk
            extra.extend(chunk)
            require(len(extra) <= 16, 'no_unbounded_extra_report')
    out['stdout_EOF'] = eof
    out['extra_report_bytes'] = len(extra)
    out['final_process_sample'] = {'pid': pid, 'pidfd_dead': pidfd is not None and not live(), 'proc_absent': not Path(f'/proc/{pid}').exists(), 'at_BOOTTIME_ns': now()}
    out['final_leaves'] = {name: leaf(name) for name in ['owner', 'workload', 'escaped']}
    require(not out['final_leaves']['workload']['populated'] and not out['final_leaves']['escaped']['populated'], 'final_owned_workload_and_sibling_empty')
    require(not (CG / 'cgroup.procs').read_text().strip() and out['final_leaves']['owner']['procs'] == [os.getpid()], 'only_live_driver_remains')
    try:
        os.waitpid(-1, os.WNOHANG)
        raise RuntimeError('unexpected_remaining_child')
    except ChildProcessError:
        out['final_global_ECHILD'] = True
    if out.get('containment_observed_before_teardown'):
        require(waited == (pid, 9) and out['final_process_sample']['pidfd_dead'] and out['final_process_sample']['proc_absent'] and not extra, 'contained_exact_cleanup')
        require(not out['teardown'], 'no_fallback_pidfd_kill_on_contained_success')
        require(now() < until, 'actual_closure_within_original_attempt')

try:
    selected = json.loads((R / 'selection.json').read_text())
    require(selected['label'] == 'pokelinux-native-auth-contained-arm-root-1' and selected['network'] == 'none', 'fixed_owned_guest_selection')
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
    out['passed'] = bool(out.get('containment_observed_before_teardown') and out.get('final_global_ECHILD') and not out.get('error_type') and not out.get('cleanup_error_type') and not out['teardown'])
    out['qualified_GREEN'] = out['passed']
    out['classification'] = 'AUTHORED_DIRECT_PATH_CONTAINMENT_GREEN' if out['passed'] else 'UNQUALIFIED_CONTAINED_ATTEMPT'
    out['completed_BOOTTIME_ns'] = now()
    write('delegated-result.json', out)
    for fd in [reader, pidfd]:
        if fd is not None:
            os.close(fd)
