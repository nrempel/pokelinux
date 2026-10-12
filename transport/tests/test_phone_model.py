import sys,unittest
from unittest.mock import patch
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_model import PhoneModel,Unavailable
import phone_model as clock_module

def status(revision=3):
    return {'v':1,'id':'a'*32,'generation':'b'*32,'revision':revision,'controls_allowed':True,
            'availability':'observed','source':'android.telecom.InCallService','calls':[{'id':7,'state':2,'actions':['answer','reject']}]}
class PhoneModelTests(unittest.TestCase):
    def test_freshness_clock_includes_suspend_on_linux(self):
        with patch.object(clock_module.time,'CLOCK_BOOTTIME',7,create=True), patch.object(clock_module.time,'clock_gettime',return_value=600,create=True) as clock:
            self.assertEqual(clock_module.elapsed_time(),600);clock.assert_called_once_with(7)
    def test_delayed_observation_is_not_fresh(self):
        model=PhoneModel();model.observe(status(),started=10,now=20)
        self.assertFalse(model.fresh(20));self.assertEqual(model.calls,[])
        model.observe(status(),started=21,now=22)
        self.assertTrue(model.fresh(22));self.assertFalse(model.fresh(25))
    def test_poll_request_id_does_not_replace_visible_controls(self):
        model=PhoneModel();model.update(status(),1);before=model.view_key(1)
        newer=status();newer['id']='c'*32;model.update(newer,2)
        self.assertEqual(before,model.view_key(2))
        self.assertNotEqual(before,model.view_key(6))
        model.update(status(4),6);self.assertNotEqual(before,model.view_key(6))
    def test_initial_and_disconnected_have_no_actions(self):
        model=PhoneModel()
        with self.assertRaises(Unavailable):model.begin('b'*32,3,7,'answer',1)
        model.update(status(),1);model.disconnect()
        self.assertEqual(model.calls,[])
        with self.assertRaises(Unavailable):model.begin('b'*32,3,7,'answer',2)
    def test_stale_snapshot_cannot_dispatch(self):
        model=PhoneModel();model.update(status(),1)
        with self.assertRaises(Unavailable):model.begin('b'*32,3,7,'answer',5)
    def test_old_ui_revision_cannot_dispatch(self):
        model=PhoneModel();model.update(status(4),1)
        with self.assertRaises(Unavailable):model.begin('b'*32,3,7,'answer',2)
    def test_only_one_pending_command(self):
        model=PhoneModel();model.update(status(),1)
        command=model.begin('b'*32,3,7,'answer',2)
        self.assertEqual(command['expected_revision'],3)
        with self.assertRaises(Unavailable):model.begin('b'*32,3,7,'reject',2)
    def test_unknown_outcome_requires_fresh_observation(self):
        model=PhoneModel();model.update(status(),1);model.begin('b'*32,3,7,'answer',2)
        model.finish(None)
        self.assertEqual(model.outcome,'unknown')
        self.assertEqual(model.calls,[])
        with self.assertRaises(Unavailable):model.begin('b'*32,3,7,'answer',2)
    def test_fresh_observation_clears_waiting_message(self):
        model=PhoneModel();model.update(status(),1);model.begin('b'*32,3,7,'answer',2)
        model.finish({'outcome':'submitted','reason':'accepted'});model.update(status(4),3)
        self.assertIsNone(model.outcome);self.assertIsNone(model.reason)
    def test_submitted_is_not_completion(self):
        model=PhoneModel();model.update(status(),1);model.begin('b'*32,3,7,'answer',2)
        model.finish({'outcome':'submitted','reason':'accepted'})
        self.assertEqual(model.outcome,'submitted');self.assertEqual(model.calls,[])

if __name__=='__main__':unittest.main()
