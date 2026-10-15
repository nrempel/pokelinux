"""Explicit two-boot witness observer; no manufactured guest replies."""
import hashlib
import re

READY = re.compile(r'^POKELINUX_PSTORE_READY epoch=([0-9a-f]{32}) boot=([0-9a-f-]{36}) major=([1-9][0-9]*) minor=0 wrote=([1-9][0-9]*)$', re.M)
RETRIEVED = re.compile(r'^POKELINUX_PSTORE_RETRIEVED current_boot=([0-9a-f-]{36}) pmsg_bytes=([1-9][0-9]*) pmsg_hex=([0-9a-f]+) console_bytes=([1-9][0-9]*) console_hex=([0-9a-f]+)$', re.M)
ACK = re.compile(r'^POKELINUX_DIAGNOSTIC_ACK epoch=([0-9a-f]{32}) event=([1-9][0-9]*) kernel_seq=([0-9]+) (.+)$', re.M)
RELEASE = '6.6.127-android15-8-g91d825611104-ab16252310-4k'

def inspect(console, qmp):
    """Return bounded observations or refuse incomplete/corrupt/stale evidence."""
    assert len(console.encode()) <= 1024 * 1024, 'bounded full console exceeded'
    console = console.replace('\r\n', '\n')
    assert not any(s in console for s in ['Kernel panic', 'POKELINUX_PROBE_FAILURE', 'POKELINUX_DIAGNOSTIC_FAILURE', 'POKELINUX_PSTORE_WITNESS_FAILURE']), 'actual kernel/native/witness failure'
    ready = list(READY.finditer(console))
    retrieval = list(RETRIEVED.finditer(console))
    assert len(ready) == len(retrieval) == 1, 'one canary write and one retrieval required'
    before, after = ready[0], retrieval[0]
    assert before.start() < after.start(), 'retrieval precedes first write'
    assert before[2] != after[1], 'different actual boot UUID required'
    pmsg = bytes.fromhex(after[3])
    old_console = bytes.fromhex(after[5])
    assert len(pmsg) == int(after[2]) <= 512 and len(old_console) == int(after[4]) <= 65536
    expected = f'POKELINUX_RECOVERY_CANARY epoch={before[1]} boot={before[2]} kernel={RELEASE}\n'.encode()
    assert int(before[4]) == len(expected) and pmsg == expected, 'retained canary is stale, corrupt or incomplete'
    acks = [{'epoch': m[1], 'event': int(m[2]), 'kernel_seq': int(m[3]), 'message': m[4], 'offset': m.start()} for m in ACK.finditer(console)]
    assert len(acks) == 6 and [a['event'] for a in acks] == [1, 2, 1, 2, 3, 4], 'two explicit native boot stages required'
    first, second = acks[:2], acks[2:]
    assert len({a['epoch'] for a in first}) == len({a['epoch'] for a in second}) == 1
    assert first[0]['epoch'] != second[0]['epoch'], 'new native execution epoch required'
    for stage in [first, second]:
        assert all(a['kernel_seq'] < b['kernel_seq'] for a, b in zip(stage, stage[1:])), 'per-boot kernel sequence order'
        assert stage[0]['message'] == 'POKELINUX_NATIVE_PID1 pid=1 uid=0 version=0.1.0'
        assert stage[1]['message'] == 'POKELINUX_PROBE_KERNEL release=' + RELEASE
    assert first[1]['offset'] < before.start() < second[0]['offset'] < after.start() < second[2]['offset']
    old_text = old_console.decode('utf-8')
    for a in first:
        marker = f"{a['message']} epoch={a['epoch']} event={a['event']}"
        assert sum(line.endswith(marker) for line in old_text.splitlines()) == 1, 'saved native marker missing or stale'
    assert second[0]['epoch'] not in old_text, 'retrieval must be previous-boot console'
    children = re.findall(r'^POKELINUX_CHILD_EXEC pid=([1-9][0-9]*) parent=1$', console, re.M)
    assert len(children) == 2
    assert second[2]['message'] == f'POKELINUX_CHILD_WAIT pid={children[1]} actual_wait=true exit=Some(37)'
    assert second[3]['message'] == 'POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false'
    resets = [x['message'] for x in qmp if x['message'].get('event') == 'RESET']
    shutdowns = [x['message'] for x in qmp if x['message'].get('event') == 'SHUTDOWN']
    assert len(resets) == 1 and resets[0]['data'] == {'guest': False, 'reason': 'host-qmp-system-reset'}, 'actual owned external warm-reset event required'
    assert len(shutdowns) == 1 and shutdowns[0]['data'] == {'guest': True, 'reason': 'guest-shutdown'}
    reset_index = next(i for i, x in enumerate(qmp) if x['message'].get('event') == 'RESET')
    shutdown_index = next(i for i, x in enumerate(qmp) if x['message'].get('event') == 'SHUTDOWN')
    assert reset_index < shutdown_index, 'actual RESET must precede final guest SHUTDOWN'
    reset_replies = [(i, x['message']) for i, x in enumerate(qmp) if x['message'].get('id') == 'reset']
    assert len(reset_replies) == 1 and reset_replies[0][1] == {'return': {}, 'id': 'reset'} and reset_replies[0][0] < shutdown_index, 'successful owned system_reset response required'
    assert 'reboot: Power down' in console
    return {
        'actual_native_boot_epochs': [first[0]['epoch'], second[0]['epoch']],
        'actual_boot_UUIDs': [before[2], after[1]],
        'actual_pmsg_major': int(before[3]), 'actual_pmsg_minor': 0,
        'retained_pmsg_bytes': len(pmsg), 'retained_pmsg_sha256': hashlib.sha256(pmsg).hexdigest(),
        'retained_console_bytes': len(old_console), 'retained_console_sha256': hashlib.sha256(old_console).hexdigest(),
        'actual_previous_native_startup_markers_recovered': 2,
        'actual_native_ACKs': acks, 'actual_QMP_RESET': resets, 'actual_QMP_SHUTDOWN': shutdowns,
        'actual_reset_precedes_shutdown_and_successful_QMP_reset_reply': True,
        'second_child_source_bound_actual_wait37': True,
        'first_child_termination_was_reset_not_an_observed_wait': True,
        'retained_pmsg': pmsg, 'retained_console': old_console,
    }

# Independent native-PID1 goal; inspect() above remains byte-for-byte unchanged.
NATIVE_PHASE = re.compile(
    r'^POKELINUX_NATIVE_PSTORE_PHASE profile=(success|failure|no-backend) '
    r'current_boot=([0-9a-f-]{36}) stage=(first|retained) '
    r'backend=(present|absent) major=(0|[1-9][0-9]*) minor=(0|[1-9][0-9]*) '
    r'pmsg_status=(present|missing) pmsg_bytes=(0|[1-9][0-9]*) pmsg_hex=([0-9a-f]*) '
    r'console_bytes=(0|[1-9][0-9]*) console_hex=([0-9a-f]*)$', re.M)
SUBMITTED = re.compile(
    r'^POKELINUX_PMSG_SUBMITTED epoch=([0-9a-f]{32}) '
    r'event=([1-9][0-9]*) bytes=([1-9][0-9]*)$', re.M)
UNAVAILABLE = re.compile(
    r'^POKELINUX_PMSG_UNAVAILABLE epoch=([0-9a-f]{32}) '
    r'phase=(discover|submit|budget) kind=([A-Za-z]+) '
    r'errno=(None|Some\(-?[0-9]+\)) reason=([^\r\n]+)$', re.M)
NATIVE_CASES = {
    'native-success-retention': ('success', 37, 2),
    'native-child-failure-retention': ('failure', 3, 2),
    'actual-no-backend-optional-control': ('no-backend', 37, 1),
}
FAILURE_MESSAGE = ('POKELINUX_PROBE_FAILURE phase=child-status kind=InvalidData '
                   'errno=None reason=unexpected-child-exit')
COMPLETE_MESSAGE = 'POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false'


def _native_lines(text, prefix, pattern):
    lines = [line for line in text.splitlines() if line.startswith(prefix)]
    matches = list(pattern.finditer(text))
    assert len(lines) == len(matches), 'malformed ' + prefix
    return matches


def inspect_native_boots(console, case_id, complete=True):
    """Ordinary native/source/retained-console prerequisites, never PMSG success."""
    profile, first_exit, boots = NATIVE_CASES[case_id]
    assert len(console.encode()) <= 1024 * 1024
    console = console.replace('\r\n', '\n')
    assert not any(word in console for word in [
        'Kernel panic', 'POKELINUX_PROBE_REFUSED', 'POKELINUX_DIAGNOSTIC_FAILURE',
        'POKELINUX_PSTORE_WITNESS_FAILURE', 'POKELINUX_RECOVERY_CANARY',
        'POKELINUX_PSTORE_READY', 'POKELINUX_PSTORE_RETRIEVED'])
    acks = [{'epoch': m[1], 'event': int(m[2]), 'kernel_seq': int(m[3]),
             'message': m[4], 'offset': m.start()}
            for m in _native_lines(console, 'POKELINUX_DIAGNOSTIC_ACK ', ACK)]
    phases = _native_lines(console, 'POKELINUX_NATIVE_PSTORE_PHASE ', NATIVE_PHASE)
    children = list(re.finditer(r'^POKELINUX_CHILD_EXEC pid=([1-9][0-9]*) parent=1$', console, re.M))
    wanted_boots = boots if complete else 1
    assert len(acks) == wanted_boots * 4 and len(phases) == len(children) == wanted_boots
    assert [a['event'] for a in acks] == [1, 2, 3, 4] * wanted_boots
    assert console.count('reboot: Power down') == wanted_boots
    stages = [acks[index:index + 4] for index in range(0, len(acks), 4)]
    old_pmsg = b''
    old_console = b''
    UUIDs = []
    for index, stage in enumerate(stages):
        phase = phases[index]
        assert phase[1] == profile and phase[3] == ('first' if index == 0 else 'retained')
        UUIDs.append(phase[2])
        assert len({a['epoch'] for a in stage}) == 1
        assert all(a['event'] <= 2**64 - 1 and a['kernel_seq'] <= 2**64 - 1 for a in stage)
        assert all(a['kernel_seq'] < b['kernel_seq'] for a, b in zip(stage, stage[1:]))
        assert stage[0]['message'] == 'POKELINUX_NATIVE_PID1 pid=1 uid=0 version=0.1.0'
        assert stage[1]['message'] == 'POKELINUX_PROBE_KERNEL release=' + RELEASE
        assert stage[1]['offset'] < children[index].start() < phase.start() < stage[2]['offset']
        assert int(children[index][1]) > 1
        selected_exit = first_exit if index == 0 else 37
        assert stage[2]['message'] == (
            f'POKELINUX_CHILD_WAIT pid={children[index][1]} actual_wait=true exit=Some({selected_exit})')
        assert stage[3]['message'] == (FAILURE_MESSAGE if selected_exit == 3 else COMPLETE_MESSAGE)
        expected_backend = profile != 'no-backend'
        assert (phase[4] == 'present') == expected_backend
        major, minor = int(phase[5]), int(phase[6])
        assert 0 <= minor <= 2**32 - 1
        assert (0 < major <= 2**32 - 1) if expected_backend else (major == minor == 0)
        pmsg, saved = bytes.fromhex(phase[9]), bytes.fromhex(phase[11])
        assert len(pmsg) == int(phase[8]) <= 4096
        assert len(saved) == int(phase[10]) <= 65536
        assert phase[7] == 'present' or not pmsg
        if index == 0:
            assert phase[7] == 'missing' and not pmsg and not saved
        else:
            assert saved, 'actual retained console is the required phase anchor'
            old_pmsg, old_console = pmsg, saved
            old_text = saved.decode('utf-8')
            for previous in stages[0]:
                marker = f"{previous['message']} epoch={previous['epoch']} event={previous['event']}"
                assert sum(line.endswith(marker) for line in old_text.splitlines()) == 1
            assert stage[0]['epoch'] not in old_text
    assert len(set(UUIDs)) == wanted_boots
    assert len({stage[0]['epoch'] for stage in stages}) == wanted_boots
    if complete and boots == 2:
        assert phases[0].groups()[4:6] == phases[1].groups()[4:6], 'actual dynamic device identity changed'
    failure_lines = [line for line in console.splitlines() if 'POKELINUX_PROBE_FAILURE' in line]
    if profile == 'failure':
        # One stderr occurrence, one kernel occurrence and one ACK; no other failure.
        assert failure_lines and all(FAILURE_MESSAGE in line for line in failure_lines)
    else:
        assert not failure_lines
    expected = b''.join(
        f"{a['message']} epoch={a['epoch']} event={a['event']}\n".encode() for a in stages[0])
    assert len(expected) <= 4 * 512
    return {'case_id': case_id, 'profile': profile, 'native_ACKs': acks,
            'actual_boot_UUIDs': UUIDs, 'actual_native_epochs': [s[0]['epoch'] for s in stages],
            'actual_device_major': int(phases[0][5]), 'actual_device_minor': int(phases[0][6]),
            'actual_present_missing_pmsg': phases[-1][7],
            'retained_pmsg': old_pmsg, 'retained_console': old_console,
            'expected_first_native_pmsg': expected,
            'ordinary_native_and_console_prerequisites': True}


def inspect_native_qmp(qmp, commands, case_id):
    """Actual responses/events plus request order; STOP/RESUME remain recorded facts."""
    boots = NATIVE_CASES[case_id][2]
    messages = [row['message'] for row in qmp]
    requests = [row['request'] for row in commands]
    identities = [request['id'] for request in requests]
    wanted = ['caps', 'status', 'roms', 'cont-first']
    if boots == 2:
        wanted += ['status-shutdown', 'reset', 'status-reset', 'cont-second']
    wanted += ['quit']
    assert identities == wanted, 'fixed unique request sequence only'
    operations = {'caps': 'qmp_capabilities', 'status': 'query-status',
                  'roms': 'human-monitor-command', 'cont-first': 'cont',
                  'status-shutdown': 'query-status', 'reset': 'system_reset',
                  'status-reset': 'query-status', 'cont-second': 'cont', 'quit': 'quit'}
    for request in requests:
        assert request['execute'] == operations[request['id']]
        expected_keys = {'execute', 'id', 'arguments'} if request['id'] == 'roms' else {'execute', 'id'}
        assert set(request) == expected_keys
        if request['id'] == 'roms':
            assert request['arguments'] == {'command-line': 'info roms'}
    replies = {identity: [(index, row) for index, row in enumerate(messages) if row.get('id') == identity]
               for identity in identities}
    assert all(len(rows) == 1 for rows in replies.values())
    for identity, rows in replies.items():
        response = rows[0][1]
        assert 'return' in response and 'error' not in response
        if identity not in ('status', 'status-shutdown', 'status-reset', 'roms'):
            assert response == {'return': {}, 'id': identity}
    assert replies['status'][0][1]['return']['running'] is False
    guest_shutdown = [(i, m) for i, m in enumerate(messages)
                      if m.get('event') == 'SHUTDOWN' and m.get('data', {}).get('guest') is True]
    assert len(guest_shutdown) == boots
    assert all(m['data'] == {'guest': True, 'reason': 'guest-shutdown'} for _, m in guest_shutdown)
    other_shutdown = [m for m in messages if m.get('event') == 'SHUTDOWN'
                      and m.get('data', {}).get('guest') is not True]
    assert len(other_shutdown) <= 1
    if other_shutdown:
        assert other_shutdown[0]['data'] == {'guest': False, 'reason': 'host-qmp-quit'}
        quit_sent = commands[-1]['sent_monotonic_ns']
        assert next(row['received_monotonic_ns'] for row in qmp if row['message'] is other_shutdown[0]) >= quit_sent
    resets = [(i, m) for i, m in enumerate(messages) if m.get('event') == 'RESET']
    if boots == 2:
        assert len(resets) == 1 and resets[0][1]['data'] == {'guest': False, 'reason': 'host-qmp-system-reset'}
        shut_status = replies['status-shutdown'][0]
        assert shut_status[1]['return']['running'] is False and shut_status[1]['return']['status'] == 'shutdown'
        reset_status = replies['status-reset'][0]
        assert reset_status[1]['return']['running'] is False
        assert isinstance(reset_status[1]['return']['status'], str)
        assert guest_shutdown[0][0] < shut_status[0] < resets[0][0] < reset_status[0]
        assert replies['reset'][0][0] < reset_status[0] < replies['cont-second'][0][0] < guest_shutdown[1][0]
    else:
        assert not resets
    assert guest_shutdown[-1][0] < replies['quit'][0][0]
    return {'actual_guest_SHUTDOWN_count': len(guest_shutdown),
            'actual_RESET_count': len(resets), 'actual_host_quit_SHUTDOWN_count': len(other_shutdown),
            'actual_post_reset_status': replies['status-reset'][0][1]['return'] if boots == 2 else None,
            'actual_quit_reply': replies['quit'][0][1], 'fixed_QMP_prerequisites': True}


def inspect_native(console, qmp, commands, case_id):
    observed = inspect_native_boots(console, case_id)
    observed.update(inspect_native_qmp(qmp, commands, case_id))
    submitted = _native_lines(console.replace('\r\n', '\n'), 'POKELINUX_PMSG_SUBMITTED ', SUBMITTED)
    unavailable = _native_lines(console.replace('\r\n', '\n'), 'POKELINUX_PMSG_UNAVAILABLE ', UNAVAILABLE)
    if NATIVE_CASES[case_id][2] == 1:
        assert not submitted and not observed['retained_pmsg'] and not observed['retained_console']
        assert len(unavailable) == 1 and unavailable[0][1] == observed['actual_native_epochs'][0]
        assert unavailable[0][2] == 'discover'
        observed['native_pmsg_goal'] = 'actual-no-backend-ordinary-native-child37'
    elif not observed['retained_pmsg'] and not submitted:
        assert len(unavailable) == 2
        assert [m[1] for m in unavailable] == observed['actual_native_epochs']
        assert all(m.groups()[1:] == ('discover', 'Unsupported', 'None', 'native-pmsg-discovery-not-implemented') for m in unavailable)
        observed['native_pmsg_goal'] = 'MISSING_NATIVE_PID1_PMSG_RECORDS'
    else:
        assert observed['actual_present_missing_pmsg'] == 'present'
        assert observed['retained_pmsg'] == observed['expected_first_native_pmsg'], 'whole native PMSG equality required'
        assert not unavailable
        wanted = [(a['epoch'], a['event'], len(f"{a['message']} epoch={a['epoch']} event={a['event']}\n".encode()))
                  for a in observed['native_ACKs']]
        assert [(m[1], int(m[2]), int(m[3])) for m in submitted] == wanted
        observed['native_pmsg_goal'] = 'whole-first-native-epoch-retained'
    observed['retained_pmsg_sha256'] = hashlib.sha256(observed['retained_pmsg']).hexdigest()
    observed['retained_console_sha256'] = hashlib.sha256(observed['retained_console']).hexdigest()
    return observed


def classify_native_cases(cases, fixed_source_bindings_verified):
    """Call only after each actual CLI/wait/EOF/CID/input closure; no inferred closure."""
    assert fixed_source_bindings_verified is True
    assert set(cases) == set(NATIVE_CASES)
    for case_id, row in cases.items():
        assert row['case_id'] == case_id and row['prerequisites_passed'] is True
        assert row['runtime_closure_passed'] is True and row['actual_cli_exit'] == 0
        assert row['actual_cli_wait'] is True and row['stdout_eof'] is True and row['stderr_eof'] is True
        assert row['all_observations_before_original_cutoffs'] is True
    goals = [cases[case]['native_pmsg_goal'] for case in list(NATIVE_CASES)[:2]]
    assert cases['actual-no-backend-optional-control']['native_pmsg_goal'] == 'actual-no-backend-ordinary-native-child37'
    if goals == ['MISSING_NATIVE_PID1_PMSG_RECORDS'] * 2:
        return {'qualified_behavioral_RED': True, 'qualified_GREEN': False,
                'reason': 'MISSING_NATIVE_PID1_PMSG_RECORDS'}
    assert goals == ['whole-first-native-epoch-retained'] * 2
    return {'qualified_behavioral_RED': False, 'qualified_GREEN': True,
            'reason': 'whole-first-native-epoch-retained-in-both-controls'}
