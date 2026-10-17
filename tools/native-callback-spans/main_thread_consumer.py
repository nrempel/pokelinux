#!/usr/bin/env python3
"""Frozen independent real-Linux main-task consumer; source only before implementation."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import select
import stat
import subprocess
import sys
import time

sys.dont_write_bytecode = True
FIRST_NS = 250_000_000
SECOND_NS = 350_000_000
RECORD_CAP = 32_768
UNTIL = None
CLOSE = None
CHILD = "import os; os.write(1,b'READY\\n'); value=os.read(0,1); assert value==b'R'; os.write(1,b'RELEASED\\n')"


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def active():
    if now() >= UNTIL:
        raise TimeoutError('original consumer 5s work bound')


def load(name, path):
    path = path.resolve(strict=True)
    sys.path.insert(0, str(path.parent))
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def stat_fields(text):
    head, tail = text.rsplit(') ', 1)
    fields = tail.split()
    return int(head.split(' ', 1)[0]), fields[0], fields[19]


def status_fields(text):
    return {line.split(':', 1)[0]: line.split(':', 1)[1].split()
            for line in text.splitlines() if ':' in line}


def oracle(pid):
    active()
    root = Path('/proc') / str(pid) / 'task' / str(pid)
    result = {}
    for name in ['stat', 'status', 'syscall', 'wchan']:
        with (root / name).open('rb') as stream:
            raw = stream.read(4096)
        assert len(raw) < 4096, 'independent control proc record at cap'
        result[name] = raw.decode('ascii')
    active()
    actual_pid, state, stamp = stat_fields(result['stat'])
    fields = status_fields(result['status'])
    assert actual_pid == pid and fields['Pid'] == fields['Tgid'] == [str(pid)]
    result.update(PID=pid, TID=pid, state=state, starttime=stamp)
    return result


def is_pipe_read(row):
    words = row['syscall'].split()
    return (row['state'] == 'S' and len(words) >= 9 and words[0] != 'running'
            and int(words[0], 0) >= 0 and int(words[1], 0) == 0
            and 'pipe' in row['wchan'].lower())


def check_record(observer):
    row = observer.record
    assert len(json.dumps(row, sort_keys=True, separators=(',', ':')).encode()) <= RECORD_CAP
    assert row['optional_diagnostic_only'] is True
    assert row['Source_channel_accessed'] is False
    assert row['live_callback_ring_read'] is False
    assert row['internal_AQ_callback_identity_proven'] is False
    assert len(row['attempts']) <= 2
    total = 0
    for sample in row['attempts']:
        for name, entry in sample['files'].items():
            raw = entry['raw_text'].encode('ascii')
            cap = 1024 if name in ['stat_before', 'stat_after'] else 4096
            assert len(raw) == entry['bytes'] <= entry['read_limit_bytes'] == cap
            assert hashlib.sha256(raw).hexdigest() == entry['sha256']
            assert entry['possibly_truncated'] == (len(raw) == cap)
            assert sample['started_boottime_ns'] <= entry['started_boottime_ns']
            assert entry['started_boottime_ns'] <= entry['completed_boottime_ns'] <= sample['completed_boottime_ns']
            total += len(raw)
    assert total <= 28_672
    return row


def compare_sample(observer, ordinal, pid, stamp):
    row = check_record(observer)
    sample = row['attempts'][ordinal - 1]
    assert sample['ordinal'] == ordinal and sample['PID'] == sample['TID'] == pid
    assert sample['proc_identity_verified'] is True
    assert sample['pidfd_live_before'] is True and sample['pidfd_live_after'] is True
    assert sample['proc_starttime_before'] == sample['proc_starttime_after'] == stamp
    assert sample['started_boottime_ns'] >= row['due_boottime_ns'][ordinal - 1]
    assert sample['completed_boottime_ns'] <= now() and not sample['errors']
    assert set(sample['files']) == {'stat_before', 'stat_after', 'status', 'syscall', 'wchan'}
    assert not any(v['possibly_truncated'] for v in sample['files'].values())
    expected = oracle(pid)
    assert is_pipe_read(expected), 'real child no longer in controlled pipe read'
    for key in ['stat_before', 'stat_after']:
        actual_pid, state, actual_stamp = stat_fields(sample['files'][key]['raw_text'])
        assert actual_pid == pid and state == expected['state'] == 'S' and actual_stamp == stamp
    fields = status_fields(sample['files']['status']['raw_text'])
    assert fields['Pid'] == fields['Tgid'] == [str(pid)]
    assert sample['files']['syscall']['raw_text'].split() == expected['syscall'].split()
    assert sample['files']['wchan']['raw_text'].strip() == expected['wchan'].strip()
    return sample


def after_due(observer, ordinal):
    due = observer.record['due_boottime_ns'][ordinal - 1]
    active()
    time.sleep(max(0, (due - now()) / 1e9) + .002)
    active()
    observer.watch()


def bind(cls, pid, pidfd, stamp):
    before = now()
    observer = cls(UNTIL)
    row = check_record(observer)
    assert row['attempts'] == [] and before <= row['created_boottime_ns'] <= now()
    observer.watch()
    assert check_record(observer)['attempts'] == []
    observer.bind_main(pid, pidfd, stamp)
    row = check_record(observer)
    trigger = row['binding_boottime_ns']
    assert before <= trigger <= now() and row['attempts'] == []
    assert row['due_boottime_ns'] == [trigger + FIRST_NS, trigger + SECOND_NS]
    return observer


def target(cls, pid, pidfd, stamp, release):
    records = {}
    observer = bind(cls, pid, pidfd, stamp)
    dead_case = bind(cls, pid, pidfd, stamp)
    assert now() < observer.record['due_boottime_ns'][0] - 1_000_000, 'missed actual early control window'
    observer.watch()
    assert check_record(observer)['attempts'] == []
    after_due(observer, 1)
    assert len(check_record(observer)['attempts']) == 1
    compare_sample(observer, 1, pid, stamp)
    assert now() < observer.record['due_boottime_ns'][1] - 1_000_000, 'missed actual between-samples control window'
    observer.watch()
    assert len(check_record(observer)['attempts']) == 1
    after_due(observer, 2)
    compare_sample(observer, 2, pid, stamp)
    before = json.dumps(check_record(observer), sort_keys=True)
    observer.bind_main(pid, pidfd, stamp)
    observer.watch()
    observer.watch()
    assert json.dumps(check_record(observer), sort_keys=True) == before
    records['two_delayed_real_samples'] = observer.record
    print('GROUP genuine-main-task-pipe-read-early-schedule-and-two-attempt-cap PASS', flush=True)

    wrong = bind(cls, pid, pidfd, str(int(stamp) + 1))
    after_due(wrong, 1)
    row = check_record(wrong)
    assert row['terminal_reason'] and (row['errors'] or any(v['errors'] for v in row['attempts']))
    assert not any(v['proc_identity_verified'] or v['files'] for v in row['attempts'])
    assert len(row['attempts']) <= 1
    before = json.dumps(row, sort_keys=True)
    after_due(wrong, 2)
    assert json.dumps(check_record(wrong), sort_keys=True) == before
    wrong.bind_main(pid, pidfd, stamp)
    wrong.watch()
    assert json.dumps(check_record(wrong), sort_keys=True) == before
    records['wrong_starttime'] = wrong.record
    print('GROUP actual-starttime-mismatch-consumed-without-retry PASS', flush=True)

    expired = cls(now())
    expired.bind_main(pid, pidfd, stamp)
    expired.watch()
    row = check_record(expired)
    assert row['terminal_reason'] and not row['attempts']
    records['expired_original_work'] = expired.record
    print('GROUP expired-original-work-no-proc-attempt PASS', flush=True)

    release()
    assert select.select([pidfd], [], [], 0)[0], 'actual retained child pidfd not dead'
    assert now() >= dead_case.record['due_boottime_ns'][1]
    dead_case.watch()
    row = check_record(dead_case)
    assert row['terminal_reason'] and len(row['attempts']) <= 1
    assert not any(v['proc_identity_verified'] or v['files'] for v in row['attempts'])
    before = json.dumps(row, sort_keys=True)
    dead_case.bind_main(pid, pidfd, stamp)
    dead_case.watch()
    assert json.dumps(check_record(dead_case), sort_keys=True) == before
    records['actual_pidfd_death'] = dead_case.record
    print('GROUP actual-child-death-consumed-without-proc-retry PASS', flush=True)
    return records


def exercise(owned, output, cls):
    output.mkdir(mode=0o700)
    scope = owned.Scope(output)
    item = None
    failure = None
    result = {'schema': 1, 'passed': False, 'scope': 'Host original-Linux child/proc boundary only; no native compositor or Source',
              'original_work_until_ns': UNTIL, 'original_close_until_ns': CLOSE, 'optional_API_selected': cls is not None}

    def release():
        stream = item['child'].stdin
        if stream is not None and not stream.closed:
            stream.write(b'R')
            stream.flush()
            stream.close()
        code = scope.finish(item, CLOSE)
        assert code == 0
        active()
        text = (output / 'pipe-child.log').read_text()
        assert text == 'READY\nRELEASED\n'
        receipt = item['wait']
        assert receipt['actual_direct_parent_wait'] and receipt['stdout_EOF']
        assert receipt['wait_before_original_cutoff'] and receipt['EOF_before_original_cutoff']
        assert receipt['stdin_pipe_closed'] and not receipt['output_overflow']

    try:
        active()
        item = scope.start([sys.executable, '-B', '-c', CHILD], 'pipe-child.log', 4096,
                           stdin=subprocess.PIPE, cutoff=UNTIL, wait_cutoff=CLOSE)
        pid = item['child'].pid
        observed = None
        while observed is None:
            active()
            scope.pump()
            scope.check()
            assert scope.live(item), 'owned child died before real blocked-read control'
            if (output / 'pipe-child.log').read_bytes() == b'READY\n':
                candidate = oracle(pid)
                if is_pipe_read(candidate):
                    observed = candidate
                    break
            time.sleep(.005)
        assert not select.select([item['pidfd']], [], [], 0)[0]
        pipe_stat = os.fstat(item['child'].stdin.fileno())
        assert stat.S_ISFIFO(pipe_stat.st_mode)
        assert os.readlink(Path('/proc') / str(pid) / 'fd' / '0') == 'pipe:[' + str(pipe_stat.st_ino) + ']'
        result['original_control'] = {'actual_PID_equals_main_TID': pid, 'proc_starttime': observed['starttime'],
                                      'actual_state': observed['state'], 'actual_wchan_class': 'pipe-read',
                                      'actual_reported_syscall_number': int(observed['syscall'].split()[0], 0),
                                      'first_syscall_argument_is_control_pipe_stdin': True,
                                      'kernel_records_scripted': False}
        if cls is None:
            release()
            print('CONTROL original-Linux-owned-child-main-task-pipe-read-proc-pidfd-wait-EOF PASS', flush=True)
        else:
            result['observer_records_private'] = target(cls, pid, item['pidfd'], observed['starttime'], release)
        result['passed'] = True
    except BaseException as error:
        failure = error
        result['failure'] = {'type': type(error).__name__, 'detail': str(error)[:512]}
    finally:
        if item is not None and item['wait'] is None:
            try:
                release()
            except BaseException as error:
                result['release_error'] = {'type': type(error).__name__, 'detail': str(error)[:512]}
                result['passed'] = False
        result['cleanup_errors'] = scope.close(CLOSE)
        result['owned_process_receipts'] = scope.receipts
        result['passed'] = bool(result['passed'] and not result['cleanup_errors'] and len(scope.receipts) == 1)
        result['completed_boottime_ns'] = now()
        raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
        assert len(raw) <= 131072
        (output / 'result.json').write_bytes(raw)
    if failure is not None:
        raise failure
    assert result['passed'], 'real child ownership/closure failed'


def main():
    global UNTIL, CLOSE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--helper', type=Path)
    parser.add_argument('--owned', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--control', action='store_true')
    args = parser.parse_args()
    started = now()
    UNTIL = started + 5_000_000_000
    CLOSE = started + 8_000_000_000
    cls = None
    if not args.control:
        assert args.helper is not None
        subject = load('main_thread_subject', args.helper)
        cls = getattr(subject, 'MainThreadObserver', None)
        if cls is None:
            print('MISSING_OPTIONAL_MAIN_THREAD_OBSERVER_API', flush=True)
            return 3
        assert callable(cls)
    owned = load('main_thread_owned', args.owned)
    active()
    exercise(owned, args.output, cls)
    active()
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
