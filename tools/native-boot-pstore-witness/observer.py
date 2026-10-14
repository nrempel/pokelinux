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
