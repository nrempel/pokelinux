#!/usr/bin/env python3
"""Run the hash-bound process ELF at its actual compiled helper paths."""
import argparse
import ctypes
import hashlib
import json
import os
import platform
import re
import signal
import subprocess
import time
import traceback
from pathlib import Path


def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--results', type=Path, default=Path('/results'))
    args = parser.parse_args()
    out = args.results
    selection = json.loads((out / 'selection.json').read_text())
    artifacts = selection['artifacts']
    result = {'completed': False, 'passed': False, 'phase': selection['phase'],
              'architecture': platform.machine(), 'uid': os.getuid(), 'kernel': platform.release(),
              'test_names': {}, 'owned_processes_observed': {}, 'orphan_waits': [],
              'remaining_owned_helpers': [], 'scope': selection['scope']}
    child = None
    observed = {}
    try:
        assert os.getuid() == 0, 'This narrow process fixture must execute in isolated Root'
        assert platform.machine() == ('aarch64' if selection['arch'] == 'arm' else 'x86_64')
        # Isolated runtime only: adopted descendants remain owned by this runner on an abort.
        libc = ctypes.CDLL(None, use_errno=True)
        assert libc.prctl(36, 1, 0, 0, 0) == 0  # PR_SET_CHILD_SUBREAPER
        for value in artifacts.values():
            assert Path(value['compiled_path']).is_file() and sha(value['compiled_path']) == value['sha256']
        command = [artifacts['process']['compiled_path'], '--test-threads=1', '--show-output']
        if selection['test']:
            command += [selection['test'], '--exact']
        (out / 'test-command.json').write_text(json.dumps(command, indent=2) + '\n')
        result['start_boottime_ns'] = now()
        with (out / 'test.log').open('w') as log:
            child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            result['test_pid'] = child.pid
            until = now() + 120_000_000_000
            while child.poll() is None and now() < until:
                for directory in Path('/proc').iterdir():
                    if not directory.name.isdecimal():
                        continue
                    pid = int(directory.name)
                    try:
                        if os.getsid(pid) != child.pid:
                            continue
                        executable = os.readlink(directory / 'exe')
                        role = next((name for name, value in artifacts.items() if value['compiled_path'] == executable), None)
                        if role:
                            observed.setdefault(pid, {'role': role, 'first_seen_boottime_ns': now()})
                    except (FileNotFoundError, ProcessLookupError, PermissionError):
                        pass
                time.sleep(.001)
            if child.poll() is None:
                result['runner_deadline_expired'] = True
                os.killpg(child.pid, signal.SIGKILL)
            result['test_exit_code'] = child.wait(timeout=5)
        text = (out / 'test.log').read_text()
        result['finish_boottime_ns'] = now()
        for name, status in re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', text, re.MULTILINE):
            result['test_names'][name] = status
        summary = re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', text)
        assert summary, 'No completed libtest summary; infrastructure failure is not a desired boundary red'
        result['counts'] = dict(zip(('passed', 'failed', 'ignored', 'measured', 'filtered'), map(int, summary.groups()[1:])))
        assert len(result['test_names']) == result['counts']['passed'] + result['counts']['failed']
        result['completed'] = True
        result['passed'] = result['test_exit_code'] == 0 and result['counts']['failed'] == 0 and result['counts']['passed'] > 0
    except BaseException:
        result['error'] = traceback.format_exc()
    finally:
        if child is not None:
            # Every process in this newly owned session is ours, including failed-test descendants.
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait(timeout=5)
            until = now() + 5_000_000_000
            while now() < until:
                try:
                    pid, status = os.waitpid(-1, os.WNOHANG)
                except ChildProcessError:
                    break
                if not pid:
                    time.sleep(.001)
                    continue
                result['orphan_waits'].append({'pid': pid, 'status': status,
                                             'returncode': os.waitstatus_to_exitcode(status), 'wait_boottime_ns': now()})
            for pid, value in observed.items():
                value['absent_after_cleanup'] = not Path(f'/proc/{pid}').exists()
                if not value['absent_after_cleanup']:
                    result['remaining_owned_helpers'].append(pid)
        result['owned_processes_observed'] = {str(pid): value for pid, value in observed.items()}
        result['all_observed_helpers_absent_after_cleanup'] = not result['remaining_owned_helpers']
        result['cleanup_evidence_limit'] = 'The Rust process harness owns and waits its children; /proc sampling proves final absence only for sampled helpers. Recorded orphan waits are actual runner waits, not original test-parent waits.'
        (out / 'result.json').write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
        print(json.dumps(result, indent=2, sort_keys=True), flush=True)
    return 0 if result['completed'] and result['all_observed_helpers_absent_after_cleanup'] else 2


if __name__ == '__main__':
    raise SystemExit(main())
