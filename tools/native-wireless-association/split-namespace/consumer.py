"""Frozen explicit open-association oracle; all I/O belongs to its actual owner.

The caller persists worker/kernel observations before AP/radio cleanup, then
calls qualify_after_cleanup and asserts once. No clock, process, file or socket
operation occurs in this module. An uncertain operation is never retried here.
"""
import json
import re

ACTION_NS = 5_000_000_000
MAX_WIRE = 32768
DESIRED_MARKER = 'WIRELESS_EXPLICIT_OPEN_REQUEST_MUST_ASSOCIATE_ACTUAL_AP'
U32 = (1 << 32) - 1
U64 = (1 << 64) - 1


def need(condition, detail):
    if not condition:
        raise ValueError(detail)


def integer(value, low, high, detail):
    need(type(value) is int and low <= value <= high, detail)
    return value


def unique(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def no_constant(value):
    raise ValueError('non-finite JSON constant')


def selectors(station, ap):
    need(type(station) is dict and set(station) ==
         {'name', 'ifindex', 'wiphy', 'wdev', 'mac_hex'}, 'station identity fields')
    need(type(ap) is dict and set(ap) ==
         {'name', 'ifindex', 'wiphy', 'wdev', 'bssid_hex', 'ssid_hex', 'frequency_mhz'},
         'AP identity fields')
    for value in [station, ap]:
        need(type(value['name']) is str and re.fullmatch('[A-Za-z0-9_.-]{1,15}', value['name']), 'interface name')
        integer(value['ifindex'], 1, U32, 'ifindex')
        integer(value['wiphy'], 0, U32, 'wiphy')
        integer(value['wdev'], 1, U64, 'wdev')
    need(station['wiphy'] != ap['wiphy'] and station['wdev'] != ap['wdev'], 'two distinct radio roles; interface indices are namespace-local')
    for value in [station['mac_hex'], ap['bssid_hex']]:
        need(type(value) is str and re.fullmatch('[0-9a-f]{12}', value), 'unicast MAC hex')
        need(value != '0' * 12 and not int(value[:2], 16) & 1, 'unicast nonzero MAC')
    need(type(ap['ssid_hex']) is str and re.fullmatch('(?:[0-9a-f]{2}){1,32}', ap['ssid_hex']), 'SSID bytes')
    # The authored fixture uses a printable nonce SSID; no credentials exist.
    need(re.fullmatch(b'pkl-hwsim-[0-9a-f]{20}', bytes.fromhex(ap['ssid_hex'])), 'authored nonce SSID')
    integer(ap['frequency_mhz'], 1, 100000, 'selected frequency')


def actual_receipt(receipt, *, argv, until, admission, code, byte_count, limit):
    need(type(receipt) is dict and receipt.get('argv') == argv, 'exact owned argv')
    for key in ['actual_direct_parent_wait', 'stdout_EOF',
                'wait_before_original_cutoff', 'EOF_before_original_cutoff']:
        need(receipt.get(key) is True, 'unqualified custody: ' + key)
    need(type(receipt.get('returncode')) is int and receipt['returncode'] == code, 'actual exit')
    need(receipt.get('original_cutoff_ns') == until, 'original cutoff changed')
    need(receipt.get('spawn_admission_cutoff_ns') == until, 'original spawn cutoff changed')
    integer(receipt.get('waited_at_boottime_ns'), admission, until - 1, 'actual wait BOOTTIME')
    integer(receipt.get('stdout_EOF_at_ns'), admission, until - 1, 'actual EOF BOOTTIME')
    need(receipt.get('stdout_bytes') == byte_count and receipt.get('stdout_limit') == limit, 'owned wire bound')
    need(receipt.get('output_overflow') is False, 'output overflow')


def observe_worker(returncode, stdout, receipt, *, executable, armed_at_ns,
                   pre_spawn_admission_ns, station, ap):
    result = {'schema': 1, 'outcome_confirmed': False,
              'known_existing_cli_refusal': False, 'requested_association_completed': False}
    try:
        selectors(station, ap)
        integer(returncode, -255, 255, 'actual worker returncode')
        need(type(stdout) is bytes and 0 < len(stdout) <= MAX_WIRE, 'worker byte bound')
        need(type(executable) is str and executable.startswith('/'), 'explicit installed normal executable')
        armed = integer(armed_at_ns, 1, U64 - ACTION_NS, 'original BOOTTIME arming')
        until = armed + ACTION_NS
        admission = integer(pre_spawn_admission_ns, armed, until - 1, 'pre-spawn admission')
        argv = [executable, 'associate-open', str(station['ifindex']), str(ap['frequency_mhz']),
                ap['bssid_hex'], ap['ssid_hex'], str(until)]
        actual_receipt(receipt, argv=argv, until=until, admission=admission,
                       code=returncode, byte_count=len(stdout), limit=MAX_WIRE)
        value = json.loads(stdout.decode('utf-8'), object_pairs_hook=unique, parse_constant=no_constant)
        result.update(actual_argv=argv, actual_returncode=returncode, actual_result=value,
                      original_armed_at_ns=armed, original_until_ns=until,
                      actual_wait_ns=receipt['waited_at_boottime_ns'], actual_EOF_ns=receipt['stdout_EOF_at_ns'],
                      requested_station=dict(station), selected_AP=dict(ap))
        if returncode == 64:
            need(value == {'schema': 1, 'status': 'invalid_request', 'reason': 'invalid_arguments'},
                 'existing normal CLI refusal changed')
            need(type(value['schema']) is int, 'schema integer')
            result.update(outcome_confirmed=True, known_existing_cli_refusal=True,
                          classification='CONFIRMED_EXISTING_CLI_INVALID_ARGUMENTS_REFUSAL')
            return result
        need(returncode == 0 and type(value) is dict, 'nonzero or malformed association result')
        expected = {'schema', 'status', 'operation', 'ifindex', 'frequency_mhz', 'original_until_ns',
                    'wiphy', 'wdev', 'bssid_hex', 'ssid_hex', 'started_boottime_ns', 'completed_boottime_ns'}
        need(set(value) == expected and type(value['schema']) is int and value['schema'] == 1,
             'exact association result fields/schema')
        need(value['status'] == 'associated' and value['operation'] == 'associate-open', 'association result status')
        for key, wanted in [('ifindex', station['ifindex']), ('frequency_mhz', ap['frequency_mhz']),
                            ('original_until_ns', until), ('wiphy', station['wiphy']), ('wdev', station['wdev'])]:
            need(type(value[key]) is int and value[key] == wanted, 'association identity: ' + key)
        need(value['bssid_hex'] == ap['bssid_hex'] and value['ssid_hex'] == ap['ssid_hex'], 'exact selected AP')
        started = integer(value['started_boottime_ns'], admission, until - 1, 'association start')
        completed = integer(value['completed_boottime_ns'], started, until - 1, 'association completion')
        need(completed <= min(receipt['waited_at_boottime_ns'], receipt['stdout_EOF_at_ns']), 'completion after wait/EOF')
        result.update(outcome_confirmed=True, requested_association_completed=True,
                      classification='CONFIRMED_NORMAL_ASSOCIATION_COMPLETION')
    except (ValueError, TypeError, KeyError, UnicodeError, OverflowError, RecursionError) as error:
        result['unconfirmed_detail'] = str(error)
    return result


def parse_iw_roles(text, expected_rows=2):
    rows = []
    phy = None
    current = None
    for line in text.splitlines():
        match = re.fullmatch(r'phy#([0-9]+)', line.strip())
        if match:
            phy = int(match[1])
        match = re.fullmatch(r'Interface ([A-Za-z0-9_.-]{1,15})', line.strip())
        if match:
            need(phy is not None, 'iw phy before interface')
            current = {'name': match[1], 'wiphy': phy}
            rows.append(current)
        elif current is not None:
            match = re.fullmatch(r'(ifindex|wdev|addr|type) (.+)', line.strip())
            if match:
                key = match[1]
                need(key not in current, 'duplicate iw role field')
                current[key] = match[2]
    need(len(rows) == expected_rows, 'exact namespace radio inventory')
    result = {}
    for row in rows:
        need(set(row) == {'name', 'wiphy', 'ifindex', 'wdev', 'addr', 'type'}, 'complete iw role')
        need(row['ifindex'].isdigit() and re.fullmatch('0x[0-9a-f]+', row['wdev']), 'iw numeric identity')
        need(re.fullmatch('[0-9a-f]{2}(?::[0-9a-f]{2}){5}', row['addr']), 'iw MAC')
        result[row['name']] = {'name': row['name'], 'wiphy': row['wiphy'], 'ifindex': int(row['ifindex']),
                               'wdev': int(row['wdev'], 16), 'mac_hex': row['addr'].replace(':', ''),
                               'type': row['type']}
    return result


def observe_kernel(*, station, ap, armed_at_ns, until_ns, link_stdout, station_dump_stdout,
                   roles_stdout, ap_roles_stdout, receipts, observed_at_ns, ap_prefix, namespace_binding):
    result = {'schema': 1, 'kernel_observation_confirmed': False, 'station_unassociated': False,
              'exact_selected_AP_association': False}
    try:
        selectors(station, ap)
        integer(armed_at_ns, 1, U64 - ACTION_NS, 'original kernel observation arming')
        need(until_ns == armed_at_ns + ACTION_NS, 'kernel observation cutoff changed')
        integer(observed_at_ns, armed_at_ns, until_ns - 1, 'post-worker kernel sample before original cutoff')
        result.update(original_until_ns=until_ns, requested_station=dict(station), selected_AP=dict(ap))
        need(type(receipts) is dict and set(receipts) == {'link', 'AP_station_dump', 'station_roles', 'AP_roles'}, 'four separate namespace-bound iw receipts')
        need(type(ap_prefix) is list and len(ap_prefix) == 4
             and ap_prefix[:2] == ['/usr/bin/nsenter', '--no-fork'] and ap_prefix[3] == '--'
             and re.fullmatch(r'--net=/proc/[1-9][0-9]*/ns/net', ap_prefix[2]), 'explicit retained AP namespace entry')
        need(type(namespace_binding) is dict and namespace_binding['keeper_pidfd_live'] is True
             and namespace_binding['AP_pidfd_live'] is True, 'retained namespace owners live')
        sta_ns = namespace_binding['station_namespace']
        ap_ns = namespace_binding['AP_namespace']
        need(set(sta_ns) == set(ap_ns) == {'dev', 'ino', 'boot_id'}, 'namespace identity shape')
        need(sta_ns['boot_id'] == ap_ns['boot_id'] and sta_ns != ap_ns, 'separate namespaces in one actual kernel epoch')
        for ns in [sta_ns, ap_ns]:
            integer(ns['dev'], 0, U64, 'namespace device'); integer(ns['ino'], 1, U64, 'namespace inode')
        need(namespace_binding['keeper_namespace'] == ap_ns
             and namespace_binding['AP_process_namespace'] == ap_ns, 'AP and retained keeper execute in the selected peer namespace')
        integer(namespace_binding['observed_at_ns'], armed_at_ns, until_ns - 1, 'post-read namespace observation')
        need(ap_prefix[2] == '--net=/proc/' + str(namespace_binding['keeper_pid']) + '/ns/net', 'exact retained keeper path')
        commands = {'link': (['/usr/bin/iw', 'dev', station['name'], 'link'], link_stdout),
                    'AP_station_dump': (ap_prefix + ['/usr/bin/iw', 'dev', ap['name'], 'station', 'dump'], station_dump_stdout),
                    'station_roles': (['/usr/bin/iw', 'dev'], roles_stdout),
                    'AP_roles': (ap_prefix + ['/usr/bin/iw', 'dev'], ap_roles_stdout)}
        for key, (argv, body) in commands.items():
            need(type(body) is bytes and len(body) <= 262144, 'bounded iw output')
            actual_receipt(receipts[key], argv=argv, until=until_ns, admission=armed_at_ns,
                           code=0, byte_count=len(body), limit=262144)
        need(len(re.findall(r'^\s*Interface ', roles_stdout.decode('utf-8'), re.M)) == 1
             and len(re.findall(r'^\s*Interface ', ap_roles_stdout.decode('utf-8'), re.M)) == 1, 'one radio per namespace')
        station_roles = parse_iw_roles(roles_stdout.decode('utf-8'), 1)
        ap_roles = parse_iw_roles(ap_roles_stdout.decode('utf-8'), 1)
        need(set(station_roles) == {station['name']} and set(ap_roles) == {ap['name']}, 'selected interface belongs to its specific namespace')
        roles = dict(station_roles, **ap_roles)
        result['namespace_binding'] = namespace_binding
        result['AP_namespace_entry_argv_prefix'] = ap_prefix
        for role, mac_key, expected_type in [(station, 'mac_hex', 'managed'), (ap, 'bssid_hex', 'AP')]:
            actual = roles[role['name']]
            need(actual['type'] == expected_type, 'actual radio mode')
            need(all(actual[key] == role[key] for key in ['name', 'ifindex', 'wiphy', 'wdev']), 'retained radio identity')
            need(actual['mac_hex'] == role[mac_key], 'retained MAC identity')
        link = link_stdout.decode('utf-8').strip()
        dump = station_dump_stdout.decode('utf-8').strip()
        if link == 'Not connected.':
            need(dump == '', 'AP has station while STA reports unassociated')
            result.update(kernel_observation_confirmed=True, station_unassociated=True,
                          classification='ACTUAL_STA_UNASSOCIATED_AND_AP_STATION_DUMP_EMPTY')
            return result
        lines = link.splitlines()
        match = re.fullmatch(r'Connected to ([0-9a-f]{2}(?::[0-9a-f]{2}){5}) \(on '
                             + re.escape(station['name']) + r'\)', lines[0])
        need(match and match[1].replace(':', '') == ap['bssid_hex'], 'iw link exact AP BSSID')
        names = [line.strip()[6:] for line in lines[1:] if line.strip().startswith('SSID: ')]
        frequencies = [line.strip()[6:] for line in lines[1:] if line.strip().startswith('freq: ')]
        need(names == [bytes.fromhex(ap['ssid_hex']).decode('ascii')], 'iw link exact nonce SSID')
        need(len(frequencies) == 1 and re.fullmatch(str(ap['frequency_mhz']) + r'(?:\.0+)?', frequencies[0]), 'iw link exact frequency')
        headers = re.findall(r'^Station ([0-9a-f]{2}(?::[0-9a-f]{2}){5}) \(on ([A-Za-z0-9_.-]{1,15})\)$', dump, re.M)
        need(headers == [(':'.join(station['mac_hex'][i:i+2] for i in range(0, 12, 2)), ap['name'])],
             'AP station table exact selected STA')
        result.update(kernel_observation_confirmed=True, exact_selected_AP_association=True,
                      classification='ACTUAL_IW_STA_LINK_AND_AP_STATION_MATCH_SELECTED_PEERS')
    except (ValueError, TypeError, KeyError, UnicodeError, OverflowError, RecursionError, IndexError) as error:
        result['unconfirmed_detail'] = str(error)
    return result


def qualify_after_cleanup(worker, kernel, *, fixture_qualified, worker_loader_qualified,
                          retained_namespace_qualified, cleanup_qualified,
                          explicit_disconnect_and_readback_qualified):
    ready = (fixture_qualified is True and worker_loader_qualified is True
             and retained_namespace_qualified is True and cleanup_qualified is True)
    confirmed = (worker.get('outcome_confirmed') is True and kernel.get('kernel_observation_confirmed') is True
                 and worker.get('original_until_ns') == kernel.get('original_until_ns')
                 and worker.get('requested_station') == kernel.get('requested_station')
                 and worker.get('selected_AP') == kernel.get('selected_AP'))
    baseline = False  # This is an existing-implementation prerequisite, never a new behavior RED.
    associated = (worker.get('requested_association_completed') is True
                  and kernel.get('exact_selected_AP_association') is True
                  and explicit_disconnect_and_readback_qualified is True)
    return {'schema': 1, 'passed': ready and confirmed and associated,
            'behavior_RED_qualified': ready and confirmed and baseline,
            'prerequisites_and_cleanup_qualified': ready, 'actual_outcome_confirmed': confirmed,
            'worker': worker, 'kernel': kernel}


def assert_desired_after_cleanup(review):
    if review.get('prerequisites_and_cleanup_qualified') is not True:
        raise RuntimeError('WIRELESS_ASSOCIATION_PREREQUISITE_OR_CLEANUP_UNQUALIFIED')
    if review.get('actual_outcome_confirmed') is not True:
        raise RuntimeError('WIRELESS_ASSOCIATION_UNCONFIRMED_OUTCOME')
    if review.get('passed') is not True:
        raise AssertionError(DESIRED_MARKER)
