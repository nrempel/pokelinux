"""Serialized local command admission. Android remains the final authority."""
import threading
import uuid
import re
from omarchy_bridge import control_request, validate_control, dial_request, validate_dial


class Dispatcher:
    def __init__(self, observation, ledger, client_factory):
        self.observation = observation
        self.ledger = ledger
        self.client_factory = client_factory
        self.lock = threading.Lock()
        self.pending = False
        self.session = None
        # Published snapshots and commands use one service-incarnation identity.
        self.observation.epoch = ledger.epoch

    def begin_session(self):
        """Fence future admission from old UI sessions, not admitted operations."""
        with self.lock:
            self.session = uuid.uuid4().hex
            self.observation.invalidate()
            return {'epoch': self.ledger.epoch, 'session': self.session,
                    'recovery': self.ledger.unresolved()}

    def execute(self, request):
        if (not isinstance(request, dict) or set(request) != {'v','id','epoch','op','command','session'}
                or type(request['v']) is not int or request['v'] != 1 or request['op'] not in ('control','dial')
                or request['epoch'] != self.ledger.epoch):
            raise ValueError('Invalid command envelope')
        command = request['command']
        dialing = request['op'] == 'dial'
        fields = {'generation','offer','number'} if dialing else {'generation','call_id','expected_revision','expected_state','action'}
        if not isinstance(command, dict) or set(command) != fields:
            raise ValueError('Invalid command')
        payload = (dial_request if dialing else control_request)(request['id'], **command)
        epoch, identity = request['epoch'], request['id']
        with self.lock:
            if self.session is None or request['session'] != self.session:
                raise ValueError('Expired UI session')
            previous = self.ledger.result(epoch, identity)
            if previous is not None:
                # Verify immutable payload even if current authority has expired.
                self.ledger.reserve(epoch, identity, payload)
                return previous
            if self.pending:
                raise ValueError('Control request pending')
            if self.ledger.unresolved()['items']:
                raise ValueError('Command recovery required')
            lane = self.observation.snapshot()['dial_status' if dialing else 'controls']
            value = lane['value']
            if not lane['fresh'] or not value or not value['can_dial' if dialing else 'controls_allowed']:
                raise ValueError('Fresh command authority required')
            if dialing:
                # Polling may issue another still-valid offer. Preserve the user's
                # original token; Android validates its account, revision and expiry.
                if command['generation'] != value['generation']:
                    raise ValueError('Dial generation changed')
            else:
                if command['generation'] != value['generation'] or command['expected_revision'] != value['revision']:
                    raise ValueError('Control state changed')
                call = next((v for v in value['calls'] if v['id'] == command['call_id']), None)
                if call is None or call['state'] != command['expected_state'] or command['action'] not in call['actions']:
                    raise ValueError('Call action unavailable')
            self.ledger.reserve(epoch, identity, payload)
            self.pending = True
            self.observation.invalidate()
        try:
            try:
                client = self.client_factory()
                response = (client.dial if dialing else client.control)(**command, operation_id=identity)
                (validate_dial if dialing else validate_control)(response, identity, command['generation'])
                outcome = {'outcome': 'unknown' if response['outcome'] == 'outcome_unknown' else response['outcome'],
                           'reason': response['reason']}
            except (OSError, ValueError):
                outcome = None  # A lost response is not permission to retry.
            self.ledger.finish(epoch, identity, outcome)
            return self.ledger.result(epoch, identity)
        finally:
            with self.lock:
                self.observation.invalidate()
                self.pending = False

    def acknowledge(self, request):
        if (not isinstance(request, dict)
                or set(request) != {'v','id','op','epoch','session','record_epoch','record_id'}
                or type(request['v']) is not int or request['v'] != 1
                or request['op'] != 'acknowledge'
                or any(not isinstance(request[k], str) or not re.fullmatch('[0-9a-f]{32}', request[k])
                       for k in ('id','epoch','session','record_epoch','record_id'))):
            raise ValueError('Invalid acknowledgment envelope')
        with self.lock:
            if request['epoch'] != self.ledger.epoch or request['session'] != self.session:
                raise ValueError('Expired UI session')
            if self.pending:
                raise ValueError('Control request pending')
            acknowledged = self.ledger.acknowledge(request['record_epoch'], request['record_id'])
            # Reject observations started before this bookkeeping transition.
            self.observation.invalidate()
            return acknowledged
