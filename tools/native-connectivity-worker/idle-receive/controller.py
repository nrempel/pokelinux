"""RED-only normal host Main/worker receive controls in one fresh disposable namespace."""
from pathlib import Path
import argparse,array,hashlib,json,os,re,select,signal,socket,stat,struct,sys,time,traceback
sys.dont_write_bytecode=True
sys.path.insert(0,'/runner')
from owned import Scope,now
from closure import close_owned
import peer
from worker_trace import WorkerTrace
p=argparse.ArgumentParser();p.add_argument('--case',choices=['idle','consumed','parent-death'],required=True);p.add_argument('--main',type=Path,required=True);p.add_argument('--worker',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--work-until-ns',type=int,required=True);p.add_argument('--close-until-ns',type=int,required=True);a=p.parse_args()
T=now();W=a.work_until_ns;C=a.close_until_ns;assert T<W<C and os.getuid()==0 and os.environ.get('POKELINUX_CONNECTIVITY_ISOLATED')=='1'
assert os.uname().machine=='x86_64' and socket.if_nameindex()==[(1,'lo')]
assert not a.output.exists();a.output.mkdir(mode=0o700);O=a.output;scope=Scope(O);fds=[];control=other=None;keeper=main=None;worker_pidfd=None;trace=None;sockets=[];request_id=0
r={'schema':1,'case':a.case,'passed':False,'prerequisites_qualified':False,'behavior_RED_qualified':False,'Settings_or_VM_or_phone_executed':False,'actual_effect_requests':0,'retry_or_reconnect_or_rollback':False,'original_work_until_ns':W,'original_close_until_ns':C,'cleanup_errors':[]}
def status(pid):
 t=Path(f'/proc/{pid}/status').read_text();return {v.split(':',1)[0]:v.split(':',1)[1].strip()for v in t.splitlines()if v.startswith(('Pid:','PPid:','Uid:','Gid:','CapInh:','CapPrm:','CapEff:','CapBnd:','CapAmb:','NoNewPrivs:'))}
def socket_new(end):
 assert now()<end;s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC);sockets.append(s);s.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);s.connect(str(endpoint));cred=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert cred==(main['child'].pid,0,0);assert now()<end;return s
def send(s,body,end):
 assert now()<end;raw=json.dumps(body,separators=(',',':')).encode();assert len(raw)<=1024 and s.send(raw)==len(raw);assert now()<end
 return {'request':body,'sent_at_ns':now(),'original_until_ns':end}
def receive(s,end):
 while now()<end:
  scope.pump();scope.check()
  try:body,anc,flags,_=s.recvmsg(1024,socket.CMSG_SPACE(12),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
  except BlockingIOError:select.select([s],[],[],min(.001,max(0,(end-now())/1e9)));continue
  assert body and not flags&~socket.MSG_CMSG_CLOEXEC and len(anc)==1
  level,kind,data=anc[0];assert(level,kind)==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS)and struct.unpack('3i',data)==(main['child'].pid,0,0)
  value=json.loads(body);assert set(value)=={'schema','id','status','state'}and value['schema']==1 and now()<end
  return value
 raise TimeoutError('original private inspect/operation deadline')
def inspect(end):
 global request_id
 until=min(end,now()+1_000_000_000);request_id+=1;s=socket_new(until)
 try:send(s,{'op':'inspect','id':request_id,'until_ns':str(until)},until);v=receive(s,until);assert v['id']==request_id;return v
 finally:s.close()
def wait_ready(end):
 while now()<end:
  if endpoint.exists():
   try:v=inspect(end)
   except (ConnectionRefusedError,BlockingIOError):
    scope.pump();select.select([],[],[],.001);continue
   if v['state']['available']:
    assert v['status']=='observed'and v['state']['configured']is False and not v['state']['pending'];return v
  assert scope.live(main),'normal Main died before startup ready';scope.pump();select.select([],[],[],.001)
 raise TimeoutError('original3s startup readiness')
def pump_until(end):
 while now()<end:scope.pump();scope.check();select.select([],[],[],min(.001,max(0,(end-now())/1e9)))
def independent(end):return peer.ipv4_snapshot('pklown0',end)
def closure_record():
 rows=[]
 for line in (O/'normal-main.log').read_text().splitlines():
  try:v=json.loads(line)
  except json.JSONDecodeError:continue
  if v.get('event')=='connectivity_main_closure':rows.append(v)
 assert len(rows)==1;v=rows[0];assert v['closed']and v['reaped']and v['output_eof']and not v['occupied']and v['namespace_handles_open'];return v
try:
 peer.call('prctl',36,1,0,0,0)
 local=os.open('/proc/thread-self/ns/net',os.O_RDONLY|os.O_CLOEXEC);fds.append(local);r['current_namespace']=peer.namespace(local)
 for exe in [a.main,a.worker]:
  assert exe.is_absolute()and exe.is_file();b=exe.read_bytes();assert b[:6]==b'\x7fELF\x02\x01'and int.from_bytes(b[18:20],'little')==62;r.setdefault('normal_ELFs',{})[exe.name]={'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
 control,other=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET)
 keeper=scope.start(['/usr/bin/python3','-B',str(Path(peer.__file__).resolve()),'keeper',str(other.fileno()),str(C)],'namespace-keeper.log',65536,cutoff=W,wait_cutoff=C,identity={'pass_fds':(other.fileno(),)});other.close();other=None
 while not select.select([control],[],[],.001)[0]:scope.pump();assert now()<W and scope.live(keeper)
 body,anc,flags,_=control.recvmsg(4096,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC)
 for level,kind,data in anc:
  assert(level,kind)==(socket.SOL_SOCKET,socket.SCM_RIGHTS);received=array.array('i');received.frombytes(data);fds.extend(received)
 assert not flags&~socket.MSG_CMSG_CLOEXEC and len(anc)==1 and len(fds)==2
 r['peer_keeper']=json.loads(body);assert peer.namespace(fds[1])==r['peer_keeper']['namespace']!=r['current_namespace']
 case=min(W,now()+6_000_000_000);r['original_case_until_ns']=case;start_until=min(case,now()+3_000_000_000);r['original_start_until_ns']=start_until
 private=Path('/run/fixture/'+a.case);private.mkdir(mode=0o700);journal=private/'journal';journal.mkdir(mode=0o700);endpoint=private/'control.sock';peerpath=f"/proc/{keeper['child'].pid}/ns/net"
 argv=[str(a.main),str(endpoint),peerpath,str(journal/'admitted'),str(a.worker),str(start_until)]
 main=scope.start(argv,'normal-main.log',65536,cutoff=start_until,wait_cutoff=C,identity={'preexec_fn':lambda:peer.reduce_caps((1<<peer.NET_ADMIN)|(1<<8))})
 initial=wait_ready(start_until);r['initial_private_inspect']=initial
 children=Path(f"/proc/{main['child'].pid}/task/{main['child'].pid}/children").read_text().split();assert len(children)==1;worker_pid=int(children[0]);worker_pidfd=os.pidfd_open(worker_pid,os.O_NONBLOCK)
 actual_exe=os.stat(f'/proc/{worker_pid}/exe');expected_exe=os.stat(a.worker);assert(actual_exe.st_dev,actual_exe.st_ino)==(expected_exe.st_dev,expected_exe.st_ino),'actual worker executable inode differs'
 r['worker_pid']=worker_pid;r['Main_status']=status(main['child'].pid);r['worker_status']=status(worker_pid)
 assert all(int(r['Main_status'][k],16)==0 for k in ['CapEff','CapPrm','CapBnd','CapInh','CapAmb'])and r['Main_status']['NoNewPrivs']=='1'
 assert int(r['worker_status']['PPid'])==main['child'].pid and all(int(r['worker_status'][k],16)==1<<peer.NET_ADMIN for k in ['CapEff','CapPrm','CapBnd'])and all(int(r['worker_status'][k],16)==0 for k in ['CapInh','CapAmb'])and r['worker_status']['NoNewPrivs']=='1'
 r['initial_independent_readback']=independent(min(case,now()+1_000_000_000));assert r['initial_independent_readback']['ipv4']==[];r['prerequisites_qualified']=True
 if a.case in ['idle','consumed']:
  apply_socket=None
  def apply_once():
   global request_id,apply_socket
   assert r['actual_effect_requests']==0
   until=min(case,now()+100_000_000);request_id+=1;apply_socket=socket_new(until)
   sent=send(apply_socket,{'op':'apply','id':request_id,'choice':'connect','until_ns':str(until)},until);r['actual_effect_requests']=1
   while now()<until:
    v=inspect(until)
    if v['state']['pending']:
     assert v['state']['available']and v['state']['configured']is False and v['state']['worker']==initial['state']['worker'];sent['actual_Main_pending_before_receiver_resume']=v;sent['original_until_ns']=until;return sent
   raise TimeoutError('no confirmed single dispatched request before its original100ms cutoff')
  assert sorted(Path(f'/proc/self/task/{os.getpid()}/children').read_text().split())==sorted([str(main['child'].pid),str(keeper['child'].pid)]),'only the retained Main/keeper may be direct children before fixed trace'
  assert len(list(Path(f'/proc/{worker_pid}/task').iterdir()))==len(list(Path(f'/proc/{main["child"].pid}/task').iterdir()))==1,'selected normal single-threaded Main/worker required'
  trace=WorkerTrace(worker_pid,worker_pidfd,main['child'].pid,scope,case,C)
  r['trace']=trace.record
  trace.run(a.case,apply_once if a.case=='consumed'else None)
  if trace.record.get('exit_event_before_terminal_wait'):
   while not select.select([worker_pidfd],[],[],0)[0]:
    assert now()<case and scope.live(main);scope.pump();select.select([worker_pidfd],[],[],min(.001,max(0,(case-now())/1e9)))
  elif trace.record.get('post_hold_observation_send'):
   while now()<case and not select.select([worker_pidfd],[],[],0)[0]:
    v=inspect(case);r['after_trace_inspect']=v
    r['same_worker_fresh_after_idle']=bool(v['state']['available']and not v['state']['pending']and v['state']['configured']is False and v['state']['worker']==initial['state']['worker']and int(v['state']['observed_at_ns'])>trace.record['trace_stops'][0]['actual_resume_started_at_ns'])
    if r['same_worker_fresh_after_idle']:break
  r['worker_terminal_before_cleanup']=bool(select.select([worker_pidfd],[],[],0)[0]);r['functional_observed_at_ns']=now()
  r['after_trace_independent_readback']=independent(min(case,now()+1_000_000_000));assert r['after_trace_independent_readback']['ipv4']==[]
  r['functional_completed_before_original_case']=now()<case
 else:
  r['parent_death_armed_at_ns']=now();signal.pidfd_send_signal(main['pidfd'],signal.SIGKILL)
  assert scope.finish(main,case)==-signal.SIGKILL;r['actual_Main_wait_and_EOF_before_adopted_worker_wait']=True
  while now()<case:
   got,word=os.waitpid(worker_pid,os.WNOHANG)
   if got:
    assert got==worker_pid and os.WIFSIGNALED(word)and os.WTERMSIG(word)==signal.SIGKILL
    r['known_worker_adopted_actual_wait']={'pid':got,'actual_original_parent':main['child'].pid,'adopted_by':os.getpid(),'returncode':os.waitstatus_to_exitcode(word),'waited_at_ns':now()};break
   scope.pump();select.select([],[],[],.001)
  assert r.get('known_worker_adopted_actual_wait')and select.select([worker_pidfd],[],[],0)[0]
  r['parent_death_control_passed']=True
except BaseException as error:r.update(error_kind=type(error).__name__,error=str(error)[:1000],traceback=traceback.format_exc())
finally:
 for s in sockets:s.close()
 if trace is not None:
  try:trace.close();r['trace']=trace.record
  except BaseException as e:r['cleanup_errors'].append('trace '+repr(e))
 if main is not None:
  try:
   if scope.live(main):r['explicit_Main_Stop_requested_at_ns']=now();signal.pidfd_send_signal(main['pidfd'],signal.SIGTERM)
   r['actual_Main_exit']=scope.finish(main,C)
   if a.case!='parent-death':r['actual_Main_worker_closure']=closure_record()
  except BaseException as e:r['cleanup_errors'].append('Main '+repr(e))
 if control is not None and keeper is not None:
  try:
   assert main is None or main['wait']is not None and main['eof'],'namespace keeper STOP fenced by Main wait/EOF'
   if scope.live(keeper):assert control.send(b'STOP',socket.MSG_DONTWAIT)==4
   r['actual_keeper_exit']=scope.finish(keeper,C);assert r['actual_keeper_exit']==0
  except BaseException as e:r['cleanup_errors'].append('keeper '+repr(e))
 r['closure']=close_owned(scope,C,reap_adopted=True);r['receipts']=scope.receipts
 r['all_handles_retired']=all(v['wait']is not None and v['eof']and v['log'].closed and v['child'].stdout.closed and v['pidfd']is None for v in scope.children)
 r['backend_retired_before_namespace_release']=bool(main is None or r.get('actual_Main_worker_closure',{}).get('closed')or r.get('known_worker_adopted_actual_wait'))
 r['namespace_FDs_retained_through_all_waits_EOF']=bool(r['all_handles_retired']and r['backend_retired_before_namespace_release']and len(fds)==2 and peer.namespace(fds[0])==r.get('current_namespace')and peer.namespace(fds[1])==r.get('peer_keeper',{}).get('namespace'))
 if r['namespace_FDs_retained_through_all_waits_EOF']:
  for fd in reversed(fds):os.close(fd)
  fds.clear();r['retained_namespace_FDs_closed']=True
 if worker_pidfd is not None:
  r['independent_worker_pidfd_terminal_after_Main_wait']=bool(select.select([worker_pidfd],[],[],0)[0]);os.close(worker_pidfd);worker_pidfd=None
 if control is not None:control.close()
 if other is not None:other.close()
 r['custody_qualified']=bool(not r['cleanup_errors']and r['all_handles_retired']and r.get('retained_namespace_FDs_closed')and r.get('independent_worker_pidfd_terminal_after_Main_wait')and r['closure']['closed']and r['closure']['wait_all_ECHILD']and not r['closure']['adopted_actual_waits']and now()<C)
 r['completed_at_ns']=now();(O/'case-result.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'case':a.case,'custody_qualified':r['custody_qualified'],'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['custody_qualified']and not r.get('error')else 2)
