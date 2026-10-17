#!/usr/bin/env python3
"""Real installed PID1 battery service and kernel test_power; no app or Source."""
import argparse,hashlib,json,os,select,socket,stat,struct,traceback,sys
from pathlib import Path
sys.dont_write_bytecode=True
from owned import Scope,now
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--manifest',type=Path,required=True);p.add_argument('--until-ns',type=int,required=True);a=p.parse_args()
UNTIL=min(a.until_ns,now()+90_000_000_000);CLOSE=UNTIL+10_000_000_000;scope=Scope(a.output.parent);serial=0;mainfd=None;readerfd=None;readerpid=None;loaded=False
UNIT='pokelinux-battery.service';SOCKET=Path('/run/pokelinux-battery/snapshot.sock');EXE=Path('/usr/libexec/pokelinux/battery-service');PARAM=Path('/sys/module/test_power/parameters');SUPPLY=Path('/sys/class/power_supply/test_battery')
result={'schema':1,'passed':False,'original_until_ns':UNTIL,'scope':'Real kernel test_power to installed production battery service and PID1 Stop','app_Source_phone_or_power_policy_claimed':False}
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def command(argv,label,until=UNTIL):
 global serial
 serial+=1;code,body=scope.run(argv,f'{serial:03d}-{label}.log',until,limit=65536);assert code==0,(label,code,body[-1000:]);return body.decode()
def properties(until=UNTIL):return dict(x.split('=',1) for x in command(['/usr/bin/systemctl','show',UNIT,'--property=LoadState,UnitFileState,ActiveState,SubState,MainPID,ControlGroup,ExecMainCode,ExecMainStatus,Result,InvocationID'],'unit-properties',until).splitlines())
def dead(fd):return bool(select.select([fd],[],[],0)[0])
def parent(pid):return int(Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[1])
def root_process(pid):
 text=Path(f'/proc/{pid}/status').read_text();assert all(next(x for x in text.splitlines() if x.startswith(k)).split()[1:]==['0']*4 for k in ['Uid:','Gid:']);assert sha(Path(f'/proc/{pid}/exe'))==sha(EXE)
def snapshot(until):
 assert now()<until and not dead(mainfd)
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
  s.settimeout(min(1,(until-now())/1e9));s.connect(str(SOCKET));assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(pid,0,0);body=b''
  while True:
   assert now()<until;s.settimeout(min(1,(until-now())/1e9));part=s.recv(513-len(body))
   if not part:break
   body+=part;assert len(body)<=512
 assert now()<until and body.endswith(b'\n') and body.count(b'\n')==1
 v=json.loads(body);assert set(v)=={'schema','revision','until_ns','available','capacity_percent','status'} and type(v['schema']) is int and v['schema']==1 and type(v['revision']) is int and v['revision']>=0 and type(v['available']) is bool and type(v['until_ns']) is str and v['until_ns'].isdigit();return v
def uevent():
 assert SUPPLY.is_symlink() and str(SUPPLY.resolve()).startswith('/sys/devices/')
 with (SUPPLY/'uevent').open('rb') as f:body=f.read(4097)
 assert len(body)<=4096 and body.endswith(b'\n');v=dict(x.split('=',1) for x in body.decode().splitlines());assert v['POWER_SUPPLY_TYPE']=='Battery';return v
def observed(cap,status,present,previous):
 until=min(UNTIL,now()+10_000_000_000)
 while now()<until:
  u=uevent();assert u['POWER_SUPPLY_PRESENT']==str(present)
  if present:assert u['POWER_SUPPLY_CAPACITY']==str(cap) and u['POWER_SUPPLY_STATUS']==status.capitalize()
  v=snapshot(until)
  if v['revision']>previous and (v['available'],v['capacity_percent'],v['status'])==(bool(present),cap,status):
   assert (now()<int(v['until_ns'])<=now()+2_000_000_000) if present else v['until_ns']=='0';assert not dead(mainfd);return {'uevent':u,'snapshot':v}
  assert not select.select([mainfd],[],[],min(.01,max(0,(until-now())/1e9)))[0]
 raise AssertionError('Kernel change must reach this same production service before original observation cutoff')
try:
 assert os.getresuid()==os.getresgid()==(0,0,0) and Path('/proc/1/comm').read_text().strip()=='systemd'
 initial=properties();result['initial_unit']=initial
 assert initial['LoadState']=='loaded','Installed production battery unit must be present'
 assert initial['UnitFileState']=='enabled' and initial['ActiveState']=='active' and initial['SubState']=='running'
 pid=int(initial['MainPID']);assert pid>1 and parent(pid)==1;mainfd=os.pidfd_open(pid,os.O_NONBLOCK);root_process(pid)
 manifest=json.loads(a.manifest.read_text());assert set(manifest['files'])=={'usr/libexec/pokelinux/battery-service','usr/lib/systemd/system/pokelinux-battery.service','etc/pokelinux/battery.conf'}
 for rel,row in manifest['files'].items():
  f=Path('/')/rel;m=f.lstat();assert stat.S_ISREG(m.st_mode) and m.st_uid==m.st_gid==0 and stat.S_IMODE(m.st_mode)==row['mode'] and m.st_size==row['bytes'] and sha(f)==row['sha256']
 assert EXE.read_bytes()[:6]==b'\x7fELF\x02\x01' and struct.unpack('<H',EXE.read_bytes()[18:20])[0]==62
 assert os.uname().release=='7.2.9-arch1-1';module=Path('/usr/lib/modules')/os.uname().release/'kernel/drivers/power/supply/test_power.ko.zst';assert module.stat().st_size==8813 and sha(module)=='da0782f3d00b46cf5f301a202ac8edee494f1fdd7894c579d152b43cbeb847fa'
 assert not PARAM.exists();command(['/usr/bin/modprobe','test_power','battery_capacity=73','battery_status=charging','battery_present=true'],'actual-test_power-load');loaded=True
 result['kernel']=os.uname().release;result['installed_module_sha256']=sha(module);result['observations']=[observed(73,'charging',1,-1)]
 (PARAM/'battery_capacity').write_text('51\n');(PARAM/'battery_status').write_text('discharging\n');assert (PARAM/'battery_capacity').read_text().strip()=='51'
 result['observations'].append(observed(51,'discharging',1,result['observations'][-1]['snapshot']['revision']))
 (PARAM/'battery_present').write_text('false\n');result['module_presence_getter_raw']=(PARAM/'battery_present').read_text()
 result['observations'].append(observed(None,None,0,result['observations'][-1]['snapshot']['revision']))
 until=min(UNTIL,now()+5_000_000_000)
 while readerfd is None and now()<until:
  for child in Path(f'/proc/{pid}/task/{pid}/children').read_text().split():
   fd=None
   try:
    child=int(child);args=Path(f'/proc/{child}/cmdline').read_bytes().split(b'\0')[:-1]
    if len(args)!=6 or args[1:3]!=[b'--read-supply',str(pid).encode()] or args[4:]!=[b'/sys/class/power_supply',b'test_battery']:continue
    fd=os.pidfd_open(child,os.O_NONBLOCK)
    if dead(fd):os.close(fd);continue
    assert parent(child)==pid;root_process(child);readerpid=child;readerfd=fd;break
   except (FileNotFoundError,ProcessLookupError):
    if fd is not None:os.close(fd)
    continue
  if readerfd is None:assert not select.select([mainfd],[],[],min(.0001,max(0,(until-now())/1e9)))[0]
 assert readerfd is not None,'An actual production --read-supply child must be retained'
 result['actual_reader_PID']=readerpid;before_stop=properties();assert before_stop['MainPID']==str(pid) and before_stop['InvocationID']==initial['InvocationID'] and not dead(mainfd)
 command(['/usr/bin/systemctl','stop',UNIT],'actual-PID1-Stop');final=properties();result['final_unit']=final
 assert (final['ActiveState'],final['SubState'],final['MainPID'],final['Result'],final['ExecMainCode'],final['ExecMainStatus'])==('inactive','dead','0','success','1','0')
 cg=Path('/sys/fs/cgroup')/initial['ControlGroup'].lstrip('/');assert not cg.exists() or not (cg/'cgroup.procs').read_text().strip()
 assert dead(mainfd) and dead(readerfd) and not Path(f'/proc/{pid}').exists() and not Path(f'/proc/{readerpid}').exists() and not SOCKET.exists()
 result['same_Main_PID_three_observations']=True;result['PID1_normal_Main_exit0_cgroup_empty_reader_dead']=True;result['guest_parent_wait_for_PID1_service_or_reader_claimed']=False;result['functional_passed']=True
except BaseException as e:result['error']=type(e).__name__+': '+str(e);result['traceback']=traceback.format_exc()
finally:
 if mainfd is not None and not dead(mainfd):
  try:command(['/usr/bin/systemctl','stop',UNIT],'bounded-failure-Stop',CLOSE)
  except BaseException as e:result['failure_Stop_error']=repr(e)
 if loaded:
  try:command(['/usr/bin/modprobe','-r','test_power'],'owned-test_power-unload',CLOSE);assert not PARAM.exists()
  except BaseException as e:result['driver_cleanup_error']=repr(e)
 for fd in [readerfd,mainfd]:
  if fd is not None:os.close(fd)
 result['cleanup_errors']=scope.close(CLOSE);result['owned_process_receipts']=scope.receipts;result['passed']=bool(result.get('functional_passed') and not result['cleanup_errors'] and not result.get('failure_Stop_error') and not result.get('driver_cleanup_error') and all(v['actual_direct_parent_wait'] and v['stdout_EOF'] and v['wait_before_original_cutoff'] and v['EOF_before_original_cutoff'] for v in scope.receipts));result['completed_ns']=now();a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'passed':result['passed'],'error':result.get('error')}),flush=True)
if not result['passed']:raise SystemExit(1)
