"""Two fixed syscall stops on one caller-retained normal worker; no payload/memory reads."""
import ctypes as C, errno, os, select, signal, struct, time
from pathlib import Path
from owned import now
L=C.CDLL(None,use_errno=True);L.ptrace.argtypes=[C.c_ulong,C.c_ulong,C.c_void_p,C.c_void_p];L.ptrace.restype=C.c_long
SEIZE=0x4206;INTERRUPT=0x4207;DETACH=17;SYSCALL=24;GETINFO=0x420e
TRACESYSGOOD=1;TRACEEXIT=0x40;EXITKILL=0x100000;EVENT_EXIT=6;EVENT_STOP=128;WALL=0x40000000
ARCH=0xc000003e;GETTID=186;RECVMSG=47;SENDTO=44;SENDMSG=46;SLEEPS={35,230}
def pc(request,pid,address=0,data=0):
 C.set_errno(0);result=L.ptrace(request,pid,C.c_void_p(address),C.c_void_p(data))
 if result==-1 and C.get_errno():raise OSError(C.get_errno(),'ptrace fixed owned worker')
 return result

def private_channel(pid):
 # Procfs metadata only. Read neither process memory nor any socket payload.
 kinds={}
 for line in Path(f'/proc/{pid}/net/unix').read_text().splitlines()[1:]:
  fields=line.split();kinds[int(fields[6])]=int(fields[4],16)
 found=[]
 for p in Path(f'/proc/{pid}/fd').iterdir():
  try:name=os.readlink(p)
  except FileNotFoundError:continue
  if name.startswith('socket:[')and kinds.get(int(name[8:-1]))==5:found.append({'fd':int(p.name),'inode':int(name[8:-1])})
 assert len(found)==1,'exactly one retained AF_UNIX/SEQPACKET worker channel required'
 return found[0]

class WorkerTrace:
 def __init__(self,pid,pidfd,main_pid,scope,until,close_until):
  self.pid=pid;self.pidfd=pidfd;self.main_pid=main_pid;self.scope=scope;self.until=until;self.close_until=close_until;assert until<close_until;self.active=False;self.stopped=False;self.pending=None;self.steps=0
  assert not select.select([pidfd],[],[],0)[0]
  status=Path(f'/proc/{pid}/status').read_text();assert int(next(v.split()[1]for v in status.splitlines()if v.startswith('PPid:')))==main_pid
  assert Path(f'/proc/{main_pid}/task/{main_pid}/children').read_text().split()==[str(pid)]
  self.channel=private_channel(pid)
  self.record={'traced_pid':pid,'actual_parent_main_pid':main_pid,'one_fixture_owned_worker_only':True,'channel':self.channel,'packet_payload_or_process_memory_read':False,'syscall_results_changed':False,'effect_resubmission':False,'original_until_ns':until,'original_close_until_ns':close_until,'trace_stops':[]}
 def attach(self):
  assert now()<self.until
  pc(SEIZE,self.pid,0,TRACESYSGOOD|TRACEEXIT|EXITKILL);self.active=True;pc(INTERRUPT,self.pid)
  status=self.wait();assert os.WIFSTOPPED(status)and status>>16==EVENT_STOP
  assert not select.select([self.pidfd],[],[],0)[0]and int(next(v.split()[1]for v in Path(f'/proc/{self.pid}/status').read_text().splitlines()if v.startswith('PPid:')))==self.main_pid
  self.record['actual_seize_interrupt_stop']=True;self.resume()
 def wait(self,cleanup=False):
  bound=self.close_until if cleanup else self.until
  while now()<bound:
   self.scope.pump()
   if not cleanup:self.scope.check()
   got,status=os.waitpid(self.pid,os.WNOHANG|WALL)
   if got:
    assert got==self.pid
    if not os.WIFSTOPPED(status):
     self.active=False;self.record['unexpected_terminal_tracer_wait']=os.waitstatus_to_exitcode(status)
     raise RuntimeError('tracer unexpectedly consumed terminal wait; normal Main ownership unqualified')
    self.stopped=True;return status
   select.select([],[],[],.00005)
  raise TimeoutError('original fixed trace cleanup cutoff' if cleanup else 'original fixed trace cutoff')
 def resume(self,delivery=0):
  assert self.active and self.stopped and now()<self.until
  pc(SYSCALL,self.pid,0,delivery);self.stopped=False
 def detach(self):
  assert self.active and self.stopped and now()<self.close_until
  pc(DETACH,self.pid);self.active=False;self.stopped=False
  self.record.update(detached_at_actual_nonterminal_stop=True,detached_at_ns=now(),normal_Main_retains_terminal_wait=True,syscall_stops=self.steps)
 def info(self):
  data=C.create_string_buffer(128);size=pc(GETINFO,self.pid,128,C.addressof(data));raw=data.raw
  assert struct.unpack_from('I',raw,4)[0]==ARCH
  self.steps+=1;assert self.steps<=8192,'fixed syscall-stop bound'
  op=raw[0]
  if op==1:
   assert size>=80;return op,struct.unpack_from('Q',raw,24)[0],struct.unpack_from('6Q',raw,32)
  assert op==2 and size>=33;return op,struct.unpack_from('q',raw,24)[0],None
 def hold(self,nanoseconds,label):
  assert self.stopped and now()<self.until
  began=now();resume_at=began+nanoseconds;assert resume_at<self.until
  row={'label':label,'kernel_stopped_at_ns':began,'requested_hold_ns':nanoseconds,'original_resume_at_ns':resume_at}
  while now()<resume_at:
   self.scope.pump();self.scope.check();select.select([],[],[],min(.001,max(0,(resume_at-now())/1e9)))
  row['actual_resume_started_at_ns']=now();self.record['trace_stops'].append(row);return row
 def run(self,mode,admit_apply=None):
  self.attach();phase='observation';sleep_entry=None;held=False;after_hold_receives=[];outstanding=None
  while now()<self.until:
   status=self.wait();sig=os.WSTOPSIG(status);event=status>>16
   if event==EVENT_EXIT:
    self.record.update(exit_event_before_terminal_wait=True,post_hold_private_receives=after_hold_receives)
    self.detach();return self.record
   if sig!=(signal.SIGTRAP|0x80):
    assert event==0,(sig,event);self.resume(0 if sig==signal.SIGTRAP else sig);continue
   op,value,args=self.info()
   if op==1:
    self.pending={'nr':value,'args':args,'entry_ns':now()}
    if mode=='consumed'and not held and outstanding is None and phase=='next-thread-check'and value==GETTID:
     assert admit_apply is not None
     self.record['consumed_gate_attribution']={'prior_private_observation_send':self.record['observation_send'],'loop_sleep_exit_ns':self.record['loop_sleep_exit_ns'],'stopped_syscall':'gettid entry','kernel_stop_entry_ns':self.pending['entry_ns'],'current_source_stage':'old ReceiveCommand pre-auth or candidate readiness local identity check; before private recvmsg'}
     outstanding=admit_apply();self.record['one_apply']=outstanding
     self.record['consumed_gate_attribution']['actual_submit_returned_at_ns']=now()
    if value==RECVMSG and args[0]==self.channel['fd']:
     if held:after_hold_receives.append({'entry_ns':now(),'fd':args[0]})
    if mode=='idle'and not held and phase=='next-thread-check'and value==GETTID:
     self.record['idle_gate_attribution']={'prior_private_observation_send':self.record['observation_send'],'loop_sleep_exit_ns':self.record['loop_sleep_exit_ns'],'stopped_syscall':'gettid entry','current_source_stage':'ReceiveCommand pre-auth after50ms envelope construction; before recvmsg','candidate_source_stage':'readiness local identity check before50ms construction'}
     self.hold(80_000_000,'empty-idle-before-auth');held=True
    if held and value in SLEEPS and self.record.get('post_hold_observation_send'):
     self.record['post_hold_private_receives']=after_hold_receives;self.detach();return self.record
   else:
    entry=self.pending;self.pending=None
    if entry is not None:
     nr=entry['nr'];a=entry['args'];at=now()
     if nr in [SENDTO,SENDMSG]and a[0]==self.channel['fd']and value==47:
      row={'syscall':nr,'fd':a[0],'bytes':value,'entry_ns':entry['entry_ns'],'exit_ns':at,'contents_inspected':False}
      if not held:self.record['observation_send']=row;phase='loop-sleep'
      elif held:self.record['post_hold_observation_send']=row
     if nr in SLEEPS and value==0 and phase=='loop-sleep':self.record['loop_sleep_exit_ns']=at;phase='next-thread-check'
     if nr==RECVMSG and a[0]==self.channel['fd']:
      if held and after_hold_receives:after_hold_receives[-1].update(return_value=value,exit_ns=at)
      if mode=='consumed'and outstanding is not None and not held:
       assert value==19,'queued exact command was not consumed; no retry'
       assert at<outstanding['original_until_ns'],'command already expired before selected consumption stop'
       self.record['actual_consumed_frame']={'fd':a[0],'bytes':value,'exit_stop_at_ns':at,'original_action_until_ns':outstanding['original_until_ns'],'contents_inspected':False}
       self.hold(80_000_000,'consumed-receive-envelope-expiry');held=True
   self.resume()
  raise TimeoutError('original bounded trace exhausted')
 def close(self):
  if not self.active:return
  # Caller still owns Main/worker pidfd and waits/EOF. Retire tracing at a real
  # stop before Main closure; never claim a stopped worker is reaped.
  if not self.stopped:
   try:pc(INTERRUPT,self.pid)
   except ProcessLookupError:pass # An EXIT stop may already await our exact wait.
   self.wait(cleanup=True)
  self.record['failure_trace_detachment']=True;self.detach()
