"""Single test-only RFB connection, seven explicit actions, no replay/reconnect."""
import argparse,hashlib,json,os,select,signal,socket,struct,time
from pathlib import Path
from owned import Scope,now
from viewer_protocol import ACTIONS,read,publish,digest,validate_action
from rfb_frames import receive_once,request_raw_frame
from rfb_input_packets import click_plan,close_plan
p=argparse.ArgumentParser()
for n in ['qemu-pid','xvfb-pid','server-pid','parent-pid','work-until','close-until']:p.add_argument('--'+n,type=int,required=True)
p.add_argument('--selection-sha',required=True);a=p.parse_args();assert os.getppid()==a.parent_pid
R=Path('/results');D=R/'viewer-input';scope=Scope(R);result={'schema':1,'selection_sha256':a.selection_sha,'actions':[],'remote_event_queues_drained_claim':False};wire=None;pending=None;held_keys=[];held_button=False;retire_seen=False;failure=None
watch_pids=[a.parent_pid,a.qemu_pid,a.xvfb_pid,a.server_pid];watch_fds=[os.pidfd_open(pid,os.O_NONBLOCK)for pid in watch_pids]
def proc(pid):
 q=Path('/proc')/str(pid);s=(q/'stat').read_text().rsplit(')',1)[1].split();return(q.stat().st_uid,int(s[1]),int(s[19]))
identities=[proc(pid)for pid in watch_pids];assert all(v[0]==1000 for v in identities)
def check(until,cleanup=False,server_may_exit=False):
 scope.pump();scope.check();assert now()<until
 fds=watch_fds[:3]if server_may_exit else watch_fds;assert not select.select(fds,[],[],0)[0]
 assert identities[:len(fds)]==[proc(pid)for pid in watch_pids[:len(fds)]]
 if not cleanup:assert read(D/'abort.json')is None and read(D/'retire.request.json')is None,'guest cancelled/retired input'
def observe_no_tcp(label):
 check(a.work_until,cleanup=True);entry={'label':label,'started_host_ns':now(),'retained_server_PID':a.server_pid,'server_live_before':True};result.setdefault('no_TCP_observations',[]).append(entry)
 entry['tables']={n:Path('/proc/net/'+n).read_text()for n in ['tcp','tcp6']};check(a.work_until,cleanup=True);entry.update(completed_host_ns=now(),server_live_after=True,no_TCP_rows=all(len(v.splitlines())==1 for v in entry['tables'].values()));assert entry['no_TCP_rows']
class Wire:
 def __init__(self):
  self.until=min(a.work_until,now()+10_000_000_000);self.cleanup=False;self.server_may_exit=False;self.received=0;self.sent=0;self.framing_intact=True;self.eof_ns=None
  check(self.until);self.socket=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);self.socket.settimeout(max(.001,(self.until-now())/1e9));self.socket.connect('/results/viewer.sock');self.peer=struct.unpack('3i',self.socket.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert self.peer==(a.server_pid,1000,1000);self.socket.setblocking(False);check(self.until)
 def guard(self):check(self.until,self.cleanup,self.server_may_exit)
 def read(self,n):
  assert 0<=n<=12*1024*1024;out=bytearray()
  while len(out)<n:
   self.guard();r,_,_=select.select([self.socket],[],[],min(.05,max(0,(self.until-now())/1e9)))
   if not r:continue
   data=self.socket.recv(min(65536,n-len(out)));assert data,'unexpected RFB EOF';self.received+=len(data);assert self.received<=24*1024*1024;out.extend(data)
  self.guard();return bytes(out)
 def write(self,data):
  view=memoryview(data);written=0;complete=False
  try:
   while view:
    self.guard();_,w,_=select.select([],[self.socket],[],min(.05,max(0,(self.until-now())/1e9)))
    if not w:continue
    count=self.socket.send(view);assert count>0;written+=count;self.sent+=count;assert self.sent<=65536;view=view[count:]
   complete=True;self.guard()
  except BaseException:
   if written and not complete:self.framing_intact=False
   raise
 def close_stream(self):
  self.cleanup=True;self.server_may_exit=True;self.until=a.close_until
  self.socket.shutdown(socket.SHUT_WR)
  while self.eof_ns is None:
   self.guard();r,_,_=select.select([self.socket],[],[],min(.05,max(0,(self.until-now())/1e9)))
   if not r:continue
   data=self.socket.recv(65536);self.received+=len(data);assert self.received<=24*1024*1024
   if not data:self.eof_ns=now()
  self.guard();self.socket.close()
try:
 # This exact guest marker is published only after real Source/shell readiness.
 while read(D/'mice.request.json')is None:
  check(a.work_until);select.select([],[],[],.005)
 assert read(D/'mice.request.json')=={'schema':1,'selection_sha256':a.selection_sha}
 result['first_frame_after_guest_shell_preflight_marker']=True
 while True:
  check(a.work_until);setup=read(D/'window-setup.json')
  if setup is not None:break
  select.select([],[],[],.005)
 assert setup['schema']==1 and setup['selection_sha256']==a.selection_sha
 receipt=setup['actual_boot_parent_wait'];assert receipt['returncode']==0 and receipt['actual_direct_parent_wait']and receipt['stdout_EOF']and receipt['wait_before_original_cutoff']and receipt['EOF_before_original_cutoff']
 assert setup['setup']['owner_PID']==a.qemu_pid and setup['setup']['explicit_native_window_setup']['request_count']==1 and setup['setup']['explicit_native_window_setup']['after_tuple']==[1080,2424,0,24,0,0]
 check(a.work_until);result['parent_owned_window_setup']=setup;result['first_frame_after_actual_setup_wait_EOF']=True
 wire=Wire();first,info=receive_once(wire);observe_no_tcp('after_initial_RFB_frame');result['first_RFB_frame']=info;del first;publish(D/'connected.json',{'schema':1,'selection_sha256':a.selection_sha,'connected':True,'peer_PID_UID_GID':list(wire.peer),'first_frame_sha256':info['frame_sha256']});wire.until=a.work_until
 for seq,action in enumerate(ACTIONS,1):
  while True:
   check(a.work_until)
   request=read(D/(f'{seq:02d}.request.json'))
   if request is not None:break
   select.select([],[],[],.005)
  pending=validate_action(request,seq,a.selection_sha);prefix=f'{seq:02d}';h0=now();ready={'schema':1,'seq':seq,'request_sha256':digest(request),'host_ready_anchor_ns':h0};publish(D/(prefix+'.ready.json'),ready)
  while True:
   check(a.work_until);commit=read(D/(prefix+'.commit.json'))
   if commit is not None:break
   select.select([],[],[],.005)
  assert commit['schema']==1 and commit['selection_sha256']==a.selection_sha and commit['seq']==seq and commit['request_sha256']==digest(request)and commit['host_ready_anchor_ns']==h0
  assert commit['guest_until_ns']==request['guest_until_ns']and commit['guest_sample_ns']+commit['remaining_ns']==request['guest_until_ns']and type(commit['remaining_ns'])is int and commit['remaining_ns']>0
  until=min(a.work_until,h0+commit['remaining_ns']);check(until);assert now()+100_000_000<until
  argv=['/usr/bin/python3','/results/source/viewer_viewport.py','--qemu-pid',str(a.qemu_pid),'--xvfb-pid',str(a.xvfb_pid),'--parent-pid',str(os.getpid()),'--until-host-ns',str(until)]
  code,body=scope.run(argv,'viewer-viewport-'+prefix+'.log',until,limit=65536,env=dict(os.environ,DISPLAY=':97'));assert code==0;viewport=json.loads(body);assert viewport['owner_PID']==a.qemu_pid
  if action=='close-1':assert viewport['keyboard_focus_valid'];plan=close_plan(action)
  else:plan=click_plan(action,*request['point'],viewport['viewport'])
  check(until);assert now()+plan['hold_ns']<until
  entry={'seq':seq,'action':action,'request_sha256':digest(request),'host_ready_anchor_ns':h0,'original_host_dispatch_until_ns':until,'guest_clock_fields_not_compared_as_host_absolute':True,'viewport':viewport,'packet_count':len(plan['packets']),'attempted_at_ns':now(),'outcome':'uncertain','remote_ack_claimed':False,'remote_event_queues_drained_claim':False};result['actions'].append(entry);publish(D/(prefix+'.attempted.json'),{k:v for k,v in entry.items()if k!='viewport'});wire.until=until
  check(until);assert now()+plan['hold_ns']<until
  for index,packet in enumerate(plan['packets']):
   if action=='close-1':
    key=struct.unpack('>BBHI',packet)[3]
    if index<2:held_keys.append(key)
   elif index==1:held_button=True
   wire.write(packet)
   if action=='close-1'and index>=2:held_keys.remove(key)
   elif action!='close-1'and index==2:held_button=False
   if index==plan['hold_after_packet_index']:
    hold_until=now()+plan['hold_ns'];assert hold_until<until
    while now()<hold_until:check(until);select.select([],[],[],min(.005,max(0,(hold_until-now())/1e9)))
  check(until);entry.update(outcome='sent_once',completed_at_ns=now(),packets_sha256=hashlib.sha256(b''.join(plan['packets'])).hexdigest());publish(D/(prefix+'.done.json'),{k:v for k,v in entry.items()if k!='viewport'});pending=None;wire.until=a.work_until
 while True:
  check(a.work_until,cleanup=True);retire=read(D/'retire.request.json')
  if retire is not None:
   assert retire['selection_sha256']==a.selection_sha and retire['completed_actions']==7 and not retire['action_failed'];retire_seen=True;break
  assert read(D/'abort.json')is None;select.select([],[],[],.005)
 wire.cleanup=True;wire.until=min(a.work_until,now()+10_000_000_000);final,final_info=request_raw_frame(wire,1080,2424,info['server_name_sha256']);observe_no_tcp('after_reopened_RFB_frame');(R/'viewer-reopened.bgrx').write_bytes(final);result['reopened_RFB_frame']=final_info;result['reopened_RFB_frame_file']='viewer-reopened.bgrx';del final
except BaseException as error:
 failure=type(error).__name__+': '+str(error);result['failure']=failure
 if pending is not None:
  path=D/(f"{pending['seq']:02d}.done.json")
  if not path.exists():publish(path,{'seq':pending['seq'],'request_sha256':digest(pending),'outcome':'uncertain','remote_ack_claimed':False,'remote_event_queues_drained_claim':False})
finally:
 result['cleanup_release_attempted']=False
 if wire is not None:
  try:
   wire.cleanup=True;wire.until=a.close_until
   if held_keys or held_button:
    assert wire.framing_intact,'partial protocol packet; appending cleanup input is unsafe';result['cleanup_release_attempted']=True
    for key in reversed(held_keys):wire.write(struct.pack('>BBHI',4,0,0,key))
    if held_button:
     point=pending['point'];wire.write(struct.pack('>BBHH',5,0,*point))
   wire.close_stream();result['RFB_stream_EOF_at_ns']=wire.eof_ns
  except BaseException as error:result['stream_cleanup_error']=type(error).__name__+': '+str(error);wire.socket.close()
 result['helper_cleanup_errors']=scope.close(a.close_until);result['owned_helper_receipts']=scope.receipts;result['retire_request_seen']=retire_seen;result['completed_host_ns']=now();result['passed']=bool(not failure and retire_seen and len(result.get('no_TCP_observations',[]))==2 and len(result['actions'])==7 and all(v['outcome']=='sent_once'for v in result['actions'])and result.get('RFB_stream_EOF_at_ns')and not result.get('stream_cleanup_error')and not result['helper_cleanup_errors']and all(v['returncode']==0 and v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']for v in scope.receipts)and now()<a.close_until)
 (R/'viewer-worker-result.json').write_text(json.dumps(result,indent=2)+'\n')
 for fd in watch_fds:os.close(fd)
 print(json.dumps({'viewer_worker_completed':True,'passed':result['passed'],'future_writes_claim_requires_parent_wait':True,'remote_queues_drained':False}),flush=True)
raise SystemExit(0 if result['passed']else 1)
