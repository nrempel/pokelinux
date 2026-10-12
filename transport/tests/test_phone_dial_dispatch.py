import copy,sys,tempfile,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_dispatch import Dispatcher
from phone_service import Observation
from phone_ledger import Ledger
from test_phone_dispatch import Backend
from test_dial_client import status

class DialDispatchTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.ledger=Ledger(Path(self.temp.name)/'state');self.addCleanup(self.ledger.close)
        self.backend=Backend();self.backend.dial_status=status;self.sent=[];self.outcome='invocation_attempted'
        def dial(**command):
            self.sent.append(command)
            if self.outcome=='lost':raise OSError('lost reply')
            return dict(v=1,id=command['operation_id'],generation=command['generation'],outcome=self.outcome,
                        reason=('dispatch_error' if self.outcome=='outcome_unknown' else 'stale_offer' if self.outcome=='not_invoked' else 'returned'))
        self.backend.dial=dial
        self.observation=Observation(lambda:self.backend)
        self.dispatch=Dispatcher(self.observation,self.ledger,lambda:self.backend)
        session=self.dispatch.begin_session();self.observation.poll_once('dial_status')
        value=self.observation.snapshot()['dial_status']['value']
        self.request=dict(v=1,id='a'*32,epoch=self.ledger.epoch,session=session['session'],op='dial',
            command=dict(generation=value['generation'],offer=value['offer'],number='5550100'))
    def test_dial_once_retains_invocation_semantics_and_payload_guard(self):
        result=self.dispatch.execute(self.request)
        self.assertEqual(result['result'],{'outcome':'invocation_attempted','reason':'returned'})
        self.assertEqual(self.dispatch.execute(self.request),result)
        self.assertEqual(len(self.sent),1)
        changed=copy.deepcopy(self.request);changed['command']['number']='5550101'
        with self.assertRaises(ValueError):self.dispatch.execute(changed)
        self.assertNotIn(b'5550100',(self.ledger.directory/'operations.sqlite').read_bytes())
        self.assertFalse(self.observation.snapshot()['dial_status']['fresh'])
    def test_new_status_offer_does_not_replace_original_user_offer(self):
        original=self.request['command']['offer']
        def newer():
            value=status();value['offer']='f'*32;return value
        self.backend.dial_status=newer
        self.observation.poll_once('dial_status')
        self.dispatch.execute(self.request)
        self.assertEqual(self.sent[0]['offer'],original)
        self.assertNotEqual(self.sent[0]['offer'],self.observation.snapshot()['dial_status']['value']['offer'])

    def test_android_not_invoked_is_preserved_without_unknown_recovery(self):
        self.outcome='not_invoked'
        result=self.dispatch.execute(self.request)
        self.assertEqual(result['result'],{'outcome':'not_invoked','reason':'stale_offer'})
        self.assertFalse(self.ledger.unresolved()['items'])
        self.assertEqual(self.dispatch.execute(self.request),result)
        self.assertEqual(len(self.sent),1)

    def test_wrong_generation_and_stale_session_never_admit(self):
        bad=copy.deepcopy(self.request);bad['command']['generation']='f'*32
        with self.assertRaises(ValueError):self.dispatch.execute(bad)
        self.dispatch.begin_session()
        self.assertFalse(self.observation.snapshot()['dial_status']['fresh'])
        with self.assertRaises(ValueError):self.dispatch.execute(self.request)
        self.assertEqual(self.sent,[])
    def test_explicit_remote_uncertainty_blocks_new_dial_until_acknowledged(self):
        self.outcome='outcome_unknown'
        self.dispatch.execute(self.request)
        recovered=self.ledger.unresolved()['items']
        self.assertEqual(recovered[0]['result'],{'outcome':'unknown','reason':'dispatch_error'})
        self.observation.poll_once('dial_status')
        with self.assertRaises(ValueError):self.dispatch.execute(dict(self.request,id='c'*32))
        self.assertEqual(len(self.sent),1)
        self.dispatch.acknowledge(dict(v=1,id='f'*32,op='acknowledge',epoch=self.ledger.epoch,
            session=self.request['session'],record_epoch=self.ledger.epoch,record_id=self.request['id']))
        self.assertFalse(self.observation.snapshot()['dial_status']['fresh'])
        with self.assertRaises(ValueError):self.dispatch.execute(dict(self.request,id='c'*32))
        self.observation.poll_once('dial_status');self.outcome='invocation_attempted'
        self.dispatch.execute(dict(self.request,id='c'*32))
        self.assertEqual(len(self.sent),2)
    def test_dial_and_control_share_pending_admission(self):
        import threading
        for first_dial in (True,False):
            with self.subTest(first_dial=first_dial):
                self.observation.poll_once('controls');self.observation.poll_once('dial_status')
                control=dict(self.request,op='control',id=('b' if first_dial else 'c')*32,
                    command=dict(generation='b'*32,call_id=7,expected_revision=3,expected_state=2,action='answer'))
                dial=dict(self.request,id=('d' if first_dial else 'e')*32)
                name='dial' if first_dial else 'control';original=getattr(self.backend,name)
                entered=threading.Event();release=threading.Event();errors=[]
                def held(**command):
                    entered.set()
                    if not release.wait(3):raise TimeoutError('fixture release missing')
                    return original(**command)
                setattr(self.backend,name,held)
                def execute():
                    try:self.dispatch.execute(dial if first_dial else control)
                    except Exception as error:errors.append(error)
                worker=threading.Thread(target=execute);worker.start()
                try:
                    self.assertTrue(entered.wait(2))
                    # Even refreshed authority cannot overlap the other operation.
                    self.observation.poll_once('controls');self.observation.poll_once('dial_status')
                    with self.assertRaisesRegex(ValueError,'pending'):
                        self.dispatch.execute(control if first_dial else dial)
                finally:release.set();worker.join(4);setattr(self.backend,name,original)
                self.assertFalse(worker.is_alive());self.assertEqual(errors,[])
        self.assertEqual(len(self.sent),1);self.assertEqual(len(self.backend.calls),1)

    def test_lost_reply_is_durable_unknown_without_retry(self):
        self.outcome='lost'
        self.assertEqual(self.dispatch.execute(self.request),{'state':'unknown','result':None})
        self.dispatch.execute(self.request)
        self.assertEqual(len(self.sent),1)

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux peer credentials required')
class NativeDialCliTests(unittest.TestCase):
    setUp=DialDispatchTests.setUp
    def test_cli_dial_and_duplicate_through_real_socket(self):
        import json,subprocess,time
        import phone_local
        from phone_service import LocalService
        service=LocalService(Path(self.temp.name)/'runtime',self.observation,self.dispatch)
        try:
            service.start()
            command=[sys.executable,str(Path(phone_local.__file__)),'--socket',str(service.path),'dial',
                '--epoch',self.request['epoch'],'--id',self.request['id'],'--session',self.request['session'],
                '--generation',self.request['command']['generation'],'--offer',self.request['command']['offer'],
                '--number',self.request['command']['number']]
            for _ in range(2):
                result=subprocess.run(command,capture_output=True,text=True,timeout=5)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertEqual(json.loads(result.stdout)['operation']['result']['outcome'],'invocation_attempted')
            self.assertEqual(len(self.sent),1)
            observed=phone_local.operation_result(service.path,self.request['epoch'],self.request['id'])
            self.assertEqual(observed['result']['outcome'],'invocation_attempted')
        finally:service.close()
