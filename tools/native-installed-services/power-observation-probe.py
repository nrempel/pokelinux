"""One fixed installed ARM pair contract, identical for original RED/fresh cold GREEN."""
from pathlib import Path
import argparse,hashlib,json,os,select,socket,stat,struct,sys,time,traceback
sys.dont_write_bytecode=True
from owned import Scope,now
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--until-ns',type=int,required=True);p.add_argument('--close-ns',type=int,required=True);a=p.parse_args()
R=Path('/results');P=Path('/run/pokelinux-pair-inputs');assert a.output.parent==R
W=a.until_ns;C=a.close_ns;M=json.loads((P/'bundle-manifest.json').read_text());serial=0;owners=[];receipts=[];close_failed=False;identities={};stopped=False
ROLES={'battery':{'unit':'pokelinux-battery.service','exe':'/usr/libexec/pokelinux/battery-service','socket':'/run/pokelinux-battery/snapshot.sock','directory':'/run/pokelinux-battery','directory_mode':0o755,'socket_mode':0o666,'kind':socket.SOCK_STREAM},'brightness':{'unit':'pokelinux-brightness.service','exe':'/usr/libexec/pokelinux/brightness-service','socket':'/run/pokelinux-brightness/actor.sock','directory':'/run/pokelinux-brightness','directory_mode':0o700,'socket_mode':0o600,'kind':socket.SOCK_SEQPACKET}}
FIELDS='LoadState,UnitFileState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,ExecMainCode,ExecMainStatus,Result'
EXPECTED={'usr/libexec/pokelinux/battery-service','usr/lib/systemd/system/pokelinux-battery.service','etc/pokelinux/battery.conf','usr/libexec/pokelinux/brightness-service','usr/lib/systemd/system/pokelinux-brightness.service','etc/pokelinux/brightness.conf','etc/modules-load.d/pokelinux-backlight-vm.conf'}
DEVICE=Path('/sys/class/backlight/pokelinux-virtual-backlight');DRIVER=Path('/sys/bus/platform/drivers/gpio-backlight');QUERY=P/'fixture-bin/brightness-readonly-probe'
r={'schema':1,'passed':False,'scope':'Identical installed-boundary/readonly pair consumer; no install/enable/start, Source, Qt or phone','original_until_ns':W,'original_close_ns':C,'cases':[],'no_service_start_or_install_operations':True}
def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def command(argv,label,until=None,expected=(0,),cleanup=False):
 global serial,close_failed
 assert not close_failed,'incomplete prior command closure forbids new child'
 serial+=1;cutoff=min(C if cleanup else W,until if until is not None else now()+10_000_000_000);scope=Scope(R);owners.append(scope);name=f'{a.output.stem}-{serial:03d}-{label}.log'
 try:
  child=scope.start(argv,name,262144,cutoff=cutoff);code=scope.finish(child,cutoff);body=(R/name).read_text();assert code in expected,(label,code,body[-1000:]);return code,body
 finally:
  errors=scope.close(C);receipts.extend(scope.receipts)
  if errors:close_failed=True;r.setdefault('owned_close_errors',[]).append(errors)
  assert not errors,errors
def properties(role,until=None,cleanup=False):
 code,body=command(['/usr/bin/systemctl','show',ROLES[role]['unit'],'--property='+FIELDS],role+'-properties',until,(0,1),cleanup);v=dict(line.split('=',1)for line in body.splitlines());assert 'LoadState'in v
 assert code==0 or v['LoadState']=='not-found';return v

def dead(fd):return bool(select.select([fd],[],[],0)[0])
def retain(role,v):
 pid=int(v['MainPID']);fd=os.pidfd_open(pid,os.O_NONBLOCK);identities[role]={'pid':pid,'pidfd':fd,'InvocationID':v['InvocationID'],'ControlGroup':v['ControlGroup']};assert pid>1 and not dead(fd)
 assert int(Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[1])==1
 for prefix in ['Uid:','Gid:']:assert next(line for line in Path(f'/proc/{pid}/status').read_text().splitlines()if line.startswith(prefix)).split()[1:]==['0']*4
 assert v['ControlGroup'].startswith('/system.slice/')and '..'not in Path(v['ControlGroup']).parts and Path(f'/proc/{pid}/cgroup').read_text().strip()=='0::'+v['ControlGroup']
 until=min(W,now()+10_000_000_000);samples=[]
 while now()<until:
  assert not dead(fd)
  try:
   exe=Path(f'/proc/{pid}/exe');target=str(exe.readlink());digest=sha(exe);samples.append({'readlink':target,'sha256':digest});assert len(samples)<=1024
   if target==ROLES[role]['exe']and digest==M['files'][target.lstrip('/')]['sha256']:
    peer(role,until);r.setdefault('readiness',{})[role]={'samples':samples,'original_until_ns':until,'root_peer_matches_Main':True};return
  except (FileNotFoundError,ConnectionRefusedError,socket.timeout):pass
  time.sleep(.002)
 raise AssertionError('actual executable/root socket readiness original cutoff')
def peer(role,until):
 v=ROLES[role];identity=identities[role];assert now()<until and not dead(identity['pidfd'])
 with socket.socket(socket.AF_UNIX,v['kind'])as s:
  s.settimeout(min(1,max(.001,(until-now())/1e9)));s.connect(v['socket']);assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(identity['pid'],0,0)

def same():
 for role,identity in identities.items():
  v=properties(role);assert v['MainPID']==str(identity['pid'])and v['InvocationID']==identity['InvocationID']and not dead(identity['pidfd'])

def installed():
 for rel,row in M['files'].items():
  f=Path('/')/rel;m=f.lstat();assert stat.S_ISREG(m.st_mode)and m.st_uid==m.st_gid==0 and stat.S_IMODE(m.st_mode)==row['mode']and m.st_size==row['bytes']and sha(f)==row['sha256']
  if row['mode']&0o111:assert struct.unpack('<H',f.read_bytes()[18:20])[0]==183
 for role,v in ROLES.items():
  directory=Path(v['directory']).lstat();s=Path(v['socket']).lstat();assert stat.S_ISDIR(directory.st_mode)and directory.st_uid==directory.st_gid==0 and stat.S_IMODE(directory.st_mode)==v['directory_mode'];assert stat.S_ISSOCK(s.st_mode)and s.st_uid==s.st_gid==0 and stat.S_IMODE(s.st_mode)==v['socket_mode']

def battery():
 assert not Path('/sys/class/power_supply/test_battery').exists()
 until=min(W,now()+10_000_000_000)
 while now()<until:
  with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as s:
   s.settimeout(min(1,max(.001,(until-now())/1e9)));s.connect(ROLES['battery']['socket']);assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(identities['battery']['pid'],0,0);body=b''
   while True:
    assert now()<until;s.settimeout(min(1,max(.001,(until-now())/1e9)));part=s.recv(513-len(body))
    if not part:break
    body+=part;assert len(body)<=512
  assert body.endswith(b'\n')and body.count(b'\n')==1;v=json.loads(body);assert set(v)=={'schema','revision','until_ns','available','capacity_percent','status'}and type(v['schema'])is int and v['schema']==1 and type(v['revision'])is int and v['revision']>=0
  assert v['available']is False and v['capacity_percent']is None and v['status']is None and v['until_ns']=='0'
  if v['revision']>0:same();return v
  time.sleep(.01)
 raise AssertionError('actual missing-supply collection original cutoff')
def kernel():
 assert DEVICE.is_symlink()and DEVICE.resolve().is_relative_to('/sys/devices');assert(DEVICE/'device/driver').resolve(strict=True)==DRIVER and(DEVICE/'device/driver/module').resolve(strict=True)==Path('/sys/module/gpio_backlight')
 values={}
 for name in ['max_brightness','brightness','actual_brightness']:
  raw=(DEVICE/name).read_bytes();assert raw.endswith(b'\n')and raw[:-1].isdigit();values[name]=int(raw[:-1])
 assert values['max_brightness']==1 and values['brightness']in[0,1]and values['actual_brightness']==values['brightness'];return values

def inspect(label,available):
 until=min(W,now()+1_000_000_000);peer('brightness',until);_,body=command([str(QUERY),str(until)],'actual-readonly-Client-'+label,until);lines=body.splitlines();assert len(lines)==1;v=json.loads(lines[0]);assert v['schema']==1 and v['actual_typed_Client_Inspect']is True and v['connected']is True and v['pending']is False and v['revision']>0 and v['requested_percent']is None and v['last_set_uncertain']is False and v['original_until_ns']==str(until)
 if available:
  current=kernel();assert v['observation']=={'max':1,'requested':current['brightness'],'actual':current['actual_brightness']}and now()<int(v['until_ns'])<=now()+2_000_000_000
 else:assert not DEVICE.exists()and v['observation']is None and v['until_ns']=='0'
 same();return v

def stop(cleanup=False):
 global stopped
 before={role:properties(role,cleanup=cleanup)for role in identities};fds=[]
 for role,identity in identities.items():
  cg=Path('/sys/fs/cgroup')/identity['ControlGroup'].lstrip('/')
  if(cg/'cgroup.procs').exists():
   for raw in(cg/'cgroup.procs').read_text().split():
    pid=int(raw)
    if pid!=identity['pid']:
     try:fds.append((pid,os.pidfd_open(pid,os.O_NONBLOCK)))
     except ProcessLookupError:pass
 try:
  command(['/usr/bin/systemctl','stop',*[v['unit']for v in ROLES.values()]],'whole-pair-PID1-Stop',min(C if cleanup else W,now()+10_000_000_000),cleanup=cleanup)
  final={}
  for role,identity in identities.items():
   v=properties(role,cleanup=cleanup);assert(v['ActiveState'],v['SubState'],v['MainPID'],v['Result'],v['ExecMainCode'],v['ExecMainStatus'])==('inactive','dead','0','success','1','0');cg=Path('/sys/fs/cgroup')/identity['ControlGroup'].lstrip('/');assert not(cg/'cgroup.procs').exists()or(cg/'cgroup.procs').read_text().strip()=='';assert dead(identity['pidfd'])and not Path(f"/proc/{identity['pid']}").exists()and not Path(ROLES[role]['socket']).exists();final[role]=v
  assert all(dead(fd)and not Path(f'/proc/{pid}').exists()for pid,fd in fds);r['whole_bundle_Stop']={'final':final,'both_Main_pidfds_dead':True,'both_cgroups_empty':True,'both_sockets_absent':True,'observed_descendants_retained_and_dead':len(fds),'fixture_parent_wait_for_PID1_children_claimed':False};stopped=True
 finally:
  for _,fd in fds:os.close(fd)
try:
 assert os.getresuid()==os.getresgid()==(0,0,0)and Path('/proc/1/comm').read_text().strip()=='systemd'and os.uname().release=='7.1.6-1-aarch64-ARCH'and os.uname().machine=='aarch64';assert M['schema']==1 and set(M['files'])==EXPECTED
 _,packages=command(['/usr/bin/pacman','-Q'],'actual353');assert packages==(P/'source/expected-installed353.txt').read_text();r['original353_exact']=True
 for role in ROLES:
  row={'role':role,'passed':False,'failure_kind':None};v=properties(role);row['initial']=v
  try:
   assert v['LoadState']=='loaded','required installed unit LoadState=loaded'
   assert v['UnitFileState']=='enabled','required installed unit UnitFileState=enabled'
   assert v['ActiveState']=='active'and v['SubState']=='running'and int(v['MainPID'])>1,'required already active PID1 Main'
   row['passed']=True
  except AssertionError as error:row['failure_kind']='installed-boundary-assertion';row['failure']=str(error)
  r['cases'].append(row)
 if not all(row['passed']for row in r['cases']):raise AssertionError('both installed enabled active boundaries required')
 r['installed_boundary_satisfied']=True
 for row in r['cases']:retain(row['role'],row['initial'])
 installed();r['battery_completed_collection_unavailable']=battery();r['brightness_initial']={'kernel':kernel(),'Client':inspect('initial',True)};same()
 platform=(DEVICE/'device').resolve(strict=True);assert platform.name=='pokelinux-virtual-backlight';(DRIVER/'unbind').write_text(platform.name+'\n');assert not DEVICE.exists();r['brightness_absent']=inspect('absent',False);same()
 until=min(W,now()+10_000_000_000);(DRIVER/'bind').write_text(platform.name+'\n')
 while not DEVICE.exists()and now()<until:time.sleep(.002)
 assert DEVICE.exists()and now()<until;r['brightness_recovered']={'kernel':kernel(),'Client':inspect('recovered',True)};assert r['brightness_recovered']['Client']['revision']>r['brightness_absent']['revision']>r['brightness_initial']['Client']['revision'];same();stop();r['functional_passed']=True
except BaseException as error:r['error']=type(error).__name__+': '+str(error);r['traceback']=traceback.format_exc()
finally:
 if identities and not stopped:
  try:stop(True)
  except BaseException as error:r['failure_Stop_error']=repr(error)
 for identity in identities.values():os.close(identity['pidfd'])
 r['owned_process_receipts']=receipts;r['all_commands_actual_wait_EOF']=all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']for v in receipts);r['completed_ns']=now();r['passed']=bool(r.get('functional_passed')and not close_failed and not r.get('failure_Stop_error')and r['all_commands_actual_wait_EOF']and now()<C);a.output.write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'passed':r['passed'],'cases':[{'role':v['role'],'passed':v['passed'],'failure_kind':v['failure_kind']}for v in r['cases']],'error':r.get('error')}),flush=True);raise SystemExit(0 if r['passed']else 1)
