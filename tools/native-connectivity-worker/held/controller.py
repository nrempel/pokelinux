"""Private held-worker expiry/Stop; exact standalone driver and normal worker ELFs."""
import sys
sys.dont_write_bytecode=True
from pathlib import Path
import argparse,array,json,os,select,signal,socket,struct,time,traceback
sys.path.insert(0,'/runner')
from owned import Scope,now
from closure import close_owned
import peer
p=argparse.ArgumentParser()
p.add_argument('--driver-elf',type=Path,required=True)
p.add_argument('--worker-elf',type=Path,required=True)
p.add_argument('--case',choices=['expiry','stop'],required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--work-until-ns',type=int,required=True)
p.add_argument('--close-until-ns',type=int,required=True)
a=p.parse_args();T=now();W=a.work_until_ns;C=a.close_until_ns
assert os.getuid()==0 and os.environ.get('POKELINUX_CONNECTIVITY_ISOLATED')=='1'and T<W<C
assert all(v.is_absolute()and v.is_file()for v in [a.driver_elf,a.worker_elf])and a.output.is_absolute()and not a.output.exists()
a.output.mkdir(mode=0o700);O=a.output;G=O/'gates';G.mkdir(mode=0o700);(O/'provision').mkdir();scope=Scope(O);provision=Scope(O/'provision');fds=[];control=None;keeper=rust=packet=None;worker_pidfd=None;watch=None;target_events=[];watch_started=None
r={'passed':False,'classification':'SETUP_FAILURE','Settings_window_or_OS_permission_executed':False,'normal_service_client_ingress_executed':False,'case':a.case,'literal_trusted_owner_authority_only':True,'original_work_until_ns':W,'original_close_until_ns':C,'cleanup_errors':[]}
def wait_file(name,end):
 path=G/name
 while now()<end:
  scope.pump();scope.check()
  if path.exists():return json.loads(path.read_text())
  if rust is not None and not scope.live(rust):raise RuntimeError('Rust exited before setup gate '+name)
  time.sleep(.002)
 raise TimeoutError('original bounded gate '+name)
def complete(s):return all(v['wait']is not None and v['eof']and v['log'].closed and v['child'].stdout.closed and v['pidfd']is None for v in s.children)
def worker_identity(pid):
 raw=Path('/proc/'+str(pid)+'/stat').read_text();tail=raw[raw.rfind(')')+2:].split()
 return {'pid':pid,'state':tail[0],'ppid':int(tail[1]),'starttime_ticks':int(tail[19])}
def watch_drain():
 assert watch is not None
 count=0
 while count<64:
  try:data,anc,flags,sender=watch.recvmsg(65536,0)
  except BlockingIOError:return {'actual_EAGAIN':True,'at_ns':now(),'datagrams':count}
  count+=1;assert not anc and not flags and sender==(0,16);offset=0
  while offset<len(data):
   assert len(data)-offset>=16
   size,kind,_,sequence,port=struct.unpack_from('=IHHII',data,offset);assert size>=16 and offset+size<=len(data)and kind in [20,21]
   body=data[offset+16:offset+size];assert len(body)>=8
   family,prefix,aflags,address_scope,index=struct.unpack_from('=BBBBI',body)
   assert family==2
   if index==r['created']['fact']['ifindex']:target_events.append({'kind':kind,'header_sequence':sequence,'header_port':port,'multicast_group':sender[1],'ifindex':index,'prefix':prefix,'scope':address_scope,'flags':aflags,'body_hex':body.hex(),'observed_at_ns':now()})
   offset+=(size+3)&~3
  assert offset==len(data)
 raise AssertionError('bounded IPv4 notification drain did not reach actual EAGAIN')
try:
 assert socket.if_nameindex()==[(1,'lo')],'fresh current namespace sanity, not exclusivity proof'
 peer.call('prctl',36,1,0,0,0)
 local=os.open('/proc/thread-self/ns/net',os.O_RDONLY|os.O_CLOEXEC);fds.append(local);r['current_namespace']=peer.namespace(local)
 control,other=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET)
 keeper=scope.start(['/usr/bin/python3','-B',str(Path(peer.__file__).resolve()),'keeper',str(other.fileno()),str(C)],'namespace-keeper.log',65536,cutoff=W,wait_cutoff=C,identity={'pass_fds':(other.fileno(),)});other.close()
 while not select.select([control],[],[],.002)[0]:scope.pump();assert now()<W and scope.live(keeper)
 body,anc,flags,_=control.recvmsg(4096,socket.CMSG_SPACE(array.array('i').itemsize));assert not flags;kept=json.loads(body)
 for level,kind,data in anc:
  assert level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS;received=array.array('i');received.frombytes(data);fds.extend(received)
 assert len(fds)==2 and peer.namespace(fds[1])==kept['namespace']and kept['namespace']!=r['current_namespace'];r['peer_keeper']=kept
 peer_path='/proc/'+str(keeper['child'].pid)+'/ns/net'
 # One upfront six-second aggregate includes Rust creation, serialized peer
 # provisioning, actual packet exchanges and the complete800ms quiet window.
 case=min(W,now()+6_000_000_000);r['original_case_until_ns']=case
 env=dict(os.environ,POKELINUX_CONNECTIVITY_ISOLATED='1',POKELINUX_CONNECTIVITY_PEER_NS=peer_path,POKELINUX_CONNECTIVITY_GATES=str(G),POKELINUX_CONNECTIVITY_CASE_UNTIL_NS=str(case))
 rust=scope.start([str(a.driver_elf),a.case,str(a.worker_elf),str(G),peer_path,str(case),str(C)],'actual-rust-held-worker.log',1024*1024,env=env,cutoff=case,wait_cutoff=C,identity={'preexec_fn':lambda:peer.reduce_caps((1<<peer.NET_ADMIN)|(1<<8))})
 r['created']=wait_file('created.json',case);assert r['created']['pid']==rust['child'].pid
 def status(pid):
  return {v.split(':')[0]:v.split(':')[1].strip()for v in Path('/proc/'+str(pid)+'/status').read_text().splitlines()if v.startswith(('PPid:','Uid:','CapInh:','CapPrm:','CapEff:','CapBnd:','CapAmb:','NoNewPrivs:'))}
 r['Main_caps']=status(rust['child'].pid)
 assert all(int(r['Main_caps'][k],16)==0 for k in ['CapEff','CapPrm','CapBnd','CapInh','CapAmb'])and r['Main_caps']['NoNewPrivs']=='1'
 worker_pid=r['created']['worker_pid'];assert worker_pid>0 and worker_pid!=rust['child'].pid
 r['worker_caps']=status(worker_pid);assert int(r['worker_caps']['PPid'])==rust['child'].pid
 assert all(int(r['worker_caps'][k],16)==1<<peer.NET_ADMIN for k in ['CapEff','CapPrm','CapBnd'])and int(r['worker_caps']['CapInh'],16)==int(r['worker_caps']['CapAmb'],16)==0 and r['worker_caps']['NoNewPrivs']=='1'
 assert r['created']['snapshot']['main_caps_dropped']and r['created']['snapshot']['namespace_handles_open']and r['created']['snapshot']['occupied']
 for inventory in [r['created']['actual_namespace_fd_inventory']]:
  assert any(v['dev']==r['current_namespace']['dev']and v['ino']==r['current_namespace']['ino']for v in inventory)and any(v['dev']==kept['namespace']['dev']and v['ino']==kept['namespace']['ino']for v in inventory)
 r['actual_Main_worker_capability_split']=True
 # Distinct short-lived setup process; all its custody closes before packet peer.
 provision_until=min(case,now()+1_000_000_000)
 code,data=provision.run(['/usr/bin/python3','-B',str(Path(peer.__file__).resolve()),'provision',peer_path,str(provision_until)],'peer-provisioner.log',provision_until,limit=65536)
 assert code==0;r['peer_provisioner']=json.loads(data);assert r['peer_provisioner']['passed']
 r['provisioner_closure']=close_owned(provision,C,reap_adopted=False);r['provisioner_receipts']=provision.receipts
 assert r['provisioner_closure']['closed']and complete(provision)
 packet=scope.start(['/usr/bin/python3','-B',str(Path(peer.__file__).resolve()),'serve',peer_path,str(G),str(case)],'cap0-packet-peer.log',65536,cutoff=case,wait_cutoff=C)
 r['peer_ready']=wait_file('peer-ready.json',case);cap=r['peer_ready']['caps'];assert r['peer_ready']['pid']==packet['child'].pid and r['peer_ready']['uid']==0 and all(int(v,16)==0 for k,v in cap.items()if k.startswith('Cap'))and cap['NoNewPrivs']=='1'
 assert r['peer_ready']['only_packet_socket_no_admin_descriptors']and set(r['peer_ready']['actual_fd_inventory'])=={'0','1','2',next(k for k,v in r['peer_ready']['actual_fd_inventory'].items()if v.startswith('socket:['))}
 r['supervisor_caps']=peer.reduce_caps();assert now()<case
 ready={'provisioner_reaped_before_packet_peer':provision.receipts[0]['waited_at_boottime_ns']<packet['spawn_started_at_ns'],'supervisor_all_caps_zero':True,'packet_peer_all_caps_zero':True,'Main_all_caps_zero':True,'worker_NET_ADMIN_only':True,'released_at_ns':now(),'original_case_until_ns':case}
 assert ready['provisioner_reaped_before_packet_peer'];peer.publish(G,'ready.json',ready);r['healthy_peer_setup_qualified']=True
 while scope.live(rust)and now()<case:
  scope.pump();scope.check()
  if(G/'request-stop-worker.json').exists()and not(G/'worker-stopped.json').exists():
   request=json.loads((G/'request-stop-worker.json').read_text());assert request['worker_pid']==worker_pid and request['worker']==r['created']['worker']and request['snapshot']['ready']
   watch=socket.socket(socket.AF_NETLINK,socket.SOCK_RAW|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC,0);watch.bind((0,16));watch_started=now();r['watch_started_at_ns']=watch_started
   r['before_stop_independent_readback']=peer.ipv4_snapshot('pklown0',min(case,now()+100_000_000));assert r['before_stop_independent_readback']['ipv4']==[]
   before=worker_identity(worker_pid);assert before['ppid']==rust['child'].pid
   worker_pidfd=os.pidfd_open(worker_pid,os.O_NONBLOCK)
   after=worker_identity(worker_pid);assert before['starttime_ticks']==after['starttime_ticks']and after['ppid']==rust['child'].pid
   fdinfo=Path('/proc/self/fdinfo/'+str(worker_pidfd)).read_text();assert any(line.split()==['Pid:',str(worker_pid)]for line in fdinfo.splitlines())
   signal_at=now();stop_until=min(case,signal_at+100_000_000);signal.pidfd_send_signal(worker_pidfd,signal.SIGSTOP)
   while True:
    scope.pump();assert now()<stop_until and not select.select([worker_pidfd],[],[],0)[0]
    actual=worker_identity(worker_pid);assert actual['starttime_ticks']==before['starttime_ticks']and actual['ppid']==rust['child'].pid
    if actual['state']in ['T','t']:break
    time.sleep(.001)
   stopped=dict(actual,worker_pid=worker_pid,worker=r['created']['worker'],actual_pidfd_SIGSTOP=True,signal_at_ns=signal_at,observed_at_ns=now(),original_stop_until_ns=stop_until,pidfd_fdinfo=fdinfo)
   r['actual_stopped']=stopped;peer.publish(G,'worker-stopped.json',stopped)
  if watch is not None:
   r['last_watch_drain']=watch_drain();assert not target_events,'unexpected local IPv4 address notification'
  if(G/'resume-worker.json').exists()and not(G/'worker-resumed.json').exists():
   assert a.case=='expiry'and worker_pidfd is not None
   request=json.loads((G/'resume-worker.json').read_text());dispatch=json.loads((G/'dispatch.json').read_text());assert request['worker']==r['created']['worker']
   assert dispatch['original_until_ns']-dispatch['started_at_ns']==100_000_000 and dispatch['original_hold_until_ns']-dispatch['started_at_ns']==350_000_000
   assert now()>=dispatch['original_hold_until_ns']and now()<request['original_resume_until_ns']and not select.select([worker_pidfd],[],[],0)[0]
   signal_at=now();signal.pidfd_send_signal(worker_pidfd,signal.SIGCONT)
   while True:
    scope.pump();assert now()<request['original_resume_until_ns']
    try:actual=worker_identity(worker_pid)
    except FileNotFoundError:actual={'pid':worker_pid,'state':'exited_after_SIGCONT'}
    if actual['state']not in ['T','t']:break
    time.sleep(.001)
   resumed={'worker_pid':worker_pid,'worker':r['created']['worker'],'actual_pidfd_SIGCONT':True,'signal_at_ns':signal_at,'observed_at_ns':now(),'state':actual['state'],'original_resume_until_ns':request['original_resume_until_ns']}
   r['actual_resumed']=resumed;peer.publish(G,'worker-resumed.json',resumed)
  if(G/'independent-finish-request.json').exists()and not(G/'independent-absence.json').exists():
   request=json.loads((G/'independent-finish-request.json').read_text())
   if a.case=='expiry':assert now()>=request['original_independent_until_ns']and r.get('actual_resumed')
   ended=watch_drain();assert not target_events
   value=peer.ipv4_snapshot('pklown0',min(case,now()+100_000_000));assert value['ipv4']==[]
   value.update(target_address_notifications=list(target_events),actual_final_EAGAIN=ended['actual_EAGAIN'],final_EAGAIN_at_ns=ended['at_ns'],notification_watch_started_at_ns=watch_started)
   r['independent_absence']=value;peer.publish(G,'independent-absence.json',value)
  time.sleep(.002)
 assert now()<case or not scope.live(rust),'original six-second functional aggregate expired'
 code=scope.finish(rust,min(case,C));r['actual_rust_exit']=code
 text=(O/'actual-rust-held-worker.log').read_text()
 r['service_closure']=json.loads((G/'service-closure.json').read_text());assert r['service_closure']['closed']
 last=r['service_closure']['after_actual_wait_EOF'];assert last['reaped']and last['output_eof']and not last['occupied']and last['exit_status']['signal']==9 and last['namespace_handles_open']
 inventory=r['service_closure']['actual_namespace_fd_inventory'];assert any(v['dev']==r['current_namespace']['dev']and v['ino']==r['current_namespace']['ino']for v in inventory)and any(v['dev']==kept['namespace']['dev']and v['ino']==kept['namespace']['ino']for v in inventory)
 r['actual_worker_exit_status']=last['exit_status'];r['worker_wait_EOF_before_Main_exit']=True
 assert watch is not None and worker_pidfd is not None and r.get('actual_stopped')
 r['final_notification_drain']=watch_drain();assert not target_events;r['target_address_notifications']=list(target_events)
 r['after_driver_independent_readback']=peer.ipv4_snapshot('pklown0',min(C,now()+100_000_000));assert r['after_driver_independent_readback']['ipv4']==[]
 if a.case=='stop':
  dispatch=json.loads((G/'stop-dispatch.json').read_text());value=json.loads((G/'stop-before-assert.json').read_text());pending=dispatch['snapshot']['owner_pending']
  assert dispatch['action']=='Connect'and dispatch['worker']==r['created']['worker']and dispatch['worker_stopped']['actual_pidfd_SIGSTOP']
  assert pending['phase']=='Dispatched'and pending['id']==dispatch['id']and pending['original_until_ns']==dispatch['original_until_ns']
  assert dispatch['original_until_ns']-dispatch['started_at_ns']==100_000_000 and r['actual_stopped']['observed_at_ns']<=dispatch['started_at_ns']<=dispatch['dispatched_at_ns']<=value['stop_called_at_ns']<dispatch['original_until_ns']
  assert value['dispatch_id']==dispatch['id']and value['original_action_until_ns']==dispatch['original_until_ns']and value['before']['owner_pending']==pending and value['before']['worker']==dispatch['worker']
  assert not target_events;r['actual_outstanding_Stop_dispatch']=dispatch
 if code==101:
  failed=json.loads((G/'failed.json').read_text());assert failed['closed']
  if a.case=='expiry':
   value=json.loads((G/'expiry-before-assert.json').read_text());dispatch=json.loads((G/'dispatch.json').read_text());expired=value['snapshot'];r['expired_snapshot_before_assert']=value;r['actual_dispatch']=dispatch
   assert 'EXPIRED_OWNER_OBSERVATION_MUST_BE_NONE'in failed['message']and 'EXPIRED_OWNER_OBSERVATION_MUST_BE_NONE'in text
   assert dispatch['original_until_ns']-dispatch['started_at_ns']==100_000_000 and dispatch['snapshot']['owner_pending']['phase']=='Dispatched'
   assert r['actual_stopped']['observed_at_ns']<=dispatch['started_at_ns']<dispatch['original_until_ns']<value['sampled_at_ns']and value['sampled_at_ns']>=dispatch['original_hold_until_ns']and value['sampled_at_ns']-dispatch['started_at_ns']>=350_000_000
   assert expired['fact']is None and expired['observed_at_ns']is None and expired['owner_observed']is not None and not expired['ready']and expired['terminal_uncertain']
   assert expired['owner_observed']['observed_at_ns']<=r['actual_stopped']['observed_at_ns']and not expired['owner_observed']['connected']and not r.get('actual_resumed')
   assert len(value['samples'])>=3 and all(sample['after_ns']-sample['before_ns']<100_000_000 for sample in value['samples'])
   r['genuine_expired_nested_observation_RED']=True;r['classification']='ACTUAL_STOPPED_WORKER_EXPIRED_OWNER_OBSERVATION_RED'
  else:
   value=json.loads((G/'stop-before-assert.json').read_text());immediate=value['immediate'];r['immediate_stop_before_assert']=value
   assert 'STOP_OWNER_OBSERVATION_MUST_BE_NONE'in failed['message']and 'STOP_OWNER_OBSERVATION_MUST_BE_NONE'in text
   assert immediate['fact']is None and immediate['observed_at_ns']is None and immediate['owner_observed']is not None and immediate['occupied']and not immediate['reaped']and not immediate['output_eof']and immediate['namespace_handles_open']and not immediate['ready']and immediate['stopping']
   assert value['stop_returned_at_ns']-value['stop_called_at_ns']<100_000_000
   r['genuine_stopped_nested_observation_RED']=True;r['classification']='ACTUAL_STOPPED_WORKER_STOP_OWNER_OBSERVATION_RED'
 else:
  assert code==0
  passed=json.loads((G/'passed.json').read_text());assert passed['passed']and passed['case']==a.case and passed['completed_at_ns']<case and not passed['Settings_window_executed']
  assert r.get('independent_absence')and not r['independent_absence']['target_address_notifications']
  if a.case=='expiry':assert r.get('actual_resumed')and r['actual_resumed']['signal_at_ns']>=json.loads((G/'dispatch.json').read_text())['original_hold_until_ns']
  r['desired_held_worker_case']=passed;r['functional_GREEN']=True;r['classification']='ACTUAL_PERSISTENT_SERVICE_'+a.case.upper()+'_GREEN'
except BaseException as error:r.update(error=repr(error),traceback=traceback.format_exc())
finally:
 # Keep BOTH retained namespace FDs through all child actual wait/EOF and
 # log/PIPE/pidfd closure; no namespace disposal or link rollback can race work.
 (G/'stop-peer').touch()
 if packet is not None:
  try:
   r['packet_peer_exit']=scope.finish(packet,min(C,now()+1_000_000_000));assert r['packet_peer_exit']==0
   r['cap0_peer_final']=json.loads((O/'cap0-packet-peer.log').read_text());assert r['cap0_peer_final']['packets']==0
  except BaseException as error:r['cleanup_errors'].append('packet '+repr(error))
 if control is not None and keeper is not None:
  try:
   if scope.live(keeper):control.sendall(b'STOP')
   r['keeper_exit']=scope.finish(keeper,min(C,now()+1_000_000_000));assert r['keeper_exit']==0
  except BaseException as error:r['cleanup_errors'].append('keeper '+repr(error))
 r['provisioner_final_closure']=close_owned(provision,C,reap_adopted=False);r['closure']=close_owned(scope,C,reap_adopted=True);r['owned_receipts']=scope.receipts
 r['namespace_FDs_retained_through_all_waits_EOF']=bool(complete(provision)and complete(scope)and len(fds)==2 and peer.namespace(fds[0])==r.get('current_namespace')and peer.namespace(fds[1])==r.get('peer_keeper',{}).get('namespace'))
 if watch is not None:watch.close()
 if worker_pidfd is not None:os.close(worker_pidfd)
 for fd in reversed(fds):os.close(fd)
 if control is not None:control.close()
 receipts=scope.receipts+provision.receipts;r['actual_wait_EOF_timely']=bool(receipts)and all(v.get('actual_direct_parent_wait')and v.get('stdout_EOF')and v.get('wait_before_original_cutoff')and v.get('EOF_before_original_cutoff')for v in receipts)
 r['custody_qualified']=bool(not r['cleanup_errors']and r['closure']['closed']and r['closure']['wait_all_ECHILD']and not r['closure']['adopted_actual_waits']and r.get('worker_wait_EOF_before_Main_exit')and r['provisioner_final_closure']['closed']and r['namespace_FDs_retained_through_all_waits_EOF']and r['actual_wait_EOF_timely']and now()<C)
 r['no_unexpected_peer_datagram_after_actual_peer_wait']=not(G/'peer-unexpected-datagram.json').exists()
 r['behavior_RED_qualified']=bool((r.get('genuine_expired_nested_observation_RED')or r.get('genuine_stopped_nested_observation_RED'))and r.get('healthy_peer_setup_qualified')and r.get('actual_Main_worker_capability_split')and r['custody_qualified']);r['passed']=bool(r.get('functional_GREEN')and r['custody_qualified']and r['no_unexpected_peer_datagram_after_actual_peer_wait']);(O/'result.json').write_text(json.dumps(r,indent=2)+'\n')
 print(json.dumps({'passed':r['passed'],'behavior_RED_qualified':r['behavior_RED_qualified'],'classification':r['classification'],'custody_qualified':r['custody_qualified'],'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['passed']else 1)
