#!/usr/bin/env python3
"""SOURCE ONLY: one selected installed radio GREEN case in a fresh owned ARM epoch."""
import fcntl
import gzip
import grp
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import select
import signal
import stat
import struct
import subprocess
import time
import traceback
import sys
sys.dont_write_bytecode = True

R = Path('/results')
INSTALL = Path('/usr/libexec/pokelinux')
FIXTURE = Path('/opt/radio-operator-fixture')
UID = 1500
UNIT = 'pokelinux-radio.service'
RUNTIME = Path('/run/pokelinux/radio')
STATE = Path('/var/lib/pokelinux/radio')
JOURNAL = STATE / 'admission'
SOCKET = RUNTIME / 'status.sock'
RECEIPTS = STATE / 'systemd-main-exit.json'
DEVICE = '/dev/binderfs/radio'
SERVICE = 'org.pokelinux.fixture.arm64-radio'
SLOT = 1
WIRE = {'registration_state_wire': 255, 'feature_wire': 128, 'feature_mask_wire': -2147483648,
    'status_code_wire': 65535, 'uri_hex': '7369703a726f757465', 'reason_hex': '6f6b',
    'feature_mask_ext_wire': 287454020, 'sip_delegate_mask_wire': 72623859790382856,
    'feature_ext_wire': 13124}
children, watches = [], []
manager = peer = logger = None
manager_watch = peer_watch = logger_watch = main_watch = worker_watch = writer_watch = None
selected = None
main_pid = worker_pid = writer_pid = peer_exit_helper_pid = registration_helper_pid = None
incarnation = journal_bytes = None
unit_selected = False
case_until = None
sequence = 0
result = {'schema': 1, 'passed': False, 'setup_complete': False, 'desired_boundary_red': False,
    'response_crash_persistence_claimed': False,
    'scope': 'one selected original-bounded installed Rust radio operator GREEN case; independent generic ARM Binder fixture',
    'phone_accessed': False, 'hardware_qualified': False, 'vendor_or_private_inputs_selected': False,
    'PAM_or_authentication_executed': False, 'GUI_executed': False,
    'NDK_or_Bionic_runtime_selected': False, 'automatic_retries_performed': False,
    'independent_worker_or_systemd_Main_reap_performed': False,
    'Rust_Supervisor_owns_worker_wait': True, 'systemd_owns_service_Main_wait': True,
    'normal_worker_killed_by_guest': False, 'cgroup_kill_by_guest': False,
    'service_stdout_backend': 'unchanged production journal stream; owned journalctl reader output is a private file',
    'cleanup': [], 'status_observations': []}


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def sha(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def u64(value):
    return type(value) is int and 0 < value < 2**64


def alive(handle):
    return not select.select([handle], [], [], 0)[0]


def remaining(deadline):
    assert now() < deadline, 'original BOOTTIME cutoff already reached'
    return deadline


def bounded(seconds, original=None):
    deadline = now() + int(seconds * 1e9)
    return min(deadline, original) if original is not None else deadline


def start(argv, name, user=False, pipe=False, deadline=None):
    log = (R / name).open('xb', buffering=0)
    identity = {'user': UID, 'group': UID, 'extra_groups': []} if user else {}
    try:
        if deadline is not None:
            assert now() < deadline, name + ' original pre-spawn admission cutoff'
        child = subprocess.Popen([str(arg) for arg in argv], stdout=subprocess.PIPE if pipe else log,
            stderr=subprocess.STDOUT, start_new_session=True, **identity)
    except BaseException:
        log.close()
        raise
    owner = {'child': child, 'log': log, 'name': name, 'wait_recorded': False,
        'argv': [str(arg) for arg in argv], 'watch': None}
    children.append(owner)  # Retain custody before fallible pipe/pidfd setup.
    if pipe:
        owner.update({'pipe': child.stdout, 'pipe_ready': False, 'pipe_data': bytearray(),
            'pipe_eof': False, 'pipe_error': None})
        os.set_blocking(child.stdout.fileno(), False)
        owner['pipe_ready'] = True
    handle = os.pidfd_open(child.pid, os.O_NONBLOCK)
    watches.append(handle)
    owner['watch'] = handle
    return child, handle


def drain(owner, deadline):
    if 'pipe' not in owner or not owner['pipe_ready'] or owner['pipe_eof'] or owner['pipe_error']:
        return
    while now() < deadline:
        try:
            chunk = os.read(owner['pipe'].fileno(), 16385 - len(owner['pipe_data']))
        except BlockingIOError:
            return
        except InterruptedError:
            continue
        if not chunk:
            owner['pipe_eof'] = True
            owner['pipe_eof_observed_boottime_ns'] = now()
            return
        owner['pipe_data'].extend(chunk)
        if len(owner['pipe_data']) > 16384:
            owner['pipe_error'] = 'status output exceeded16384-byte bound'
            owner['pipe'].close()
            return


def finish(child, name, deadline):
    owner = next(owner for owner in children if owner['child'] is child)
    if owner['wait_recorded']:
        return owner['wait_receipt']['returncode']
    while child.poll() is None:
        assert now() < deadline, name + ' direct wait exceeded original cutoff before completion'
        drain(owner, deadline)
        time.sleep(.001)
    value = child.wait(timeout=0)
    completed_at = now()  # Keep this immediate post-wait check; no renewed budget.
    receipt = {'pid': child.pid, 'role': name, 'actual_direct_parent_wait': True,
        'returncode': value, 'completed_boottime_ns': completed_at, 'argv': owner['argv'],
        'original_deadline_ns': deadline, 'clock': 'CLOCK_BOOTTIME',
        'actual_wait_observed_before_original_cutoff': completed_at < deadline,
        'late_actual_wait_observation': completed_at >= deadline}
    result['cleanup'].append(receipt)
    owner['wait_recorded'] = True
    owner['wait_receipt'] = receipt
    if 'pipe' in owner:
        while not owner['pipe_eof'] and not owner['pipe_error'] and now() < deadline:
            drain(owner, deadline)
            if not owner['pipe_eof'] and not owner['pipe_error']:
                select.select([owner['pipe']], [], [], .001)
        receipt.update({'stdout_sink': 'owned nonblocking pipe', 'stdout_maximum_bytes': 16384,
            'stdout_bytes': len(owner['pipe_data']), 'stdout_EOF': owner['pipe_eof'],
            'stdout_EOF_observed_boottime_ns': owner.get('pipe_eof_observed_boottime_ns'),
            'stdout_error': owner['pipe_error']})
        pipe_closed_at = now()
        receipt['stdout_EOF_and_actualwait_before_original_cutoff'] = bool(
            completed_at < deadline and owner['pipe_eof'] and not owner['pipe_error'] and pipe_closed_at < deadline)
        # Regular private evidence-file writes occur only after protocol-child
        # wait/EOF observation. They are not the status client's stdout sink.
        owner['log'].write(owner['pipe_data'][:16384])
        owner['pipe'].close()
    owner['log'].close()
    if completed_at >= deadline:
        result.setdefault('late_direct_waits', []).append(receipt)
        raise TimeoutError(name + ' actual direct wait observed at or after original cutoff')
    if 'pipe' in owner and not receipt['stdout_EOF_and_actualwait_before_original_cutoff']:
        result.setdefault('unqualified_status_pipe_closure', []).append(receipt)
        raise TimeoutError(name + ' bounded output and EOF not qualified before original cutoff')
    return value


def command(argv, label, deadline=None, expected=0, user=False, pipe=False):
    global sequence
    sequence += 1
    name = f'{sequence:03d}-{label}.log'
    until = remaining(deadline if deadline is not None else bounded(20))
    child, handle = start(argv, name, user=user, pipe=pipe, deadline=until)
    code = finish(child, label, until)
    data = (R / name).read_bytes()
    assert len(data) <= 262144, label + ' output exceeded private bound'
    if expected is not None:
        assert code == expected, label + ' actual exit ' + str(code)
    return code, data, child.pid


def wait_for(check, deadline, label):
    while now() < deadline:
        value = check()
        if value:
            assert now() < deadline, label + ' observed after original cutoff'
            return value
        time.sleep(.005)
    raise TimeoutError(label + ' original BOOTTIME cutoff')


def identity(pid, expected_sha, argv, parent, fixture_child=False, expected_uid=UID):
    fields = {key: value.strip() for key, value in
        (row.split(':', 1) for row in Path(f'/proc/{pid}/status').read_text().splitlines() if ':' in row)}
    assert fields['Pid'] == fields['Tgid'] == str(pid) and fields['PPid'] == str(parent)
    assert fields['Uid'].split() == fields['Gid'].split() == [str(expected_uid)] * 4
    groups = [int(value) for value in fields['Groups'].split()]
    assert (not groups if fixture_child else set(groups) <= {expected_uid})
    assert fields['CapEff'] == '0000000000000000'
    assert os.readlink(f'/proc/{pid}/ns/pid') == os.readlink('/proc/self/ns/pid')
    actual = [os.fsdecode(arg) for arg in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')[:-1]]
    assert actual == [str(arg) for arg in argv]
    assert sha(f'/proc/{pid}/exe') == expected_sha
    return {'pid': pid, 'ppid': parent, 'uid': expected_uid, 'gid': expected_uid, 'groups': groups,
        'postexec_ELF_sha256': expected_sha, 'actual_argv': actual,
        'proc_cgroup': Path(f'/proc/{pid}/cgroup').read_text(),
        'tasks': sorted(int(path.name) for path in Path(f'/proc/{pid}/task').iterdir())}


def lines(name):
    path = R / name
    assert path.stat().st_size <= 262144, name + ' exceeded bounded private log'
    return path.read_text().splitlines()


def events():
    data = (R / 'service-journal.log').read_bytes()
    assert len(data) <= 262144
    values = []
    for line in data.split(b'\n')[:-1]:
        if line.startswith(b'{'):
            value = json.loads(line)
            if value.get('schema') == 1 and value.get('event') in [
                'worker-start', 'writer-start', 'control-ready', 'transport-active', 'raw-observed',
                'registration-observed', 'intent-synced', 'writer-reap', 'permit-released',
                'worker-reap', 'start-refused']:
                values.append(value)
    return values


def event(name, count=1):
    values = [value for value in events() if value['event'] == name]
    assert len(values) <= count, name + ' event count exceeded exact selected count'
    return values[0] if len(values) == count else None


def live_fixture(require_peer=True, require_worker=False, require_main=True):
    assert manager.poll() is None and alive(manager_watch), 'public manager died before fixture closure'
    assert logger.poll() is None and alive(logger_watch), 'owned journal reader died before lifecycle receipts'
    if require_peer:
        assert peer.poll() is None and alive(peer_watch), 'public peer died before selected control'
    if require_main and main_watch is not None:
        assert alive(main_watch), 'installed service Main died before operator Stop'
    if require_worker:
        assert worker_watch is not None and alive(worker_watch), 'worker died before setup/status oracle'


def setup_witnesses(require_all=True):
    manager_lines, peer_lines = lines('manager.log'), lines('peer.log')
    lookup = f'SERVICE-LOOKUP pid={worker_pid} uid={UID} found=true'
    register = f'REGISTER pid={worker_pid} uid={UID} response_remote=true indication_remote=true'
    submit = f'CALLBACK-SUBMITTED target_pid={worker_pid} serial=7 bytes=1'
    expected_transfers = 1  # Only addService emits SERVICE-TRANSFERRED; checkService emits LOOKUP.
    assert manager_lines.count('SERVICE-TRANSFERRED') <= expected_transfers
    assert manager_lines.count(lookup) <= 1
    assert sum(line.startswith('REGISTER ') for line in peer_lines) <= 1
    assert sum(line.startswith('CALLBACK-SUBMITTED ') for line in peer_lines) <= 1
    complete = (manager_lines.count('SERVICE-TRANSFERRED') == expected_transfers
        and manager_lines.count(lookup) == peer_lines.count(register) == peer_lines.count(submit) == 1)
    if require_all:
        assert complete
        return {'worker_lookup': lookup, 'worker_lookup_count': 1, 'peer_registration': register,
            'registration_count': 1, 'callback_successful_submission': submit, 'submission_count': 1,
            'add_service_transfer_count': expected_transfers, 'separate_peer_exit_helper_pid': peer_exit_helper_pid,
            'callback_submission_is_not_owner_receipt': True}
    return complete


def show(label, deadline):
    properties = 'MainPID,InvocationID,ControlGroup,ActiveState,SubState,Result,ExecMainCode,ExecMainStatus,User,Group,KillMode,ExecStop,Restart,TimeoutStopUSec'
    _, data, _ = command(['systemctl', 'show', UNIT, '--all', '--property=' + properties, '--no-pager'], label, deadline)
    fields = dict(line.split('=', 1) for line in data.decode().splitlines() if '=' in line)
    assert fields['User'] == fields['Group'] == 'pokelinux-radio'
    assert fields['KillMode'] == 'mixed' and fields['Restart'] == 'no'
    assert fields['TimeoutStopUSec'] == '5s'
    _, path_data, path_pid = command(['busctl', '--system', 'call',
        'org.freedesktop.systemd1', '/org/freedesktop/systemd1',
        'org.freedesktop.systemd1.Manager', 'GetUnit', 's', UNIT],
        label + '-typed-get-unit', deadline)
    assert len(path_data) <= 1024
    path_text = path_data.decode('utf-8')
    match = re.fullmatch(r'o ("/org/freedesktop/systemd1/unit/[A-Za-z0-9_]+")\n', path_text)
    assert match is not None, 'exact typed GetUnit object-path output required'
    unit_path = json.loads(match.group(1))
    _, stop_data, stop_pid = command(['busctl', '--system', 'get-property',
        'org.freedesktop.systemd1', unit_path,
        'org.freedesktop.systemd1.Service', 'ExecStop'],
        label + '-typed-exec-stop', deadline)
    assert stop_data == b'a(sasbttttuii) 0\n', 'exact typed empty ExecStop array required'
    observed = now()
    assert observed < deadline
    result.setdefault('typed_unit_ExecStop_observations', []).append({
        'label': label, 'unit': UNIT, 'unit_object_path': unit_path,
        'GetUnit_raw_output': path_text, 'ExecStop_raw_output': stop_data.decode('ascii'),
        'ExecStop_signature': 'a(sasbttttuii)', 'ExecStop_array_length': 0,
        'GetUnit_actual_child_pid': path_pid, 'ExecStop_actual_child_pid': stop_pid,
        'original_deadline_ns': deadline, 'observed_boottime_ns': observed,
        'stdout_sinks': 'regular evidence files; no PIPE EOF claim'})
    return fields


def cgroup_snapshot(group, label, empty=False):
    assert group.startswith('/system.slice/') and '..' not in Path(group).parts
    root = Path('/sys/fs/cgroup') / group.lstrip('/')
    value = {'label': label, 'control_group': group, 'existed': root.exists()}
    if root.exists():
        value['procs'] = (root / 'cgroup.procs').read_text()
        value['events'] = (root / 'cgroup.events').read_text()
        value['mountinfo'] = Path('/proc/self/mountinfo').read_text()
        if empty:
            assert not value['procs'].split()
            assert dict(line.split() for line in value['events'].splitlines())['populated'] == '0'
    elif empty:
        value['absent_after_previously_bound_unit_cgroup'] = True
    else:
        raise AssertionError('actual active unit cgroup absent')
    result.setdefault('cgroup_observations', []).append(value)
    return value


def receipts():
    if not RECEIPTS.exists():
        return []
    metadata = RECEIPTS.lstat()
    assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == metadata.st_gid == UID
    assert stat.S_IMODE(metadata.st_mode) == 0o600 and metadata.st_size <= 16384
    try:
        data = json.loads(RECEIPTS.read_text())
    except json.JSONDecodeError:
        return []  # A live ExecStopPost may be between truncate and completed fsync.
    assert isinstance(data, list) and len(data) <= 2
    for row in data:
        assert row['observer_uid'] == row['observer_gid'] == UID
        assert row['observer_ppid'] == 1 and u64(row['observed_boottime_ns'])
        assert re.fullmatch('[0-9a-f]{32}', row['INVOCATION_ID'])
    (R / 'systemd-main-exit.json').write_text(json.dumps(data, indent=2) + '\n')
    return data


def status(label, deadline):
    _, data, helper_pid = command([INSTALL / 'radio-status', SOCKET, 'status'], label,
        min(bounded(3), deadline), user=True, pipe=True)
    observed = now()
    assert len(data.splitlines()) == 1
    value = json.loads(data)
    assert set(value) == {'server_pid', 'phase', 'failure', 'incarnation', 'revision', 'last_observation', 'request_history'}
    assert value['server_pid'] == main_pid and value['incarnation'] == incarnation
    assert value['phase'] in ['initializing', 'observed-active', 'unavailable', 'stopping']
    assert type(value['revision']) is int and 0 <= value['revision'] < 2**64
    history = value['last_observation']
    if history is not None:
        assert history['kind'] in ['raw', 'registration-wire'] and history['attempt'] == 1
        assert u64(history['revision']) and history['revision'] <= value['revision']
        if history['kind'] == 'raw':
            assert set(history) == {'kind', 'revision', 'attempt', 'slot_id', 'serial',
                'rilc_msg_id', 'declared_length', 'error', 'payload_hex'}
            assert type(history['slot_id']) is int and 0 <= history['slot_id'] < 2**32
            assert type(history['serial']) is int and -(2**31) <= history['serial'] < 2**31
            assert type(history['rilc_msg_id']) is int and 0 <= history['rilc_msg_id'] < 2**32
            assert type(history['error']) is int and -(2**31) <= history['error'] < 2**31
            assert type(history['declared_length']) is int and 0 <= history['declared_length'] <= 4096
            assert re.fullmatch('[0-9a-f]*', history['payload_hex'])
            assert len(history['payload_hex']) % 2 == 0 and len(history['payload_hex']) <= 2 * 2048
        else:
            assert set(history) == {'kind', 'revision', 'attempt', 'slot_id', 'message_id', 'wire'}
            assert history['slot_id'] == SLOT and history['message_id'] == 3012
            assert history['wire'] == WIRE
    observation = {'label': label, 'helper_pid': helper_pid, 'completed_boottime_ns': observed, 'value': value}
    result['status_observations'].append(observation)
    assert observed < deadline, 'status observation after original selected cutoff'
    return value


def registration_witnesses():
    manager_lines, peer_lines = lines('manager.log'), lines('peer.log')
    lookup = f'SERVICE-LOOKUP pid={registration_helper_pid} uid={UID} found=true'
    submission = (f'REGISTRATION-WIRE-SUBMITTED target_pid={worker_pid} target_uid={UID} '
        'control=120 message=3012 slot=1 fragment=false bytes=37')
    assert manager_lines.count(lookup) == 1
    assert [line for line in peer_lines if line.startswith('REGISTRATION-WIRE-SUBMITTED ')] == [submission]
    result['actual_registration_trigger_witnesses'] = {'helper_pid': registration_helper_pid,
        'helper_named_lookup': lookup, 'peer_submission': submission, 'fixed_control': 120,
        'selected_slot': SLOT, 'no_registration_by_helper': True,
        'peer_submission_is_not_owner_receipt': True}
    return result['actual_registration_trigger_witnesses']


def journal_unchanged():
    metadata = JOURNAL.lstat()
    assert metadata.st_uid == metadata.st_gid == UID and stat.S_IMODE(metadata.st_mode) == 0o600
    assert stat.S_ISREG(metadata.st_mode) and JOURNAL.read_bytes() == journal_bytes


def native_reap(deadline, main_alive=True):
    def ready():
        live_fixture(require_peer=selected['expected_outcome'] == 'GREEN', require_main=main_alive)
        return event('worker-reap')
    row = wait_for(ready, deadline, 'native worker actualwait receipt')
    assert row['worker_pid'] == worker_pid and row['incarnation'] == incarnation
    assert row['wait_signal'] == 9 and row['wait_code'] is None
    assert row['occupied'] is False and row['admitted'] is False
    assert u64(row['waited_at_ns']) and u64(row['original_cleanup_until_ns'])
    assert row['waited_at_ns'] < row['original_cleanup_until_ns']
    assert row['original_cleanup_until_ns'] - row['waited_at_ns'] <= 2_000_000_000
    assert not alive(worker_watch) and not Path(f'/proc/{worker_pid}').exists()
    result['native_worker_actualwait'] = row
    result['worker_pidfd_dead_and_proc_absent_after_Rust_owner_wait'] = True
    return row


try:
    assert os.getuid() == os.geteuid() == 0 and Path('/proc/1/comm').read_text().strip() == 'systemd'
    selected = json.loads((R / 'selection.json').read_text())
    assert selected['phase'] == 'radio-operator-green-arm' and selected['expected_outcome'] == 'GREEN'
    for relative, digest in selected['files'].items():
        assert sha(R / relative) == digest, relative
    assert os.uname().machine == 'aarch64'
    assert sorted(path.name for path in Path('/sys/class/net').iterdir()) == ['lo']
    result['kernel'] = {'machine': os.uname().machine, 'release': os.uname().release, 'version': os.uname().version}
    result['common_PID_namespace'] = os.readlink('/proc/self/ns/pid')
    result['trusted_guest_proc_mountinfo'] = Path('/proc/self/mountinfo').read_text()
    config = gzip.decompress(Path('/proc/config.gz').read_bytes()).decode()
    result['kernel_Binder_config'] = [line for line in config.splitlines() if 'BINDER' in line]
    (R / 'kernel.config').write_text(config)
    INSTALL.mkdir(parents=True, exist_ok=True)
    FIXTURE.mkdir(mode=0o755)
    for name in ['radio-service', 'radio-status', 'radio-worker', 'operator-controller', 'peer', 'manager']:
        target = (FIXTURE if name in ['operator-controller', 'peer', 'manager'] else INSTALL) / name
        assert not target.exists()
        target.write_bytes((R / 'bins' / name).read_bytes())
        target.chmod(0o555)
        assert target.stat().st_uid == 0 and sha(target) == selected['binaries'][name]
    for target, origin in [
        (Path('/usr/lib/systemd/system') / UNIT, R / 'source/repository/native-radio/packaging/pokelinux-radio.service'),
        (Path('/usr/lib/sysusers.d/pokelinux-radio.conf'), R / 'source/repository/native-radio/packaging/pokelinux-radio.sysusers'),
        (INSTALL / 'radio-systemd-exit-observer.py', R / 'source/systemd-exit-observer.py')]:
        assert not target.exists()
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(origin.read_bytes())
        target.chmod(0o444)
        assert target.stat().st_uid == 0 and sha(target) == sha(origin)
    assert selected['case'] in ['accepted','expired-before-admission','permit-expires','stop-before-consumption','writer-held','manager-stop-held-writer']
    result['case'] = selected['case']
    assert not any(entry.pw_uid == UID or entry.pw_name == 'pokelinux-radio' for entry in pwd.getpwall())
    assert not any(entry.gr_gid == UID or entry.gr_name == 'pokelinux-radio' for entry in grp.getgrall())
    account_file = Path('/tmp/radio-fixture-sysusers.conf')
    account_file.write_text('g pokelinux-radio 1500\nu pokelinux-radio 1500:1500 "Public ARM radio fixture" /nonexistent /usr/bin/nologin\n')
    account_file.chmod(0o444)
    command(['systemd-sysusers', account_file], 'create-dedicated-fixture-principal')
    command(['systemd-sysusers', '/usr/lib/sysusers.d/pokelinux-radio.conf'], 'installed-original-sysusers')
    account = pwd.getpwnam('pokelinux-radio')
    assert account.pw_uid == account.pw_gid == UID and grp.getgrnam('pokelinux-radio').gr_gid == UID
    result['dedicated_fixture_principal'] = {'uid': UID, 'gid': UID, 'name': 'pokelinux-radio',
        'PAM_invoked': False, 'production_sysusers_bytes_unchanged': True}
    for name in ['operator_client.py']:
        target = FIXTURE / name
        target.write_bytes((R / 'source' / name).read_bytes()); target.chmod(0o444)
        assert target.stat().st_uid == 0 and sha(target) == sha(R / 'source' / name)
    root_config = Path('/etc/pokelinux/radio-add-pdn.json')
    root_config.parent.mkdir(parents=True, exist_ok=True)
    os.chown(root_config.parent, 0, UID); root_config.parent.chmod(0o750)
    assert not root_config.exists()
    root_config.write_bytes((R / 'source/config.json').read_bytes())
    os.chown(root_config, 0, UID); root_config.chmod(0o640)
    metadata = root_config.lstat()
    assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == 0 and metadata.st_gid == UID
    assert stat.S_IMODE(metadata.st_mode) == 0o640
    result['pre_start_config'] = {'path': str(root_config), 'sha256': sha(root_config),
        'source_sha256': sha(R / 'source/config.json'), 'uid': 0, 'gid': UID,
        'mode': '0640', 'provisioned_before_unit_start': True, 'authorizes_dispatch': False}
    conf = Path('/etc/pokelinux/radio.conf')
    conf.parent.mkdir(parents=True, exist_ok=True)
    assert not conf.exists()
    conf.write_text(f'BINDER_DEVICE={DEVICE}\nBINDER_SERVICE={SERVICE}\nLEASE_MS=30000\nSLOT_ID=1\n')
    conf.chmod(0o444)
    dropin = Path('/etc/systemd/system') / (UNIT + '.d')
    dropin.mkdir(parents=True)
    (dropin / '50-private-exit-observer.conf').write_text(
        '[Service]\nExecStopPost=/usr/bin/python3 /usr/libexec/pokelinux/radio-systemd-exit-observer.py /var/lib/pokelinux/radio/systemd-main-exit.json\n')
    (dropin / '50-private-exit-observer.conf').chmod(0o444)
    result['installed_unit_sha256'] = sha(Path('/usr/lib/systemd/system') / UNIT)
    assert selected['installed_unit_KillMode'] == 'mixed'
    assert selected['installed_unit_ExecStop_present'] is False
    result['installed_sysusers_sha256'] = sha('/usr/lib/sysusers.d/pokelinux-radio.conf')
    result['selected_private_dropin'] = (dropin / '50-private-exit-observer.conf').read_text()
    command(['systemctl', 'daemon-reload'], 'installed-unit-daemon-reload')
    binderfs = Path('/dev/binderfs')
    binderfs.mkdir(mode=0o755)
    command(['mount', '-t', 'binder', '-o', 'max=8,stats=global', 'binder', binderfs], 'binderfs-mount')
    with (binderfs / 'binder-control').open('rb', buffering=0) as control:
        device = bytearray(struct.pack('256sII', b'radio', 0, 0))
        fcntl.ioctl(control, 0xc1086201, device, True)
    _, major, minor = struct.unpack('256sII', device)
    radio = binderfs / 'radio'
    os.chown(radio, UID, UID)
    radio.chmod(0o666)
    with radio.open('rb', buffering=0) as driver:
        version = bytearray(4)
        fcntl.ioctl(driver, 0xc0046209, version, True)
    assert struct.unpack('i', version)[0] == 8
    result['guest_Binder_device'] = {'path': DEVICE, 'major': major, 'minor': minor,
        'protocol': 8, 'uid': UID, 'gid': UID, 'mode': '0666', 'created_in': 'private guest only', 'host_device_attached': False, 'deployment_permission_policy_claimed': False}
    manager_argv = [str(FIXTURE / 'manager')]
    manager, manager_watch = start(['setpriv','--reuid=1000','--regid=1000','--clear-groups',*manager_argv], 'manager.log')
    wait_for(lambda: f'READY-MANAGER pid={manager.pid}' in lines('manager.log'), bounded(20), 'public context manager')
    result['actual_manager_binding'] = identity(manager.pid, selected['binaries']['manager'], manager_argv, os.getpid(), True, expected_uid=1000)
    peer_argv = [str(FIXTURE / 'peer'), 'named']
    peer, peer_watch = start(['setpriv','--reuid=1000','--regid=1000','--clear-groups',*peer_argv], 'peer.log')
    wait_for(lambda: f'READY pid={peer.pid}' in lines('peer.log'), bounded(20), 'public named peer')
    result['actual_peer_binding'] = identity(peer.pid, selected['binaries']['peer'], peer_argv, os.getpid(), True, expected_uid=1000)
    logger, logger_watch = start(['journalctl', '--follow', '--unit=' + UNIT, '--output=cat', '--no-pager', '--boot=0', '--lines=all'], 'service-journal.log')
    assert logger.poll() is None and alive(logger_watch)
    started_at = now()
    setup_until = started_at + 10_000_000_000
    case_until = started_at + 29_000_000_000
    assert all(not os.path.lexists(str(JOURNAL) + suffix) for suffix in ['', '.writer', '.add-pdn'])
    result['fresh_epoch_all_markers_absent_before_start'] = True
    unit_selected = True
    command(['systemctl', 'start', UNIT], 'installed-unit-start', setup_until)
    first_state = show('installed-unit-active-properties', setup_until)
    assert first_state['ActiveState'] == 'active' and first_state['SubState'] == 'running'
    main_pid = int(first_state['MainPID'])
    assert main_pid > 1 and re.fullmatch('[0-9a-f]{32}', first_state['InvocationID'])
    first_invocation = first_state['InvocationID']
    main_watch = os.pidfd_open(main_pid, os.O_NONBLOCK)
    watches.append(main_watch)
    service_argv = [str(INSTALL / 'radio-service'), str(RUNTIME), str(JOURNAL), str(INSTALL / 'radio-worker'), DEVICE, SERVICE, '30000', '1']
    result['first_invocation_properties'] = first_state
    def started():
        live_fixture()
        return event('worker-start')
    worker_start = wait_for(started, setup_until, 'installed native worker-start')
    assert worker_start['main_pid'] == main_pid and worker_start['selected_slot'] == SLOT
    result['actual_systemd_Main_binding'] = identity(main_pid, selected['binaries']['radio-service'], service_argv, 1)
    main_stdout = os.stat(f'/proc/{main_pid}/fd/1')
    assert stat.S_ISSOCK(main_stdout.st_mode) or stat.S_ISFIFO(main_stdout.st_mode)
    fdinfo = Path(f'/proc/{main_pid}/fdinfo/1').read_text()
    flags = int(next(line.split()[1] for line in fdinfo.splitlines() if line.startswith('flags:')), 8)
    assert flags & os.O_NONBLOCK
    result['actual_service_stdout'] = {'mode': main_stdout.st_mode, 'fdinfo': fdinfo,
        'type': 'socket' if stat.S_ISSOCK(main_stdout.st_mode) else 'fifo', 'nonblocking': True,
        'production_journal_stream_retained': True}
    worker_pid = worker_start['worker_pid']
    assert type(worker_pid) is int and worker_pid > 1 and worker_pid not in [main_pid, peer.pid, manager.pid, os.getpid()]
    incarnation = worker_start['incarnation']
    assert re.fullmatch('[0-9a-f]{32}', incarnation)
    until_ns = worker_start['until_ns']
    assert u64(until_ns) and now() < until_ns
    worker_argv = [str(INSTALL / 'radio-worker'), '--pokelinux-process-worker', '--device', DEVICE,
        '--service', SERVICE, '--slot', '1', '--until-ns', str(until_ns)]
    worker_watch = os.pidfd_open(worker_pid, os.O_NONBLOCK)
    watches.append(worker_watch)
    result['actual_native_worker_binding'] = identity(worker_pid, selected['binaries']['radio-worker'], worker_argv, main_pid)
    active_group = cgroup_snapshot(first_state['ControlGroup'], 'active-installed-unit')
    assert str(main_pid) in active_group['procs'].split() and str(worker_pid) in active_group['procs'].split()
    assert first_state['ControlGroup'] in result['actual_systemd_Main_binding']['proc_cgroup']
    assert first_state['ControlGroup'] in result['actual_native_worker_binding']['proc_cgroup']
    def all_setup():
        live_fixture(require_worker=True)
        return event('transport-active') and event('raw-observed') and setup_witnesses(False)
    wait_for(all_setup, setup_until, 'positive transport and accepted RAW setup')
    active_event, raw_event = event('transport-active'), event('raw-observed')
    assert active_event['incarnation'] == raw_event['incarnation'] == incarnation
    assert active_event['attempt'] == raw_event['attempt'] == 1 and active_event['client_id'] == 42
    assert u64(active_event['revision']) and active_event['revision'] < raw_event['revision']
    for directory in [RUNTIME, STATE]:
        metadata = directory.lstat()
        assert stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == metadata.st_gid == UID
        assert stat.S_IMODE(metadata.st_mode) == 0o700
    result['actual_installed_private_directories'] = [{'path': str(path), 'uid': path.stat().st_uid, 'gid': path.stat().st_gid, 'mode': oct(stat.S_IMODE(path.stat().st_mode))} for path in [RUNTIME, STATE]]
    metadata = SOCKET.lstat()
    assert stat.S_ISSOCK(metadata.st_mode) and metadata.st_uid == metadata.st_gid == UID
    assert stat.S_IMODE(metadata.st_mode) == 0o600
    journal_bytes = ('admission-v1\n' + incarnation + '\n1\n').encode()
    journal_unchanged()
    result['journal_before_oracle'] = {'sha256': sha(JOURNAL), 'content': journal_bytes.decode(), 'uid': UID, 'mode': '0600'}
    result['positive_setup'] = {'worker_start': worker_start, 'transport_active': active_event,
        'raw_observed': raw_event, 'independent_public_peer': setup_witnesses(),
        'raw_observed_is_decoded_transport_input_and_not_status_history': True}
    assert now() < setup_until and alive(worker_watch)
    initial_status = status('setup-status-client-transport-codec', setup_until)
    assert initial_status['phase'] == 'observed-active' and initial_status['failure'] is None
    assert initial_status['revision'] >= raw_event['revision'] and initial_status['revision'] > 0
    initial_raw = initial_status['last_observation']
    assert initial_raw is not None and initial_raw['kind'] == 'raw'
    assert initial_raw['revision'] == raw_event['revision']
    assert {key: initial_raw[key] for key in initial_raw if key != 'revision'} == {
        'kind': 'raw', 'attempt': 1, 'slot_id': 0, 'serial': 7, 'rilc_msg_id': 137,
        'declared_length': 1, 'error': 0, 'payload_hex': '07'}
    result['actual_status_client_setup_qualified'] = True
    result['setup_complete'] = True
    import green_adapter
    adapter = green_adapter.make_adapter(globals())
    if selected['case'] == 'manager-stop-held-writer':
        import manager_stop_consumer
        outcome = manager_stop_consumer.run(adapter)
        assert outcome['classification'] == 'actual-manager-stop-case'
    else:
        import consumer
        outcome = consumer.run(adapter, selected['case'], baseline=False)
        assert outcome['classification'] == 'actual-boundary-case'
    assert outcome['case'] == selected['case']
    assert outcome['oracle_satisfied'] is True and outcome['response_crash_persistence_claimed'] is False
    result['consumer_outcome'] = outcome
    result['expected_outcome_observed'] = True
    result['passed'] = True

except BaseException as error:
    result['error'] = type(error).__name__ + ': ' + str(error)
    result['traceback'] = traceback.format_exc()
    print(result['traceback'], flush=True)
finally:
    # Any failed case may ask only the selected installed manager to stop its unit.
    # This disposal does not prove native worker wait or successful unit shutdown.
    if unit_selected and not result.get('normal_unit_Stop_qualified'):
        result['failure_only_systemd_stop_disposal'] = True
        try:
            command(['systemctl', 'stop', UNIT], 'failure-only-selected-unit-stop', bounded(6), expected=None)
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(type(error).__name__ + ': ' + str(error))
    for owner in reversed(children):
        child = owner['child']
        try:
            if not owner['wait_recorded'] and child.poll() is None:
                if child is logger:
                    if owner['watch'] is not None:
                        signal.pidfd_send_signal(owner['watch'], signal.SIGTERM)
                    else:
                        child.terminate()
                elif child is manager or child is peer:
                    if owner['watch'] is not None:
                        signal.pidfd_send_signal(owner['watch'], signal.SIGKILL)
                    else:
                        child.kill()
                else:
                    os.killpg(child.pid, signal.SIGKILL)
                    result['failure_only_process_group_disposal'] = True
            finish(child, owner['name'], bounded(3))
        except BaseException as error:
            result.setdefault('cleanup_errors', []).append(type(error).__name__ + ': ' + str(error))
    result['all_guest_direct_children_actually_waited'] = all(owner['wait_recorded'] for owner in children)
    for role, child in [('manager', manager), ('peer', peer), ('journal_reader', logger)]:
        result[role + '_actual_wait_after_case'] = next((row for row in result['cleanup'] if child is not None and row['pid'] == child.pid), None)
    for handle in watches:
        os.close(handle)
    result['all_selected_guest_inputs_unchanged'] = bool(selected and all(sha(R / name) == digest for name, digest in selected['files'].items()))
    result['installed_ELFs_unchanged'] = bool(selected and all(((FIXTURE if name in ['operator-controller', 'peer', 'manager'] else INSTALL) / name).is_file() and sha((FIXTURE if name in ['operator-controller', 'peer', 'manager'] else INSTALL) / name) == digest for name, digest in selected['binaries'].items()))
    result['installed_sysusers_original_bytes_retained'] = bool(selected and Path('/usr/lib/sysusers.d/pokelinux-radio.conf').exists() and sha('/usr/lib/sysusers.d/pokelinux-radio.conf') == selected['source_closure']['native-radio/packaging/pokelinux-radio.sysusers'])
    result['installed_unit_original_bytes_retained'] = bool(selected and (Path('/usr/lib/systemd/system') / UNIT).exists()
        and sha(Path('/usr/lib/systemd/system') / UNIT) == selected['source_closure']['native-radio/packaging/pokelinux-radio.service'])
    closure = bool(result['all_guest_direct_children_actually_waited'] and result['manager_actual_wait_after_case']
        and result['peer_actual_wait_after_case'] and result['journal_reader_actual_wait_after_case']
        and result['all_selected_guest_inputs_unchanged'] and result['installed_unit_original_bytes_retained']
        and result['installed_ELFs_unchanged'] and result['installed_sysusers_original_bytes_retained']
        and result.get('journal_retained_and_unchanged') and result.get('normal_unit_Stop_qualified')
        and result.get('same_journal_restart_refusal') and result.get('native_writer_actualwait')
        and result.get('all_markers_retained_and_unchanged') and not result.get('cleanup_errors')
        and not result.get('late_direct_waits') and not result.get('unqualified_status_pipe_closure')
        and not result.get('failure_only_process_group_disposal')
        and not result.get('failure_only_systemd_stop_disposal'))
    result['guest_case_closure_qualified'] = closure
    result['passed'] = bool(result['passed'] and closure)
    result['desired_boundary_red'] = bool(result['desired_boundary_red'] and closure)
    result['expected_outcome_observed'] = bool(result.get('expected_outcome_observed') and closure)
    (R / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result.get(key) for key in ['passed', 'setup_complete', 'desired_boundary_red', 'expected_outcome_observed', 'error']}, indent=2), flush=True)
    os.sync()
    subprocess.run(['systemctl', 'poweroff'], check=True, timeout=15)
