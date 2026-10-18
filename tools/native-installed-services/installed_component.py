"""One frozen native C++ case, with PID1 service custody instead of host child custody."""
import hashlib,json,os,select,socket,stat,struct,subprocess,time
from pathlib import Path
from owned import Scope,now
UNIT='pokelinux-network-status.service';EXE='/usr/libexec/pokelinux/pokelinux-network-status';SOCKET='/run/pokelinux-network/snapshot.sock'
PROPERTIES='LoadState,UnitFileState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,ExecMainPID,ExecMainCode,ExecMainStatus,Result,NRestarts'
def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def props(command,until,cleanup=False):
 body=command(['/usr/bin/systemctl','show',UNIT,'--property='+PROPERTIES],'network-unit-properties',seconds=10,absolute_until=until,cleanup=cleanup)
 return dict(v.split('=',1)for v in body.splitlines())
def onlylo():
 assert sorted(p.name for p in Path('/sys/class/net').iterdir())==['lo']and int(Path('/sys/class/net/lo/flags').read_text().strip(),16)&8
 names=socket.if_nameindex();proc=[v.split(':',1)[0].strip()for v in Path('/proc/net/dev').read_text().splitlines()if ':'in v];assert names==[(1,'lo')]and proc==['lo']
 return {'sysfs':['lo'],'if_nameindex':names,'proc_net_dev':proc,'network_mutations':False}
def bind(command,until,expected_exe_sha,expected=None):
 while now()<until:
  v=props(command,until)
  if v['ActiveState']=='active'and v['SubState']=='running'and int(v['MainPID'])>1:break
  time.sleep(.01)
 else:raise AssertionError('original PID1 Main admission cutoff')
 assert v['LoadState']=='loaded'and v['UnitFileState']=='enabled'and v['InvocationID']and v['ControlGroup'].startswith('/system.slice/')and v['NRestarts']=='0'
 pid=int(v['MainPID']);fd=os.pidfd_open(pid,os.O_NONBLOCK)
 try:
  assert not select.select([fd],[],[],0)[0]
  proc=Path('/proc')/str(pid);fields=(proc/'stat').read_text().rsplit(') ',1)[1].split();assert int(fields[1])==1
  status=(proc/'status').read_text()
  for k in ['Uid:','Gid:']:assert next(x for x in status.splitlines()if x.startswith(k)).split()[1:]==['0']*4
  assert str((proc/'exe').readlink())==EXE and sha(proc/'exe')==expected_exe_sha
  ns=Path('/proc/self/ns/net').stat();peer=(proc/'ns/net').stat();assert(ns.st_dev,ns.st_ino)==(peer.st_dev,peer.st_ino)
  after=props(command,until);assert all(after[k]==v[k]for k in ['MainPID','InvocationID','ControlGroup','ActiveState','SubState','NRestarts'])
  if expected:assert pid==expected['PID']and v['InvocationID']==expected['InvocationID']and fields[19]==expected['starttime']
  return {'PID':pid,'fd':fd,'InvocationID':v['InvocationID'],'ControlGroup':v['ControlGroup'],'starttime':fields[19],'initial_PID1':v,'Root_UID_GID_and_exe_verified':True,'PID1_is_actual_parent':True,'inherited_namespace_matches':True}
 except BaseException:os.close(fd);raise
def identity_record(item):return {k:v for k,v in item.items()if k!='fd'}
def snapshot(item,until):
 assert not select.select([item['fd']],[],[],0)[0]
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as p:
  p.settimeout(max(.001,min(.15,(until-now())/1e9)));p.connect(SOCKET);assert struct.unpack('3i',p.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(item['PID'],0,0);body=b''
  while True:
   assert now()<until;p.settimeout(max(.001,min(.15,(until-now())/1e9)));v=p.recv(513-len(body))
   if not v:break
   body+=v;assert len(body)<=512
 assert body.endswith(b'\n')and body.count(b'\n')==1
 v=json.loads(body);assert set(v)=={'schema','revision','until_ns','available','local_interfaces','address_records'}and type(v['schema'])is int and v['schema']==1 and type(v['revision'])is int
 raw=v['until_ns'];assert type(raw)is str and(raw=='0'or raw.isascii()and raw.isdecimal()and not raw.startswith('0'))and int(raw)<=18446744073709551615
 if v['available']is True:
  assert v['revision']>0 and type(v['local_interfaces'])is type(v['address_records'])is int and v['local_interfaces']==v['address_records']==0 and now()<int(raw)<=now()+2_000_000_000;return v
 assert v['available']is False and raw=='0'and v['local_interfaces']is v['address_records']is None
 return None
def ready(item,until):
 while now()<until:
  try:
   v=snapshot(item,until)
   if v:return v
  except(FileNotFoundError,ConnectionRefusedError,socket.timeout):pass
  time.sleep(.01)
 raise AssertionError('original actual healthy-empty readiness cutoff')
def stop(item,command,until):
 cg=Path('/sys/fs/cgroup')/item['ControlGroup'].lstrip('/');members=[]
 if(cg/'cgroup.procs').exists():
  for raw in(cg/'cgroup.procs').read_text().split():
   pid=int(raw)
   if pid!=item['PID']:
    try:members.append((pid,os.pidfd_open(pid,os.O_NONBLOCK)))
    except ProcessLookupError:pass
 try:
  command(['/usr/bin/systemctl','stop',UNIT],'network-PID1-Stop',seconds=10,absolute_until=until,cleanup=True);v=props(command,until,True)
  assert[v[k]for k in ['ActiveState','SubState','MainPID','Result','ExecMainCode','ExecMainStatus']]==['inactive','dead','0','success','1','0']and v['ExecMainPID']==str(item['PID'])
  assert select.select([item['fd']],[],[],0)[0]and not Path('/proc',str(item['PID'])).exists()and not Path(SOCKET).exists()
  assert not(cg/'cgroup.procs').exists()or not(cg/'cgroup.procs').read_text().strip()
  assert all(select.select([fd],[],[],0)[0]and not Path('/proc',str(pid)).exists()for pid,fd in members)
  return {'final_PID1':v,'retained_Main_pidfd_dead':True,'proc_and_socket_absent':True,'cgroup_empty':True,'observed_descendant_pidfds_dead':len(members),'native_service_wait_is_PID1_owned_not_Scope_parent_wait':True}
 finally:
  for _,fd in members:os.close(fd)
def run(command,R,executable,env,initial,work,close,exe_sha):
 R=Path(R);START=now();UNTIL=min(work,START+12_000_000_000);CUSTODY=min(close,START+16_000_000_000);scope=Scope(R);item=None;second=None;first_stopped=False;second_stopped=False
 r={'schema':1,'passed':False,'original_case_start_ns':START,'original_case_until_ns':UNTIL,'original_case_close_ns':CUSTODY,'same_frozen_Cpp_consumer':True,'PID1_owns_service_Main_and_collectors':True,'normal_main_window_Source_executed':False}
 def wait_marker(prefix):
  while now()<UNTIL:
   scope.pump();scope.check();rows=(R/'network-QML-native-component.log').read_text().splitlines();matches=[v for v in rows if v.startswith(prefix)]
   assert len(matches)<=1
   if matches:return matches[0]
   if not scope.live(item):raise AssertionError('component exited before '+prefix)
   time.sleep(.002)
  raise AssertionError('original frozen QML case marker cutoff: '+prefix)
 def send(body):
  assert now()<UNTIL and scope.live(item);assert item['child'].stdin.write(body)==len(body);item['child'].stdin.flush()
 try:
  r['kernel_only_loopback']=onlylo();current=props(command,UNTIL);assert current['MainPID']==str(initial['PID'])and current['InvocationID']==initial['InvocationID']and not select.select([initial['fd']],[],[],0)[0]
  r['first_PID1']=identity_record(initial);r['initial_independent_socket']=ready(initial,UNTIL)
  item=scope.start([executable,'/usr/share/pokelinux/appearance',str(UNTIL)],'network-QML-native-component.log',131072,stdin=subprocess.PIPE,env=env,identity={'user':11050,'group':11050,'extra_groups':[]},cutoff=CUSTODY,wait_cutoff=CUSTODY)
  r['initial_marker']=wait_marker('NETWORK_QML_INITIAL_ZERO ')
  r['first_Stop']=stop(initial,command,UNTIL);first_stopped=True;send(b'L\n')
  r['expiry_marker']=wait_marker('NETWORK_QML_EXPIRED_DASH ')
  command(['/usr/bin/systemctl','start',UNIT],'network-one-fresh-PID1-start',seconds=10,absolute_until=UNTIL)
  second=bind(command,UNTIL,exe_sha);assert second['InvocationID']!=initial['InvocationID'];r['fresh_PID1']=identity_record(second);r['fresh_independent_socket']=ready(second,UNTIL);send(b'R\n');item['child'].stdin.close()
  r['reconnected_marker']=wait_marker('NETWORK_QML_RECONNECTED_ZERO ');r['PASS_marker']=wait_marker('PASS real normal service QML 0/0 then expired dash then fresh reconnection 0/0')
  assert scope.finish(item,CUSTODY)==0;r['actual_component_parent_wait_exit']=0
  r['second_Stop']=stop(second,command,CUSTODY);second_stopped=True;r['functional_passed']=True
 except BaseException as error:
  r['error']=type(error).__name__+': '+str(error)
 finally:
  if item and item['child'].stdin and not item['child'].stdin.closed:item['child'].stdin.close()
  r['component_cleanup_errors']=scope.close(CUSTODY);r['component_owned_receipts']=scope.receipts
  for identity,done in [(second,second_stopped),(initial,first_stopped)]:
   if identity and not done:
    try:r.setdefault('failure_service_Stops',[]).append(stop(identity,command,min(close,CUSTODY)))
    except BaseException as error:r.setdefault('failure_service_Stop_errors',[]).append(repr(error))
  if second:os.close(second['fd'])
  r['completed_ns']=now();r['passed']=bool(r.get('functional_passed')and not r['component_cleanup_errors']and not r.get('failure_service_Stop_errors')and len(scope.children)==len(scope.receipts)==1 and all(v['returncode']==0 and v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']for v in scope.receipts)and now()<CUSTODY)
  (R/'network-QML-installed-component-result.json').write_text(json.dumps(r,indent=2)+'\n')
 assert r['passed'];return r
