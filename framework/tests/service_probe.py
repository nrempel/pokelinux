#!/usr/bin/env python3
"""Exercise the production background service against the enrolled emulator."""
import hashlib
import json
import subprocess
import sys
import os
from pathlib import Path
import socket
import tempfile
import time
import uuid
from omarchy_bridge import Client
from phone_ipc import send
from phone_service import Observation, LocalService
from phone_dispatch import Dispatcher
from phone_ledger import Ledger
from phone_local import snapshot


def probe(config, runtime_parent=None, action='answer'):
    assert action in ('answer', 'reject')
    client = Client(config)
    transmissions = []
    class CountedClient:
        # Count transmissions without replacing the real TLS/Android adapter.
        def controls(self): return client.controls()
        def dial_status(self): return client.dial_status()
        def control(self, **values):
            transmissions.append(values['operation_id'])
            return client.control(**values)
    counted = CountedClient()
    with tempfile.TemporaryDirectory(prefix='service-', dir=runtime_parent) as directory:
        ledger = Ledger(Path(directory) / 'state')
        observation = Observation(lambda: counted, interval=.1)
        dispatcher = Dispatcher(observation, ledger, lambda: counted)
        authority = dispatcher.begin_session()
        service = LocalService(Path(directory) / 'runtime', observation, dispatcher)
        try:
            service.start()
            def local_cli(operation, identity, command=None):
                executable = Path(__file__).resolve().parents[2] / 'transport/phone_local.py'
                argv = [sys.executable, str(executable), '--socket', str(service.path), operation,
                        '--epoch', ledger.epoch, '--id', identity]
                if command is not None:
                    argv.extend(['--session', authority['session']])
                    for flag, key in (('--generation', 'generation'), ('--call-id', 'call_id'),
                                      ('--revision', 'expected_revision'), ('--state', 'expected_state'),
                                      ('--action', 'action')):
                        argv.extend([flag, str(command[key])])
                process = subprocess.run(argv, capture_output=True, text=True, timeout=5)
                assert process.returncode == 0, process.stderr
                value = json.loads(process.stdout)
                assert value['available'] and value['epoch'] == ledger.epoch and value['id'] == identity
                return value['operation']
            def observed(predicate):
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline:
                    state = snapshot(service.path)
                    if state['controls']['fresh'] and predicate(state['controls']['value']):
                        return state
                    time.sleep(.1)
                raise AssertionError('Expected Android call state was not observed')
            first = observed(lambda value: value['calls'] and value['calls'][0]['state'] == 2)
            # No local UI/client connection remains open during this wait.
            deadline = time.monotonic() + 10
            while observation.snapshot()['sequence'] <= first['sequence']:
                assert time.monotonic() < deadline
                time.sleep(.1)
            second = observed(lambda value: value['calls'] and value['calls'][0]['state'] == 2)
            assert second['epoch'] == first['epoch']
            assert second['sequence'] > first['sequence']
            def command(state, selected_action):
                value = state['controls']['value']
                call = value['calls'][0]
                return dict(generation=value['generation'], call_id=call['id'],
                            expected_revision=value['revision'], expected_state=call['state'], action=selected_action)
            identity = uuid.uuid4().hex
            submitted = command(second, action)
            with socket.socket(socket.AF_UNIX) as connection:
                connection.settimeout(2)
                connection.connect(str(service.path))
                send(connection, dict(v=1,id=identity,epoch=ledger.epoch,session=authority['session'],op='control',command=submitted))
                # Close before reading the outcome. Service owns the admitted work.
            deadline = time.monotonic() + 10
            while True:
                receipt = local_cli('result', identity)
                if receipt and receipt['state'] != 'pending': break
                assert time.monotonic() < deadline, receipt
                time.sleep(.1)
            assert receipt == {'state':'finished','result':{'outcome':'submitted','reason':'accepted'}}, receipt
            assert local_cli('control', identity, submitted) == receipt
            assert transmissions == [identity], transmissions
            result = {'passed': True, 'uid': os.getuid(), 'incoming_state': 2,
                      'observation_without_clients': True, 'native_cli_commands': True, 'duplicate_not_redispatched': True,
                      'action': action, 'command_after_local_disconnect': True}
            if action == 'answer':
                active = observed(lambda value: value['calls'] and value['calls'][0]['state'] == 4)
                result.update(answer_after_local_disconnect=True, active_observed=True)
                end = local_cli('control', uuid.uuid4().hex, command(active, 'disconnect'))
                assert end == {'state':'finished','result':{'outcome':'submitted','reason':'accepted'}}, end
                observed(lambda value: not value['calls'])
                result['disconnect_observed_idle'] = True
                assert len(transmissions) == 2
            else:
                observed(lambda value: not value['calls'])
                result['decline_observed_idle'] = True
            result['sources'] = {name: hashlib.sha256((Path(__file__).resolve().parents[2] / 'transport' / name).read_bytes()).hexdigest()
                                 for name in ('phone_service.py','phone_dispatch.py','phone_ledger.py','phone_local.py','phone_ipc.py','phone_model.py','omarchy_bridge.py')}
            return result
        finally:
            service.close()
            ledger.close()
