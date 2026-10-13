#!/usr/bin/env python3
"""Real storage child plus explicit test-only reap error; not an unkillable syscall."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--mode', choices=['error', 'withheld'], required=True)
parser.add_argument('--binary-root', type=Path, required=True)
parser.add_argument('--results-dir', type=Path, required=True)
args = parser.parse_args()
assert os.geteuid() == 0 and os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1'
libc = ctypes.CDLL(None, use_errno=True)
libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
libc.prctl.restype = ctypes.c_int
assert libc.prctl(36, 1, 0, 0, 0) == 0, ctypes.get_errno()  # PR_SET_CHILD_SUBREAPER
args.results_dir.mkdir(parents=True, exist_ok=True)
qualifier = 'lifecycle-storage-cleanup-' + args.mode
install = Path('/opt') / qualifier
runtime = Path('/run') / qualifier
policy = Path('/var/lib') / qualifier
for path, mode in [(install, 0o755), (runtime, 0o700), (policy, 0o700)]:
    path.mkdir(); path.chmod(mode)
executable = install / 'lifecycle-daemon'
shutil.copyfile(args.binary_root / 'lifecycle-daemon', executable)
executable.chmod(0o755)
owner = subprocess.Popen([str(executable), '--isolated-storage-reap-error', str(runtime), str(policy), args.mode], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
handle = None
result = {'passed': False, 'mode': args.mode, 'mechanism': 'real child process and injected wait/reap '+args.mode+'; no unkillable filesystem syscall', 'kernel': os.uname().release, 'machine': os.uname().machine, 'daemon_sha256': hashlib.sha256(executable.read_bytes()).hexdigest()}
try:
    assert select.select([owner.stdout], [], [], 2)[0], 'owner did not establish actual storage child'
    line = owner.stdout.readline().strip()
    assert line.startswith('owned-child:'), line
    pid = int(line.split(':', 1)[1])
    handle = os.pidfd_open(pid)
    start = time.monotonic()
    try:
        owner.wait(timeout=.7)
    except subprocess.TimeoutExpired as exc:
        raise AssertionError('final cleanup held by wait/reap '+args.mode+' beyond fixed budget') from exc
    elapsed = time.monotonic() - start
    assert owner.returncode == -signal.SIGABRT, (owner.returncode, owner.stderr.read())
    assert elapsed < .7
    assert select.select([handle], [], [], 1)[0], 'owned child not terminated'
    # This fixture adopts/reaps the actual child after the owner intentionally aborts.
    until = time.monotonic() + 1
    reaped = False
    while time.monotonic() < until:
        returned, _ = os.waitpid(pid, os.WNOHANG)
        if returned == pid:
            reaped = True
            break
        time.sleep(.005)
    assert reaped and not Path(f'/proc/{pid}').exists(), 'fixture subreaper did not reclaim actual child'
    name = 'bounded_final_cleanup_reap_'+args.mode+'_aborts_without_dropping_owned_child'
    result.update(passed=True, groups=[name], cleanup_elapsed_seconds=elapsed)
    print('PASS '+name, flush=True)
finally:
    if owner.poll() is None:
        owner.kill()
    owner.communicate(timeout=2)
    if handle is not None:
        until = time.monotonic() + 2
        while True:
            try:
                returned, _ = os.waitpid(pid, os.WNOHANG)
                if returned == pid:
                    break
            except ChildProcessError:
                break
            assert time.monotonic() < until, 'fixture cleanup could not reap actual adopted child'
            time.sleep(.005)
        os.close(handle)
    (args.results_dir / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
