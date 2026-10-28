"""Frozen address/packet desired oracle; no I/O, authority or action retries."""
import json
import re

ACTION_NS = 1_000_000_000
CASE_NS = 6_000_000_000
QUIET_NS = 800_000_000
MAX_WIRE = 32768
BASELINE_ARM_SHA256 = 'e3b449ccad3e4856e442f33e5a27424c6c3ae4fcbb90fb06ad6906c4c85f76a2'
BASELINE_REFUSAL = b'Error: Kind(PermissionDenied)\n'
DESIRED_MARKER = 'WIRELESS_FIXED_IPV4_REQUIRES_INDEPENDENT_PACKET_LIFECYCLE'
U64 = (1 << 64) - 1


def need(value, detail):
    if not value:
        raise ValueError(detail)


def integer(value, low, high, detail):
    need(type(value) is int and low <= value <= high, detail)
    return value


def unique(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, 'duplicate JSON field')
        result[key] = value
    return result


def no_constant(_value):
    raise ValueError('non-finite JSON constant')


def decode(body):
    need(type(body) is bytes and 0 < len(body) <= MAX_WIRE, 'wire bound')
    return json.loads(body.decode('utf-8'), object_pairs_hook=unique, parse_constant=no_constant)


def target(value):
    need(type(value) is dict and set(value) == {'namespace', 'ifindex', 'name'}, 'target fields')
    ns = value['namespace']
    need(type(ns) is dict and set(ns) == {'dev', 'ino'}, 'namespace fields')
    integer(ns['dev'], 0, U64, 'namespace device')
    integer(ns['ino'], 1, U64, 'namespace inode')
    integer(value['ifindex'], 2, (1 << 32) - 1, 'existing non-loopback index')
    need(type(value['name']) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,15}', value['name']), 'interface name')
    need(value['name'] != 'lo', 'loopback target refused')


def argv(executable, operation, selected, until):
    target(selected)
    need(operation in ['profile-up', 'profile-down'], 'explicit profile operation')
    return [executable, operation, str(selected['namespace']['dev']), str(selected['namespace']['ino']),
            str(selected['ifindex']), selected['name'], str(until)]


def receipt(value, *, expected_argv, until, armed, code, byte_count, limit=MAX_WIRE):
    need(type(value) is dict and value.get('argv') == expected_argv, 'exact owned argv')
    for key in ['actual_direct_parent_wait', 'stdout_EOF', 'wait_before_original_cutoff', 'EOF_before_original_cutoff']:
        need(value.get(key) is True, 'actual custody: ' + key)
    need(type(value.get('returncode')) is int and value['returncode'] == code, 'actual worker exit')
    need(value.get('original_cutoff_ns') == until and value.get('spawn_admission_cutoff_ns') == until, 'original cutoff')
    integer(value.get('waited_at_boottime_ns'), armed, until - 1, 'actual wait time')
    integer(value.get('stdout_EOF_at_ns'), armed, until - 1, 'actual EOF time')
    need(value.get('stdout_bytes') == byte_count and value.get('stdout_limit') == limit
         and value.get('output_overflow') is False, 'bounded exact output')


def observe_worker(code, body, actual, *, executable, operation, selected, armed_at_ns, admission_ns, elf_sha256):
    result = {'schema': 1, 'outcome_confirmed': False, 'known_baseline_refusal': False,
              'profile_completion_confirmed': False}
    try:
        target(selected)
        armed = integer(armed_at_ns, 1, U64 - ACTION_NS, 'original action arm')
        until = armed + ACTION_NS
        admission = integer(admission_ns, armed, until - 1, 'spawn admission')
        need(type(body) is bytes and len(body) <= MAX_WIRE, 'actual wire bound')
        receipt(actual, expected_argv=argv(executable, operation, selected, until), until=until,
                armed=admission, code=code, byte_count=len(body))
        result.update(operation=operation, target=selected, original_armed_at_ns=armed,
                      original_until_ns=until, actual_wait_ns=actual['waited_at_boottime_ns'],
                      actual_EOF_ns=actual['stdout_EOF_at_ns'], actual_returncode=code,
                      actual_stdout_utf8=body.decode('utf-8'), elf_sha256=elf_sha256)
        if operation == 'profile-up' and elf_sha256 == BASELINE_ARM_SHA256 and code == 1 and body == BASELINE_REFUSAL:
            result.update(outcome_confirmed=True, known_baseline_refusal=True,
                          classification='ACTUAL_NORMAL_BASELINE_PROFILE_SELECTOR_REFUSAL')
            return result
        need(code == 0, 'normal profile completion exit')
        value = decode(body)
        need(type(value) is dict and set(value) == {'schema', 'status', 'operation', 'namespace', 'ifindex',
             'ifname', 'profile', 'configured', 'observed_lifetime', 'observed_at_boottime_ns',
             'original_until_ns', 'started_boottime_ns', 'completed_boottime_ns'}, 'exact profile result fields')
        need(type(value['schema']) is int and value['schema'] == 1 and value['status'] == 'completed'
             and value['operation'] == operation, 'profile status')
        target({'namespace': value['namespace'], 'ifindex': value['ifindex'], 'name': value['ifname']})
        need(value['namespace'] == selected['namespace'] and type(value['ifindex']) is int
             and value['ifindex'] == selected['ifindex'] and value['ifname'] == selected['name'], 'selected identity')
        need(value['profile'] == {'local': '192.0.2.1', 'prefix_len': 30}
             and type(value['profile']['prefix_len']) is int, 'only fixed profile')
        need(type(value['configured']) is bool and value['configured'] == (operation == 'profile-up'), 'observed configuration')
        need(value['observed_lifetime'] is None, 'pre-existing interface has no invented create witness')
        need(type(value['original_until_ns']) is int and value['original_until_ns'] == until, 'unchanged original action deadline')
        started = integer(value['started_boottime_ns'], admission, until - 1, 'normal start')
        observed = integer(value['observed_at_boottime_ns'], started, until - 1, 'actual profile readback time')
        done = integer(value['completed_boottime_ns'], observed, until - 1, 'normal completion')
        need(done <= min(actual['waited_at_boottime_ns'], actual['stdout_EOF_at_ns']), 'completion precedes actual wait/EOF')
        result.update(outcome_confirmed=True, profile_completion_confirmed=True,
                      configured=value['configured'], actual_result=value,
                      classification='CONFIRMED_NORMAL_FIXED_PROFILE_COMPLETION')
    except (ValueError, TypeError, KeyError, UnicodeError, OverflowError, RecursionError) as error:
        result['unconfirmed_detail'] = str(error)
    return result


def kernel_profile(body, *, selected, configured):
    target(selected)
    value = json.loads(body.decode('utf-8'), object_pairs_hook=unique, parse_constant=no_constant)
    need(type(value) is list and len(value) == 1, 'one selected address record')
    record = value[0]
    need(record['ifindex'] == selected['ifindex'] and record['ifname'] == selected['name'], 'independent selected address identity')
    addresses = record['addr_info']
    if configured:
        need(len(addresses) == 1 and addresses[0]['family'] == 'inet'
             and addresses[0]['local'] == '192.0.2.1' and addresses[0]['prefixlen'] == 30
             and addresses[0]['scope'] == 'global', 'independent exact fixed IPv4')
    else:
        need(addresses == [], 'independent address absence')
    return {'confirmed': True, 'configured': configured, 'target': selected}


def route_profile(text, selected, configured):
    rows = [line.split() for line in text.splitlines()[1:]]
    need(all(len(row) >= 8 for row in rows), 'bounded route table fields')
    need(not any(row[1] == '00000000' for row in rows), 'no default route')
    subnet = [row for row in rows if row[1] == '000200C0' and row[7] == 'FCFFFFFF']
    if configured:
        need(len(subnet) == 1 and subnet[0][0] == selected['name'] and subnet[0][2] == '00000000', 'only selected station subnet route')
    else:
        need(subnet == [], 'no station subnet route after removal/refusal')
    return {'confirmed': True, 'default_route_absent': True, 'station_subnet_route_present': configured}


def observe_packet(value, *, payload_hex, actual, expected_argv, until, armed, profile_worker, selected):
    need(type(value) is dict and value['passed'] is True and value['mode'] == 'roundtrip', 'packet result')
    need(value['namespace'] == selected['namespace'], 'packet client namespace')
    need(value['payload_hex'] == payload_hex and value['reply_hex'] == '41434b31' + payload_hex, 'exact nonce/ACK')
    need(value['peer'] == ['192.0.2.2', 36042] and value['source'][0] == '192.0.2.1', 'packet peer/source')
    need(all(int(v, 16) == 0 for k, v in value['caps'].items() if k.startswith('Cap'))
         and value['caps']['NoNewPrivs'] == '1', 'packet client cap0')
    need(max(profile_worker['actual_wait_ns'], profile_worker['actual_EOF_ns']) <= actual['spawn_started_at_ns'], 'profile worker actually retired before packets')
    integer(value['started_at_ns'], armed, until - 1, 'packet start')
    integer(value['completed_at_ns'], value['started_at_ns'], until - 1, 'packet completion')
    need(value['original_until_ns'] == until, 'packet original cutoff')
    # Receipt output byte_count is supplied by the owning caller, not reconstructed JSON.
    need(actual['argv'] == expected_argv and actual['returncode'] == 0 and actual['stdout_EOF']
         and actual['actual_direct_parent_wait'] and actual['original_cutoff_ns'] == until
         and actual['wait_before_original_cutoff'] and actual['EOF_before_original_cutoff'], 'packet actual wait/EOF')
    return {'confirmed': True, 'payload_hex': payload_hex, 'source': value['source'], 'peer': value['peer']}


def qualify_after_cleanup(case, *, association_and_explicit_disconnect_qualified, fixture_cleanup_qualified):
    ready = (association_and_explicit_disconnect_qualified is True and fixture_cleanup_qualified is True
             and case.get('owned_case_custody_qualified') is True and case.get('peer_ready_qualified') is True
             and case.get('AP_profile_setup_qualified') is True)
    red = (ready and case.get('baseline_refusal_qualified') is True
           and case.get('station_address_absent_after_refusal') is True
           and case.get('peer_packets_after_refusal') == 0)
    green = (ready and case.get('profile_up_qualified') is True and case.get('profile_down_qualified') is True
             and case.get('two_independent_nonce_ACKs_qualified') is True
             and case.get('station_address_absent_after_removal') is True
             and case.get('peer_quiet_qualified') is True
             and case.get('L2_remained_associated_through_removal_and_quiet') is True)
    return {'schema': 1, 'passed': green, 'behavior_RED_qualified': red,
            'prerequisites_and_cleanup_qualified': ready, 'actual_outcome_confirmed': red or green,
            'case': case}


def assert_desired_after_cleanup(review):
    if review.get('prerequisites_and_cleanup_qualified') is not True:
        raise RuntimeError('WIRELESS_ADDRESS_PACKET_PREREQUISITE_OR_CLEANUP_UNQUALIFIED')
    if review.get('actual_outcome_confirmed') is not True:
        raise RuntimeError('WIRELESS_ADDRESS_PACKET_UNCONFIRMED_OUTCOME')
    if review.get('passed') is not True:
        raise AssertionError(DESIRED_MARKER)


def l2_observation(*, selected_station, selected_AP, outputs, receipts, ap_prefix, armed, until, namespace_binding):
    """Current L2 state inside the original packet case, not another Connect."""
    need(namespace_binding['keeper_pidfd_live'] is True and namespace_binding['AP_pidfd_live'] is True, 'radio owners live')
    need(namespace_binding['station_namespace'] != namespace_binding['AP_namespace']
         and namespace_binding['keeper_namespace'] == namespace_binding['AP_namespace']
         and namespace_binding['AP_process_namespace'] == namespace_binding['AP_namespace'], 'separate retained L2 namespaces')
    integer(namespace_binding['observed_at_ns'], armed, until - 1, 'current L2 namespace sample')
    commands={'link':['/usr/bin/iw','dev',selected_station['name'],'link'],
              'AP_station_dump':ap_prefix+['/usr/bin/iw','dev',selected_AP['name'],'station','dump'],
              'station_roles':['/usr/bin/iw','dev'],'AP_roles':ap_prefix+['/usr/bin/iw','dev']}
    need(set(outputs)==set(receipts)==set(commands),'four L2 readbacks')
    for key,command in commands.items():
        receipt(receipts[key],expected_argv=command,until=until,armed=armed,code=0,byte_count=len(outputs[key]),limit=262144)
    for key,role,mac_key,kind in [('station_roles',selected_station,'mac_hex','managed'),('AP_roles',selected_AP,'bssid_hex','AP')]:
        text=outputs[key].decode('utf-8')
        need(re.findall(r'^phy#([0-9]+)',text,re.M)==[str(role['wiphy'])], 'selected L2 phy')
        fields={}
        for line in text.splitlines():
            match=re.fullmatch(r'(Interface|ifindex|wdev|addr|type) (.+)',line.strip())
            if match:
                need(match[1] not in fields,'duplicate L2 field');fields[match[1]]=match[2]
        need(fields=={'Interface':role['name'],'ifindex':str(role['ifindex']),'wdev':hex(role['wdev']),
             'addr':':'.join(role[mac_key][i:i+2]for i in range(0,12,2)),'type':kind},'selected current L2 identity/mode')
    link=outputs['link'].decode('utf-8').strip().splitlines()
    expected_mac=':'.join(selected_AP['bssid_hex'][i:i+2]for i in range(0,12,2))
    need(link and link[0]=='Connected to '+expected_mac+' (on '+selected_station['name']+')','STA still associated with exact AP')
    names=[line.strip()[6:]for line in link[1:]if line.strip().startswith('SSID: ')]
    frequencies=[line.strip()[6:]for line in link[1:]if line.strip().startswith('freq: ')]
    need(names==[bytes.fromhex(selected_AP['ssid_hex']).decode('ascii')]
         and len(frequencies)==1 and re.fullmatch(str(selected_AP['frequency_mhz'])+r'(?:\.0+)?',frequencies[0]),'current exact SSID/frequency')
    headers=re.findall(r'^Station ([0-9a-f]{2}(?::[0-9a-f]{2}){5}) \(on ([A-Za-z0-9_.-]{1,15})\)$',outputs['AP_station_dump'].decode('utf-8').strip(),re.M)
    need(headers==[(':'.join(selected_station['mac_hex'][i:i+2]for i in range(0,12,2)),selected_AP['name'])],'AP still lists exact STA')
    return {'confirmed':True,'observed_at_ns':namespace_binding['observed_at_ns'],'namespace_binding':namespace_binding}
