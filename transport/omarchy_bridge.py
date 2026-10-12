#!/usr/bin/env python3
"""Authenticated Android phone client. No automatic command retries. Python standard library only."""
import argparse
import hashlib
import hmac
import ipaddress
import json
import re
import socket
import ssl
import struct
import time
import uuid
from pathlib import Path

MAX_FRAME = 65536

class ProtocolError(ValueError):
    pass

def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ProtocolError('duplicate JSON key')
        result[key] = value
    return result

def _read(sock, size):
    data = bytearray()
    while len(data) < size:
        chunk = sock.recv(size - len(data))
        if not chunk:
            raise ProtocolError('truncated frame')
        data.extend(chunk)
    return bytes(data)

def receive(sock):
    size, = struct.unpack('!I', _read(sock, 4))
    if not 0 < size <= MAX_FRAME:
        raise ProtocolError('invalid frame size')
    try:
        return json.loads(_read(sock, size).decode('utf-8'), object_pairs_hook=_object,
                          parse_constant=lambda _: (_ for _ in ()).throw(ProtocolError('non-finite number')))
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise ProtocolError('invalid JSON') from error

def send(sock, value):
    body = json.dumps(value, separators=(',', ':'), allow_nan=False).encode()
    if not 0 < len(body) <= MAX_FRAME:
        raise ProtocolError('invalid frame size')
    sock.sendall(struct.pack('!I', len(body)) + body)

def validate_snapshot(value, request_id):
    fields = {'v','id','generation','observed_ms','dialer_role','availability','calls','source','phone_state'}
    if not isinstance(value, dict) or set(value) != fields:
        raise ProtocolError('invalid response fields')
    if type(value['v']) is not int or value['v'] != 1 or value['id'] != request_id:
        raise ProtocolError('response version or request mismatch')
    if not isinstance(value['generation'], str) or not re.fullmatch('[0-9a-f]{32}', value['generation']):
        raise ProtocolError('invalid generation')
    if type(value['observed_ms']) is not int or value['observed_ms'] < 0 or type(value['dialer_role']) is not bool:
        raise ProtocolError('invalid observation')
    if value['source'] != 'android.telecom.InCallService' or value['availability'] not in ('observed','unavailable'):
        raise ProtocolError('invalid source')
    phone=value['phone_state']
    if not isinstance(phone,dict) or set(phone)!={'availability','state','source','scope'} or phone['source']!='android.telephony.TelephonyCallback' or phone['scope']!='default_subscription':
        raise ProtocolError('invalid phone state source')
    if phone['availability']=='observed':
        if type(phone['state']) is not int or phone['state'] not in (0,1,2):
            raise ProtocolError('invalid phone state')
    elif phone['availability']!='unavailable' or phone['state'] is not None:
        raise ProtocolError('unavailable phone state has a value')
    calls = value['calls']
    if not isinstance(calls,list) or len(calls)>64:
        raise ProtocolError('invalid calls')
    ids=set()
    for call in calls:
        if not isinstance(call,dict) or set(call)!={'id','state'} or type(call['id']) is not int or call['id']<1 or type(call['state']) is not int or not 0<=call['state']<=20 or call['id'] in ids:
            raise ProtocolError('invalid call')
        ids.add(call['id'])
    if value['availability']=='observed' and (not value['dialer_role'] or not calls):
        raise ProtocolError('detailed observation lacks role or calls')
    if (not value['dialer_role'] or value['availability']=='unavailable') and calls:
        raise ProtocolError('unavailable snapshot contains calls')
    return value

def control_request(request_id, generation, call_id, expected_revision, expected_state, action):
    if any(not isinstance(v,str) or not re.fullmatch('[0-9a-f]{32}',v) for v in (request_id,generation)):
        raise ProtocolError('invalid command identity')
    if type(call_id) is not int or not 1 <= call_id <= 2147483647:
        raise ProtocolError('invalid call id')
    if type(expected_revision) is not int or not 1 <= expected_revision <= 9007199254740991:
        raise ProtocolError('invalid expected revision')
    if type(expected_state) is not int or not 0 <= expected_state <= 20:
        raise ProtocolError('invalid expected state')
    if action not in ('answer','reject','disconnect'):
        raise ProtocolError('unsupported control')
    return {'v':1,'id':request_id,'op':'control','generation':generation,
            'call_id':call_id,'expected_revision':expected_revision,'expected_state':expected_state,'action':action}

def validate_controls(value, request_id):
    fields={'v','id','generation','revision','controls_allowed','availability','source','calls'}
    if not isinstance(value,dict) or set(value)!=fields:
        raise ProtocolError('invalid control snapshot')
    if type(value['v']) is not int or value['v']!=1 or value['id']!=request_id:
        raise ProtocolError('control snapshot request mismatch')
    if not isinstance(value['generation'],str) or not re.fullmatch('[0-9a-f]{32}',value['generation']):
        raise ProtocolError('invalid control generation')
    if type(value['revision']) is not int or not 1<=value['revision']<=9007199254740991 or type(value['controls_allowed']) is not bool:
        raise ProtocolError('invalid control revision or permission')
    if value['source']!='android.telecom.InCallService' or value['availability'] not in ('observed','unavailable'):
        raise ProtocolError('invalid control source')
    calls=value['calls']
    if not isinstance(calls,list) or len(calls)>64:
        raise ProtocolError('invalid control calls')
    if (value['availability']=='unavailable' and calls) or (value['availability']=='observed' and not calls):
        raise ProtocolError('invalid control availability')
    seen=set()
    for call in calls:
        if not isinstance(call,dict) or set(call)!={'id','state','actions'} or type(call['id']) is not int or call['id']<1 or type(call['state']) is not int or not 0<=call['state']<=20 or call['id'] in seen:
            raise ProtocolError('invalid control call')
        seen.add(call['id'])
        actions=call['actions']
        if not isinstance(actions,list) or any(not isinstance(action,str) for action in actions) or len(actions)!=len(set(actions)):
            raise ProtocolError('invalid call actions')
        permitted={'answer','reject'} if call['state']==2 else {'disconnect'} if call['state'] in (1,3,4,9) else set()
        if not value['controls_allowed'] or len(calls)!=1: permitted=set()
        if not set(actions)<=permitted:
            raise ProtocolError('actions are not valid for the observation')
    return value

def validate_control(value, request_id, generation):
    if not isinstance(value,dict) or set(value)!={'v','id','generation','outcome','reason'}:
        raise ProtocolError('invalid control response')
    if type(value['v']) is not int or value['v']!=1 or value['id']!=request_id or value['generation']!=generation:
        raise ProtocolError('control response identity mismatch')
    reasons={'submitted':{'accepted'}, 'unknown':{'dispatch_error'},
             'rejected':{'disabled','stale_generation','stale_revision','stale_state','missing_call','unavailable','replay_conflict','capacity','deadline'}}
    if not isinstance(value['outcome'],str) or not isinstance(value['reason'],str):
        raise ProtocolError('invalid control outcome type')
    if value['outcome'] not in reasons or value['reason'] not in reasons[value['outcome']]:
        raise ProtocolError('invalid control outcome')
    return value

def canonical_number(value):
    if not isinstance(value,str) or re.fullmatch(r'\+?[0-9]{3,32}',value) is None:
        raise ProtocolError('Use 3 to 32 digits with an optional leading plus')
    return value

def dial_request(request_id, generation, offer, number):
    if any(not isinstance(value,str) or not re.fullmatch('[0-9a-f]{32}',value) for value in (request_id,generation,offer)):
        raise ProtocolError('invalid dial identity')
    return {'v':1,'id':request_id,'op':'dial','generation':generation,'offer':offer,'number':canonical_number(number)}

def validate_dial_status(value, request_id):
    if not isinstance(value,dict) or set(value)!={'v','id','generation','can_dial','offer','pending','reason'}:
        raise ProtocolError('invalid dial snapshot')
    if type(value['v']) is not int or value['v']!=1 or value['id']!=request_id:
        raise ProtocolError('dial snapshot identity mismatch')
    if not isinstance(value['generation'],str) or not re.fullmatch('[0-9a-f]{32}',value['generation']):
        raise ProtocolError('invalid dial generation')
    if type(value['can_dial']) is not bool or type(value['pending']) is not bool or not isinstance(value['offer'],str) or not isinstance(value['reason'],str):
        raise ProtocolError('invalid dial snapshot types')
    if value['can_dial']:
        if value['pending'] or value['reason']!='ready' or not re.fullmatch('[0-9a-f]{32}',value['offer']):
            raise ProtocolError('inconsistent dial offer')
    elif value['offer'] or value['reason'] not in ('disabled','role','permission','busy','account_unavailable','unavailable','offer_capacity','pending') or value['pending']!=(value['reason']=='pending'):
        raise ProtocolError('inconsistent dial unavailability')
    return value

def validate_dial(value, request_id, generation):
    if not isinstance(value,dict) or set(value)!={'v','id','generation','outcome','reason'}:
        raise ProtocolError('invalid dial response')
    if type(value['v']) is not int or value['v']!=1 or value['id']!=request_id or value['generation']!=generation:
        raise ProtocolError('dial response identity mismatch')
    reasons={'invocation_attempted':{'returned'},'outcome_unknown':{'dispatch_error'},
             'not_invoked':{'invalid_request','stale_generation','replay_conflict','capacity','pending','stale_offer','stale_revision','account_changed','deadline','storage_error','unavailable','disabled','role','permission','busy','account_unavailable','emergency'}}
    if not isinstance(value['outcome'],str) or not isinstance(value['reason'],str) or value['outcome'] not in reasons or value['reason'] not in reasons[value['outcome']]:
        raise ProtocolError('invalid dial outcome')
    return value

def validate_config(config):
    required={'host','port','server_name','server_sha256','ca','certificate','private_key'}
    if not isinstance(config,dict) or set(config)!=required:
        raise ValueError('configuration fields do not match v1')
    if not isinstance(config['host'],str): raise ValueError('numeric transport address required')
    ipaddress.ip_address(config['host'])
    if type(config['port']) is not int or not 1<=config['port']<=65535:
        raise ValueError('invalid port')
    if any(not isinstance(config[name],str) or not config[name] for name in required-{'port'}):
        raise ValueError('invalid configuration value')
    if not re.fullmatch('[0-9a-f]{64}',config['server_sha256']):
        raise ValueError('server_sha256 must be a lowercase certificate SHA256')

class Client:
    def __init__(self, config):
        validate_config(config)
        self.config = dict(config)
        if Path(config['private_key']).stat().st_mode & 0o077:
            raise ValueError('private key must not be accessible to group or others')
        self.context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH, cafile=config['ca'])
        self.context.minimum_version = ssl.TLSVersion.TLSv1_3
        self.context.maximum_version = ssl.TLSVersion.TLSv1_3
        self.context.load_cert_chain(config['certificate'], config['private_key'])

    def status(self):
        request_id=uuid.uuid4().hex
        return self._exchange({'v':1,'id':request_id,'op':'status'},
                              lambda value:validate_snapshot(value,request_id))

    def controls(self):
        request_id=uuid.uuid4().hex
        return self._exchange({'v':1,'id':request_id,'op':'controls'},
                              lambda value:validate_controls(value,request_id))

    def control(self, generation, call_id, expected_revision, expected_state, action, *, operation_id=None):
        request_id=operation_id if operation_id is not None else uuid.uuid4().hex
        request=control_request(request_id,generation,call_id,expected_revision,expected_state,action)
        # One exchange only. A lost response is ambiguous and must never trigger a retry.
        return self._exchange(request,lambda value:validate_control(value,request_id,generation))

    def dial_status(self):
        request_id=uuid.uuid4().hex
        return self._exchange({'v':1,'id':request_id,'op':'dial_status'},
                              lambda value:validate_dial_status(value,request_id))

    def dial(self, generation, offer, number, *, operation_id=None):
        request_id=operation_id if operation_id is not None else uuid.uuid4().hex
        request=dial_request(request_id,generation,offer,number)
        # A returned invocation is not proof of a connected call. Never retry.
        return self._exchange(request,lambda value:validate_dial(value,request_id,generation))

    def _exchange(self,request,validate):
        config=self.config
        # A hard wall-clock budget also closes drip-fed reads and blocked writes.
        import threading
        with socket.create_connection((config.get('host','127.0.0.1'), config.get('port',37179)), timeout=5) as raw:
            with self.context.wrap_socket(raw, server_hostname=config['server_name']) as sock:
                def expire():
                    try: sock.shutdown(socket.SHUT_RDWR)
                    except OSError: pass
                timer=threading.Timer(5,expire); timer.daemon=True; timer.start()
                try:
                    digest=hashlib.sha256(sock.getpeercert(binary_form=True)).hexdigest()
                    if not hmac.compare_digest(digest,config['server_sha256']):
                        raise ssl.SSLCertVerificationError('server pin mismatch')
                    send(sock,request)
                    return validate(receive(sock))
                finally: timer.cancel()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--watch',action='store_true')
    args=parser.parse_args()
    delay=1
    while True:
        try:
            if args.config.stat().st_mode & 0o022:
                raise ValueError('configuration must not be writable by group or others')
            client=Client(json.loads(args.config.read_text()))
            value=client.status()
            value['transport']='connected'
            delay=1
        except (OSError,ValueError) as error:
            value={'transport':'unavailable','availability':'unavailable','calls':[], 'error':type(error).__name__}
            if not args.watch:
                print(json.dumps(value),flush=True)
                return 1
            delay=min(delay*2,30)
        print(json.dumps(value),flush=True)
        if not args.watch: return 0
        time.sleep(delay)

if __name__=='__main__':
    raise SystemExit(main())
