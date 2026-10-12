import sys,tempfile,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_dispatch import Dispatcher
from phone_service import Observation
from phone_ledger import Ledger
from test_control_client import snapshot

class Backend:
    def __init__(self):self.calls=[];self.fail=False
    def controls(self):return snapshot()
    def control(self,**values):
        self.calls.append(values)
        if self.fail:raise OSError('lost reply')
        return dict(v=1,id=values['operation_id'],generation=values['generation'],outcome='submitted',reason='accepted')

class DispatchTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.ledger=Ledger(Path(self.temp.name)/'state');self.addCleanup(self.ledger.close)
        self.backend=Backend();self.now=10
        self.observation=Observation(lambda:self.backend,clock=lambda:self.now)
        self.dispatch=Dispatcher(self.observation,self.ledger,lambda:self.backend)
        session=self.dispatch.begin_session()
        self.observation.poll_once('controls')
        self.request=dict(v=1,id='a'*32,epoch=self.ledger.epoch,session=session['session'],op='control',command=dict(generation='b'*32,call_id=7,expected_revision=3,expected_state=2,action='answer'))
    def test_one_dispatch_and_historical_duplicate_after_state_expires(self):
        first=self.dispatch.execute(self.request)
        self.now=100
        self.assertEqual(self.dispatch.execute(self.request),first)
        self.assertEqual(len(self.backend.calls),1)
        self.assertEqual(self.backend.calls[0]['operation_id'],'a'*32)
    def test_conflicting_duplicate_never_dispatches(self):
        self.dispatch.execute(self.request)
        self.request['command']['action']='reject'
        with self.assertRaises(ValueError):self.dispatch.execute(self.request)
        self.assertEqual(len(self.backend.calls),1)
    def test_stale_and_wrong_epoch_never_dispatch(self):
        self.now=14
        with self.assertRaises(ValueError):self.dispatch.execute(self.request)
        self.now=10;self.request['epoch']='c'*32
        with self.assertRaises(ValueError):self.dispatch.execute(self.request)
        self.assertFalse(self.backend.calls)
    def test_unknown_reply_is_stored_without_retry(self):
        self.backend.fail=True
        self.assertEqual(self.dispatch.execute(self.request)['state'],'unknown')
        self.assertEqual(self.dispatch.execute(self.request)['state'],'unknown')
        self.assertEqual(len(self.backend.calls),1)
    def test_original_revision_not_replaced_and_refresh_required(self):
        self.request['command']['expected_revision']=2
        with self.assertRaises(ValueError):self.dispatch.execute(self.request)
        self.assertFalse(self.backend.calls)
        self.request['command']['expected_revision']=3
        self.dispatch.execute(self.request)
        self.assertFalse(self.observation.snapshot()['controls']['fresh'])
    def test_extra_command_fields_rejected(self):
        self.request['command']['operation_id']='d'*32
        with self.assertRaises(ValueError):self.dispatch.execute(self.request)
        self.assertFalse(self.backend.calls)

if __name__=='__main__':unittest.main()

class SessionFenceTests(unittest.TestCase):
    setUp = DispatchTests.setUp
    def test_unclaimed_session_cannot_admit_legacy_control(self):
        dispatcher=Dispatcher(self.observation,self.ledger,lambda:self.backend)
        with self.assertRaises(ValueError):dispatcher.execute(self.request)
        self.assertFalse(self.backend.calls)
    def test_observation_started_before_ack_cannot_restore_authority(self):
        import threading
        current=self.dispatch.begin_session()
        self.ledger.reserve(self.ledger.epoch,'a'*32,{'action':'answer'})
        self.ledger.finish(self.ledger.epoch,'a'*32,None)
        entered=threading.Event();release=threading.Event()
        original=self.backend.controls
        def delayed():
            entered.set()
            if not release.wait(3):raise TimeoutError('fixture release missing')
            return original()
        self.backend.controls=delayed
        worker=threading.Thread(target=lambda:self.observation.poll_once('controls'));worker.start()
        try:
            self.assertTrue(entered.wait(2))
            self.dispatch.acknowledge(dict(v=1,id='e'*32,op='acknowledge',epoch=self.ledger.epoch,
                session=current['session'],record_epoch=self.ledger.epoch,record_id='a'*32))
        finally:release.set();worker.join(4)
        self.assertFalse(worker.is_alive())
        self.assertFalse(self.observation.snapshot()['controls']['fresh'])
        self.backend.controls=original
        self.observation.poll_once('controls')
        self.assertTrue(self.observation.snapshot()['controls']['fresh'])
        self.assertFalse(self.backend.calls)

    def test_unknown_blocks_new_action_until_acknowledged_and_refreshed(self):
        current=self.dispatch.begin_session()
        self.observation.poll_once('controls')
        original=dict(self.request,session=current['session'])
        self.backend.fail=True
        self.dispatch.execute(original)
        self.observation.poll_once('controls')
        later=dict(original,id='d'*32)
        with self.assertRaisesRegex(ValueError,'recovery'):self.dispatch.execute(later)
        acknowledgment=dict(v=1,id='e'*32,op='acknowledge',epoch=self.ledger.epoch,
                            session=current['session'],record_epoch=self.ledger.epoch,record_id=self.request['id'])
        with self.assertRaises(ValueError):self.dispatch.acknowledge(dict(acknowledgment,session='f'*32))
        self.assertTrue(self.dispatch.acknowledge(acknowledgment))
        self.assertEqual(len(self.backend.calls),1)
        self.assertEqual(self.ledger.result(self.ledger.epoch,self.request['id'])['state'],'unknown')
        with self.assertRaisesRegex(ValueError,'Fresh'):self.dispatch.execute(later)
        self.observation.poll_once('controls')
        self.backend.fail=False
        self.dispatch.execute(later)
        self.assertEqual(len(self.backend.calls),2)

    def test_new_session_fences_late_old_and_unfenced_requests(self):
        old=self.dispatch.begin_session()
        current=self.dispatch.begin_session()
        self.assertNotEqual(old['session'],current['session'])
        self.assertEqual(current['epoch'],self.ledger.epoch)
        self.assertEqual(current['recovery'],{'items':[],'more':False})
        for request in (self.request,dict(self.request,session=old['session'])):
            with self.assertRaises(ValueError):self.dispatch.execute(request)
        self.assertFalse(self.backend.calls)
        self.assertIsNone(self.ledger.result(self.ledger.epoch,self.request['id']))
        self.observation.poll_once('controls')
        self.dispatch.execute(dict(self.request,session=current['session']))
        self.assertEqual(len(self.backend.calls),1)

    def test_takeover_reports_admitted_work_without_cancelling_or_replaying(self):
        import threading
        original=self.backend.control
        entered=threading.Event();release=threading.Event();errors=[]
        def delayed(**values):
            entered.set()
            if not release.wait(3):raise TimeoutError('fixture release missing')
            return original(**values)
        self.backend.control=delayed
        session=self.dispatch.begin_session()
        self.observation.poll_once('controls')
        def command():
            try:self.dispatch.execute(dict(self.request,session=session['session']))
            except Exception as error:errors.append(error)
        worker=threading.Thread(target=command);worker.start()
        try:
            self.assertTrue(entered.wait(2))
            replacement=self.dispatch.begin_session()
            self.assertEqual(replacement['recovery'],{'more':False,'items':[{
                'epoch':self.ledger.epoch,'id':self.request['id'],'state':'pending','result':None}]})
            self.assertFalse(self.observation.snapshot()['controls']['fresh'])
        finally:release.set();worker.join(4)
        self.assertFalse(worker.is_alive());self.assertFalse(errors,errors)
        self.assertEqual(len(self.backend.calls),1)
