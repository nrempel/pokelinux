"""Same positive installed-network consumer for missing-unit RED and fresh cold GREEN."""
from pathlib import Path
import argparse,hashlib,json,os,select,socket,stat,struct,sys,time,traceback
sys.dont_write_bytecode=True
from owned import Scope,now
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--until-ns',type=int,required=True);p.add_argument('--close-ns',type=int,required=True);a=p.parse_args()
R=Path('/results');P=Path('/run/pokelinux-network-inputs');S=json.loads((R/'selection.json').read_text());assert a.output.parent==R
W=a.until_ns;C=a.close_ns;UNIT='pokelinux-network-status.service';EXE='/usr/libexec/pokelinux/pokelinux-network-status';SOCKET='/run/pokelinux-network/snapshot.sock';owners=[];receipts=[];serial=0;blocked=False;identity=None;stopped=False
r={'schema':1,'passed':False,'scope':'Same installed boundary/readonly healthy-empty local network consumer; no install/start/network mutation','original_until_ns':W,'original_close_ns':C,'failure_kind':'setup','no_install_enable_reload_start_or_network_mutation':True}
def sha(path):
 with Path(path).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def command(argv,label,until=None,expected=(0,),cleanup=False):
 global serial,blocked
 assert not blocked,'prior incomplete custody fences admission';serial+=1;scope=Scope(R);owners.append(scope);cutoff=min(C if cleanup else W,until or now()+10_000_000_000);name=f'{a.output.stem}-{serial:03d}-{label}.log'
 try:
  child=scope.start(argv,name,131072,cutoff=cutoff);code=scope.finish(child,cutoff);body=(R/name).read_text();assert code in expected,(label,code,body[-800:]);return code,body
 finally:
  errors=scope.close(C);receipts.extend(scope.receipts)
  if errors:blocked=True;r.setdefault('owned_close_errors',[]).append(errors)
  assert not errors,errors
def props(cleanup=False):
 code,body=command(['/usr/bin/systemctl','show',UNIT,'--property=LoadState,UnitFileState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,ExecMainCode,ExecMainStatus,Result'],'unit-properties',expected=(0,1),cleanup=cleanup);v=dict(line.split('=',1)for line in body.splitlines());assert 'LoadState'in v and(code==0 or v['LoadState']=='not-found');return v
def onlylo():
 assert sorted(x.name for x in Path('/sys/class/net').iterdir())==['lo'];assert int(Path('/sys/class/net/lo/flags').read_text().strip(),16)&8
 names=[line.split(':',1)[0].strip()for line in Path('/proc/net/dev').read_text().splitlines()[2:]];assert names==['lo'];return {'independent_sysfs_and_proc_only_loopback':True,'coarse_expected_interfaces':0,'coarse_expected_address_records':0}
def same():
 v=props();assert v['MainPID']==str(identity['pid'])and v['InvocationID']==identity['InvocationID']and not select.select([identity['pidfd']],[],[],0)[0];return v

def stop(cleanup=False):
 global stopped
 cg=Path('/sys/fs/cgroup')/identity['ControlGroup'].lstrip('/');fds=[]
 if(cg/'cgroup.procs').exists():
  for raw in(cg/'cgroup.procs').read_text().split():
   pid=int(raw)
   if pid!=identity['pid']:
    try:fds.append((pid,os.pidfd_open(pid,os.O_NONBLOCK)))
    except ProcessLookupError:pass
 try:
  command(['/usr/bin/systemctl','stop',UNIT],'PID1-Stop',min(C if cleanup else W,now()+10_000_000_000),cleanup=cleanup);v=props(cleanup)
  assert[v[k]for k in ['ActiveState','SubState','MainPID','Result','ExecMainCode','ExecMainStatus']]==['inactive','dead','0','success','1','0'];assert not(cg/'cgroup.procs').exists()or not(cg/'cgroup.procs').read_text().strip();assert select.select([identity['pidfd']],[],[],0)[0]and not Path(f"/proc/{identity['pid']}").exists()and not Path(SOCKET).exists();assert all(select.select([fd],[],[],0)[0]and not Path(f'/proc/{pid}').exists()for pid,fd in fds)
  r['normal_Stop']={'final':v,'Main_pidfd_dead':True,'Main_proc_and_socket_absent':True,'cgroup_empty':True,'observed_descendant_pidfds_dead':len(fds),'fixture_parent_wait_for_PID1_children_claimed':False};stopped=True
 finally:
  for _,fd in fds:os.close(fd)
try:
 assert os.getresuid()==os.getresgid()==(0,0,0)and os.uname().machine=='aarch64'and os.uname().release=='7.1.6-1-aarch64-ARCH'and Path('/proc/1/comm').read_text().strip()=='systemd'
 _,packages=command(['/usr/bin/pacman','-Q'],'actual353');assert packages==(P/'source/expected-installed353.txt').read_text();r['original353_exact']=True;r['independent_kernel_baseline']=onlylo();initial=props();r['initial']=initial
 try:
  assert initial['LoadState']=='loaded','required installed unit LoadState=loaded'
  assert initial['UnitFileState']=='enabled','required installed unit UnitFileState=enabled'
  assert initial['ActiveState']=='active'and initial['SubState']=='running'and int(initial['MainPID'])>1,'required already-active PID1 Main'
 except AssertionError:
  r['failure_kind']='installed-boundary-assertion';raise
 r['installed_boundary_satisfied']=True;r['failure_kind']='installed-runtime-assertion'
 files=S['installed_files'];assert set(files)=={'usr/libexec/pokelinux/pokelinux-network-status','usr/lib/systemd/system/pokelinux-network-status.service'}
 for rel,pin in files.items():
  f=Path('/')/rel;m=f.lstat();assert stat.S_ISREG(m.st_mode)and m.st_uid==m.st_gid==0 and m.st_size==pin['bytes']and stat.S_IMODE(m.st_mode)==pin['mode']and sha(f)==pin['sha256']
 pid=int(initial['MainPID']);identity={'pid':pid,'pidfd':os.pidfd_open(pid,os.O_NONBLOCK),'InvocationID':initial['InvocationID'],'ControlGroup':initial['ControlGroup']};assert identity['InvocationID']and identity['ControlGroup'].startswith('/system.slice/')and not select.select([identity['pidfd']],[],[],0)[0];assert int(Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[1])==1
 for prefix in ['Uid:','Gid:']:assert next(line for line in Path(f'/proc/{pid}/status').read_text().splitlines()if line.startswith(prefix)).split()[1:]==['0']*4
 ns=Path('/proc/self/ns/net').stat();main_ns=Path(f'/proc/{pid}/ns/net').stat();assert(ns.st_dev,ns.st_ino)==(main_ns.st_dev,main_ns.st_ino)
 until=min(W,now()+10_000_000_000);samples=[]
 while now()<until:
  assert not select.select([identity['pidfd']],[],[],0)[0]
  try:
   exe=Path(f'/proc/{pid}/exe');target=str(exe.readlink());digest=sha(exe);samples.append({'path':target,'sha256':digest});assert len(samples)<=1024
   if target!=EXE or digest!=files[EXE.lstrip('/')]['sha256']:time.sleep(.002);continue
   directory=Path('/run/pokelinux-network').lstat();sock=Path(SOCKET).lstat();assert stat.S_ISDIR(directory.st_mode)and directory.st_uid==directory.st_gid==0 and stat.S_IMODE(directory.st_mode)==0o755 and stat.S_ISSOCK(sock.st_mode)and sock.st_uid==sock.st_gid==0 and stat.S_IMODE(sock.st_mode)==0o666
   with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as peer:
    peer.settimeout(min(1,max(.001,(until-now())/1e9)));peer.connect(SOCKET);assert struct.unpack('3i',peer.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(pid,0,0);body=b''
    while True:
     assert now()<until;peer.settimeout(min(1,max(.001,(until-now())/1e9)));part=peer.recv(513-len(body))
     if not part:break
     body+=part;assert len(body)<=512
   assert body.endswith(b'\n')and body.count(b'\n')==1;v=json.loads(body);assert set(v)=={'schema','revision','until_ns','available','local_interfaces','address_records'}and type(v['schema'])is int and v['schema']==1 and type(v['revision'])is int and v['revision']>=0
   raw=v['until_ns'];assert isinstance(raw,str)and(raw=='0'or(raw.isascii()and raw.isdecimal()and raw[0]!='0'))and int(raw)<=18446744073709551615
   if v['available']is True:
    assert v['revision']>0 and type(v['local_interfaces'])is int and type(v['address_records'])is int and v['local_interfaces']==v['address_records']==0 and now()<int(raw)<=now()+2_000_000_000;onlylo();same();r['healthy_empty_snapshot']=v;r['readiness']={'exact_exe_samples':samples,'root_peer_matches_Main':True,'inherited_namespace_matches_consumer':True,'original_until_ns':until,'stream_EOF':True,'no_request_bytes_sent':True};break
   assert v['available']is False and v['local_interfaces']is None and v['address_records']is None and raw=='0'
  except(FileNotFoundError,ConnectionRefusedError,socket.timeout):pass
  time.sleep(.01)
 else:raise AssertionError('original healthy-empty collection/readiness cutoff')
 same();stop();r['functional_passed']=True;r['failure_kind']=None
except BaseException as error:r['error']=type(error).__name__+': '+str(error);r['traceback']=traceback.format_exc()
finally:
 if identity and not stopped:
  try:stop(True)
  except BaseException as error:r['failure_Stop_error']=repr(error)
 if identity:os.close(identity['pidfd'])
 r['owned_process_receipts']=receipts;r['all_commands_actual_wait_EOF']=bool(receipts)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']for v in receipts);r['completed_ns']=now();r['passed']=bool(r.get('functional_passed')and not blocked and not r.get('failure_Stop_error')and r['all_commands_actual_wait_EOF']and now()<C);a.output.write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'passed':r['passed'],'failure_kind':r['failure_kind'],'error':r.get('error')}),flush=True);raise SystemExit(0 if r['passed']else 1)
