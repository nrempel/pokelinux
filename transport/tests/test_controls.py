import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omarchy_bridge import ProtocolError, control_request, validate_control

class ControlTests(unittest.TestCase):
    def request(self, **overrides):
        args=dict(request_id='a'*32, generation='b'*32, call_id=7, expected_revision=3, expected_state=2, action='answer')
        args.update(overrides)
        return control_request(**args)

    def test_command_carries_observation_and_unique_id(self):
        self.assertEqual(self.request(), {'v':1,'id':'a'*32,'op':'control','generation':'b'*32,'call_id':7,'expected_revision':3,'expected_state':2,'action':'answer'})

    def test_rejects_unknown_action_and_invalid_observation(self):
        for change in ({'action':'dial'}, {'action':'shell'}, {'call_id':True}, {'call_id':0}, {'expected_revision':True}, {'expected_revision':0}, {'expected_state':True}, {'expected_state':21}, {'generation':'old'}, {'request_id':'duplicate'}):
            with self.subTest(change=change), self.assertRaises(ProtocolError):
                self.request(**change)

    def test_submitted_does_not_mean_completed(self):
        value={'v':1,'id':'a'*32,'generation':'b'*32,'outcome':'submitted','reason':'accepted'}
        self.assertEqual(validate_control(value,'a'*32,'b'*32),value)
        value['outcome']='completed'
        with self.assertRaises(ProtocolError): validate_control(value,'a'*32,'b'*32)

    def test_response_must_match_generation_and_request(self):
        base={'v':1,'id':'a'*32,'generation':'b'*32,'outcome':'rejected','reason':'stale_state'}
        for field,value in (('id','c'*32),('generation','c'*32),('v',True),('reason','arbitrary'),('reason',[]),('outcome',{}),('extra',1)):
            with self.subTest(field=field), self.assertRaises(ProtocolError):
                validate_control(dict(base,**{field:value}),'a'*32,'b'*32)

    def test_valid_rejection_and_ambiguous_result(self):
        for outcome,reason in [('rejected','disabled'),('rejected','stale_generation'),('rejected','stale_state'),('rejected','stale_revision'),('rejected','missing_call'),('rejected','unavailable'),('rejected','replay_conflict'),('rejected','capacity'),('rejected','deadline'),('unknown','dispatch_error')]:
            value={'v':1,'id':'a'*32,'generation':'b'*32,'outcome':outcome,'reason':reason}
            self.assertEqual(validate_control(value,'a'*32,'b'*32),value)

if __name__=='__main__': unittest.main()
