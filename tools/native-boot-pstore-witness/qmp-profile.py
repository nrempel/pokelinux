"""Fixed post-PowerOff QMP sequencing, used inside the prior owned runner.

The caller retains the authenticated QMP peer/pidfd, all requests/replies, raw
serial bytes, fixed45s cutoff, ROM preflight, pipes/waits and exact CID closure.
This object never starts a process, opens a device, or manufactures an event.
"""


class NativePostPowerOff:
    def __init__(self, case_id, observer, send, preflight):
        self.case_id = case_id
        self.observer = observer
        self.send = send
        self.preflight = preflight
        self.boots = observer.NATIVE_CASES[case_id][2]
        self.sent = set()
        self.done = set()
        self.records = []
        self.greeting = False
        self.initial_cont_completed = False
        self.reset_seen = False
        self.guest_shutdowns = 0
        self.quit_replied = False
        self.initial_status = None
        self.shutdown_status = None
        self.reset_status = None
        self.boot_blob_preflight = None

    def request(self, execute, identity, arguments=None):
        assert identity not in self.sent, 'no duplicate QMP request'
        self.send(execute, identity, arguments)
        self.sent.add(identity)

    def receive(self, message):
        self.records.append(message)
        assert len(self.records) <= 96
        if 'QMP' in message:
            assert not self.greeting
            self.greeting = True
            self.request('qmp_capabilities', 'caps')
            return
        if 'event' in message:
            if message['event'] == 'SHUTDOWN':
                if message.get('data', {}).get('guest') is True:
                    assert message['data'] == {'guest': True, 'reason': 'guest-shutdown'}
                    self.guest_shutdowns += 1
                    assert self.guest_shutdowns <= self.boots
                else:
                    assert 'quit' in self.sent
                    assert message['data'] == {'guest': False, 'reason': 'host-qmp-quit'}
            elif message['event'] == 'RESET':
                assert self.boots == 2 and 'reset' in self.sent and not self.reset_seen
                assert message['data'] == {'guest': False, 'reason': 'host-qmp-system-reset'}
                self.reset_seen = True
            # STOP and RESUME, if present, are retained actual QEMU facts.
            return
        identity = message.get('id')
        assert identity in self.sent and identity not in self.done
        assert 'return' in message and 'error' not in message
        if identity not in ('status', 'status-shutdown', 'status-reset', 'roms'):
            assert message == {'return': {}, 'id': identity}
        self.done.add(identity)
        if identity == 'caps':
            self.request('query-status', 'status')
        elif identity == 'status':
            self.initial_status = message['return']
            assert self.initial_status['running'] is False
            self.request('human-monitor-command', 'roms', {'command-line': 'info roms'})
        elif identity == 'roms':
            self.boot_blob_preflight = self.preflight(message['return'])
            assert self.boot_blob_preflight['passed'] is True
            self.request('cont', 'cont-first')
        elif identity == 'cont-first':
            self.initial_cont_completed = True
        elif identity == 'status-shutdown':
            self.shutdown_status = message['return']
            assert self.shutdown_status['running'] is False and self.shutdown_status['status'] == 'shutdown'
            self.request('system_reset', 'reset')
        elif identity == 'status-reset':
            self.reset_status = message['return']
            assert self.reset_status['running'] is False
            assert isinstance(self.reset_status['status'], str)
            self.request('cont', 'cont-second')
        elif identity == 'quit':
            self.quit_replied = True

    def console_complete(self, actual_console, boots):
        # QMP and serial are separate actual channels. Await bounded arrival,
        # never infer a missing phase from an early SHUTDOWN notification.
        assert len(actual_console.encode()) <= 1024 * 1024
        lines = actual_console.replace('\r\n', '\n').splitlines()
        counts = [sum(line.startswith(prefix) for line in lines) for prefix in (
            'POKELINUX_DIAGNOSTIC_ACK ', 'POKELINUX_CHILD_EXEC ',
            'POKELINUX_NATIVE_PSTORE_PHASE ')]
        expected = [4 * boots, boots, boots]
        assert all(value <= limit for value, limit in zip(counts, expected))
        return counts == expected and actual_console.count('reboot: Power down') == boots

    def drive(self, actual_console):
        """Called after draining actual QMP and serial; no sleep or new deadline."""
        if self.boots == 2 and self.guest_shutdowns >= 1 and 'status-shutdown' not in self.sent:
            assert 'cont-first' in self.done
            if not self.console_complete(actual_console, 1):
                return
            self.observer.inspect_native_boots(actual_console, self.case_id, complete=False)
            self.request('query-status', 'status-shutdown')
        if self.reset_seen and 'reset' in self.done and 'status-reset' not in self.sent:
            self.request('query-status', 'status-reset')
        if self.guest_shutdowns == self.boots and 'quit' not in self.sent:
            final_cont = 'cont-second' if self.boots == 2 else 'cont-first'
            assert final_cont in self.done
            # Missing PMSG is deliberately allowed here; all ordinary facts must pass.
            if not self.console_complete(actual_console, self.boots):
                return
            self.observer.inspect_native_boots(actual_console, self.case_id)
            self.request('quit', 'quit')
