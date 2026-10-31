"""Frozen normal Main/actor persistent-wireless acceptance consumer.

run_case binds the existing generic ARM hwsim owner's actual station, AP,
namespace keeper and Scope. Only the outer owner may qualify and assert after
its AP, radios, namespace handles and VM custody have actually closed.
"""
import importlib.util
from pathlib import Path
import sys
sys.dont_write_bytecode = True

_SOURCE = Path(__file__).resolve().parents[1] / 'native-wireless-association/address-packets/address_consumer.py'
if not _SOURCE.is_file():
    _SOURCE = Path(__file__).resolve().with_name('address_consumer.py')
_SPEC = importlib.util.spec_from_file_location('wireless_address_observations', _SOURCE)
address = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(address)
need = address.need
integer = address.integer


def existing_identity(fact, selected):
    """Normalized recorder observation, not a new production wire schema."""
    address.target(selected)
    need(type(fact) is dict, 'existing-link observation object')
    need(fact.get('target') == selected, 'same exact existing-link target')
    need('observed_lifetime' in fact and fact['observed_lifetime'] is None,
         'existing link must not invent a witnessed create lifetime')
    need(type(fact.get('configured')) is bool, 'explicit profile configuration')
    return fact['configured']


def persistent_identity(samples, *, selected, worker, requester_closed_at, until):
    """Check monotonic samples after requester closure; radio checked separately."""
    integer(requester_closed_at, 1, address.U64 - 1, 'requester closure time')
    integer(until, requester_closed_at + 1, address.U64, 'original observation cutoff')
    need(type(worker) is str and len(worker) == 32
         and all(c in '0123456789abcdef' for c in worker), 'bound worker identity')
    need(type(samples) is list and 2 <= len(samples) <= 32, 'bounded repeated observations')
    previous = requester_closed_at
    for sample in samples:
        need(type(sample) is dict and sample.get('worker') == worker,
             'same persistent worker, no replacement')
        need(sample.get('retained_worker_pidfd_live') is True,
             'retained worker must still be observed live')
        need(existing_identity(sample.get('profile'), selected), 'profile remains configured')
        previous = integer(sample.get('observed_at_ns'), previous + 1, until - 1,
                           'fresh monotonic observation after actual requester closure')
    return previous


def independent_connected_readback(*, selected, address_body, address_receipt,
                                   armed, until, **l2):
    """Reuse actual address and station/AP parsers; IP alone never suffices."""
    need(type(address_body) is bytes and len(address_body) <= 262144, 'bounded raw address readback')
    address.receipt(address_receipt,
                    expected_argv=['/usr/bin/ip', '-j', 'address', 'show', 'dev', selected['name']],
                    until=until, armed=armed, code=0,
                    byte_count=len(address_body), limit=262144)
    profile = address.kernel_profile(address_body, selected=selected, configured=True)
    need(l2['selected_station']['ifindex'] == selected['ifindex']
         and l2['selected_station']['name'] == selected['name']
         and {k: l2['namespace_binding']['station_namespace'][k] for k in ['dev', 'ino']} == selected['namespace'],
         'radio and profile select the same interface and namespace')
    radio = address.l2_observation(armed=armed, until=until, **l2)
    return {'profile': profile, 'radio': radio}


def explicit_disconnect_order(*, requested_at, original_until, profile_removed_at,
                              disconnect_submitted_at, radio_disconnected_at):
    """Recorded operation order only; never infer radio cleanup from process exit."""
    integer(requested_at, 1, address.U64 - address.ACTION_NS, 'explicit Disconnect request')
    integer(original_until, requested_at + 1, requested_at + address.ACTION_NS,
            'unchanged action cutoff')
    integer(profile_removed_at, requested_at, original_until - 1, 'profile removal readback')
    integer(disconnect_submitted_at, profile_removed_at, original_until - 1,
            'radio Disconnect follows profile removal')
    integer(radio_disconnected_at, disconnect_submitted_at, original_until - 1,
            'radio readback within original cutoff')


DESIRED_MARKER = 'NORMAL_WIRELESS_SERVICE_REQUIRES_PERSISTENT_ACTOR_PACKET_LIFECYCLE'
ACTOR_TEST = ('service::restore_tests::connectivity_wireless_actor_tests::'
              'settings_sdk_wireless_packet_lifecycle')
START_NS = 3_000_000_000
ACTOR_CAPS = sum(1 << number for number in [5, 6, 7, 8])


def pin(path):
    import hashlib
    path = Path(path)
    with path.open('rb') as stream:
        return {'bytes': path.stat().st_size,
                'sha256': hashlib.file_digest(stream, 'sha256').hexdigest()}


def prepare(run, inputs, selection, work_until):
    """Install all six exact selected ARM ELFs; use the existing owner's runner."""
    import os
    import stat
    from owned import now
    rows = selection['service_binaries']
    need(set(rows) == {'main', 'worker', 'driver', 'settings_app', 'launcher', 'storage'},
         'all six actual normal/fixture ELF selections')
    binaries = {}
    for role, row in rows.items():
        source, destination = inputs / row['relative_path'], Path(row['guest_path'])
        need(pin(source) == row['ELF'], 'selected ELF pin: ' + role)
        need(destination.is_absolute() and not os.path.lexists(destination), 'fresh protected ELF path')
        for directory in destination.parents:
            value = directory.lstat()
            need(stat.S_ISDIR(value.st_mode) and value.st_uid == value.st_gid == 0
                 and not value.st_mode & 0o022, 'protected executable ancestry')
        with source.open('rb') as src, destination.open('xb') as dst:
            while block := src.read(65536):
                need(now() < work_until, 'original ELF installation cutoff')
                dst.write(block)
            dst.flush()
            os.fsync(dst.fileno())
        destination.chmod(0o755)
        os.chown(destination, 0, 0)
        value = destination.lstat()
        need(stat.S_ISREG(value.st_mode) and value.st_uid == value.st_gid == 0
             and stat.S_IMODE(value.st_mode) == 0o755 and value.st_nlink == 1
             and 'security.capability' not in os.listxattr(destination)
             and pin(destination) == row['ELF'], 'exact installed protected ELF')
        with destination.open('rb') as stream:
            header = stream.read(20)
        need(header[:6] == b'\x7fELF\x02\x01'
             and int.from_bytes(header[18:20], 'little') == 183, 'actual ARM64 ELF')
        loader = run(['/usr/bin/ldd', '-r', str(destination)], 'normal-wireless-' + role + '-loader')
        if type(loader) is bytes:
            loader = loader.decode()
        need(loader.strip() and 'not found' not in loader.lower()
             and 'undefined symbol' not in loader.lower(), 'exact ELF loader: ' + role)
        binaries[role] = destination
    return binaries


def run_case(*, root, inputs, selection, station, ap, ap_prefix, namespace_binding,
             outer_scope, work_until, close_until, binaries, owned_scope):
    """Drive real actor/SDK, Main, one worker and existing cap0 packet helpers.

    The caller has already created two exclusive hwsim radios, moved AP to its
    retained namespace and started owned foreground hostapd. No association or
    station address mutation is permitted before this case. There is one Main
    startup, one granted Connect and one explicit Disconnect; no effect retry.
    """
    import hashlib
    import json
    import os
    import re
    import select
    import signal
    import socket
    import stat
    import struct
    import time
    import traceback
    from owned import now
    import worker_caps
    import wifi_peer

    out = root / 'wireless-service-case'
    out.mkdir(mode=0o700)
    scope = owned_scope
    need(scope.output == out and not scope.children, 'fresh existing owned Scope for this case')
    gates, runtime, journal = [out / name for name in ['gates', 'runtime', 'journal']]
    for path in [gates, runtime, journal]:
        path.mkdir(mode=0o700)
    source = inputs / 'source'
    peer_script = source / 'actor_peer.py'  # unchanged native-connectivity-actor/peer.py
    provision_script = source / 'wifi_peer.py'
    client_script = source / 'udp_client.py'
    need(all(path.is_file() for path in [peer_script, provision_script, client_script]),
         'existing actual peer and packet helpers staged')
    selected = {'namespace': {key: namespace_binding()['station_namespace'][key] for key in ['dev', 'ino']},
                'ifindex': station['ifindex'], 'name': station['name']}
    address.target(selected)
    endpoint, profile = runtime / 'control.sock', runtime / 'startup-profile.json'
    startup = {'backend': 'wireless_open', 'target': selected,
               'frequency_mhz': ap['frequency_mhz'],
               'bssid': list(bytes.fromhex(ap['bssid_hex'])),
               'ssid': list(bytes.fromhex(ap['ssid_hex']))}
    with profile.open('x') as stream:
        os.chmod(profile, 0o600)
        stream.write(json.dumps(startup) + '\n')
        stream.flush()
        os.fsync(stream.fileno())
    info = profile.lstat()
    need(stat.S_ISREG(info.st_mode) and info.st_uid == info.st_gid == 0
         and stat.S_IMODE(info.st_mode) == 0o600 and info.st_nlink == 1,
         'protected exact startup profile')
    main = driver = packet = None
    worker_pid = worker_pidfd = None
    worker_id = None
    stopping = False
    pending_clients = set()
    r = {'schema': 1, 'passed': False, 'baseline_profile_rejection_candidate': False,
         'normal_Main_and_actual_actor_selected': True, 'startup_profile': startup,
         'startup_profile_pin': pin(profile), 'target': selected,
         'original_work_until_ns': work_until, 'original_close_until_ns': close_until,
         'Connect_commands_admitted': 0, 'Disconnect_commands_admitted': 0,
         'UDP_roundtrip_clients_admitted': 0, 'radio_or_address_retry_admitted': False,
         'normal_Settings_GUI_Source_auth_executed': False, 'phone_accessed': False}

    def pump():
        outer_scope.pump()
        outer_scope.check()
        scope.pump()
        scope.check()
        binding = namespace_binding()
        need({key: binding['station_namespace'][key] for key in ['dev', 'ino']} == selected['namespace'],
             'same retained station namespace')
        if worker_pid is not None and not stopping:
            need(main is not None and scope.live(main), 'original normal Main remains live')
            children = Path('/proc/' + str(main['child'].pid) + '/task/'
                            + str(main['child'].pid) + '/children').read_text().split()
            need(children == [str(worker_pid)]
                 and not select.select([worker_pidfd], [], [], 0)[0],
                 'same actual worker lives; no replacement')

    def run(argv, label, end, limit=262144, identity=None):
        pump()
        need(now() < end, 'original command cutoff')
        code, body = scope.run(argv, label, end, limit=limit, identity=identity)
        pump()
        need(code == 0 and now() < end, 'actual command completion: ' + label)
        return body, scope.receipts[-1]

    def publish(name, value):
        wifi_peer.publish(gates, name, value)

    def gate(name, end):
        while now() < end:
            pump()
            path = gates / name
            if path.exists():
                return json.loads(path.read_text())
            need(driver is None or scope.live(driver), 'actor exited before concrete gate ' + name)
            time.sleep(.001)
        raise TimeoutError('original gate cutoff: ' + name)

    def process_caps(pid):
        keys = ('PPid:', 'Uid:', 'Gid:', 'CapInh:', 'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:', 'NoNewPrivs:')
        return {line.split(':')[0]: line.split(':')[1].strip()
                for line in Path('/proc/' + str(pid) + '/status').read_text().splitlines()
                if line.startswith(keys)}

    def caps(value, retained):
        need(all(int(value[key], 16) == retained for key in ['CapEff', 'CapPrm', 'CapBnd'])
             and int(value['CapInh'], 16) == int(value['CapAmb'], 16) == 0
             and value['NoNewPrivs'] == '1', 'actual process capability cohort')

    def inspect(end):
        """Readonly normal protocol, independently credential-bound to Main."""
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_NONBLOCK | socket.SOCK_CLOEXEC) as connection:
            connection.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            connection.connect(str(endpoint))
            expected = (main['child'].pid, 0, 0)
            need(struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)) == expected,
                 'readonly actual normal Main peer')
            request = {'op': 'inspect', 'id': 1, 'until_ns': str(end)}
            body = json.dumps(request).encode()
            need(connection.send(body) == len(body), 'one readonly request send')
            while now() < end:
                pump()
                try:
                    body, ancillary, flags, _ = connection.recvmsg(1024, socket.CMSG_SPACE(12))
                except BlockingIOError:
                    time.sleep(.001)
                    continue
                need(body and not flags and len(ancillary) == 1
                     and ancillary[0][:2] == (socket.SOL_SOCKET, socket.SCM_CREDENTIALS)
                     and struct.unpack('3i', ancillary[0][2]) == expected, 'normal readonly reply credentials')
                value = json.loads(body, object_pairs_hook=address.unique)
                need(set(value) == {'schema', 'id', 'status', 'state'}
                     and value['schema'] == value['id'] == 1, 'normal private reply shape')
                state = value['state']
                need(set(state) == {'available', 'configured', 'observed_at_ns', 'until_ns',
                                    'pending', 'terminal_uncertain', 'worker'}, 'normal state shape')
                if state['available']:
                    at, expires = int(state['observed_at_ns']), int(state['until_ns'])
                    need(value['status'] == 'observed' and type(state['configured']) is bool
                         and not state['pending'] and not state['terminal_uncertain']
                         and 0 < at <= now() < expires == at + 250_000_000
                         and re.fullmatch('[0-9a-f]{32}', state['worker']), 'fresh actual normal service observation')
                else:
                    need(value['status'] == 'unavailable' and state['configured'] is None
                         and state['observed_at_ns'] == state['until_ns'] == '0', 'actual unavailable state')
                value['received_at_ns'] = now()
                need(now() < end, 'original readonly reply cutoff')
                return value
        raise TimeoutError('original normal Main Inspect cutoff')

    def readback(label, configured, end):
        """Four namespace-bound radio commands plus address/route readback."""
        armed = now()
        outputs, receipts = {}, {}
        for key, argv in [
            ('link', ['/usr/bin/iw', 'dev', station['name'], 'link']),
            ('AP_station_dump', ap_prefix + ['/usr/bin/iw', 'dev', ap['name'], 'station', 'dump']),
            ('station_roles', ['/usr/bin/iw', 'dev']),
            ('AP_roles', ap_prefix + ['/usr/bin/iw', 'dev'])]:
            outputs[key], receipts[key] = run(argv, label + '-' + key + '.log', end)
            address.receipt(receipts[key], expected_argv=argv, until=end, armed=armed,
                            code=0, byte_count=len(outputs[key]), limit=262144)
        # The same existing parsers validate identity/mode even while disconnected.
        for key, role, mac, kind in [('station_roles', station, 'mac_hex', 'managed'),
                                     ('AP_roles', ap, 'bssid_hex', 'AP')]:
            text = outputs[key].decode()
            need(re.findall(r'^phy#([0-9]+)', text, re.M) == [str(role['wiphy'])], 'exact radio phy')
            fields = {}
            for line in text.splitlines():
                match = re.fullmatch(r'(Interface|ifindex|wdev|addr|type) (.+)', line.strip())
                if match:
                    need(match[1] not in fields, 'duplicate independent radio identity')
                    fields[match[1]] = match[2]
            need(fields == {'Interface': role['name'], 'ifindex': str(role['ifindex']),
                            'wdev': hex(role['wdev']), 'addr': ':'.join(role[mac][i:i+2] for i in range(0, 12, 2)),
                            'type': kind}, 'same selected actual radio identity/mode')
        binding = namespace_binding()
        if configured:
            radio = address.l2_observation(selected_station=station, selected_AP=ap,
                outputs=outputs, receipts=receipts, ap_prefix=ap_prefix, armed=armed,
                until=end, namespace_binding=binding)
        else:
            need(outputs['link'].strip() == b'Not connected.'
                 and outputs['AP_station_dump'].strip() == b'', 'independent station/AP disconnection')
            radio = {'confirmed': True, 'station_unassociated': True, 'observed_at_ns': binding['observed_at_ns'],
                     'namespace_binding': binding}
        body, actual = run(['/usr/bin/ip', '-j', 'address', 'show', 'dev', selected['name']],
                           label + '-GETADDR.log', end)
        address.receipt(actual, expected_argv=['/usr/bin/ip', '-j', 'address', 'show', 'dev', selected['name']],
                        until=end, armed=armed, code=0, byte_count=len(body), limit=262144)
        profile_observation = address.kernel_profile(body, selected=selected, configured=configured)
        route, route_actual = run(['/usr/bin/cat', '/proc/net/route'], label + '-routes.log', end, limit=65536)
        routes = address.route_profile(route.decode(), selected, configured)
        value = {'radio': radio, 'profile': profile_observation, 'routes': routes,
                 'receipts': dict(receipts, GETADDR=actual, routes=route_actual),
                 'raw_sha256': {key: hashlib.sha256(body).hexdigest() for key, body in outputs.items()},
                 'observed_at_ns': now(), 'original_until_ns': end}
        r[label + '_independent_readback'] = value
        return value

    def packet_client(tag, request):
        need(tag not in pending_clients, 'one actual cap0 client per nonce')
        pending_clients.add(tag)
        payload = bytes.fromhex(request['nonce_hex'])
        need(len(payload) == 37 and payload[:5] == b'PKL1' + bytes([tag]), 'actual actor fresh nonce')
        end = request['original_until_ns']
        integer(end, request['requested_at_ns'] + 1,
                min(case, request['requested_at_ns'] + address.ACTION_NS), 'original packet cutoff')
        argv = ['/usr/bin/python3', '-B', str(client_script), 'roundtrip', payload.hex(), str(end)]
        r['UDP_roundtrip_clients_admitted'] += 1
        body, actual = run(argv, 'cap0-nonce-' + str(tag) + '.log', end, limit=32768,
                           identity={'preexec_fn': lambda: worker_caps.reduce_caps(0)})
        value = address.decode(body)
        caps(value['caps'], 0)
        need(value['passed'] and value['mode'] == 'roundtrip' and value['payload_hex'] == payload.hex()
             and value['reply_hex'] == '41434b31' + payload.hex()
             and value['namespace'] == selected['namespace']
             and value['peer'] == ['192.0.2.2', 36042] and value['source'][0] == '192.0.2.1'
             and value['original_until_ns'] == end, 'actual cap0 nonce/ACK and namespace')
        address.receipt(actual, expected_argv=argv, until=end, armed=request['requested_at_ns'],
                        code=0, byte_count=len(body), limit=32768)
        inventory = value['actual_fd_inventory']
        need(len(inventory) == 4 and set(['0', '1', '2']).issubset(inventory)
             and sum(path.startswith('socket:[') for path in inventory.values()) == 1,
             'cap0 client owns only packet socket and standard descriptors')
        value['actual_cap0_client_wait_EOF'] = True
        r['nonce_' + str(tag)] = {'value': value, 'receipt': actual}
        publish('nonce-' + str(tag) + '-complete.json', value)

    def main_stop():
        nonlocal stopping
        stopping = True
        if main is not None and scope.live(main):
            signal.pidfd_send_signal(main['pidfd'], signal.SIGTERM)
        if main is not None:
            r['normal_Main_exit'] = scope.finish(main, close_until)
            rows = []
            non_json = []
            for line in (out / main['name']).read_text().splitlines():
                try:
                    value = json.loads(line)
                except json.JSONDecodeError:
                    non_json.append(line)
                    continue
                if type(value) is dict and value.get('event') == 'connectivity_main_closure':
                    rows.append(value)
            r['Main_non_JSON_lines'] = non_json
            need(len(rows) == 1, 'one normal Main-owned worker closure receipt')
            terminal = rows[0]
            r['Main_reported_worker_closure'] = terminal
            need(terminal['closed'] and terminal['reaped'] and terminal['output_eof']
                 and not terminal['occupied'] and terminal['namespace_handles_open']
                 and terminal['completed_at_ns'] < main['wait']['waited_at_boottime_ns'] < close_until,
                 'normal Main retained worker namespace/wait/EOF custody')
            if worker_pidfd is not None:
                need(select.select([worker_pidfd], [], [], 0)[0], 'original worker independently terminal')
                r['independent_original_worker_pidfd_terminal'] = True

    try:
        need(os.getresuid() == os.getresgid() == (0, 0, 0), 'actual guest UID0 owner')
        need(os.environ.get('POKELINUX_CONNECTIVITY_ISOLATED') == '1', 'explicit exclusive test environment')
        need(set(binaries) == set(selection['service_binaries']), 'all exact binary selections prepared')
        for role, path in binaries.items():
            need(pin(path) == selection['service_binaries'][role]['ELF'], 'unchanged actual ELF: ' + role)
        probe_end = min(work_until, now() + address.ACTION_NS)
        probe_body, probe_actual = run(['/usr/bin/python3', '-B', str(client_script),
            'path-probe', str(gates)], 'actual-cap0-guest-gate-permission-preflight.log',
            probe_end, limit=32768, identity={'preexec_fn': lambda: worker_caps.reduce_caps(0)})
        probe = address.decode(probe_body)
        caps(probe['caps'], 0)
        need(probe['passed'] and probe['uid'] == probe['gid'] == 0
             and probe['actual_create_fsync_close_unlink'] and probe['probe_absent'],
             'actual post-exec cap0 gate write and retirement')
        r['guest_cap0_gate_permission_preflight'] = {'value': probe, 'receipt': probe_actual}
        binding = namespace_binding()
        peer_path = ap_prefix[2].split('=', 1)[1]
        ap_ns = binding['AP_namespace']
        provision_end = min(work_until, now() + address.ACTION_NS)
        argv = ['/usr/bin/python3', '-B', str(provision_script), 'provision', peer_path,
                str(provision_end), ap['name'], str(ap['ifindex']), str(ap_ns['dev']), str(ap_ns['ino'])]
        body, actual = run(argv, 'AP-fixed-peer-provisioner.log', provision_end, limit=32768)
        provision = address.decode(body)
        need(provision['passed'] and provision['peer_index'] == ap['ifindex']
             and provision['addresses'] == [{'family': 2, 'prefix': 30, 'ifindex': ap['ifindex'], 'local': '192.0.2.2'}],
             'actual existing AP peer setup')
        r['AP_profile_setup_qualified'] = True
        r['AP_profile_provisioner'] = {'value': provision, 'receipt': actual}
        case_armed = now()
        case = case_armed + address.CASE_NS
        need(case < work_until, 'unchanged six-second original functional aggregate')
        start_until = min(case, case_armed + START_NS)
        r.update(original_case_armed_at_ns=case_armed, original_case_until_ns=case,
                 original_start_until_ns=start_until)
        readback('initial-before-Main', False, start_until)
        packet = scope.start(['/usr/bin/python3', '-B', str(peer_script), 'serve', peer_path,
                              str(gates), str(case)], 'cap0-AP-packet-peer.log', 65536,
                             cutoff=case, wait_cutoff=close_until)
        ready_peer = gate('peer-ready.json', start_until)
        caps(ready_peer['caps'], 0)
        need(ready_peer['pid'] == packet['child'].pid and ready_peer['uid'] == 0
             and ready_peer['bound'] == ['192.0.2.2', 36042]
             and ready_peer['only_packet_socket_no_admin_descriptors']
             and actual['waited_at_boottime_ns'] < packet['spawn_started_at_ns']
             and actual['stdout_EOF_at_ns'] < packet['spawn_started_at_ns'], 'cap0 peer after actual provisioner wait/EOF')
        ns = Path('/proc/' + str(packet['child'].pid) + '/ns/net').stat()
        need((ns.st_dev, ns.st_ino) == (ap_ns['dev'], ap_ns['ino']), 'actual cap0 AP peer namespace')
        r['peer_ready_qualified'] = True
        r['peer_ready'] = ready_peer
        argv = [str(binaries['main']), str(endpoint), str(profile), str(journal / 'admitted'),
                str(binaries['worker']), str(start_until)]
        r['normal_Main_exact_argv'] = argv
        main = scope.start(argv, 'normal-connectivity-main.log', 1048576,
            env=dict(os.environ, POKELINUX_CONNECTIVITY_ISOLATED='1'),
            cutoff=start_until, wait_cutoff=close_until,
            identity={'preexec_fn': lambda: worker_caps.reduce_caps((1 << worker_caps.NET_ADMIN) | (1 << 8))})
        startup_observation = None
        while now() < start_until:
            pump()
            need(scope.live(main), 'normal Main startup runtime exited')
            if endpoint.is_socket():
                end = min(start_until, now() + address.ACTION_NS)
                startup_observation = inspect(end)
                if startup_observation['state']['available']:
                    break
                # A terminal worker cannot later qualify readiness. The normal
                # Main closure receipt will bind its actual exit after Stop.
                children = Path('/proc/' + str(main['child'].pid) + '/task/'
                                + str(main['child'].pid) + '/children').read_text().split()
                if not children and startup_observation['state']['terminal_uncertain']:
                    break
            time.sleep(.002)
        need(startup_observation is not None and now() < start_until, 'bounded actual normal startup observation')
        rejected = not startup_observation['state']['available']
        r['Main_caps'] = process_caps(main['child'].pid)
        caps(r['Main_caps'], 0)
        r['actual_normal_startup_observation'] = startup_observation
        r['baseline_profile_rejection_candidate'] = rejected
        if not rejected:
            children = Path('/proc/' + str(main['child'].pid) + '/task/'
                            + str(main['child'].pid) + '/children').read_text().split()
            need(len(children) == 1, 'one original actual normal worker')
            worker_pid = int(children[0])
            worker_pidfd = os.pidfd_open(worker_pid, os.O_NONBLOCK)
            worker_id = startup_observation['state']['worker']
            r['original_worker_pid'] = worker_pid
            r['original_worker_identity'] = worker_id
            r['worker_caps'] = process_caps(worker_pid)
            caps(r['worker_caps'], 1 << worker_caps.NET_ADMIN)
            need(int(r['worker_caps']['PPid']) == main['child'].pid, 'actual original worker parent')
        driver = scope.start([str(binaries['driver']), '--exact', ACTOR_TEST,
                              '--ignored', '--nocapture', '--test-threads=1'],
            'actual-wireless-registered-settings-actor.log', 1048576,
            env=dict(os.environ, POKELINUX_CONNECTIVITY_ISOLATED='1',
                CONNECTIVITY_ACTOR_ENDPOINT=str(endpoint), CONNECTIVITY_CASE_GATES=str(gates),
                CONNECTIVITY_MAIN_PID=str(main['child'].pid), CONNECTIVITY_CASE_UNTIL_NS=str(case),
                CONNECTIVITY_CASE_CLOSE_UNTIL_NS=str(close_until), CONNECTIVITY_START_UNTIL_NS=str(start_until),
                NATIVE_LIFECYCLE_STORAGE_HELPER=str(binaries['storage']),
                NATIVE_LIFECYCLE_LAUNCHER_HELPER=str(binaries['launcher']),
                NATIVE_SETTINGS_CONNECTIVITY_APP_FIXTURE=str(binaries['settings_app'])),
            cutoff=start_until, wait_cutoff=close_until,
            identity={'preexec_fn': lambda: worker_caps.reduce_caps(ACTOR_CAPS)})
        created = gate('created.json', start_until)
        need(created['driver_pid'] == driver['child'].pid and created['main_pid'] == main['child'].pid,
             'actual driver/normal Main gate binding')
        r['driver_caps'] = process_caps(driver['child'].pid)
        caps(r['driver_caps'], ACTOR_CAPS)
        r['healthy_setup_qualified'] = True
        publish('ready.json', {'provisioner_reaped_before_packet_peer': True,
            'initial_link_ifindex': selected['ifindex'], 'startup_profile_rejected_candidate': rejected,
            'released_at_ns': now(), 'original_case_until_ns': case})
        handled = set()
        while now() < case and not (gates / 'functional-finished.json').exists():
            pump()
            need(scope.live(main) and scope.live(driver) and scope.live(packet), 'actual owned peers remain live')
            for phase, configured in [('initial', False), ('no-grant', False), ('connected', True),
                                       ('refused', False), ('disconnected', False)]:
                path = gates / (phase + '-proof.json')
                if path.exists() and phase not in handled:
                    proof = json.loads(path.read_text())
                    end = min(case, proof['original_until_ns'])
                    value = readback(phase, configured, end)
                    snapshot = wifi_peer.ipv4_snapshot(selected['name'], end)
                    need(snapshot['ifindex'] == selected['ifindex'], 'same actual snapshot interface')
                    publish(phase + '-readback.json', snapshot)
                    handled.add(phase)
                    if phase == 'connected':
                        r['Connect_commands_admitted'] = 1
                    if phase == 'disconnected':
                        r['Disconnect_commands_admitted'] = 1
            for tag in [1, 2]:
                path = gates / ('nonce-' + str(tag) + '-request.json')
                if path.exists() and tag not in pending_clients:
                    packet_client(tag, json.loads(path.read_text()))
            for gate_name, reply_name in [('first-requester-closed.json', 'persistent-after-first-wait.json'),
                                          ('before-disconnect.json', 'disconnect-ready.json')]:
                if (gates / gate_name).exists() and gate_name not in handled:
                    witness = json.loads((gates / gate_name).read_text())
                    end = min(case, now() + address.ACTION_NS)
                    value = readback(gate_name[:-5], True, end)
                    observation = inspect(end)
                    need(observation['state']['available'] and observation['state']['configured']
                         and observation['state']['worker'] == worker_id, 'same fresh persistent worker wire identity')
                    if gate_name.startswith('first-'):
                        need(not Path('/proc/' + str(witness['pid'])).exists(), 'first requester actually absent after Apps wait')
                        r['first_requester_actual_wait'] = witness
                    publish(reply_name, {'same_worker_radio_profile_confirmed': True,
                        'worker': worker_id, 'observed_at_ns': value['observed_at_ns'],
                        'normal_Main_observation': observation, 'original_until_ns': end})
                    handled.add(gate_name)
            time.sleep(.001)
        functional = gate('functional-finished.json', case)
        actor_closed = gate('actor-closed.json', case)
        r['functional'] = functional
        r['actual_actor_and_apps_closure'] = actor_closed
        need(actor_closed['actor_stopped_and_joined'] and actor_closed['app_stdout_stderr'] == 'null'
             and len(actor_closed['actual_Apps_waits']) == (1 if rejected or functional.get('genuine_refused_candidate') else 2)
             and all(value['code'] == 0 and value['signal'] is None and not value['PIPE_EOF_claim']
                     for value in actor_closed['actual_Apps_waits'])
             and actor_closed['completed_at_ns'] < case, 'actual actor join and Apps waits')
        for key in ['first_sdk_stop_join'] + ([] if rejected or functional.get('genuine_refused_candidate') else ['second_sdk_stop_join']):
            stopped = functional[key]
            need(stopped['stop_requested'] and stopped['joined'] and stopped['join_status'] == 0
                 and stopped['actual_task_count_after_join'] == 1
                 and stopped['stop_call_before_ns'] <= stopped['joined_at_ns'] < stopped['fixture_stop_cutoff_ns'] <= case
                 and stopped['fixture_stop_cutoff_ns'] - stopped['stop_call_before_ns'] <= 200_000_000,
                 'actual SDK stop/join under unchanged original cutoff')
        if rejected:
            need(functional.get('startup_profile_rejected_candidate') is True,
                 'actual actor SDK confirms normal startup unavailable')
            readback('after-runtime-profile-rejection', False, min(case, now() + address.ACTION_NS))
        else:
            need(functional.get('functional_green') is True, 'actual normal actor functional sequence completed')
            packets = gate('peer-packets.json', case)
            need(len(packets['packets']) == 2 and [row['payload_hex'] for row in packets['packets']]
                 == [r['nonce_' + str(tag)]['value']['payload_hex'] for tag in [1, 2]], 'independent two nonce peer receipt')
            quiet = gate('peer-quiet.json', case)
            need(quiet['received_datagrams'] == 0 and quiet['quiet_end_actual_EAGAIN']
                 and quiet['quiet_end_EAGAIN_at_ns'] >= quiet['original_until_ns'], 'full original independent peer quiet')
            r['peer_packets'], r['peer_quiet'] = packets, quiet
        main_stop()
        need(r['normal_Main_exit'] == 0, 'normal Main actual exit0')
        publish('stop-peer', {})
        r['packet_peer_exit'] = scope.finish(packet, close_until)
        need(r['packet_peer_exit'] == 0, 'normal cap0 packet peer Stop/wait')
        terminal_peer = json.loads((out / packet['name']).read_text())
        need(terminal_peer['peer_stopped'] and terminal_peer['packets'] == (0 if rejected else 2),
             'actual peer terminal exact packet count')
        # Retire Main/peer PIPE and pidfd handles before releasing the actual
        # driver's cleanup gate; Scope.finish itself deliberately retains them.
        for item in [main, packet]:
            item['log'].close()
            item['child'].stdout.close()
            os.close(item['pidfd'])
            item['pidfd'] = None
        publish('cleanup-complete.json', {'main_worker_packet_actual_wait_EOF_closed': True,
            'all_namespace_handles_retained': True, 'completed_at_ns': now(),
            'original_close_until_ns': close_until})
        r['driver_exit'] = scope.finish(driver, close_until)
        need(r['driver_exit'] == 0, 'actual actor reporting driver exit0')
        r['runtime_outcome_confirmed'] = True
    except BaseException as error:
        r.update(error=repr(error), traceback=traceback.format_exc())
    finally:
        stopping = True
        if packet is not None:
            publish('stop-peer', {})
        if main is not None and scope.live(main):
            try:
                signal.pidfd_send_signal(main['pidfd'], signal.SIGTERM)
            except BaseException as error:
                r['emergency_Main_Stop_error'] = repr(error)
        r['cleanup_errors'] = scope.close(close_until)
        r['receipts'] = scope.receipts
        r['all_handles_closed'] = all(value['wait'] is not None and value['eof'] and value['log'].closed
            and value['child'].stdout.closed and value['pidfd'] is None for value in scope.children)
        r['owned_case_custody_qualified'] = bool(not r['cleanup_errors'] and r['all_handles_closed']
            and len(scope.children) == len(scope.receipts) and now() < close_until
            and all(value['actual_direct_parent_wait'] and value['stdout_EOF']
                and value['wait_before_original_cutoff'] and value['EOF_before_original_cutoff']
                and value['returncode'] == 0 and not value['output_overflow'] for value in scope.receipts))
        if worker_pidfd is not None:
            os.close(worker_pidfd)
        r['no_unexpected_peer_datagram'] = not (gates / 'peer-unexpected-datagram.json').exists()
        r['completed_at_ns'] = now()
        (out / 'case-result.json').write_text(json.dumps(r, indent=2) + '\n')
    return r


def qualify_after_cleanup(case, *, fixture_cleanup_qualified, normal_ELF_loaders_qualified,
                          retained_namespaces_qualified, selection):
    """No desired success or RED before the original hwsim owner's full closure."""
    ready = bool(fixture_cleanup_qualified is True and normal_ELF_loaders_qualified is True
        and retained_namespaces_qualified is True and case.get('healthy_setup_qualified') is True
        and case.get('AP_profile_setup_qualified') is True and case.get('peer_ready_qualified') is True
        and case.get('owned_case_custody_qualified') is True and case.get('no_unexpected_peer_datagram') is True
        and case.get('runtime_outcome_confirmed') is True)
    baseline = selection.get('baseline_profile_rejection', {})
    actual_pins = selection['service_binaries']
    closure = case.get('Main_reported_worker_closure', {})
    rejected = bool(ready and case.get('baseline_profile_rejection_candidate') is True
        and baseline.get('main_ELF') == actual_pins['main']['ELF']
        and baseline.get('worker_ELF') == actual_pins['worker']['ELF']
        and baseline.get('Main_exit_code') == case.get('normal_Main_exit') == 0
        and baseline.get('worker_exit_code') == closure.get('worker_exit_code') == 1
        and baseline.get('worker_error_line') == 'Error: Kind(InvalidData)'
        and case.get('Main_non_JSON_lines') == [baseline['worker_error_line']]
        and case.get('functional', {}).get('startup_profile_rejected_candidate') is True
        and case.get('UDP_roundtrip_clients_admitted') == 0
        and case.get('after-runtime-profile-rejection_independent_readback', {}).get('radio', {}).get('station_unassociated') is True)
    green = bool(ready and case.get('baseline_profile_rejection_candidate') is False
        and case.get('functional', {}).get('functional_green') is True
        and case.get('Connect_commands_admitted') == case.get('Disconnect_commands_admitted') == 1
        and case.get('UDP_roundtrip_clients_admitted') == 2
        and case.get('independent_original_worker_pidfd_terminal') is True
        and case.get('first_requester_actual_wait') is not None
        and case.get('disconnected_independent_readback', {}).get('radio', {}).get('station_unassociated') is True
        and not case.get('radio_or_address_retry_admitted'))
    return {'schema': 1, 'passed': green, 'behavior_RED_qualified': rejected,
            'prerequisites_and_cleanup_qualified': ready,
            'actual_outcome_confirmed': rejected or green,
            'classification': ('QUALIFIED_NORMAL_WIRELESS_STARTUP_PROFILE_REJECTION_RED' if rejected
                else 'NORMAL_ACTOR_PERSISTENT_WIRELESS_PACKET_GREEN' if green
                else 'PREREQUISITE_CLEANUP_OR_RUNTIME_OUTCOME_UNQUALIFIED')}


def assert_desired_after_cleanup(review):
    if review.get('prerequisites_and_cleanup_qualified') is not True:
        raise RuntimeError('WIRELESS_SERVICE_PREREQUISITE_OR_CLEANUP_UNQUALIFIED')
    if review.get('actual_outcome_confirmed') is not True:
        raise RuntimeError('WIRELESS_SERVICE_RUNTIME_OUTCOME_UNCONFIRMED')
    if review.get('passed') is not True:
        raise AssertionError(DESIRED_MARKER)
