"""Desired normal Settings Rust SDK/app fixture + actual actor/Main packet case."""
import sys
sys.dont_write_bytecode = True
from pathlib import Path
import argparse
import array
import errno
import json
import os
import select
import signal
import socket
import time
import traceback
sys.path.insert(0, '/runner')
from owned import Scope, now
from closure import close_owned
import peer

# This one actual unprivileged consumer runs before the retaining controller
# drops SETUID/SETGID. It proves socket access denial, not an app grant decision.
if len(sys.argv) > 1 and sys.argv[1] == 'denied-peer':
    assert len(sys.argv) == 4
    endpoint, deadline = sys.argv[2], int(sys.argv[3])
    assert now() < deadline and os.getuid() == os.geteuid() == 0
    peer.call('prctl', 38, 1, 0, 0, 0)
    os.setgroups([])
    os.setresgid(11050, 11050, 11050)
    os.setresuid(11050, 11050, 11050)
    assert os.getuid() == os.geteuid() == os.getgid() == os.getegid() == 11050 and os.getgroups() == []
    cap = peer.caps()
    assert int(cap['CapEff'], 16) == int(cap['CapPrm'], 16) == 0 and cap['NoNewPrivs'] == '1'
    denied = None
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC) as connection:
        try:
            connection.settimeout(max(0, deadline-now()) / 1e9)
            connection.connect(endpoint)
        except OSError as error:
            denied = error.errno
    assert denied == errno.EACCES and now() < deadline
    print(json.dumps({'passed': True, 'actual_uid': os.getuid(), 'actual_gid': os.getgid(),
        'groups': os.getgroups(), 'caps': cap, 'connect_errno': denied,
        'private_access_denied': True, 'application_permission_decision': False,
        'observed_at_ns': now(), 'original_until_ns': deadline}), flush=True)
    raise SystemExit(0)

p = argparse.ArgumentParser()
p.add_argument('--main-elf', type=Path, required=True)
p.add_argument('--driver-elf', type=Path, required=True)
p.add_argument('--worker-elf', type=Path, required=True)
p.add_argument('--settings-app-elf', type=Path, required=True)
p.add_argument('--launcher-elf', type=Path, required=True)
p.add_argument('--storage-elf', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--work-until-ns', type=int, required=True)
p.add_argument('--close-until-ns', type=int, required=True)
a = p.parse_args()
T, W, C = now(), a.work_until_ns, a.close_until_ns
assert os.getuid() == os.geteuid() == os.getgid() == os.getegid() == 0
assert os.environ.get('POKELINUX_CONNECTIVITY_ISOLATED') == '1' and T < W < C
assert all(v.is_absolute() and v.is_file() for v in [a.main_elf, a.driver_elf, a.worker_elf, a.settings_app_elf, a.launcher_elf, a.storage_elf])
assert a.output.is_absolute() and not a.output.exists()
a.output.mkdir(mode=0o700)
O = a.output
G = O / 'gates'
G.mkdir(mode=0o700)
R = O / 'runtime'
R.mkdir(mode=0o700)
J = O / 'journal'
J.mkdir(mode=0o700)
for name in ['services', 'driver', 'provision']:
    (O / name).mkdir(mode=0o700)
services, drivers, provision = Scope(O / 'services'), Scope(O / 'driver'), Scope(O / 'provision')
endpoint = R / 'control.sock'
fds = []
worker_pidfd = None
control = None
keeper = main = driver = packet = None
worker_pid = None
main_stop_requested = False
ACTOR_CAPS = sum(1 << n for n in [5, 6, 7, 8])  # KILL, SETGID, SETUID, SETPCAP only; no network administration.
cleanup_released = False
r = {'passed': False, 'behavior_RED_qualified': False, 'classification': 'SETUP_FAILURE',
    'normal_Main_and_registered_Settings_actor_selected': True,
    'Settings_UID11050_actor_permission_executed': True, 'normal_Settings_Rust_SDK_in_app_fixture': True, 'normal_Qt_CABI_executed': False, 'Source_auth_GUI_executed': False,
    'initial_capability_cohort': 'NET_ADMIN SYS_ADMIN SETPCAP SETUID SETGID KILL',
    'actor_capability_cohort': 'SETUID SETGID KILL SETPCAP',
    'original_work_until_ns': W, 'original_close_until_ns': C, 'cleanup_errors': []}

def pump():
    for scope in [services, drivers, provision]:
        scope.pump()
        scope.check()
    if not main_stop_requested and worker_pid is not None and main is not None and services.live(main):
        actual = Path('/proc/' + str(main['child'].pid) + '/task/' + str(main['child'].pid) + '/children').read_text().split()
        assert actual == [str(worker_pid)], 'same actual retained network worker; no replacement'
        assert not select.select([worker_pidfd], [], [], 0)[0], 'same worker must remain living until normal Main Stop'

def complete(scope):
    return all(v['wait'] is not None and v['eof'] and v['log'].closed
        and v['child'].stdout.closed and v['pidfd'] is None for v in scope.children)

def wait_file(name, deadline, watch_driver=True):
    path = G / name
    while now() < deadline:
        pump()
        if path.exists():
            return json.loads(path.read_text())
        if watch_driver and driver is not None and not drivers.live(driver):
            raise RuntimeError('Rust driver exited before setup/functional gate ' + name)
        if main is not None and not services.live(main):
            raise RuntimeError('normal Main exited before setup/functional gate ' + name)
        time.sleep(.002)
    raise TimeoutError('original bounded gate ' + name)

def proc_status(pid):
    prefixes = ('PPid:', 'Uid:', 'Gid:', 'CapInh:', 'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:', 'NoNewPrivs:')
    return {line.split(':')[0]: line.split(':')[1].strip()
        for line in Path('/proc/' + str(pid) + '/status').read_text().splitlines()
        if line.startswith(prefixes)}

def cap0(value):
    return all(int(value[k], 16) == 0 for k in ['CapEff', 'CapPrm', 'CapBnd', 'CapInh', 'CapAmb']) and value['NoNewPrivs'] == '1'

def independent_readbacks(case):
    for phase, configured in [('initial', False), ('no-grant', False), ('connected', True), ('refused', False), ('disconnected', False)]:
        if (G / (phase + '-proof.json')).exists() and not (G / (phase + '-readback.json')).exists():
            value = peer.ipv4_snapshot('pklown0', min(case, now() + 1_000_000_000))
            index = r['initial_link_ifindex']
            assert value['ifindex'] == index
            assert value['ipv4'] == ([{'ifindex': index, 'prefix': 30, 'local': '192.0.2.1'}] if configured else [])
            assert value['default_route_absent'] and peer.namespace(fds[0]) == r['current_namespace']
            r[phase + '_independent_readback'] = value
            peer.publish(G, phase + '-readback.json', value)

try:
    # Exclusivity is provided by the trusted outer fresh namespace. This is only sanity.
    assert socket.if_nameindex() == [(1, 'lo')]
    peer.call('prctl', 36, 1, 0, 0, 0)
    local = os.open('/proc/thread-self/ns/net', os.O_RDONLY | os.O_CLOEXEC)
    fds.append(local)
    r['current_namespace'] = peer.namespace(local)
    control, other = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    keeper = services.start(['/usr/bin/python3', '-B', str(Path(peer.__file__).resolve()),
        'keeper', str(other.fileno()), str(C)], 'namespace-keeper.log', 65536,
        cutoff=W, wait_cutoff=C, identity={'pass_fds': (other.fileno(),)})
    other.close()
    while not select.select([control], [], [], .002)[0]:
        pump()
        assert now() < W and services.live(keeper)
    body, ancillary, flags, _ = control.recvmsg(4096, socket.CMSG_SPACE(array.array('i').itemsize))
    assert not flags
    kept = json.loads(body)
    rights_records = 0
    for level, kind, data in ancillary:
        assert level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS
        rights_records += 1
        received = array.array('i')
        received.frombytes(data)
        fds.extend(received)
    assert rights_records == 1 and len(fds) == 2
    assert peer.namespace(fds[1]) == kept['namespace'] and kept['namespace'] != r['current_namespace']
    r['peer_keeper'] = kept
    peer_path = '/proc/' + str(keeper['child'].pid) + '/ns/net'
    # One original six-second functional aggregate: normal startup, peer setup,
    # denial, real Rust Apply, traffic/background control and full800ms quiet.
    case = min(W, now() + 6_000_000_000)
    start_until = min(case, now() + 1_000_000_000)
    r['original_case_until_ns'], r['original_start_until_ns'] = case, start_until
    main = services.start([str(a.main_elf), str(endpoint), peer_path, str(J / 'admitted'),
        str(a.worker_elf), str(start_until)], 'normal-connectivity-main.log', 1024 * 1024,
        env=dict(os.environ, POKELINUX_CONNECTIVITY_ISOLATED='1'), cutoff=start_until,
        wait_cutoff=C, identity={'preexec_fn': lambda: peer.reduce_caps((1 << peer.NET_ADMIN) | (1 << 8))})
    while not endpoint.is_socket():
        pump()
        assert now() < start_until and services.live(main)
        time.sleep(.002)
    driver = drivers.start([str(a.driver_elf), '--exact',
        'service::restore_tests::connectivity_sdk_actor_tests::settings_sdk_connect_packet_lifecycle',
        '--ignored', '--nocapture', '--test-threads=1'], 'actual-registered-settings-actor.log', 1024 * 1024,
        env=dict(os.environ, POKELINUX_CONNECTIVITY_ISOLATED='1',
            CONNECTIVITY_ACTOR_ENDPOINT=str(endpoint), CONNECTIVITY_CASE_GATES=str(G),
            CONNECTIVITY_MAIN_PID=str(main['child'].pid), CONNECTIVITY_CASE_UNTIL_NS=str(case),
            CONNECTIVITY_CASE_CLOSE_UNTIL_NS=str(C), CONNECTIVITY_START_UNTIL_NS=str(start_until),
            NATIVE_LIFECYCLE_STORAGE_HELPER=str(a.storage_elf), NATIVE_LIFECYCLE_LAUNCHER_HELPER=str(a.launcher_elf),
            NATIVE_SETTINGS_CONNECTIVITY_APP_FIXTURE=str(a.settings_app_elf)),
        cutoff=start_until, wait_cutoff=C, identity={'preexec_fn': lambda: peer.reduce_caps(ACTOR_CAPS)})
    r['created'] = wait_file('created.json', start_until)
    assert r['created']['driver_pid'] == driver['child'].pid and r['created']['main_pid'] == main['child'].pid
    r['Main_caps'] = proc_status(main['child'].pid)
    r['driver_caps'] = proc_status(driver['child'].pid)
    assert cap0(r['Main_caps'])
    assert all(int(r['driver_caps'][k], 16) == ACTOR_CAPS for k in ['CapEff','CapPrm','CapBnd'])
    assert all(int(r['driver_caps'][k],16) == 0 for k in ['CapInh','CapAmb']) and r['driver_caps']['NoNewPrivs'] == '1'
    worker_children = Path('/proc/' + str(main['child'].pid) + '/task/' + str(main['child'].pid) + '/children').read_text().split()
    assert len(worker_children) == 1, 'exactly one actual worker child under normal Main'
    worker_pid = int(worker_children[0])
    r['worker_pid'] = worker_pid
    worker_pidfd = os.pidfd_open(worker_pid, os.O_NONBLOCK)
    r['worker_caps'] = proc_status(worker_pid)
    assert int(r['worker_caps']['PPid']) == main['child'].pid
    assert all(int(r['worker_caps'][k], 16) == 1 << peer.NET_ADMIN for k in ['CapEff', 'CapPrm', 'CapBnd'])
    assert int(r['worker_caps']['CapInh'], 16) == int(r['worker_caps']['CapAmb'], 16) == 0 and r['worker_caps']['NoNewPrivs'] == '1'
    r['actual_Main_driver_worker_capability_split'] = True
    initial = peer.ipv4_snapshot('pklown0', min(case, now() + 1_000_000_000))
    assert initial['ipv4'] == []
    r['initial_link_ifindex'] = initial['ifindex']
    # A separate actual unprivileged process cannot open the private root endpoint.
    deny_until = min(case, now() + 1_000_000_000)
    deny_code, deny_data = provision.run(['/usr/bin/python3', '-B', str(Path(__file__).resolve()),
        'denied-peer', str(endpoint), str(deny_until)], 'actual-UID11050-private-denial.log', deny_until, limit=65536)
    assert deny_code == 0
    r['private_access_denial'] = json.loads(deny_data)
    assert r['private_access_denial']['private_access_denied'] and not r['private_access_denial']['application_permission_decision']
    # The short-lived privileged peer provisioner is completely closed before packet peer.
    provision_until = min(case, now() + 1_000_000_000)
    code, data = provision.run(['/usr/bin/python3', '-B', str(Path(peer.__file__).resolve()),
        'provision', peer_path, str(provision_until)], 'peer-provisioner.log', provision_until, limit=65536)
    assert code == 0
    r['peer_provisioner'] = json.loads(data)
    assert r['peer_provisioner']['passed']
    r['provisioner_closure'] = close_owned(provision, C, reap_adopted=False)
    assert r['provisioner_closure']['closed'] and complete(provision)
    provisioner_receipt = next(v for v in provision.receipts if v['role'] == 'peer-provisioner.log')
    packet = services.start(['/usr/bin/python3', '-B', str(Path(peer.__file__).resolve()),
        'serve', peer_path, str(G), str(case)], 'cap0-packet-peer.log', 65536, cutoff=case, wait_cutoff=C)
    r['peer_ready'] = wait_file('peer-ready.json', case)
    cap = r['peer_ready']['caps']
    assert r['peer_ready']['pid'] == packet['child'].pid and r['peer_ready']['uid'] == 0 and cap0(cap)
    inventory = r['peer_ready']['actual_fd_inventory']
    assert r['peer_ready']['only_packet_socket_no_admin_descriptors']
    assert set(inventory) == {'0', '1', '2', next(k for k, v in inventory.items() if v.startswith('socket:['))}
    r['supervisor_caps'] = peer.reduce_caps()
    assert cap0(r['supervisor_caps']) and now() < case
    ready = {'provisioner_reaped_before_packet_peer': provisioner_receipt['waited_at_boottime_ns'] < packet['spawn_started_at_ns'],
        'supervisor_all_caps_zero': True, 'packet_peer_all_caps_zero': True, 'Main_all_caps_zero': True,
        'actor_only_SETUID_SETGID_KILL_SETPCAP': True, 'worker_NET_ADMIN_only': True, 'actual_UID11050_private_access_denied': True,
        'released_at_ns': now(), 'original_case_until_ns': case, 'initial_link_ifindex': r['initial_link_ifindex']}
    assert ready['provisioner_reaped_before_packet_peer']
    peer.publish(G, 'ready.json', ready)
    r['healthy_setup_qualified'] = True
    while not (G / 'functional-finished.json').exists() and now() < case:
        pump()
        independent_readbacks(case)
        assert drivers.live(driver) and services.live(main), 'functional actor/normal Main must remain alive until result gate'
        time.sleep(.002)
    assert (G / 'functional-finished.json').exists() and now() < case
    functional = json.loads((G / 'functional-finished.json').read_text())
    r['functional'] = functional
    actor_closed = json.loads((G / 'actor-closed.json').read_text())
    assert actor_closed['actor_stopped_and_joined'] and actor_closed['app_stdout_stderr'] == 'null'
    assert len(actor_closed['actual_Apps_waits']) == (1 if functional.get('genuine_refused_candidate') else 2)
    assert all(v['code'] == 0 and v['signal'] is None and v['PIPE_EOF_claim'] is False for v in actor_closed['actual_Apps_waits'])
    assert actor_closed['completed_at_ns'] < case
    stops = [functional['first_sdk_stop_join']]
    if not functional.get('genuine_refused_candidate'):
        stops.append(functional['second_sdk_stop_join'])
    for stopped in stops:
        assert stopped['stop_requested'] and stopped['joined'] and stopped['join_status'] == 0
        assert stopped['actual_task_count_after_join'] == 1
        assert stopped['stop_call_before_ns'] <= stopped['joined_at_ns'] < stopped['fixture_stop_cutoff_ns'] <= case
        assert stopped['fixture_stop_cutoff_ns'] - stopped['stop_call_before_ns'] <= 200_000_000
    r['SDK_stop_join_observations'] = stops
    r['actual_actor_and_app_closure_before_Main_Stop'] = actor_closed
    r['functional_finished_before_original_case_cutoff'] = functional['functional_finished_at_ns'] < case
    # Signal only retained Main pidfd; normal Main owns its worker Stop/reap/EOF.
    main_stop_requested = True
    signal.pidfd_send_signal(main['pidfd'], signal.SIGTERM)
    r['normal_Main_exit'] = services.finish(main, C)
    assert r['normal_Main_exit'] == 0
    closure_rows = []
    for line in (O / 'services' / 'normal-connectivity-main.log').read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(row, dict) and row.get('event') == 'connectivity_main_closure':
            closure_rows.append(row)
    assert len(closure_rows) == 1
    terminal = closure_rows[0]
    r['Main_reported_worker_closure'] = terminal
    assert terminal['closed'] and terminal['reaped'] and terminal['output_eof'] and not terminal['occupied'] and terminal['namespace_handles_open']
    assert terminal['worker_exit_code'] is not None or terminal['worker_exit_signal'] is not None
    assert terminal['completed_at_ns'] < main['wait']['waited_at_boottime_ns'] < C
    assert select.select([worker_pidfd], [], [], 0)[0], 'independently retained worker pidfd must observe terminal state'
    r['independent_worker_pidfd_terminal_after_Main_wait'] = True
    # Stop peer before its functional cutoff; terminal quiet receipt already belongs to body.
    (G / 'stop-peer').touch()
    r['packet_peer_exit'] = services.finish(packet, C)
    assert r['packet_peer_exit'] == 0
    control.sendall(b'STOP')
    r['keeper_exit'] = services.finish(keeper, C)
    assert r['keeper_exit'] == 0
    r['services_closure_before_Rust_assertion'] = close_owned(services, C, reap_adopted=False)
    assert r['services_closure_before_Rust_assertion']['closed'] and complete(services) and complete(provision)
    retained = len(fds) == 2 and peer.namespace(fds[0]) == r['current_namespace'] and peer.namespace(fds[1]) == r['peer_keeper']['namespace']
    assert retained
    peer.publish(G, 'cleanup-complete.json', {'main_worker_packet_keeper_actual_wait_EOF_closed': True,
        'all_namespace_handles_retained': retained, 'completed_at_ns': now(), 'original_close_until_ns': C})
    cleanup_released = True
    r['driver_exit'] = drivers.finish(driver, C)
    assertion = json.loads((G / 'desired-assertion.json').read_text())
    r['Rust_assertion_after_backend_cleanup'] = assertion
    assert terminal['completed_at_ns'] <= assertion['cleanup_completed_at_ns'] <= assertion['asserted_after_cleanup_at_ns'] < C
    driver_text = (O / 'driver' / 'actual-registered-settings-actor.log').read_text()
    if functional.get('genuine_refused_candidate'):
        attempted = json.loads((G / 'connect-attempt.json').read_text())
        returned = json.loads((G / 'connect-return.json').read_text())
        assert r['driver_exit'] == 101 and 'SETTINGS_CONNECTIVITY_SDK_REQUIRES_ACCEPTED_CONNECT_AND_REAL_OBSERVED_TRUE' in driver_text
        assert returned['choice_accepted'] is False
        assert ready['released_at_ns'] <= attempted['attempted_at_ns'] <= returned['returned_at_ns'] < attempted['original_until_ns'] <= case
        assert r['refused_independent_readback']['ipv4'] == [] and not (G / 'peer-packets.json').exists()
        quiet = json.loads((G / 'peer-quiet.json').read_text())
        assert quiet['received_datagrams'] == 0 and quiet['quiet_end_actual_EAGAIN']
        assert quiet['quiet_end_EAGAIN_at_ns'] >= quiet['original_until_ns']
        r['genuine_registered_granted_SDK_Connect_REFUSED_RED'] = True
        r['classification'] = 'ACTUAL_REGISTERED_GRANTED_SETTINGS_SDK_CONNECT_REFUSED_RED'
    else:
        assert r['driver_exit'] == 0 and functional['functional_green']
        packets = json.loads((G / 'peer-packets.json').read_text())
        assert len(packets['packets']) == 2
        assert packets['packets'][0]['payload_hex'] == functional['first_packet']['nonce_hex']
        assert packets['packets'][1]['payload_hex'] == functional['background_packet_after_withdraw']['nonce_hex']
        quiet = json.loads((G / 'peer-quiet.json').read_text())
        assert quiet['received_datagrams'] == 0 and quiet['quiet_end_actual_EAGAIN'] and quiet['quiet_end_EAGAIN_at_ns'] >= quiet['original_until_ns']
        assert quiet['completed_at_ns'] >= quiet['original_until_ns'] and not (G / 'peer-unexpected-datagram.json').exists()
        r['packet_peer_receipts'] = packets
        r['functional_GREEN'] = True
        r['classification'] = 'ACTUAL_NORMAL_SETTINGS_SDK_FIXTURE_ACTOR_MAIN_PACKET_GREEN'
except BaseException as error:
    r.update(error=repr(error), traceback=traceback.format_exc())
finally:
    # All emergency work preserves the separately selected original close cutoff.
    # A missing cleanup gate or forced adopted-child reap can never qualify RED/GREEN.
    (G / 'stop-peer').touch()
    if control is not None and keeper is not None:
        try:
            if services.live(keeper):
                control.sendall(b'STOP')
        except BaseException as error:
            r['cleanup_errors'].append('keeper control ' + repr(error))
    if not cleanup_released and main is not None:
        main_stop_requested = True
        try:
            if services.live(main) and main['pidfd'] is not None:
                signal.pidfd_send_signal(main['pidfd'], signal.SIGTERM)
        except BaseException as error:
            r['cleanup_errors'].append('Main TERM ' + repr(error))
    r['provision_final_closure'] = close_owned(provision, C, reap_adopted=False)
    r['services_final_closure'] = close_owned(services, C, reap_adopted=False)
    r['driver_final_closure'] = close_owned(drivers, C, reap_adopted=True)
    receipts = services.receipts + drivers.receipts + provision.receipts
    r['owned_receipts'] = receipts
    r['all_actual_wait_EOF_timely'] = bool(receipts) and all(v.get('actual_direct_parent_wait') and v.get('stdout_EOF')
        and v.get('wait_before_original_cutoff') and v.get('EOF_before_original_cutoff') for v in receipts)
    r['namespace_FDs_retained_through_all_waits_EOF'] = bool(complete(services) and complete(drivers) and complete(provision)
        and len(fds) == 2 and peer.namespace(fds[0]) == r.get('current_namespace')
        and peer.namespace(fds[1]) == r.get('peer_keeper', {}).get('namespace'))
    r['custody_qualified'] = bool(not r['cleanup_errors'] and cleanup_released and r.get('independent_worker_pidfd_terminal_after_Main_wait')
        and r['provision_final_closure']['closed'] and r['services_final_closure']['closed'] and r['driver_final_closure']['closed']
        and r['driver_final_closure']['wait_all_ECHILD'] and not r['driver_final_closure']['adopted_actual_waits']
        and r['namespace_FDs_retained_through_all_waits_EOF'] and r['all_actual_wait_EOF_timely'] and now() < C)
    if worker_pidfd is not None:
        os.close(worker_pidfd)
    for fd in reversed(fds):
        os.close(fd)
    if control is not None:
        control.close()
    r['no_unexpected_peer_datagram_after_actual_peer_wait'] = not (G / 'peer-unexpected-datagram.json').exists()
    r['behavior_RED_qualified'] = bool(r.get('genuine_registered_granted_SDK_Connect_REFUSED_RED') and r.get('healthy_setup_qualified')
        and r.get('actual_Main_driver_worker_capability_split') and r['custody_qualified'])
    r['passed'] = bool(r.get('functional_GREEN') and r['custody_qualified'] and r['no_unexpected_peer_datagram_after_actual_peer_wait'])
    (O / 'result.json').write_text(json.dumps(r, indent=2) + '\n')
    print(json.dumps({'passed': r['passed'], 'behavior_RED_qualified': r['behavior_RED_qualified'],
        'classification': r['classification'], 'custody_qualified': r['custody_qualified'], 'error': r.get('error')}), flush=True)
raise SystemExit(0 if r['passed'] else 1)
