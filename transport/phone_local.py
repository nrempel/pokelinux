#!/usr/bin/env python3
"""One-shot local client for the QML shell. No automatic command retry path."""
import argparse
import json
import os
from pathlib import Path
import re
import socket
import sys
import uuid
from omarchy_bridge import control_request, dial_request, validate_controls, validate_dial_status
from phone_ipc import send, receive


def validate_response(value, request_id):
    if (not isinstance(value, dict) or set(value) != {'v', 'id', 'snapshot'}
            or type(value['v']) is not int or value['v'] != 1 or value['id'] != request_id):
        raise ValueError('Invalid local response')
    state = value['snapshot']
    if not isinstance(state, dict) or set(state) != {'epoch', 'sequence', 'controls', 'dial_status'}:
        raise ValueError('Invalid observation')
    if (not isinstance(state['epoch'], str) or not re.fullmatch('[0-9a-f]{32}', state['epoch'])
            or type(state['sequence']) is not int or state['sequence'] < 0):
        raise ValueError('Invalid observation identity')
    for name, validator in (('controls', validate_controls), ('dial_status', validate_dial_status)):
        lane = state[name]
        if not isinstance(lane, dict) or set(lane) != {'fresh', 'value'} or type(lane['fresh']) is not bool:
            raise ValueError('Invalid observation freshness')
        if lane['value'] is None:
            if lane['fresh']:
                raise ValueError('Missing fresh observation')
        else:
            if not isinstance(lane['value'], dict):
                raise ValueError('Invalid observed value')
            validator(lane['value'], lane['value'].get('id'))
    return state


def snapshot(path):
    identity = uuid.uuid4().hex
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        connection.connect(str(path))
        send(connection, {'v': 1, 'id': identity, 'op': 'snapshot'})
        return validate_response(receive(connection), identity)


def validate_operation(value, identity, epoch):
    if (not isinstance(value, dict) or set(value) != {'v', 'id', 'epoch', 'operation'}
            or type(value['v']) is not int or value['v'] != 1
            or value['id'] != identity or value['epoch'] != epoch):
        raise ValueError('Invalid operation response')
    operation = value['operation']
    if operation is None:
        return None  # Absence is not evidence that a delayed command cannot execute.
    if (not isinstance(operation, dict) or set(operation) != {'state', 'result'}
            or operation['state'] not in ('pending', 'unknown', 'finished')):
        raise ValueError('Invalid operation state')
    result = operation['result']
    if operation['state'] == 'finished':
        if (not isinstance(result, dict) or set(result) != {'outcome', 'reason'}
                or any(not isinstance(v, str) or not re.fullmatch('[a-z_]{1,64}', v)
                       for v in result.values())):
            raise ValueError('Invalid operation outcome')
    elif result is not None:
        raise ValueError('Unresolved operation has an outcome')
    return operation


def validate_recovery(value, identity):
    if (not isinstance(value, dict) or set(value) != {'v','id','recovery'}
            or type(value['v']) is not int or value['v'] != 1 or value['id'] != identity):
        raise ValueError('Invalid recovery response')
    recovery = value['recovery']
    if (not isinstance(recovery, dict) or set(recovery) != {'epoch','items','more'}
            or not isinstance(recovery['epoch'], str) or not re.fullmatch('[0-9a-f]{32}', recovery['epoch'])
            or not isinstance(recovery['items'], list) or len(recovery['items']) > 64
            or type(recovery['more']) is not bool
            or (recovery['more'] and len(recovery['items']) != 64)):
        raise ValueError('Invalid recovery page')
    seen = set()
    for item in recovery['items']:
        if (not isinstance(item, dict) or set(item) != {'epoch','id','state','result'}
                or any(not isinstance(item[k], str) or not re.fullmatch('[0-9a-f]{32}', item[k])
                       for k in ('epoch','id'))):
            raise ValueError('Invalid recovered identity')
        key = (item['epoch'], item['id'])
        if key in seen:
            raise ValueError('Duplicate recovered identity')
        seen.add(key)
        operation = validate_operation(dict(v=1,id=item['id'],epoch=item['epoch'],
            operation={'state':item['state'],'result':item['result']}),item['id'],item['epoch'])
        if operation['state'] == 'finished' and operation['result']['outcome'] != 'unknown':
            raise ValueError('Resolved record in uncertainty recovery')
    return recovery


def recover(path):
    identity = uuid.uuid4().hex
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        connection.connect(str(path))
        send(connection, {'v':1,'id':identity,'op':'recovery'})
        return validate_recovery(receive(connection), identity)


def validate_session(value, identity):
    if (not isinstance(value, dict) or set(value) != {'v','id','session'}
            or type(value['v']) is not int or value['v'] != 1 or value['id'] != identity):
        raise ValueError('Invalid session response')
    session = value['session']
    if (not isinstance(session, dict) or set(session) != {'epoch','session','recovery'}
            or not isinstance(session['session'], str)
            or not re.fullmatch('[0-9a-f]{32}', session['session'])
            or not isinstance(session['recovery'], dict)
            or set(session['recovery']) != {'items','more'}):
        raise ValueError('Invalid session descriptor')
    validate_recovery({'v':1,'id':identity,'recovery':dict(epoch=session['epoch'], **session['recovery'])}, identity)
    return session


def begin_session(path):
    """Claim control authority once; never refresh authority for an old command."""
    identity = uuid.uuid4().hex
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        connection.connect(str(path))
        send(connection, {'v':1,'id':identity,'op':'session'})
        return validate_session(receive(connection), identity)


def acknowledge(path, epoch, session, record_epoch, record_id):
    identity = uuid.uuid4().hex
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        connection.connect(str(path))
        send(connection, dict(v=1,id=identity,op='acknowledge',epoch=epoch,session=session,
                              record_epoch=record_epoch,record_id=record_id))
        response = receive(connection)
        if (not isinstance(response, dict) or set(response) != {'v','id','acknowledged'}
                or type(response['v']) is not int or response['v'] != 1
                or response['id'] != identity or type(response['acknowledged']) is not bool):
            raise ValueError('Invalid acknowledgment response')
        return response['acknowledged']


def _operation(path, request):
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        connection.connect(str(path))
        send(connection, request)
        return validate_operation(receive(connection), request['id'], request['epoch'])


def control(path, epoch, identity, command, session=None):
    request = {'v': 1, 'id': identity, 'epoch': epoch, 'op': 'control', 'command': command}
    if session is not None:
        request['session'] = session
    result = _operation(path, request)
    if result is None:
        raise ValueError('Missing admitted operation')
    return result


def dial(path, epoch, identity, command, session):
    dial_request(identity, **command)
    result = _operation(path, dict(v=1,id=identity,epoch=epoch,op='dial',command=command,session=session))
    if result is None:
        raise ValueError('Missing admitted operation')
    return result


def operation_result(path, epoch, identity):
    return _operation(path, {'v': 1, 'id': identity, 'epoch': epoch, 'op': 'result'})


def _identity(value):
    if not re.fullmatch('[0-9a-f]{32}', value):
        raise argparse.ArgumentTypeError('Expected a 32-character lowercase hexadecimal identity')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--socket', type=Path)
    commands = parser.add_subparsers(dest='operation')
    commands.add_parser('snapshot')
    commands.add_parser('recovery')
    commands.add_parser('session')
    acknowledgment = commands.add_parser('acknowledge')
    for field in ('epoch','session','record-epoch','record-id'):
        acknowledgment.add_argument('--'+field, required=True, type=_identity)
    for operation in ('control', 'dial', 'result'):
        sub = commands.add_parser(operation)
        sub.add_argument('--epoch', required=True, type=_identity)
        sub.add_argument('--id', required=True, type=_identity)
        if operation in ('control','dial'):
            sub.add_argument('--session', required=True, type=_identity)
            sub.add_argument('--generation', required=True, type=_identity)
        if operation == 'dial':
            sub.add_argument('--offer', required=True, type=_identity)
            sub.add_argument('--number', required=True)
        if operation == 'control':
            sub.add_argument('--call-id', required=True, type=int)
            sub.add_argument('--revision', required=True, type=int)
            sub.add_argument('--state', required=True, type=int)
            sub.add_argument('--action', required=True, choices=('answer', 'reject', 'disconnect'))
    args = parser.parse_args()
    command = None
    if args.operation == 'control':
        command = dict(generation=args.generation, call_id=args.call_id,
                       expected_revision=args.revision, expected_state=args.state, action=args.action)
        try:
            control_request(args.id, **command)
        except ValueError as error:
            parser.error(str(error))
    elif args.operation == 'dial':
        command = dict(generation=args.generation, offer=args.offer, number=args.number)
        try:
            dial_request(args.id, **command)
        except ValueError as error:
            parser.error(str(error))
    path = args.socket
    if path is None:
        if not os.environ.get('XDG_RUNTIME_DIR'):
            parser.error('XDG_RUNTIME_DIR or --socket required')
        path = Path(os.environ['XDG_RUNTIME_DIR']) / 'omarchy-phone/phone.sock'
    identity = {} if args.operation in (None, 'snapshot', 'recovery', 'session', 'acknowledge') else {'epoch': args.epoch, 'id': args.id}
    try:
        if args.operation == 'control':
            payload = {'operation': control(path, args.epoch, args.id, command, session=args.session)}
        elif args.operation == 'dial':
            payload = {'operation': dial(path, args.epoch, args.id, command, args.session)}
        elif args.operation == 'session':
            payload = {'session': begin_session(path)}
        elif args.operation == 'acknowledge':
            payload = {'acknowledged': acknowledge(path, args.epoch, args.session, args.record_epoch, args.record_id)}
        elif args.operation == 'recovery':
            payload = {'recovery': recover(path)}
        elif args.operation == 'result':
            payload = {'operation': operation_result(path, args.epoch, args.id)}
        else:
            payload = {'snapshot': snapshot(path)}
        print(json.dumps(dict(available=True, **identity, **payload), separators=(',', ':')))
    except (OSError, ValueError):
        # Transport failure is uncertainty, not a rejected Android command.
        print(json.dumps(dict(available=False, **identity), separators=(',', ':')))
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
