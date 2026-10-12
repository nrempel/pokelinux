"""Synthetic Android adapter; real installed daemon, ledger and Unix protocol."""
import json,signal,sys,threading,time,uuid
from pathlib import Path
sys.path.insert(0,'/usr/lib/omarchy-phone')
from phone_service import Observation,LocalService
from phone_dispatch import Dispatcher
from phone_ledger import Ledger
base=Path('/tmp/phone-shell-fixture')
class Backend:
    def __init__(self):self.lock=threading.RLock();self.state=0 if '--idle' in sys.argv else 2;self.revision=3;self.sent=[]
    def controls(self):
        with self.lock:
            calls=[] if self.state==0 else [dict(id=7,state=self.state,actions=['answer','reject'] if self.state==2 else ['disconnect'])]
            return dict(v=1,id=uuid.uuid4().hex,generation='c'*32,revision=self.revision,controls_allowed=True,
                        availability='observed' if calls else 'unavailable',source='android.telecom.InCallService',calls=calls)
    def dial_status(self):return dict(v=1,id=uuid.uuid4().hex,generation='c'*32,can_dial=False,offer='',pending=False,reason='disabled')
    def control(self,**command):
        with self.lock:
            assert command['expected_revision']==self.revision and command['expected_state']==self.state
            self.sent.append(command)
            self.state=4 if command['action']=='answer' else 0
            self.revision+=1
            (base/'transmissions.json').write_text(json.dumps(self.sent))
            if '--hold-answer' in sys.argv and command['action']=='answer':
                deadline=time.monotonic()+90
                while not (base/'answer-release').exists():
                    if time.monotonic()>deadline:raise TimeoutError('test did not release admitted Answer')
                    time.sleep(.05)
            if '--recovery' in sys.argv and command['action']=='answer':
                raise OSError('synthetic reply loss after call state changed')
            return dict(v=1,id=command['operation_id'],generation=command['generation'],outcome='submitted',reason='accepted')
backend=Backend();ledger=Ledger(base/'state');observation=Observation(lambda:backend,interval=.1)
if '--seed-history' in sys.argv:
    ledger.reserve(ledger.epoch,'e'*32,{'action':'answer'})
    ledger.finish(ledger.epoch,'e'*32,{'outcome':'submitted','reason':'accepted'})
if '--recovery' in sys.argv:
    ledger.reserve(ledger.epoch,'d'*32,{'action':'answer'})
    ledger.finish(ledger.epoch,'d'*32,None)
runtime=Path(sys.argv[1]) if len(sys.argv)>1 else base/'runtime'
service=LocalService(runtime,observation,Dispatcher(observation,ledger,lambda:backend))
stop=threading.Event()
signal.signal(signal.SIGTERM,lambda *_:stop.set())
try:
    service.start();(base/'ready').write_text('ready')
    while not stop.wait(.1):
        if (base/'incoming').exists():
            (base/'incoming').unlink()
            with backend.lock:backend.state=2;backend.revision+=1
finally:service.close();ledger.close()
