import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_model import PhoneModel,Unavailable
from test_dial_client import status
from test_control_client import snapshot

class PhoneDialTests(unittest.TestCase):
 def test_expired_and_delayed_offers_cannot_dial(self):
  m=PhoneModel();m.observe_dial(status(),10,10)
  with self.assertRaises(Unavailable):m.begin_dial('5550100',13.1)
  m.observe_dial(status(),20,24)
  with self.assertRaises(Unavailable):m.begin_dial('5550100',24)
 def test_duplicate_click_and_unknown_result(self):
  m=PhoneModel();m.observe_dial(status(),10,10)
  request=m.begin_dial('5550100',10);self.assertEqual(request['offer'],status()['offer'])
  with self.assertRaises(Unavailable):m.begin_dial('5550100',10)
  m.finish_dial(None);self.assertEqual(m.dial_outcome,'outcome_unknown')
  with self.assertRaises(Unavailable):m.begin_dial('5550100',10)
 def test_outgoing_pending_does_not_disable_incoming_answer(self):
  m=PhoneModel();m.observe_dial(status(),10,10);m.begin_dial('5550100',10)
  m.update(snapshot(),10)
  self.assertEqual(m.begin('b'*32,3,7,'answer',10)['action'],'answer')
 def test_server_pending_and_disconnect_disable_dial(self):
  m=PhoneModel();m.observe_dial(status(can_dial=False,offer='',pending=True,reason='pending'),10,10)
  with self.assertRaises(Unavailable):m.begin_dial('5550100',10)
  m.observe_dial(status(),11,11);m.disconnect()
  with self.assertRaises(Unavailable):m.begin_dial('5550100',11)
 def test_invalid_number_never_sets_pending(self):
  m=PhoneModel();m.observe_dial(status(),10,10)
  with self.assertRaises(ValueError):m.begin_dial('*123#',10)
  self.assertFalse(m.dial_pending)

if __name__=='__main__':unittest.main()
