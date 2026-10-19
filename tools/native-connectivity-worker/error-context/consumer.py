"""One actual cap0 normal-worker EPERM; desired assertion only after closure."""
import argparse, errno, hashlib, json, os, re, stat, sys
from pathlib import Path
import owned
import closure
from owned import Scope, now
from closure import close_owned

OWNED = '5a738b9529a1a0e5d5b0bd20e27e2ec4244d4e61d7c8e0dad96fe1ffbef3c423'
CLOSURE = '646e9cfd5f755b626fe8babfa26637ddac328ede05cdab75aa77cc51332d70d6'
MARKER = 'WORKER_FAILURE_MUST_IDENTIFY_REDUCE_CAPABILITIES_AND_ORIGINAL_EPERM'
MESSAGE = r'"(?:[^"\\\r\n]|\\.){0,300}"'
SOURCE = r'Os \{ code: 1, kind: PermissionDenied, message: ' + MESSAGE + r' \}'
BASELINE = re.compile(r'Error: ' + SOURCE + r'\n')
DESIRED = re.compile(r'Error: Custom \{ kind: PermissionDenied, error: WorkerFailure \{ stage: ReduceCapabilities, original_until_ns: None, detected_at_ns: ([0-9]{1,20}), source: ' + SOURCE + r' \} \}\n')

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def process_metadata(pid):
    before = now()
    prefixes = ('Name:', 'State:', 'Tgid:', 'Pid:', 'PPid:', 'Uid:', 'Gid:', 'Threads:', 'CapInh:', 'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:', 'NoNewPrivs:')
    values = {line.split(':', 1)[0]: line.split(':', 1)[1].strip()
              for line in Path(f'/proc/{pid}/status').read_text().splitlines()
              if line.startswith(prefixes)}
    tasks = sorted(int(p.name) for p in Path(f'/proc/{pid}/task').iterdir())
    return {'sample_started_at_ns': before, 'sample_completed_at_ns': now(),
            'status': values, 'tasks': tasks, 'atomic_snapshot_claimed': False,
            'running_claimed': False}

def cap0_single_thread(metadata, pid):
    s = metadata['status']
    return (all(int(s[k], 16) == 0 for k in ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb'])
            and s['NoNewPrivs'] == '1' and s['Threads'] == '1'
            and int(s['Pid']) == int(s['Tgid']) == pid and metadata['tasks'] == [pid])

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--worker', required=True, type=Path)
    p.add_argument('--worker-sha256', required=True)
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args()
    assert a.output.is_absolute() and not a.output.exists()
    a.output.mkdir(mode=0o700)
    scope = Scope(a.output)
    r = {'passed': False, 'behavior_RED_qualified': False, 'observation_qualified': False,
         'classification': 'SETUP_FAILURE_OR_UNCONFIRMED', 'worker_effect_claimed': False,
         'stderr_mode': 'merged_stdout_PIPE', 'worker_output_limit_bytes': 4096}
    child = ns = None
    natural = False
    started = now()
    W, C = started + 1_000_000_000, started + 3_000_000_000
    r.update(started_at_ns=started, original_work_until_ns=W, original_close_until_ns=C)
    try:
        assert digest(owned.__file__) == OWNED and digest(closure.__file__) == CLOSURE
        assert a.worker.is_absolute() and a.worker.is_file() and not a.worker.is_symlink()
        m = a.worker.stat()
        assert stat.S_ISREG(m.st_mode) and m.st_mode & 0o111 and not m.st_mode & 0o6000
        assert m.st_size <= 8 * 1024 * 1024
        assert re.fullmatch('[0-9a-f]{64}', a.worker_sha256) and digest(a.worker) == a.worker_sha256
        try:
            os.getxattr(a.worker, 'security.capability')
        except OSError as e:
            assert e.errno in (errno.ENODATA, errno.ENOTSUP)
        else:
            raise AssertionError('worker must have no file capabilities')
        with a.worker.open('rb') as stream:
            header = stream.read(20)
        assert header[:6] == b'\x7fELF\x02\x01' and int.from_bytes(header[18:20], 'little') == 62
        parent = process_metadata(os.getpid())
        assert cap0_single_thread(parent, os.getpid())
        assert os.getuid() == os.geteuid() and os.getgid() == os.getegid()
        assert os.getuid() in (0, 1000)
        r.update(worker={'path': str(a.worker), 'bytes': m.st_size, 'sha256': a.worker_sha256,
                         'ELF_machine': 62, 'normal_release_selection_required_from_outer_provider': True},
                 parent_metadata=parent)
        ns = Path('/proc/self/ns/net').open('rb')
        ns_meta = os.fstat(ns.fileno())
        argv = [str(a.worker), '--pokelinux-process-worker',
                f'/proc/{os.getpid()}/fd/{ns.fileno()}', str(ns_meta.st_dev), str(ns_meta.st_ino), str(W)]
        r['actual_argv'] = argv
        environment = dict(os.environ, LANG='C', LC_ALL='C')
        child = scope.start(argv, 'actual-normal-worker.log', 4096, env=environment,
                            cutoff=W, wait_cutoff=W)
        # No Scope live/pump/finish/poll before these samples: an unreaped
        # terminal child may legitimately be observed as a zombie, not running.
        metadata = process_metadata(child['child'].pid)
        r['child_metadata_before_parent_wait'] = metadata
        r['child_observed_before_any_parent_poll_or_wait'] = True
        assert cap0_single_thread(metadata, child['child'].pid)
        s = metadata['status']
        assert [int(v) for v in s['Uid'].split()] == [os.getuid()] * 4
        assert [int(v) for v in s['Gid'].split()] == [os.getgid()] * 4
        assert int(s['PPid']) == os.getpid()
        code = scope.finish(child, W)
        natural = True
        body = (a.output / 'actual-normal-worker.log').read_bytes()
        text = body.decode('utf-8', errors='strict')
        baseline = BASELINE.fullmatch(text) is not None
        match = DESIRED.fullmatch(text)
        detected = int(match.group(1)) if match else None
        within = (match is not None and child['spawn_started_at_ns'] <= detected
                  <= child['wait']['waited_at_boottime_ns'] < W)
        r.update(actual_worker_exit=code, natural_exit_without_cleanup_signal=True,
                 raw_output_sha256=hashlib.sha256(body).hexdigest(), raw_output_bytes=len(body),
                 exact_unwrapped_original_EPERM=baseline,
                 exact_desired_context=match is not None, detected_at_ns=detected,
                 detection_in_actual_spawn_wait_span=within,
                 normal_worker_observation_qualified=(code == 1 and (baseline or within)))
    except BaseException as e:
        r['setup_or_observation_error'] = type(e).__name__ + ': ' + str(e)
    finally:
        r['closure'] = close_owned(scope, C, reap_adopted=True)
        if ns is not None:
            ns.close()
        r['namespace_handle_closed'] = ns is None or ns.closed
        r['owned_receipts'] = scope.receipts
        r['all_parent_handles_closed'] = all(v['wait'] is not None and v['eof'] and v['log'].closed
            and v['child'].stdout.closed and v['pidfd'] is None for v in scope.children)
        r['completed_at_ns'] = now()
    closed = (r['closure']['closed'] and r['closure']['wait_all_ECHILD']
              and not r['closure']['adopted_actual_waits']
              and not r['closure']['adopted_pidfds_signaled']
              and r['all_parent_handles_closed'] and r['namespace_handle_closed'])
    observation = (natural and r.get('normal_worker_observation_qualified', False)
                   and len(scope.receipts) == 1 and closed
                   and scope.receipts[0]['returncode'] == 1
                   and scope.receipts[0]['actual_direct_parent_wait']
                   and scope.receipts[0]['wait_before_original_cutoff']
                   and scope.receipts[0]['stdout_EOF'] and scope.receipts[0]['EOF_before_original_cutoff']
                   and not scope.receipts[0]['output_overflow']
                   and r['completed_at_ns'] < C)
    r['observation_qualified'] = observation
    r['passed'] = observation and r.get('detection_in_actual_spawn_wait_span', False)
    r['behavior_RED_qualified'] = observation and r.get('exact_unwrapped_original_EPERM', False)
    if r['passed']:
        r['classification'] = 'ACTUAL_NORMAL_WORKER_ORIGINAL_EPERM_STAGE_CONTEXT_GREEN'
    elif r['behavior_RED_qualified']:
        r['classification'] = 'ACTUAL_NORMAL_WORKER_ORIGINAL_EPERM_MISSING_STAGE_RED'
    (a.output / 'result.json').write_text(json.dumps(r, indent=2) + '\n')
    if not observation:
        raise RuntimeError('UNCONFIRMED: setup, observation, timing or custody qualification failed')
    assert r['passed'], MARKER

if __name__ == '__main__':
    main()
