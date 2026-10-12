import copy
import sys
import tempfile
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_ledger import Ledger

class RecoveryLedgerTests(unittest.TestCase):
    def test_existing_database_gains_acknowledgments_without_losing_records(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'state';path.mkdir(mode=0o700)
            database=path/'operations.sqlite'
            with sqlite3.connect(database) as db:
                db.execute('CREATE TABLE operations (epoch TEXT NOT NULL,id TEXT NOT NULL,digest TEXT NOT NULL,state TEXT NOT NULL,result TEXT,PRIMARY KEY(epoch,id))')
                db.execute('INSERT INTO operations VALUES (?,?,?,?,NULL)',('a'*32,'b'*32,'c'*64,'pending'))
            database.chmod(0o600)
            ledger=Ledger(path)
            try:
                self.assertEqual(ledger.unresolved()['items'],[{'epoch':'a'*32,'id':'b'*32,'state':'unknown','result':None}])
                self.assertTrue(ledger.acknowledge('a'*32,'b'*32))
                self.assertEqual(ledger.db.execute('SELECT digest,state FROM operations').fetchone(),('c'*64,'unknown'))
            finally:ledger.close()

    def test_acknowledgment_survives_restart_without_changing_outcome_or_replay_guard(self):
        from phone_ledger import LedgerError
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'state'; ledger=Ledger(path)
            epoch=ledger.epoch; identity='a'*32; command={'action':'answer'}
            try:
                ledger.reserve(epoch,identity,command)
                with self.assertRaises(LedgerError):ledger.acknowledge(epoch,identity)
                self.assertEqual(len(ledger.unresolved()['items']),1)
                ledger.finish(epoch,identity,None)
                before=ledger.result(epoch,identity)
                ledger.acknowledge(epoch,identity)
                ledger.acknowledge(epoch,identity)
                self.assertEqual(ledger.result(epoch,identity),before)
                self.assertEqual(ledger.unresolved(),{'items':[],'more':False})
                self.assertFalse(ledger.reserve(epoch,identity,command))
                with self.assertRaises(LedgerError):ledger.reserve(epoch,identity,{'action':'reject'})
                ledger.close();ledger=Ledger(path)
                self.assertEqual(ledger.result(epoch,identity),before)
                self.assertFalse(ledger.unresolved()['items'])
            finally:ledger.close()

    def test_failed_ack_commit_preserves_uncertainty_and_original_record(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as temp:
            ledger=Ledger(Path(temp)/'state')
            try:
                self.assertEqual(ledger.db.execute('PRAGMA journal_mode').fetchone()[0],'delete')
                self.assertEqual(ledger.db.execute('PRAGMA synchronous').fetchone()[0],2)
                epoch=ledger.epoch;identity='e'*32
                ledger.reserve(epoch,identity,{'action':'answer'})
                ledger.finish(epoch,identity,None)
                original=ledger.db.execute('SELECT * FROM operations').fetchall()
                ledger.db.execute('PRAGMA foreign_keys=ON')
                ledger.db.executescript("""
                    CREATE TABLE fault_parent(id INTEGER PRIMARY KEY);
                    CREATE TABLE fault_child(id INTEGER REFERENCES fault_parent(id)
                        DEFERRABLE INITIALLY DEFERRED);
                    CREATE TRIGGER fail_ack_commit AFTER INSERT ON acknowledgments
                    BEGIN INSERT INTO fault_child VALUES (1); END;
                """)
                # The insert succeeds; the deferred constraint fails COMMIT.
                with self.assertRaises(sqlite3.IntegrityError):ledger.acknowledge(epoch,identity)
                self.assertEqual(ledger.db.execute('SELECT * FROM acknowledgments').fetchall(),[])
                self.assertEqual(ledger.db.execute('SELECT * FROM operations').fetchall(),original)
                self.assertEqual(ledger.unresolved()['items'][0]['id'],identity)
                ledger.db.execute('DROP TRIGGER fail_ack_commit')
                self.assertTrue(ledger.acknowledge(epoch,identity))
                self.assertFalse(ledger.unresolved()['items'])
                self.assertEqual(ledger.db.execute('SELECT * FROM operations').fetchall(),original)
            finally:ledger.close()

    def test_acknowledging_page_reveals_hidden_blockers(self):
        with tempfile.TemporaryDirectory() as temp:
            ledger=Ledger(Path(temp)/'state')
            try:
                for n in range(65):
                    ledger.reserve(ledger.epoch,f'{n:032x}',{'action':'answer'})
                    ledger.finish(ledger.epoch,f'{n:032x}',{'outcome':'unknown','reason':'lost_reply'})
                self.assertTrue(ledger.unresolved()['more'])
                for item in ledger.unresolved()['items']:ledger.acknowledge(item['epoch'],item['id'])
                page=ledger.unresolved()
                self.assertFalse(page['more']);self.assertEqual(len(page['items']),1)
                self.assertEqual(page['items'][0]['id'],f'{64:032x}')
            finally:ledger.close()

    def test_restart_discovers_uncertain_operations_without_payload_or_mutation(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'state'
            with_ledger=Ledger(path)
            epoch=with_ledger.epoch
            command={'number':'5550100','action':'answer'}
            for n in range(4):with_ledger.reserve(epoch,f'{n:032x}',command)
            with_ledger.finish(epoch,f'{1:032x}',None)
            with_ledger.finish(epoch,f'{2:032x}',{'outcome':'unknown','reason':'lost_reply'})
            with_ledger.finish(epoch,f'{3:032x}',{'outcome':'submitted','reason':'accepted'})
            with_ledger.close()
            ledger=Ledger(path)
            try:
                before=(path/'operations.sqlite').read_bytes()
                recovery=ledger.unresolved()
                self.assertEqual([r['id'] for r in recovery['items']],[f'{n:032x}' for n in range(3)])
                self.assertFalse(recovery['more'])
                self.assertEqual([r['state'] for r in recovery['items']],['unknown','unknown','finished'])
                self.assertTrue(all(r['epoch']==epoch for r in recovery['items']))
                self.assertNotIn('5550100',str(recovery))
                self.assertEqual(before,(path/'operations.sqlite').read_bytes())
                recovery['items'].clear()
                self.assertEqual(len(ledger.unresolved()['items']),3)
            finally:ledger.close()

    def test_overflow_is_explicit_not_false_complete(self):
        with tempfile.TemporaryDirectory() as temp:
            ledger=Ledger(Path(temp)/'state')
            try:
                for n in range(65):ledger.reserve(ledger.epoch,f'{n:032x}',{'action':'answer'})
                recovery=ledger.unresolved()
                self.assertEqual(len(recovery['items']),64)
                self.assertTrue(recovery['more'])
                self.assertTrue(all(r['state']=='pending' for r in recovery['items']))
            finally:ledger.close()

class RecoveryClientTests(unittest.TestCase):
    def response(self):
        return {'v':1,'id':'a'*32,'recovery':{'epoch':'b'*32,'more':False,'items':[
            {'epoch':'c'*32,'id':'d'*32,'state':'unknown','result':None}]}}
    def test_response_requires_matching_identity_bounded_unresolved_records(self):
        from phone_local import validate_recovery
        good=self.response()
        self.assertEqual(validate_recovery(good,'a'*32),good['recovery'])
        invalid=[]
        for key,value in [('epoch','bad'),('more',1),('items',[good['recovery']['items'][0]]*65)]:
            r=copy.deepcopy(good);r['recovery'][key]=value;invalid.append(r)
        r=copy.deepcopy(good);r['id']='e'*32;invalid.append(r)
        r=copy.deepcopy(good);r['recovery']['items']*=2;invalid.append(r)
        r=copy.deepcopy(good);r['recovery']['items'][0]['result']={'outcome':'submitted','reason':'accepted'};r['recovery']['items'][0]['state']='finished';invalid.append(r)
        r=copy.deepcopy(good);r['recovery']['items'][0]['number']='5550100';invalid.append(r)
        for value in invalid:
            with self.subTest(value=value),self.assertRaises(ValueError):validate_recovery(value,'a'*32)

    def test_session_response_validates_nested_recovery_and_identity(self):
        from phone_local import validate_session
        descriptor={'epoch':'b'*32,'session':'c'*32,'recovery':{'items':[],'more':False}}
        good={'v':1,'id':'a'*32,'session':descriptor}
        self.assertEqual(validate_session(good,'a'*32),descriptor)
        invalid=[]
        for field,value in [('epoch','bad'),('session',None),('recovery',{'items':[],'more':True})]:
            response=copy.deepcopy(good);response['session'][field]=value;invalid.append(response)
        response=copy.deepcopy(good);response['id']='d'*32;invalid.append(response)
        response=copy.deepcopy(good);response['session']['extra']=True;invalid.append(response)
        for value in invalid:
            with self.subTest(value=value),self.assertRaises(ValueError):validate_session(value,'a'*32)

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux peer credentials required')
class RecoverySocketTests(unittest.TestCase):
    def test_lost_android_reply_is_discovered_after_restart_without_replay(self):
        import time
        from phone_dispatch import Dispatcher
        from phone_service import LocalService,Observation
        from phone_local import control,recover,begin_session
        from test_phone_dispatch import Backend
        from test_dial_client import status
        backend=Backend();backend.fail=True;backend.dial_status=status
        with tempfile.TemporaryDirectory(prefix='recover-',dir='/tmp') as temp:
            path=Path(temp)
            original_epoch=None
            for attempt in range(2):
                ledger=Ledger(path/'state')
                observation=Observation(lambda:backend,interval=.02)
                service=LocalService(path/'runtime',observation,Dispatcher(observation,ledger,lambda:backend))
                try:
                    service.start()
                    if attempt==0:
                        authority=begin_session(service.path)
                        deadline=time.monotonic()+2
                        while not observation.snapshot()['controls']['fresh']:
                            self.assertLess(time.monotonic(),deadline);time.sleep(.01)
                        original_epoch=ledger.epoch
                        result=control(service.path,original_epoch,'a'*32,dict(generation='b'*32,
                            call_id=7,expected_revision=3,expected_state=2,action='answer'),session=authority['session'])
                        self.assertEqual(result,{'state':'unknown','result':None})
                    else:self.assertNotEqual(ledger.epoch,original_epoch)
                    for _ in range(2):
                        result=recover(service.path)
                        self.assertEqual(result,{'epoch':ledger.epoch,'more':False,'items':[
                            {'epoch':original_epoch,'id':'a'*32,'state':'unknown','result':None}]})
                    self.assertEqual(len(backend.calls),1)
                finally:service.close();ledger.close()

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux peer credentials required')
class SessionSocketTests(unittest.TestCase):
    def test_cli_acknowledges_without_dispatch_and_preserves_unknown(self):
        import json,subprocess,time
        import phone_local
        from phone_service import LocalService,Observation
        from phone_dispatch import Dispatcher
        from test_phone_dispatch import Backend
        from test_dial_client import status
        backend=Backend();backend.dial_status=status
        with tempfile.TemporaryDirectory(prefix='ack-',dir='/tmp') as temp:
            root=Path(temp);ledger=Ledger(root/'state')
            ledger.reserve(ledger.epoch,'a'*32,{'action':'answer'})
            ledger.finish(ledger.epoch,'a'*32,None)
            observation=Observation(lambda:backend,interval=.02)
            service=LocalService(root/'runtime',observation,Dispatcher(observation,ledger,lambda:backend))
            try:
                service.start()
                client=[sys.executable,str(Path(phone_local.__file__)),'--socket',str(service.path)]
                session=subprocess.run(client+['session'],capture_output=True,text=True,timeout=5)
                self.assertEqual(session.returncode,0,session.stderr)
                authority=json.loads(session.stdout)['session']
                args=['acknowledge','--epoch',ledger.epoch,'--session',authority['session'],
                      '--record-epoch',ledger.epoch,'--record-id','a'*32]
                for _ in range(2):
                    result=subprocess.run(client+args,capture_output=True,text=True,timeout=5)
                    self.assertEqual(result.returncode,0,result.stderr)
                    self.assertTrue(json.loads(result.stdout)['acknowledged'])
                self.assertFalse(backend.calls)
                self.assertFalse(phone_local.recover(service.path)['items'])
                self.assertEqual(ledger.result(ledger.epoch,'a'*32)['state'],'unknown')
            finally:service.close();ledger.close()

    def test_takeover_fences_old_cli_admission_and_requires_fresh_observation(self):
        import time
        import phone_local
        from phone_service import LocalService,Observation
        from phone_dispatch import Dispatcher
        from test_phone_dispatch import Backend
        from test_dial_client import status
        backend=Backend();backend.dial_status=status
        with tempfile.TemporaryDirectory(prefix='session-',dir='/tmp') as temp:
            root=Path(temp);ledger=Ledger(root/'state')
            observation=Observation(lambda:backend,interval=.02)
            service=LocalService(root/'runtime',observation,Dispatcher(observation,ledger,lambda:backend))
            try:
                service.start()
                old=phone_local.begin_session(service.path)
                new=phone_local.begin_session(service.path)
                self.assertNotEqual(old['session'],new['session'])
                command=dict(generation='b'*32,call_id=7,expected_revision=3,expected_state=2,action='answer')
                with self.assertRaises(ValueError):phone_local.control(service.path,ledger.epoch,'a'*32,command,session=old['session'])
                with self.assertRaises(ValueError):phone_local.control(service.path,ledger.epoch,'a'*32,command)
                self.assertFalse(backend.calls)
                deadline=time.monotonic()+2
                while not observation.snapshot()['controls']['fresh']:
                    self.assertLess(time.monotonic(),deadline);time.sleep(.01)
                self.assertEqual(phone_local.control(service.path,ledger.epoch,'a'*32,command,session=new['session'])['state'],'finished')
                self.assertEqual(len(backend.calls),1)
            finally:service.close();ledger.close()

if __name__=='__main__':unittest.main()
