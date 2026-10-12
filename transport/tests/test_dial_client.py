import sys, unittest
from pathlib import Path
from unittest.mock import Mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from omarchy_bridge import Client, ProtocolError, dial_request, validate_dial_status, validate_dial

ID='a'*32; GEN='b'*32; OFFER='c'*32

def status(**changes):
    return dict(dict(v=1,id=ID,generation=GEN,can_dial=True,offer=OFFER,pending=False,reason='ready'),**changes)

class DialClientTests(unittest.TestCase):
    def test_ready_and_blocked_status(self):
        self.assertEqual(validate_dial_status(status(),ID),status())
        for reason in ('disabled','role','permission','busy','account_unavailable','unavailable','offer_capacity','pending'):
            value=status(can_dial=False,offer='',pending=reason=='pending',reason=reason)
            self.assertEqual(validate_dial_status(value,ID),value)
    def test_inconsistent_or_malformed_status(self):
        for changes in (dict(v=True),dict(id=GEN),dict(generation='x'),dict(can_dial=1),dict(pending=0),dict(offer=''),dict(pending=True),dict(reason='busy'),dict(offer=[]),dict(reason=[]),dict(extra=1)):
            with self.subTest(changes=changes),self.assertRaises(ProtocolError):validate_dial_status(status(**changes),ID)
        with self.assertRaises(ProtocolError):validate_dial_status(status(can_dial=False,offer='',pending=False,reason='pending'),ID)
    def test_canonical_request_and_rejection_before_transport(self):
        self.assertEqual(list(dial_request(ID,GEN,OFFER,'+15550100')),['v','id','op','generation','offer','number'])
        for number in ('12','tel:5550100','*123#','555 0100','１２３',None,'1'*33):
            with self.subTest(number=number),self.assertRaises(ProtocolError):dial_request(ID,GEN,OFFER,number)
        for values in [('x',GEN,OFFER),(ID,'x',OFFER),(ID,GEN,'')]:
            with self.assertRaises(ProtocolError):dial_request(*values,'5550100')
    def test_result_is_invocation_not_call_success(self):
        for outcome,reason in [('invocation_attempted','returned'),('outcome_unknown','dispatch_error'),('not_invoked','pending'),('not_invoked','account_changed')]:
            value=dict(v=1,id=ID,generation=GEN,outcome=outcome,reason=reason)
            self.assertEqual(validate_dial(value,ID,GEN),value)
            for changes in (dict(v=True),dict(id=GEN),dict(generation=ID),dict(outcome='accepted'),dict(reason='accepted'),dict(reason=[])):
                with self.assertRaises(ProtocolError):validate_dial(dict(value,**changes),ID,GEN)
    def test_one_exchange_and_same_operation_id(self):
        client=object.__new__(Client);client._exchange=Mock(side_effect=OSError('lost acknowledgement'))
        with self.assertRaises(OSError):client.dial(GEN,OFFER,'5550100',operation_id=ID)
        self.assertEqual(client._exchange.call_count,1)
        self.assertEqual(client._exchange.call_args.args[0],dial_request(ID,GEN,OFFER,'5550100'))
    def test_status_uses_matching_response_validator(self):
        client=object.__new__(Client)
        client._exchange=lambda req,validate:validate(status(id=req['id']))
        self.assertTrue(client.dial_status()['can_dial'])

if __name__=='__main__':unittest.main()
