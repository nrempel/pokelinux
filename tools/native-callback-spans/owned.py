"""Owned direct-child PIPE, pidfd and immutable cutoff custody for one composition tracer."""
import os
from pathlib import Path
import select
import signal
import subprocess
import time


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


class Scope:
    def __init__(self, output):
        self.output = Path(output)
        self.children = []
        self.receipts = []

    def start(self, argv, name, limit, stdin=None, env=None, cutoff=None, identity=None, wait_cutoff=None):
        log = (self.output / name).open('xb', buffering=0)
        try:
            if cutoff is not None:
                assert now() < cutoff, name + ' original pre-spawn cutoff'
            spawn_started_at_ns = now()
            child = subprocess.Popen([str(v) for v in argv], stdin=stdin or subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env, close_fds=True, start_new_session=True, **(identity or {}))
        except BaseException:
            log.close()
            raise
        item = {'child': child, 'pidfd': None, 'pipe_ready': False, 'log': log,
            'argv': [str(v) for v in argv], 'name': name, 'limit': limit, 'bytes': 0,
            'eof': False, 'overflow': False, 'wait': None,
            'spawn_admission_cutoff_ns': cutoff, 'original_wait_cutoff_ns': wait_cutoff if wait_cutoff is not None else cutoff,
            'spawn_started_at_ns': spawn_started_at_ns, 'spawned_at_ns': now()}
        self.children.append(item)  # Retain custody before any fallible post-spawn setup.
        os.set_blocking(child.stdout.fileno(), False)
        item['pipe_ready'] = True
        item['pidfd'] = os.pidfd_open(child.pid, os.O_NONBLOCK)
        return item

    def pump(self):
        for item in self.children:
            if item['eof'] or not item['pipe_ready']:
                continue
            # One bounded read per child per turn also drains the live public peer.
            try:
                data = os.read(item['child'].stdout.fileno(), 65536)
            except (BlockingIOError, InterruptedError):
                continue
            if not data:
                item['eof'] = True
                item['eof_at_ns'] = now()
                continue
            room = max(0, item['limit'] - item['bytes'])
            item['log'].write(data[:room])
            item['bytes'] += len(data)
            item['overflow'] |= item['bytes'] > item['limit']

    def check(self):
        assert not any(v['overflow'] for v in self.children), 'bounded private output exceeded'

    def live(self, item):
        return item['child'].poll() is None and (item['pidfd'] is None or not select.select([item['pidfd']], [], [], 0)[0])

    def finish(self, item, cutoff, check=True):
        if item['wait'] is not None:
            return item['wait']['returncode']
        if item['original_wait_cutoff_ns'] is not None:
            cutoff = min(cutoff, item['original_wait_cutoff_ns'])
        while item['child'].poll() is None:
            assert now() < cutoff, item['name'] + ' original direct-wait cutoff'
            self.pump()
            if check:
                self.check()
            time.sleep(.005)
        code = item['child'].wait(timeout=0)
        waited_at = now()  # Immediate post-actual-wait check; save even late receipts.
        receipt = {'role': item['name'], 'pid': item['child'].pid, 'argv': item['argv'],
            'returncode': code, 'actual_direct_parent_wait': True, 'waited_at_boottime_ns': waited_at,
            'original_cutoff_ns': cutoff, 'spawn_admission_cutoff_ns': item['spawn_admission_cutoff_ns'],
            'original_wait_cutoff_ns': item['original_wait_cutoff_ns'], 'wait_before_original_cutoff': waited_at < cutoff}
        item['wait'] = receipt
        self.receipts.append(receipt)
        while not item['eof'] and now() < cutoff:
            self.pump()
            time.sleep(.001)
        receipt.update({'stdin_pipe_present': item['child'].stdin is not None,
            'stdin_pipe_closed': item['child'].stdin is None or item['child'].stdin.closed,
            'stdout_bytes': item['bytes'], 'stdout_limit': item['limit'],
            'output_overflow': item['overflow'], 'stdout_EOF': item['eof'],
            'stdout_EOF_at_ns': item.get('eof_at_ns'), 'EOF_before_original_cutoff': bool(
                item['eof'] and item['eof_at_ns'] < cutoff)})
        assert waited_at < cutoff, item['name'] + ' actual wait observed late'
        assert receipt['EOF_before_original_cutoff'], item['name'] + ' EOF observed late or absent'
        if check:
            self.check()
        return code

    def run(self, argv, name, cutoff, limit=1048576, stdin=None, env=None, identity=None):
        assert now() < cutoff, name + ' original command admission cutoff'
        item = self.start(argv, name, limit, stdin=stdin, env=env, cutoff=cutoff, identity=identity)
        code = self.finish(item, cutoff)
        data = (self.output / name).read_bytes()
        return code, data

    def close(self, cutoff):
        errors = []
        for item in reversed(self.children):
            try:
                stream = item['child'].stdin
                if stream is not None and not stream.closed:
                    stream.close()
                if item['wait'] is None:
                    if self.live(item):
                        if item['pidfd'] is not None:
                            signal.pidfd_send_signal(item['pidfd'], signal.SIGKILL)
                        else:
                            item['child'].kill()  # Exact retained Popen during partial setup.
                    self.finish(item, cutoff, check=False)
            except BaseException as error:
                errors.append(type(error).__name__ + ': ' + str(error))
            item['log'].close()
            item['child'].stdout.close()
            if item['pidfd'] is not None:
                os.close(item['pidfd'])
        return errors
