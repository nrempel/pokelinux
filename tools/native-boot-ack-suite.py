#!/usr/bin/env python3
"""Actual ACK producer/write-boundary test; no kmsg, VM or device execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import socket
import stat
import subprocess
import time

CASE_NS = 5_000_000_000
CLEANUP_NS = 2_000_000_000
PACKET_MAX = 1024
PACKET_COUNT_MAX = 16
STDERR_MAX = 8192
# Independently authored complete records, not parsed from the fixture source.
EXPECTED = {
    'kernel': (
        b'POKELINUX_DIAGNOSTIC_ACK epoch=0123456789abcdef0123456789abcdef '
        b'event=2 kernel_seq=313 POKELINUX_PROBE_KERNEL '
        b'release=7.1.6-1-aarch64-ARCH\n'),
    'complete': (
        b'POKELINUX_DIAGNOSTIC_ACK epoch=0123456789abcdef0123456789abcdef '
        b'event=4 kernel_seq=315 POKELINUX_PROBE_COMPLETE '
        b'persistent_storage=false phone_services=false\n'),
}


def stamp():
    return time.monotonic_ns()


def save(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def digest(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= limit:
            raise ValueError('input is not a bounded nonempty regular file: ' + str(path))
        h = hashlib.sha256()
        total = 0
        while True:
            block = os.read(fd, 65536)
            if not block:
                break
            total += len(block)
            if total > limit:
                raise ValueError('input grew beyond bound: ' + str(path))
            h.update(block)
        after = os.fstat(fd)
        if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns,
                before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size,
                                      after.st_mtime_ns, after.st_ctime_ns):
            raise ValueError('input changed while hashing: ' + str(path))
        return {'path': str(path), 'bytes': total, 'sha256': h.hexdigest(),
                'dev': before.st_dev, 'ino': before.st_ino,
                'mode': stat.S_IMODE(before.st_mode)}
    finally:
        os.close(fd)


def inputs(fixture):
    root = Path(__file__).resolve().parents[1]
    crate = root / 'native-boot-probe'
    paths = [Path(__file__).resolve(), crate / 'Cargo.toml', crate / 'Cargo.lock']
    for directory in [crate / 'src', crate / 'examples']:
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError('required source directory unavailable')
        for path in sorted(directory.rglob('*')):
            if path.is_symlink():
                raise ValueError('symlink in selected source tree')
            if path.is_file():
                paths.append(path)
    if len(paths) > 64:
        raise ValueError('source inventory exceeds bound')
    rows = [digest(path, 1024 * 1024) for path in paths]
    rows.append(digest(fixture, 32 * 1024 * 1024))
    return sorted(rows, key=lambda row: row['path'])


def run_case(fixture, case, output):
    output.mkdir(mode=0o700)
    expected = EXPECTED[case]
    (output / 'expected.bin').write_bytes(expected)
    argv = [str(fixture), case]
    save(output / 'command.json', argv)
    started = stamp()
    until = started + CASE_NS
    cleanup_until = until + CLEANUP_NS
    row = {'case': case, 'started_monotonic_ns': started,
           'original_until_monotonic_ns': until,
           'original_cleanup_until_monotonic_ns': cleanup_until,
           'actual_direct_wait': False, 'socket_EOF': False,
           'stderr_EOF': False, 'errors': []}
    child = None
    receiver = sender = None
    selector = selectors.DefaultSelector()
    packets = []
    stderr = bytearray()
    prepared = False

    def observed_wait():
        if child is not None and not row['actual_direct_wait']:
            code = child.poll()
            if code is not None:
                row['exit'] = child.wait(timeout=0)
                row['actual_direct_wait'] = True
                row['waited_monotonic_ns'] = stamp()

    def collect(deadline):
        while stamp() < deadline:
            observed_wait()
            if row['actual_direct_wait'] and row['socket_EOF'] and row['stderr_EOF']:
                return
            for key, _ in selector.select(min(0.02, max(0, deadline - stamp()) / 1e9)):
                try:
                    if key.data == 'socket':
                        data, rights, flags, _ = receiver.recvmsg(PACKET_MAX, 0)
                        if rights or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC):
                            raise ValueError('truncated packet or unexpected ancillary data')
                        if not data:
                            row['socket_EOF'] = True
                            row['socket_EOF_monotonic_ns'] = stamp()
                            selector.unregister(receiver)
                        else:
                            if len(packets) == PACKET_COUNT_MAX:
                                raise ValueError('packet count exceeds bound')
                            packets.append(data)
                    else:
                        data = os.read(child.stderr.fileno(), 4096)
                        if not data:
                            row['stderr_EOF'] = True
                            row['stderr_EOF_monotonic_ns'] = stamp()
                            selector.unregister(child.stderr)
                        else:
                            if len(stderr) + len(data) > STDERR_MAX:
                                raise ValueError('stderr exceeds bound')
                            stderr.extend(data)
                except BlockingIOError:
                    pass
        raise TimeoutError('original observation cutoff expired')

    try:
        receiver, sender = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        if receiver.getsockopt(socket.SOL_SOCKET, socket.SO_TYPE) != socket.SOCK_SEQPACKET:
            raise ValueError('actual socket is not SEQPACKET')
        receiver.setblocking(False)
        if stamp() >= until:
            raise TimeoutError('pre-spawn original cutoff expired')
        # Retain this exact Popen immediately after successful creation.
        child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=sender,
                                 stderr=subprocess.PIPE, cwd=output, env={}, close_fds=True)
        row['pid'] = child.pid
        sender.close()
        sender = None
        os.set_blocking(child.stderr.fileno(), False)
        selector.register(receiver, selectors.EVENT_READ, 'socket')
        selector.register(child.stderr, selectors.EVENT_READ, 'stderr')
        prepared = True
        collect(until)
    except Exception as failure:
        row['errors'].append(type(failure).__name__ + ': ' + str(failure))
    finally:
        if sender is not None:
            sender.close()
        if child is not None:
            try:
                observed_wait()
                if not row['actual_direct_wait']:
                    child.kill()
                    row['cleanup_SIGKILL_attempted'] = True
                    child.wait(timeout=max(0, cleanup_until - stamp()) / 1e9)
                    row['exit'] = child.returncode
                    row['actual_direct_wait'] = True
                    row['waited_monotonic_ns'] = stamp()
                if prepared and not (row['socket_EOF'] and row['stderr_EOF']):
                    collect(cleanup_until)
            except Exception as failure:
                row['errors'].append('cleanup ' + type(failure).__name__ + ': ' + str(failure))
            finally:
                if child.stderr is not None:
                    child.stderr.close()
        selector.close()
        if receiver is not None:
            receiver.close()
    row['completed_monotonic_ns'] = stamp()
    row['packet_count'] = len(packets)
    row['packet_lengths'] = [len(packet) for packet in packets]
    row['whole_concatenation_matches_expected'] = b''.join(packets) == expected
    row['all_observations_before_original_cutoff'] = bool(
        row['actual_direct_wait'] and row['socket_EOF'] and row['stderr_EOF']
        and max(row['waited_monotonic_ns'], row['socket_EOF_monotonic_ns'],
                row['stderr_EOF_monotonic_ns'], row['completed_monotonic_ns']) < until)
    row['prerequisites_passed'] = bool(
        not row['errors'] and row.get('exit') == 0 and not stderr
        and row['all_observations_before_original_cutoff']
        and row['whole_concatenation_matches_expected'] and packets)
    row['single_whole_packet'] = row['prerequisites_passed'] and len(packets) == 1
    row['actual_split_observed'] = row['prerequisites_passed'] and len(packets) > 1
    save(output / 'packets.json', [{'bytes': len(data), 'hex': data.hex()} for data in packets])
    (output / 'stderr.bin').write_bytes(stderr)
    save(output / 'result.json', row)
    if child is not None and not row['actual_direct_wait']:
        # An occupied owner cannot continue as a normal completed suite.
        os.abort()
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', required=True, type=Path)
    parser.add_argument('--private-output-dir', required=True, type=Path)
    args = parser.parse_args()
    fixture = args.fixture.absolute()
    output = args.private_output_dir.absolute()
    output.mkdir(mode=0o700, exist_ok=False)
    summary = {'schema': 1, 'qualified_behavioral_RED': False, 'qualified_GREEN': False,
               'scope': 'actual host ACK producer/write boundary only', 'cases': [], 'errors': []}
    before = None
    try:
        before = inputs(fixture)
        save(output / 'inputs-before.json', before)
        if not os.access(fixture, os.X_OK):
            raise ValueError('fixture is not executable')
        # Both normal receipts are written before a boundary RED can be returned.
        for case in EXPECTED:
            summary['cases'].append(run_case(fixture, case, output / case))
    except Exception as failure:
        summary['errors'].append(type(failure).__name__ + ': ' + str(failure))
    try:
        after = inputs(fixture)
        save(output / 'inputs-after.json', after)
        summary['all_source_and_executable_inputs_unchanged'] = before is not None and after == before
    except Exception as failure:
        summary['errors'].append('post-input ' + type(failure).__name__ + ': ' + str(failure))
        summary['all_source_and_executable_inputs_unchanged'] = False
    normal = (not summary['errors'] and summary['all_source_and_executable_inputs_unchanged']
              and len(summary['cases']) == 2
              and all(case['prerequisites_passed'] for case in summary['cases']))
    if normal and all(case['single_whole_packet'] for case in summary['cases']):
        summary['qualified_GREEN'] = True
        summary['classification'] = 'single-complete-ACK-packet-in-both-cases'
        code = 0
    elif normal and any(case['actual_split_observed'] for case in summary['cases']):
        summary['qualified_behavioral_RED'] = True
        summary['classification'] = 'ACTUAL_ACK_SPLIT_ACROSS_WRITES'
        code = 1
    else:
        summary['classification'] = 'prerequisite-failure-not-behavioral-RED'
        code = 2
    summary['suite_exit'] = code
    save(output / 'summary.json', summary)
    print(json.dumps({'classification': summary['classification'], 'suite_exit': code}))
    return code


if __name__ == '__main__':
    raise SystemExit(main())
