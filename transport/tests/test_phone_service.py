import os
import socket
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from phone_service import Observation, LocalService
from phone_ipc import send, receive
from phone_local import snapshot as local_snapshot
from test_control_client import snapshot
from test_dial_client import status


class FakeAndroid:
    def __init__(self):
        self.count = 0
    def controls(self):
        self.count += 1
        return snapshot()
    def dial_status(self):
        return status()


class ObservationTests(unittest.TestCase):
    def setUp(self):
        self.now = 10
        self.client = FakeAndroid()
        self.observation = Observation(lambda: self.client, clock=lambda: self.now)
    def test_observation_expires_without_becoming_idle(self):
        self.observation.poll_once('controls')
        self.assertTrue(self.observation.snapshot()['controls']['fresh'])
        self.now = 14
        value = self.observation.snapshot()['controls']
        self.assertFalse(value['fresh'])
        self.assertTrue(value['value']['calls'])
    def test_delayed_response_is_already_stale(self):
        def slow():
            self.now = 15
            return snapshot()
        self.client.controls = slow
        self.observation.poll_once('controls')
        self.assertFalse(self.observation.snapshot()['controls']['fresh'])
    def test_failure_does_not_refresh_cached_state(self):
        self.observation.poll_once('controls')
        def fail():
            raise OSError('Disconnected')
        self.client.controls = fail
        self.observation.poll_once('controls')
        value = self.observation.snapshot()['controls']
        self.assertFalse(value['fresh'])
        self.assertIsNotNone(value['value'])
    def test_snapshots_are_detached_and_lanes_independent(self):
        self.observation.poll_once('dial_status')
        value = self.observation.snapshot()
        self.assertFalse(value['controls']['fresh'])
        self.assertTrue(value['dial_status']['fresh'])
        value['dial_status']['value']['can_dial'] = False
        self.assertTrue(self.observation.snapshot()['dial_status']['value']['can_dial'])


@unittest.skipUnless(hasattr(socket, 'SO_PEERCRED'), 'Linux service peer authentication')
class LocalServiceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.client = FakeAndroid()
        self.observation = Observation(lambda: self.client, interval=.02)
        self.service = LocalService(Path(self.directory.name) / 'runtime', self.observation)
        self.service.start()
        self.addCleanup(self.service.close)
    def request(self, request):
        with socket.socket(socket.AF_UNIX) as connection:
            connection.settimeout(2)
            connection.connect(str(self.service.path))
            send(connection, request)
            return receive(connection)
    def test_polling_survives_no_clients_and_disconnection(self):
        time.sleep(.08)
        first = self.client.count
        self.assertGreater(first, 0)
        value = self.request({'v': 1, 'id': 'a' * 32, 'op': 'snapshot'})
        self.assertTrue(value['snapshot']['controls']['fresh'])
        time.sleep(.08)
        self.assertGreater(self.client.count, first)
        again = self.request({'v': 1, 'id': 'b' * 32, 'op': 'snapshot'})
        self.assertEqual(value['snapshot']['epoch'], again['snapshot']['epoch'])
    def test_commands_and_extra_fields_rejected(self):
        for request in ({'v': 1, 'id': 'a' * 32, 'op': 'dial'},
                        {'v': 1, 'id': 'a' * 32, 'op': 'snapshot', 'extra': 1}):
            self.assertEqual(self.request(request)['error'], 'invalid_request')
    def test_production_local_client_reads_running_service(self):
        deadline = time.monotonic() + 2
        while not self.observation.snapshot()['controls']['fresh']:
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.01)
        value = local_snapshot(self.service.path)
        self.assertEqual(value['epoch'], self.observation.epoch)
        self.assertTrue(value['controls']['fresh'])
    def test_exclusive_owner_and_private_socket(self):
        with self.assertRaises(ValueError):
            LocalService(self.service.path.parent, self.observation)
        self.assertEqual(self.service.path.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.service.path.parent.stat().st_mode & 0o777, 0o700)

if __name__ == '__main__':
    unittest.main()

class ObservationBarrierTests(unittest.TestCase):
    def test_command_invalidation_discards_inflight_poll(self):
        started, release = threading.Event(), threading.Event()
        class DelayedAndroid(FakeAndroid):
            def controls(self):
                started.set()
                if not release.wait(2):
                    raise OSError('test deadline')
                return snapshot()
        observation = Observation(DelayedAndroid)
        thread = threading.Thread(target=observation.poll_once, args=('controls',))
        thread.start()
        try:
            self.assertTrue(started.wait(1))
            observation.invalidate('controls')
        finally:
            release.set()
            thread.join(2)
        self.assertFalse(thread.is_alive())
        self.assertFalse(observation.snapshot()['controls']['fresh'])
        observation.poll_once('controls')
        self.assertTrue(observation.snapshot()['controls']['fresh'])


@unittest.skipUnless(hasattr(socket, 'SO_PEERCRED'), 'Linux service peer authentication')
class LocalCommandTests(unittest.TestCase):
    def test_socket_command_disconnect_duplicate_and_readonly_result(self):
        from phone_dispatch import Dispatcher
        from phone_ledger import Ledger
        from test_phone_dispatch import Backend
        with tempfile.TemporaryDirectory() as directory:
            ledger = Ledger(Path(directory) / 'state')
            self.addCleanup(ledger.close)
            backend = Backend()
            backend.dial_status = status
            observation = Observation(lambda: backend, interval=.02)
            dispatcher = Dispatcher(observation, ledger, lambda: backend)
            authority = dispatcher.begin_session()
            service = LocalService(Path(directory) / 'runtime', observation, dispatcher=dispatcher)
            try:
                service.start()
                deadline = time.monotonic() + 2
                while not observation.snapshot()['controls']['fresh']:
                    self.assertLess(time.monotonic(), deadline)
                    time.sleep(.01)
                command = dict(v=1,id='a'*32,epoch=ledger.epoch,session=authority['session'],op='control',command=dict(
                    generation='b'*32,call_id=7,expected_revision=3,expected_state=2,action='answer'))
                def request(value, disconnect=False):
                    with socket.socket(socket.AF_UNIX) as connection:
                        connection.settimeout(2)
                        connection.connect(str(service.path))
                        send(connection, value)
                        if not disconnect:
                            return receive(connection)
                request(command, disconnect=True)
                deadline = time.monotonic() + 2
                while ledger.result(ledger.epoch, 'a'*32) != {'state':'finished','result':{'outcome':'submitted','reason':'accepted'}}:
                    self.assertLess(time.monotonic(), deadline)
                    time.sleep(.01)
                result = request(dict(v=1,id='a'*32,epoch=ledger.epoch,op='result'))
                self.assertEqual(result['operation']['state'], 'finished')
                self.assertEqual(request(command), result)
                from phone_local import control, operation_result
                self.assertEqual(operation_result(service.path, ledger.epoch, 'a'*32), result['operation'])
                self.assertEqual(control(service.path, ledger.epoch, 'a'*32, command['command'], session=authority['session']), result['operation'])
                self.assertEqual(len(backend.calls), 1)
                command['command']['action'] = 'reject'
                self.assertEqual(request(command)['error'], 'command_rejected')
                self.assertEqual(len(backend.calls), 1)
                absent = request(dict(v=1,id='c'*32,epoch=ledger.epoch,op='result'))
                self.assertIsNone(absent['operation'])
            finally:
                service.close()
