#!/usr/bin/env python3
"""Create bounded authored SMS-STATUS-REPORT inputs for original AOSP comparison."""
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'native-messaging/tests/fixtures/status-report-input.tsv'

def bcd(value):
    return ((value % 10) << 4) | (value // 10)

def stamp(year=26, month=10, day=12, hour=13, minute=14, second=15, quarters=0):
    return bytes([bcd(v) for v in (year, month, day, hour, minute, second)] + [bcd(abs(quarters)) | (8 if quarters < 0 else 0)])

def number(value):
    digits = value.lstrip('+')
    padded = digits + ('f' if len(digits) % 2 else '')
    return bytes([0x91 if value.startswith('+') else 0x81] + [int(padded[i + 1] + padded[i], 16) for i in range(0, len(padded), 2)])

def report(reference=255, recipient='+15550100', center=None, sent=None, discharged=None, status=0, flags=2, extra=b''):
    prefix = number(center) if center else b''
    address = number(recipient)
    return bytes([len(prefix)]) + prefix + bytes([flags, reference, len(recipient.lstrip('+'))]) + address + (sent or stamp()) + (discharged or stamp(second=16)) + bytes([status]) + extra

rows=[]
def add(name, pdu, policy='compare'):
    rows.append((name,pdu.hex(),policy))

base=report()
add('basic-max-reference',base)
for value in (0,1,31,32,63,64,95,96,127,128,255):
    add('status-'+str(value),report(reference=0,status=value))
for recipient in ('+1','12345','+12345678901234567890'):
    add('recipient-'+str(len(recipient.lstrip('+'))),report(recipient=recipient))
add('national-recipient',report(recipient='15550100')[:4] + bytes([0xa1]) + report(recipient='15550100')[5:])
for center in ('+15550100101','12345','+1234567890123456789012'):
    add('center-'+str(len(center.lstrip('+'))),report(center=center))
for name, value in [('leap-2000',stamp(year=0,month=2,day=29)),('leap-2024',stamp(year=24,month=2,day=29)),('year-1990',stamp(year=90)),('year-1999',stamp(year=99)),('year-2089',stamp(year=89))]:
    add(name,report(sent=value,discharged=value))
for quarters in (8,-8,22,-22,79,-79):
    add('timezone-'+str(quarters),report(sent=stamp(quarters=quarters),discharged=stamp(second=16,quarters=-quarters)))
add('command-mms-loop',report(flags=2|4|8|32))
add('unused-flags-ignored',report(flags=2|16|128))
add('optional-empty',report(extra=bytes([0])))
add('optional-pid',report(extra=bytes([1,64])))
add('optional-dcs-without-data',report(extra=bytes([2,255])))
add('optional-empty-gsm',report(extra=bytes([4,0])))
add('optional-empty-unicode',report(extra=bytes([6,8,0])))
submit={row.split('\t')[1]:bytes.fromhex(row.split('\t')[6]) for row in (ROOT/'native-messaging/tests/fixtures/aosp-submit.tsv').read_text().splitlines()}
for name in ('hello','alphabet','extension','controls','chinese','emoji','gsm-limit','unicode-limit'):
    encoded=submit[name]
    offset=4+(encoded[2]+1)//2
    add('optional-'+name,report(recipient='+1' if name in ('gsm-limit','unicode-limit') else '+15550100',extra=bytes([7,encoded[offset],encoded[offset+1]])+encoded[offset+2:]))
for coding in (0x10,0x11,0x12,0x13,0x18,0x19,0x1a,0x1b):
    name='hello' if coding<0x18 else 'chinese'
    encoded=submit[name];offset=4+(encoded[2]+1)//2
    add('optional-class-'+str(coding),report(extra=bytes([6,coding])+encoded[offset+2:]))
for count in (2,3,4):
    add('empty-pi-extension-'+str(count),report(extra=bytes([0x80]*(count-1)+[0])))
add('max-complete-bound',report(recipient='+1',center='+1234567890123456789012',extra=bytes([7,0,8,140])+bytes([0,32])*70))
add('tpdu-165-short-prefix',report(recipient='+123',extra=bytes([7,0,8,140])+bytes([0,32])*70),'malformed')
for length in range(len(base)):
    add('required-cut-'+str(length),base[:length],'malformed')
for region, offset in [('sent',9),('discharge',16)]:
    for field, invalid in [('month-zero',(1,0)),('month-13',(1,0x31)),('day-zero',(2,0)),('day-32',(2,0x23)),('hour-24',(3,0x42)),('minute-60',(4,6)),('second-60',(5,6)),('bad-bcd',(0,0x2f)),('bad-timezone',(6,0xa0))]:
        pdu=bytearray(base);pdu[offset+invalid[0]]=invalid[1]
        add(region+'-'+field,bytes(pdu),'malformed')
add('non-leap-february',report(sent=stamp(year=25,month=2,day=29)),'malformed')
add('zero-recipient',base[:3]+bytes([0])+base[4:],'malformed')
add('bad-recipient-digit',base[:5]+bytes([0x5a])+base[6:],'malformed')
odd=report(recipient='+1555010')
add('odd-recipient-filler',odd[:8]+bytes([0])+odd[9:],'malformed')
add('smsc-toa-only',bytes([1,0x91])+base[1:],'malformed')
add('trailing-byte',base+bytes([0,0]),'malformed')
add('unterminated-pi',base+bytes([0x80]),'malformed')
add('too-many-pi',base+bytes([0x80]*4+[0]),'malformed')
add('pi-missing-pid',base+bytes([1]),'malformed')
add('pi-missing-dcs',base+bytes([2]),'malformed')
add('pi-missing-length',base+bytes([4]),'malformed')
add('utf16-odd',base+bytes([6,8,1,65]),'malformed')
add('utf16-unpaired',base+bytes([6,8,2,0xd8,0]),'malformed')
add('gsm-orphan-escape',base+bytes([4,1,0x1b]),'malformed')
add('data-short',base+bytes([4,2,65]),'malformed')
add('gsm-length-161',base+bytes([4,161])+bytes(141),'malformed')
add('unicode-length-142',base+bytes([6,8,142])+bytes(142),'malformed')
add('oversize',report(extra=bytes(200)),'malformed')
add('wrong-mti-submit',bytes([0,1])+base[2:],'unsupported')
add('unsupported-number-plan',base[:4]+bytes([0x92])+base[5:],'unsupported')
add('reserved-pi',base+bytes([8]),'unsupported')
add('future-pi-extension',base+bytes([0x80,1]),'unsupported')
add('compressed-text',base+bytes([6,0x20,0]),'unsupported')
add('binary-text',base+bytes([6,4,0]),'unsupported')
add('mwi-text',base+bytes([6,0xc0,0]),'unsupported')
add('header-data',bytes([0,0x42])+base[2:]+bytes([4,1,0]),'unsupported')
assert len({row[0] for row in rows})==len(rows)
assert max(len(bytes.fromhex(row[1])) for row in rows)<512
OUT.write_text(''.join('\t'.join(row)+'\n' for row in rows))
print(f'{len(rows)} synthetic report inputs: '+', '.join(f'{sum(row[2]==policy for row in rows)} {policy}' for policy in ('compare','malformed','unsupported')))
