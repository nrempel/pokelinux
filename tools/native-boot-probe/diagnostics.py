"""Current probe console acceptance; kernel output alone is not a readback ACK."""
import re

ACK = re.compile(r'^POKELINUX_DIAGNOSTIC_ACK epoch=([0-9a-f]{32}) '
                 r'event=([1-9][0-9]{0,19}) kernel_seq=(0|[1-9][0-9]{0,19}) (.{1,512})$')


def validate(console, mode='green'):
    acks = []

    def result(passed, reason):
        return {'passed': passed, 'reason': reason, 'acks': acks}

    if mode not in ('green', 'missing-child') or len(console.encode()) > 4 * 1024 * 1024:
        return result(False, 'unsupported mode or console bound')
    for line in console.splitlines():
        if not line.startswith('POKELINUX_DIAGNOSTIC_ACK '):
            continue
        match = ACK.fullmatch(line)
        if not match:
            return result(False, 'malformed readback acknowledgement')
        epoch, event, sequence, message = match.groups()
        if int(event) > 2**64 - 1 or int(sequence) > 2**64 - 1:
            return result(False, 'acknowledgement integer outside bound')
        acks.append({'epoch': epoch, 'event': int(event),
                     'kernel_seq': int(sequence), 'message': message})
    if any(value in console for value in ['Kernel panic', 'POKELINUX_PROBE_REFUSED',
                                         'POKELINUX_DIAGNOSTIC_FAILURE']):
        return result(False, 'panic, refusal or diagnostic failure')
    if mode == 'green' and 'POKELINUX_PROBE_FAILURE' in console:
        return result(False, 'probe failure observed')
    count = 4 if mode == 'green' else 3
    if len(acks) != count or [a['event'] for a in acks] != list(range(1, count + 1)):
        return result(False, 'missing, extra or reordered acknowledgements')
    if len({a['epoch'] for a in acks}) != 1 or any(
            a['kernel_seq'] >= b['kernel_seq'] for a, b in zip(acks, acks[1:])):
        return result(False, 'changed epoch or non-increasing kernel sequence')
    if acks[0]['message'] != 'POKELINUX_NATIVE_PID1 pid=1 uid=0 version=0.1.0':
        return result(False, 'wrong init acknowledgement')
    if acks[1]['message'] != 'POKELINUX_PROBE_KERNEL release=7.1.6-1-aarch64-ARCH':
        return result(False, 'wrong kernel acknowledgement')
    phases = re.findall(r'POKELINUX_PROBE_FAILURE phase=([a-z-]+)', console)
    children = re.findall(r'^POKELINUX_CHILD_EXEC pid=([1-9][0-9]*) parent=1$', console, re.M)
    if mode == 'green':
        waited = re.fullmatch(r'POKELINUX_CHILD_WAIT pid=([1-9][0-9]*) '
                              r'actual_wait=true exit=Some\(37\)', acks[2]['message'])
        if (phases or waited is None or int(waited[1]) <= 1 or children != [waited[1]]
                or acks[3]['message'] != 'POKELINUX_PROBE_COMPLETE persistent_storage=false phone_services=false'):
            return result(False, 'failure or missing matching child/completion evidence')
    elif (children or not phases or set(phases) != {'child-spawn'} or not acks[2]['message'].startswith(
            'POKELINUX_PROBE_FAILURE phase=child-spawn kind=NotFound errno=Some(2) reason=')):
        return result(False, 'missing-child failure was not acknowledged')
    if 'reboot: Power down' not in console:
        return result(False, 'controlled powerdown not observed')
    return result(True, 'ordered readback acknowledgements and powerdown observed')
