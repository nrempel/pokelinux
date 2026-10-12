"""UI state for the native phone client. No sockets, GTK, or implicit retries."""
import copy
import json
import uuid
import time
from omarchy_bridge import validate_controls,validate_dial_status,canonical_number

def elapsed_time():
    # CLOCK_MONOTONIC excludes Linux suspend. BOOTTIME expires pre-sleep UI state.
    if hasattr(time,'CLOCK_BOOTTIME'):
        return time.clock_gettime(time.CLOCK_BOOTTIME)
    return time.monotonic()

class Unavailable(ValueError):
    pass

class PhoneModel:
    def __init__(self):
        self.snapshot=None
        self.last_update=None
        self.pending=False
        self.outcome=None
        self.reason=None
        self.dial_snapshot=None;self.dial_update=None;self.dial_pending=False
        self.dial_outcome=None;self.dial_reason=None

    @property
    def calls(self):
        return copy.deepcopy(self.snapshot['calls']) if self.snapshot else []

    def update(self,value,now):
        validate_controls(value,value.get('id'))
        self.snapshot=copy.deepcopy(value)
        self.last_update=now
        if not self.pending and self.outcome in ("submitted","rejected"):
            self.outcome=None;self.reason=None

    def observe(self,value,started,now):
        if not 0<=now-started<=3:
            self.disconnect();return
        self.update(value,started)

    def disconnect(self):
        self.dial_snapshot=None;self.dial_update=None
        self.snapshot=None
        self.last_update=None

    def fresh(self,now):
        return self.snapshot is not None and self.last_update is not None and 0<=now-self.last_update<=3

    def view_key(self,now):
        # A transport correlation ID is not a UI change. Preserve touch targets
        # across unchanged polls, including between touch-down and touch-up.
        view={key:value for key,value in self.snapshot.items() if key!='id'} if self.snapshot else None
        return json.dumps([view,self.fresh(now),self.pending,self.outcome,self.reason],sort_keys=True)

    def begin(self,generation,revision,call_id,action,now):
        if self.pending or not self.fresh(now):raise Unavailable('Fresh phone state required')
        current=self.snapshot
        if generation!=current['generation'] or revision!=current['revision'] or not current['controls_allowed']:
            raise Unavailable('Phone state changed')
        call=next((call for call in current['calls'] if call['id']==call_id),None)
        if call is None or action not in call['actions']:raise Unavailable('Action unavailable')
        self.pending=True;self.outcome=None;self.reason=None
        return dict(generation=generation,call_id=call_id,expected_revision=revision,
                    expected_state=call['state'],action=action,operation_id=uuid.uuid4().hex)

    def finish(self,response):
        self.pending=False
        self.outcome=response['outcome'] if response is not None else 'unknown'
        self.reason=response['reason'] if response is not None else 'connection_lost'
        # Even submitted/rejected results need a new observation before another action.
        self.disconnect()

    def observe_dial(self,value,started,now):
        validate_dial_status(value,value.get('id'))
        if not 0<=now-started<=3:
            self.dial_snapshot=None;self.dial_update=None;return
        self.dial_snapshot=copy.deepcopy(value);self.dial_update=started

    def can_dial(self,now):
        return bool(not self.dial_pending and self.dial_snapshot and self.dial_update is not None
                    and 0<=now-self.dial_update<=3 and self.dial_snapshot['can_dial'])

    def begin_dial(self,number,now):
        if not self.can_dial(now):raise Unavailable('Fresh outgoing call permission required')
        number=canonical_number(number)
        value=self.dial_snapshot
        self.dial_pending=True;self.dial_outcome=None;self.dial_reason=None
        return dict(generation=value['generation'],offer=value['offer'],number=number,operation_id=uuid.uuid4().hex)

    def finish_dial(self,response):
        self.dial_pending=False
        self.dial_outcome=response['outcome'] if response is not None else 'outcome_unknown'
        self.dial_reason=response['reason'] if response is not None else 'connection_lost'
        self.dial_snapshot=None;self.dial_update=None
