#!/usr/bin/env python3
"""Two post-ready server boundary probes; no QEMU, guest, capture, or input."""
import json
import os
from pathlib import Path
import select
import signal
import socket
import struct
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, '/results/source')
from owned import Scope, now
from human_viewer_host import HumanViewerOwner
from viewer_protocol import read


R = Path('/results')
mode = sys.argv[1]
assert mode in ('no-client', 'early-disconnect')
started = now()
work = started + 80_000_000_000
close = started + 90_000_000_000
scope = Scope(R)
result = {'schema': 1, 'mode': mode, 'passed': False,
          'scope': 'Actual post-ready x11vnc lifecycle only',
          'synthetic_post_ready_prerequisites': True,
          'native_window_or_QMP_or_guest_qualified': False,
          'human_connection_workflow_qualified': False,
          'VM_executed': False, 'input_messages_sent': 0,
          'started_host_ns': started, 'original_work_until_ns': work,
          'original_close_until_ns': close}
peer = None
try:
    assert os.getuid() == os.getgid() == 1000
    xvfb = scope.start(['Xvfb', ':97', '-screen', '0', '1080x2424x24',
                        '-dpi', '100', '-nolisten', 'tcp'], 'Xvfb.log', 65536,
                       cutoff=work, wait_cutoff=close)
    until = min(work, now() + 10_000_000_000)
    while not Path('/tmp/.X11-unix/X97').is_socket():
        scope.pump(); scope.check()
        assert scope.live(xvfb) and now() < until
        select.select([], [], [], .005)
    selection = json.loads((R / 'selection.json').read_bytes())
    viewer = HumanViewerOwner(scope, R, selection, xvfb, xvfb, work, close,
                              lambda: None, lambda: [])
    # This fixture deliberately bypasses prepare(). Its actual /usr/bin/true
    # wait is only a completed setup sentinel, never a native-window receipt.
    setup = scope.start(['/usr/bin/true'], 'synthetic-ready-sentinel.log', 1024,
                        cutoff=work, wait_cutoff=close)
    assert scope.finish(setup, close) == 0
    viewer.setup = setup
    viewer.host_ready = True
    viewer.record['synthetic_post_ready_prerequisites'] = True
    before = now()
    viewer.start_server({'schema': 1, 'selection_sha256': viewer.sha,
                         'requested_dwell_ns': 75_000_000_000,
                         'guest_until_ns': 1})
    assert before <= viewer.dwell_until - 75_000_000_000 <= now()
    assert viewer.no_client_until < viewer.dwell_until <= work
    assert viewer.record['guest_absolute_clock_compared_to_host'] is False
    assert viewer.record['guest_until_ns_recorded_local_only'] == 1
    if mode == 'early-disconnect':
        peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        peer.settimeout(3)
        peer.connect(str(R / 'viewer.sock'))

        def receive(count):
            data = bytearray()
            while len(data) < count:
                scope.pump(); scope.check()
                part = peer.recv(count - len(data))
                assert part
                data.extend(part)
            return bytes(data)

        banner = receive(12)
        assert banner == b'RFB 003.008\n'
        peer.sendall(banner)
        count = receive(1)[0]
        assert 0 < count <= 16 and 1 in receive(count)
        peer.sendall(b'\x01')
        assert receive(4) == b'\x00\x00\x00\x00'
        peer.sendall(b'\x00')
        init = receive(24)
        assert struct.unpack('>HH', init[:4]) == (1080, 2424)
        name_count = struct.unpack('>I', init[20:])[0]
        assert name_count <= 4096
        receive(name_count)
        peer.close(); peer = None
        result['test_peer_completed_RFB_init_then_closed_host_ns'] = now()
    while not viewer.retired:
        viewer.pump()
        assert now() < work
        select.select([], [], [], .005)
    retired = read(viewer.directory / 'retired.json')
    assert retired['no_future_host_writes'] is True
    assert retired['remote_event_queues_drained_claim'] is False
    assert retired['server_actual_wait'] == viewer.server['wait']
    assert viewer.passed and retired['operational_success'] is True
    if mode == 'no-client':
        assert viewer.ending == 'no-client-observed-timeout'
        assert not viewer.client_observed and viewer.stop_requested
        assert viewer.record['owned_server_stop_requested_host_ns'] >= viewer.no_client_until
        assert now() < viewer.dwell_until
    else:
        assert viewer.ending == 'client-disconnected'
        assert viewer.client_observed and not viewer.stop_requested
        assert now() < viewer.no_client_until
        assert viewer.server['wait']['returncode'] == 0
    viewer.tcp('test-after-server-actual-wait-and-EOF')
    result['actual_production_owner_record'] = viewer.record
    result['actual_retired_marker'] = retired
    signal.pidfd_send_signal(xvfb['pidfd'], signal.SIGTERM)
    assert scope.finish(xvfb, close) == 0
except BaseException as error:
    result['failure'] = type(error).__name__ + ': ' + str(error)
finally:
    if peer is not None:
        peer.close()
    errors = scope.close(close)
    result.update(cleanup_errors=errors, owned_process_receipts=scope.receipts,
                  completed_host_ns=now())
    result['passed'] = ('failure' not in result and not errors and
                        len(scope.receipts) == 3 and
                        all(row['actual_direct_parent_wait'] and row['stdout_EOF'] and
                            row['wait_before_original_cutoff'] and
                            row['EOF_before_original_cutoff'] and
                            not row['output_overflow'] for row in scope.receipts) and
                        now() < close)
    (R / 'boundary-result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'mode': mode, 'passed': result['passed'],
                      'failure': result.get('failure')}), flush=True)
if not result['passed']:
    raise SystemExit(1)
