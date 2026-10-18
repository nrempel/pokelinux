"""Private one-shot packet lifecycle draft; consumes one prebuilt exact Rust test ELF."""
import sys
sys.dont_write_bytecode=True
from pathlib import Path
import argparse,array,json,os,select,socket,time,traceback
sys.path.insert(0,'/runner')
from owned import Scope,now
from closure import close_owned
import peer
p=argparse.ArgumentParser()
p.add_argument('--test-elf',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--work-until-ns',type=int,required=True)
p.add_argument('--close-until-ns',type=int,required=True)
a=p.parse_args();T=now();W=a.work_until_ns;C=a.close_until_ns
assert os.getuid()==0 and os.environ.get('POKELINUX_CONNECTIVITY_ISOLATED')=='1'and T<W<C
assert a.test_elf.is_absolute()and a.test_elf.is_file()and a.output.is_absolute()and not a.output.exists()
a.output.mkdir(mode=0o700);O=a.output;G=O/'gates';G.mkdir(mode=0o700);(O/'provision').mkdir();scope=Scope(O);provision=Scope(O/'provision');fds=[];control=None;keeper=rust=packet=None
r={'passed':False,'classification':'SETUP_FAILURE','Settings_window_or_OS_permission_executed':False,'literal_trusted_owner_authority_only':True,'original_work_until_ns':W,'original_close_until_ns':C,'cleanup_errors':[]}
def wait_file(name,end):
 path=G/name
 while now()<end:
  scope.pump();scope.check()
  if path.exists():return json.loads(path.read_text())
  if rust is not None and not scope.live(rust):raise RuntimeError('Rust exited before setup gate '+name)
  time.sleep(.002)
 raise TimeoutError('original bounded gate '+name)
def complete(s):return all(v['wait']is not None and v['eof']and v['log'].closed and v['child'].stdout.closed and v['pidfd']is None for v in s.children)
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
 rust=scope.start([str(a.test_elf),'--ignored','--exact','actual_fixed_ipv4_packet_lifecycle','--nocapture','--test-threads=1'],'actual-rust-packet-lifecycle.log',1024*1024,env=env,cutoff=case,wait_cutoff=C,identity={'preexec_fn':lambda:peer.reduce_caps(1<<peer.NET_ADMIN)})
 r['created']=wait_file('created.json',case);assert r['created']['pid']==rust['child'].pid
 r['rust_caps']= {v.split(':')[0]:v.split(':')[1].strip()for v in Path('/proc/'+str(rust['child'].pid)+'/status').read_text().splitlines()if v.startswith(('CapInh:','CapPrm:','CapEff:','CapBnd:','CapAmb:','NoNewPrivs:'))}
 assert all(int(r['rust_caps'][k],16)==1<<peer.NET_ADMIN for k in ['CapEff','CapPrm','CapBnd'])and int(r['rust_caps']['CapInh'],16)==int(r['rust_caps']['CapAmb'],16)==0 and r['rust_caps']['NoNewPrivs']=='1'
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
 ready={'provisioner_reaped_before_packet_peer':provision.receipts[0]['waited_at_boottime_ns']<packet['spawn_started_at_ns'],'supervisor_all_caps_zero':True,'packet_peer_all_caps_zero':True,'released_at_ns':now(),'original_case_until_ns':case}
 assert ready['provisioner_reaped_before_packet_peer'];peer.publish(G,'ready.json',ready);r['healthy_peer_setup_qualified']=True
 while scope.live(rust)and now()<case:
  scope.pump();scope.check()
  for phase,connected in [('connected',True),('disconnected',False)]:
   if(G/(phase+'-proof.json')).exists()and not(G/(phase+'-readback.json')).exists():
    value=peer.ipv4_snapshot('pklown0',min(case,now()+1_000_000_000));assert value['ipv4']==([{'ifindex':r['created']['fact']['ifindex'],'prefix':30,'local':'192.0.2.1'}]if connected else[])
    assert peer.namespace(local)==r['current_namespace'];r[phase+'_independent_Python_readback']=value;peer.publish(G,phase+'-readback.json',value)
  time.sleep(.002)
 assert now()<case or not scope.live(rust),'original six-second functional aggregate expired'
 code=scope.finish(rust,min(case,C));r['actual_rust_exit']=code
 text=(O/'actual-rust-packet-lifecycle.log').read_text()
 if code==101:
  returned=json.loads((G/'connect-return.json').read_text());attempt=json.loads((G/'connect-attempt.json').read_text())
  assert returned['result_debug']=='Err(Unsupported)'and 'CONNECT_REQUIRES_ACTUAL_FIXED_IPV4_SUCCESS'in text and 'test result: FAILED. 0 passed; 1 failed; 0 ignored;'in text
  assert ready['released_at_ns']<=attempt['attempted_at_ns']<=returned['returned_at_ns']<attempt['original_until_ns']<=case
  r['genuine_Unsupported_Connect_RED']=True;r['classification']='ACTUAL_FIXED_IPV4_CONNECT_UNSUPPORTED_RED'
 else:
  assert code==0 and 'test result: ok. 1 passed; 0 failed; 0 ignored;'in text
  passed=json.loads((G/'passed.json').read_text());assert passed['passed']and passed['completed_at_ns']<case and not passed['Settings_window_executed']
  packets=json.loads((G/'peer-packets.json').read_text());assert len(packets['packets'])==2
  assert packets['packets'][0]['payload_hex']==passed['first_packet']['nonce_hex']and packets['packets'][1]['payload_hex']==passed['background_packet_after_withdraw']['nonce_hex']
  quiet=json.loads((G/'peer-quiet.json').read_text());assert quiet['received_datagrams']==0 and quiet['completed_at_ns']>=quiet['original_until_ns']and quiet['quiet_end_actual_EAGAIN']and quiet['quiet_end_EAGAIN_at_ns']>=quiet['original_until_ns']and not(G/'peer-unexpected-datagram.json').exists()
  r['desired_packet_lifecycle']=passed;r['packet_peer_receipts']=packets;r['functional_GREEN']=True;r['classification']='ACTUAL_FIXED_IPV4_PACKET_LIFECYCLE_GREEN'
except BaseException as error:r.update(error=repr(error),traceback=traceback.format_exc())
finally:
 # Keep BOTH retained namespace FDs through all child actual wait/EOF and
 # log/PIPE/pidfd closure; no namespace disposal or link rollback can race work.
 (G/'stop-peer').touch()
 if packet is not None:
  try:r['packet_peer_exit']=scope.finish(packet,min(C,now()+1_000_000_000));assert r['packet_peer_exit']==0
  except BaseException as error:r['cleanup_errors'].append('packet '+repr(error))
 if control is not None and keeper is not None:
  try:
   if scope.live(keeper):control.sendall(b'STOP')
   r['keeper_exit']=scope.finish(keeper,min(C,now()+1_000_000_000));assert r['keeper_exit']==0
  except BaseException as error:r['cleanup_errors'].append('keeper '+repr(error))
 r['provisioner_final_closure']=close_owned(provision,C,reap_adopted=False);r['closure']=close_owned(scope,C,reap_adopted=True);r['owned_receipts']=scope.receipts
 r['namespace_FDs_retained_through_all_waits_EOF']=bool(complete(provision)and complete(scope)and len(fds)==2 and peer.namespace(fds[0])==r.get('current_namespace')and peer.namespace(fds[1])==r.get('peer_keeper',{}).get('namespace'))
 for fd in reversed(fds):os.close(fd)
 if control is not None:control.close()
 receipts=scope.receipts+provision.receipts;r['actual_wait_EOF_timely']=bool(receipts)and all(v.get('actual_direct_parent_wait')and v.get('stdout_EOF')and v.get('wait_before_original_cutoff')and v.get('EOF_before_original_cutoff')for v in receipts)
 r['custody_qualified']=bool(not r['cleanup_errors']and r['closure']['closed']and r['closure']['wait_all_ECHILD']and r['provisioner_final_closure']['closed']and r['namespace_FDs_retained_through_all_waits_EOF']and r['actual_wait_EOF_timely']and now()<C)
 r['no_unexpected_peer_datagram_after_actual_peer_wait']=not(G/'peer-unexpected-datagram.json').exists()
 r['behavior_RED_qualified']=bool(r.get('genuine_Unsupported_Connect_RED')and r.get('healthy_peer_setup_qualified')and r['custody_qualified']);r['passed']=bool(r.get('functional_GREEN')and r['custody_qualified']and r['no_unexpected_peer_datagram_after_actual_peer_wait']);(O/'result.json').write_text(json.dumps(r,indent=2)+'\n')
 print(json.dumps({'passed':r['passed'],'behavior_RED_qualified':r['behavior_RED_qualified'],'classification':r['classification'],'custody_qualified':r['custody_qualified'],'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['passed']else 1)
