"""Independent typed assertions for the proposed current JSON status; source only."""
from dataclasses import dataclass
import re

U64=(1<<64)-1

def integer(v,minimum=0):
    assert type(v) is int and minimum<=v<U64
    return v

def hx(v,n=None):
    assert type(v) is str and re.fullmatch('[0-9a-f]*',v) and len(v)%2==0
    if n is not None:assert len(v)==2*n
    return bytes.fromhex(v)

@dataclass(frozen=True)
class Response:
    error:int
    payload:bytes
    ingress:int
    received:int
    publication:int

@dataclass(frozen=True)
class Request:
    revision:int
    intent:str
    outcome:str
    deadline:int
    digest:str
    response:Response|None

def request(v,incarnation,digest):
    if v is None:return None
    assert set(v)=={'request_revision','intent','incarnation','attempt','serial','client_id','request',
        'slot','payload_sha256','deadline_ns','outcome','response'}
    assert v['incarnation']==incarnation and (v['attempt'],v['serial'],v['client_id'],v['request'],v['slot'])==(1,1,42,105,1)
    hx(v['intent'],16);hx(v['payload_sha256'],32);assert v['payload_sha256']==digest
    assert v['outcome'] in ('outcome-unknown','not-sent')
    response=v['response'];r=None
    if response is not None:
        assert set(response)=={'vendor_error','payload_hex','ingress_sequence','received_at_ns','publication_revision'}
        assert type(response['vendor_error']) is int and -(1<<31)<=response['vendor_error']<(1<<31)
        payload=hx(response['payload_hex']);assert len(payload)<=4096
        r=Response(response['vendor_error'],payload,integer(response['ingress_sequence'],1),
            integer(response['received_at_ns'],1),integer(response['publication_revision'],1))
        assert r.received<v['deadline_ns']
        assert v['outcome']=='outcome-unknown'
    return Request(integer(v['request_revision'],1),v['intent'],v['outcome'],integer(v['deadline_ns'],1),digest,r)

def status(v,pid,incarnation,digest):
    assert set(v)=={'server_pid','phase','failure','incarnation','revision','last_observation','request_history'}
    assert v['server_pid']==pid and v['incarnation']==incarnation
    assert v['phase'] in ('initializing','observed-active','unavailable','stopping')
    integer(v['revision']);hx(incarnation,16)
    # last_observation is independent transport history; it cannot stand in for
    # request attribution. Existing envelope checks its current tagged schema.
    return request(v['request_history'],incarnation,digest)

def first_response(before,after):
    assert before.intent==after.intent and before.deadline==after.deadline
    assert after.outcome=='outcome-unknown' and after.response is not None
    assert after.response.error==-27 and after.response.payload==bytes.fromhex('7000ff31')
    assert after.revision>before.revision
    return after

def retained(first,later):
    assert first.intent==later.intent and first.digest==later.digest and first.deadline==later.deadline
    assert later.revision>=first.revision and later.response==first.response
    assert later.outcome=='outcome-unknown'
