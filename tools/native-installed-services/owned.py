"""Owned direct-child PIPE, pidfd and immutable cutoff custody for one composition tracer."""
import os
import hashlib
import resource
import stat
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

    def start(self, argv, name, limit, stdin=None, env=None, cutoff=None, identity=None, wait_cutoff=None, stderr_regular=None):
        log = (self.output / name).open('xb', buffering=0)
        stderr_file = None
        stderr_identity = None
        spawn_options = dict(identity or {})
        try:
            if stderr_regular is not None:
                stderr_path = Path(stderr_regular)
                assert stderr_path.parent.resolve() == self.output.resolve(), 'stderr sink outside owned output'
                stderr_file = stderr_path.open('x+b', buffering=0)
                observed = os.fstat(stderr_file.fileno())
                assert stat.S_ISREG(observed.st_mode), 'stderr sink must be a fresh regular file'
                stderr_identity = {'device': observed.st_dev, 'inode': observed.st_ino}
                assert 'preexec_fn' not in spawn_options, 'explicit stderr sink cannot replace another child setup'
                _, hard = resource.getrlimit(resource.RLIMIT_FSIZE)
                file_limit = limit if hard == resource.RLIM_INFINITY else min(limit, hard)
                def bound_stderr_file():
                    resource.setrlimit(resource.RLIMIT_FSIZE, (file_limit, file_limit))
                spawn_options['preexec_fn'] = bound_stderr_file

            if cutoff is not None:
                assert now() < cutoff, name + ' original pre-spawn cutoff'
            spawn_started_at_ns = now()
            child = subprocess.Popen([str(v) for v in argv], stdin=stdin or subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=stderr_file if stderr_file is not None else subprocess.STDOUT, env=env, close_fds=True, start_new_session=True, **spawn_options)
        except BaseException:
            if stderr_file is not None:
                stderr_file.close()
            log.close()
            raise
        item = {'child': child, 'pidfd': None, 'pipe_ready': False, 'log': log,
            'argv': [str(v) for v in argv], 'name': name, 'limit': limit, 'bytes': 0,
            'eof': False, 'overflow': False, 'wait': None,
            'stderr_file': stderr_file, 'stderr_path': str(stderr_regular) if stderr_regular is not None else None,
            'stderr_identity': stderr_identity, 'stderr_bytes': 0, 'stderr_overflow': False,
            'stderr_file_limit': file_limit if stderr_file is not None else None,
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

    def observe_stderr(self, item):
        stream = item['stderr_file']
        if stream is not None and not stream.closed:
            observed = os.fstat(stream.fileno())
            assert stat.S_ISREG(observed.st_mode)
            assert (observed.st_dev, observed.st_ino) == (item['stderr_identity']['device'], item['stderr_identity']['inode'])
            item['stderr_bytes'] = observed.st_size
            item['stderr_overflow'] |= observed.st_size > item['stderr_file_limit']

    def close_stderr(self, item):
        stream = item['stderr_file']
        if stream is None:
            if item['wait'] is not None:
                item['wait']['stderr_mode'] = 'merged_stdout_PIPE'
            return
        if not stream.closed:
            self.observe_stderr(item)
            size = item['stderr_bytes']
            hasher = hashlib.sha256()
            offset = 0
            while offset < size:
                data = os.pread(stream.fileno(), min(65536, size-offset), offset)
                assert data, 'regular stderr changed while hashing'
                hasher.update(data)
                offset += len(data)
            self.observe_stderr(item)
            assert item['stderr_bytes'] == size, 'regular stderr size changed after direct wait'
            stream.close()
            item['stderr_closed_at_ns'] = now()
            item['stderr_sha256'] = hasher.hexdigest()
        if item['wait'] is not None:
            original = item['wait']['original_cutoff_ns']
            item['wait'].update({'stderr_mode': 'regular_file', 'stderr_regular_path': item['stderr_path'],
                'stderr_regular_identity': item['stderr_identity'], 'stderr_regular_bytes': item['stderr_bytes'],
                'stderr_regular_file_limit': item['stderr_file_limit'], 'stderr_regular_overflow': item['stderr_overflow'],
                'stderr_regular_sha256': item['stderr_sha256'], 'stderr_regular_descriptor_closed': stream.closed,
                'stderr_regular_closed_at_ns': item['stderr_closed_at_ns'],
                'stderr_regular_closed_after_direct_wait': item['stderr_closed_at_ns'] >= item['wait']['waited_at_boottime_ns'],
                'stderr_regular_close_before_original_cutoff': original is not None and item['stderr_closed_at_ns'] < original,
                'stderr_regular_is_PIPE': False})

    def check(self):
        for item in self.children:
            self.observe_stderr(item)
        assert not any(v['overflow'] or v['stderr_overflow'] for v in self.children), 'bounded private output exceeded'

    def live(self, item):
        return item['child'].poll() is None and (item['pidfd'] is None or not select.select([item['pidfd']], [], [], 0)[0])

    def finish(self, item, cutoff, check=True, failure_cleanup=False):
        if item['wait'] is not None and not failure_cleanup:
            return item['wait']['returncode']
        original = cutoff
        if item['original_wait_cutoff_ns'] is not None:
            original = min(original, item['original_wait_cutoff_ns'])
        original = min(original, item.get('finish_cutoff_ns', original))
        item['finish_cutoff_ns'] = original
        if item['wait'] is not None:
            original = item['wait']['original_cutoff_ns']
        # Cleanup may obtain custody within its separately selected original
        # scope-close budget. It cannot renew command qualification or success.
        custody_until = cutoff if failure_cleanup else original
        if item['wait'] is None:
            while item['child'].poll() is None:
                assert now() < custody_until, item['name'] + ' original direct-wait cutoff'
                self.pump()
                if check:
                    self.check()
                time.sleep(.005)
            code = item['child'].wait(timeout=0)
            waited_at = now()  # Save the actual wait even if command timing failed.
            receipt = {'role': item['name'], 'pid': item['child'].pid, 'argv': item['argv'],
                'returncode': code, 'actual_direct_parent_wait': True, 'waited_at_boottime_ns': waited_at,
                'original_cutoff_ns': original, 'spawn_admission_cutoff_ns': item['spawn_admission_cutoff_ns'],
                'original_wait_cutoff_ns': item['original_wait_cutoff_ns'], 'wait_before_original_cutoff': waited_at < original}
            item['wait'] = receipt
            self.receipts.append(receipt)
        receipt = item['wait']
        while not item['eof'] and now() < custody_until:
            self.pump()
            time.sleep(.001)
        receipt.update({'stdin_pipe_present': item['child'].stdin is not None,
            'stdin_pipe_closed': item['child'].stdin is None or item['child'].stdin.closed,
            'stdout_bytes': item['bytes'], 'stdout_limit': item['limit'],
            'output_overflow': item['overflow'], 'stdout_EOF': item['eof'],
            'stdout_EOF_at_ns': item.get('eof_at_ns'), 'EOF_before_original_cutoff': bool(
                item['eof'] and item['eof_at_ns'] < original)})
        if failure_cleanup:
            receipt.update({'failure_cleanup_cutoff_ns': custody_until,
                'wait_before_failure_cleanup_cutoff': receipt['waited_at_boottime_ns'] < custody_until,
                'EOF_before_failure_cleanup_cutoff': bool(item['eof'] and item['eof_at_ns'] < custody_until)})
        assert receipt['waited_at_boottime_ns'] < custody_until, item['name'] + ' actual wait observed late'
        assert item['eof'] and item['eof_at_ns'] < custody_until, item['name'] + ' EOF observed late or absent'
        # The child has been actually waited and its PIPE drained before a
        # regular stderr file is hashed/closed, including failure cleanup.
        self.close_stderr(item)
        if failure_cleanup and item['stderr_file'] is not None:
            receipt['stderr_regular_close_before_failure_cleanup_cutoff'] = item['stderr_closed_at_ns'] < custody_until
            assert receipt['stderr_regular_close_before_failure_cleanup_cutoff'], item['name'] + ' regular stderr cleanup handle closed late'
        assert receipt['wait_before_original_cutoff'], item['name'] + ' actual wait observed late'
        assert receipt['EOF_before_original_cutoff'], item['name'] + ' EOF observed late or absent'
        if item['stderr_file'] is not None:
            assert receipt['stderr_regular_close_before_original_cutoff'], item['name'] + ' regular stderr handle closed late'
        if check:
            self.check()
        return receipt['returncode']

    def run(self, argv, name, cutoff, limit=1048576, stdin=None, env=None, identity=None, stderr_regular=None):
        assert now() < cutoff, name + ' original command admission cutoff'
        item = self.start(argv, name, limit, stdin=stdin, env=env, cutoff=cutoff, identity=identity, stderr_regular=stderr_regular)
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
                if item['wait'] is None or not item['eof']:
                    if item['wait'] is None and self.live(item):
                        if item['pidfd'] is not None:
                            signal.pidfd_send_signal(item['pidfd'], signal.SIGKILL)
                        else:
                            item['child'].kill()  # Exact retained Popen during partial setup.
                    self.finish(item, cutoff, check=False, failure_cleanup=True)
            except BaseException as error:
                errors.append(type(error).__name__ + ': ' + str(error))
            if item['wait'] is None or not item['eof']:
                # The original cleanup budget may also fail. Keep the exact
                # Popen, pidfd, PIPE, log and stderr handle under this Scope's
                # custody; returning an error is not process retirement.
                errors.append(item['name'] + ' cleanup incomplete: actual wait/PIPE EOF unresolved; handles retained')
                continue
            try:
                self.close_stderr(item)
            except BaseException as error:
                errors.append(type(error).__name__ + ': ' + str(error))
                if item['stderr_file'] is not None:
                    item['stderr_file'].close()
            item['log'].close()
            item['child'].stdout.close()
            if item['pidfd'] is not None:
                os.close(item['pidfd'])
                item['pidfd'] = None
        return errors
