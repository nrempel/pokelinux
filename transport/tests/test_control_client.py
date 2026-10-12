import sys,unittest
from pathlib import Path
from unittest.mock import Mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from omarchy_bridge import Client,ProtocolError,validate_controls

def snapshot():
    return {'v':1,'id':'a'*32,'generation':'b'*32,'revision':3,'controls_allowed':True,
            'availability':'observed','source':'android.telecom.InCallService','calls':[{'id':7,'state':2,'actions':['answer','reject']}]}

class ControlClientTests(unittest.TestCase):
    def test_validate_action_snapshot(self):self.assertEqual(validate_controls(snapshot(),'a'*32),snapshot())
    def test_invalid_revision_or_action_rejected(self):
        for field,value in [('revision',True),('revision',0),('controls_allowed',1),('generation','bad')]:
            with self.subTest(field=field),self.assertRaises(ProtocolError):validate_controls(dict(snapshot(),**{field:value}),'a'*32)
        for calls in [[{'id':7,'state':4,'actions':['answer']}],[{'id':7,'state':2,'actions':['answer','answer']}],[{'id':7,'state':2,'actions':['shell']}]]:
            with self.subTest(calls=calls),self.assertRaises(ProtocolError):validate_controls(dict(snapshot(),calls=calls),'a'*32)
    def test_no_actions_without_authorization_or_single_call(self):
        for values in [dict(controls_allowed=False),dict(availability='unavailable'),dict(calls=snapshot()['calls']+[{'id':8,'state':4,'actions':[]}])]:
            with self.subTest(values=values),self.assertRaises(ProtocolError):validate_controls(dict(snapshot(),**values),'a'*32)
    def test_transport_failure_is_never_retried(self):
        client=object.__new__(Client);client._exchange=Mock(side_effect=OSError('connection lost'))
        with self.assertRaises(OSError):client.control('b'*32,7,3,2,'answer',operation_id='a'*32)
        self.assertEqual(client._exchange.call_count,1)
    def test_explicit_operation_id_is_preserved(self):
        client=object.__new__(Client);client._exchange=Mock(return_value='accepted by test adapter')
        client.control('b'*32,7,3,2,'answer',operation_id='a'*32)
        request=client._exchange.call_args.args[0]
        self.assertEqual(request['id'],'a'*32);self.assertEqual(request['expected_revision'],3)

if __name__=='__main__':unittest.main()
