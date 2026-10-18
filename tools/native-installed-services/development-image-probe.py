"""One positive combined boundary/readonly contract for missing-install RED and cold boots."""
from pathlib import Path
import argparse,hashlib,json,os,select,stat,sys,time,traceback
sys.dont_write_bytecode=True
from owned import Scope,now
import installed_component
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--until-ns',type=int,required=True);p.add_argument('--close-ns',type=int,required=True);a=p.parse_args()
R=Path('/results');P=Path('/run/pokelinux-development-inputs');S=json.loads((R/'selection.json').read_text());M=json.loads((P/'full-manifest.json').read_text());W=a.until_ns;C=a.close_ns;assert a.output.parent==R
ROLES={'battery':('pokelinux-battery.service','/usr/libexec/pokelinux/battery-service','/run/pokelinux-battery/snapshot.sock'),'brightness':('pokelinux-brightness.service','/usr/libexec/pokelinux/brightness-service','/run/pokelinux-brightness/actor.sock'),'network':('pokelinux-network-status.service','/usr/libexec/pokelinux/pokelinux-network-status','/run/pokelinux-network/snapshot.sock')}
SETTINGS=['usr/libexec/pokelinux/pokelinux-settings','usr/share/pokelinux/appearance/PhoneAppearance.qml','usr/share/applications/org.omarchy.Settings.desktop'];FIELDS='LoadState,UnitFileState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,Result,ExecMainCode,ExecMainStatus';owners=[];receipts=[];serial=0;blocked=False;identities={};network_initial=None;functional_done=False
r={'schema':1,'passed':False,'scope':'Combined installed ARM three-owner coexistence then original serial component/power recovery and Stops; no normal Main/window/Source/phone','original_until_ns':W,'original_close_ns':C,'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip(),'cases':[],'failure_kind':'setup','manual_start_before_initial_coexistence':False}
def sha(path):
 with Path(path).open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def command(argv,label,seconds=10,limit=262144,env=None,identity=None,absolute_until=None,cleanup=False,expected=(0,)):
 global serial,blocked
 assert not blocked,'incomplete prior command custody fences admission';serial+=1;scope=Scope(R);owners.append(scope);until=min(C if cleanup else W,now()+int(seconds*1e9),absolute_until if absolute_until is not None else W);name=f'combined-{serial:03d}-{label}.log'
 try:
  child=scope.start(argv,name,limit,env=env,identity=identity,cutoff=until);code=scope.finish(child,until);body=(R/name).read_text();assert code in expected,(label,code,body[-1000:]);return body
 finally:
  errors=scope.close(C);receipts.extend(scope.receipts)
  if errors:blocked=True;r.setdefault('owned_close_errors',[]).append(errors)
  assert not errors,errors
def props(role,cleanup=False):
 body=command(['/usr/bin/systemctl','show',ROLES[role][0],'--property='+FIELDS],role+'-properties',cleanup=cleanup,expected=(0,1));v=dict(line.split('=',1)for line in body.splitlines());assert 'LoadState'in v;return v
def exact_file(relative):
 row=M['files'][relative];f=Path('/')/relative;m=f.lstat();assert stat.S_ISREG(m.st_mode)and m.st_uid==m.st_gid==0 and m.st_size==row['bytes']and stat.S_IMODE(m.st_mode)==row['mode']and sha(f)==row['sha256']
def metadata(path):
 f=Path(path);m=f.lstat();assert stat.S_ISREG(m.st_mode)and m.st_uid==m.st_gid==0;return {'bytes':m.st_size,'sha256':sha(f),'mode':stat.S_IMODE(m.st_mode),'uid':m.st_uid,'gid':m.st_gid,'inode':m.st_ino,'mtime_ns':m.st_mtime_ns}
def preservation():
 anchor=S['preservation_anchor'];assert anchor and set(anchor)=={'/etc/pokelinux/battery.conf','/etc/pokelinux/brightness.conf','/var/lib/pokelinux-development-fixture/unrelated-sentinel'};actual={p:metadata(p)for p in anchor};assert actual==anchor;return actual
def power_same():
 for role in ['battery','brightness']:
  v=props(role);i=identities[role];assert v['MainPID']==str(i['pid'])and v['InvocationID']==i['InvocationID']and not select.select([i['fd']],[],[],0)[0]
try:
 assert S['providers_complete']and len(M['files'])==12 and os.getresuid()==os.getresgid()==(0,0,0)and os.uname().machine=='aarch64'and os.uname().release=='7.1.6-1-aarch64-ARCH'and Path('/proc/1/comm').read_text().strip()=='systemd'
 before=command(['/usr/bin/pacman','-Q'],'original353');assert before==(P/'source/expected-installed353.txt').read_text();r['original353_exact']=True;r['independent_only_loopback']=installed_component.onlylo()
 # Evaluate all new installed boundaries before any fixture/normal component launch.
 for role in ROLES:
  v=props(role);row={'role':role,'passed':False,'initial':v}
  try:assert v['LoadState']=='loaded'and v['UnitFileState']=='enabled'and v['ActiveState']=='active'and v['SubState']=='running'and int(v['MainPID'])>1;row['passed']=True
  except AssertionError:row['failure_kind']='installed-boundary-assertion'
  r['cases'].append(row)
 row={'role':'Settings-installed-files','passed':False}
 try:
  for relative in SETTINGS:exact_file(relative)
  row['passed']=True
 except(FileNotFoundError,AssertionError)as error:row['failure_kind']='installed-boundary-assertion';row['error']=repr(error)
 r['cases'].append(row)
 if not all(v['passed']for v in r['cases']):r['failure_kind']='installed-boundary-assertion';raise AssertionError('all three enabled already-active owners and installed Settings files required')
 r['failure_kind']='combined-runtime-assertion';r['all_three_already_active_before_first_Stop_or_start']=True
 for relative in M['files']:exact_file(relative)
 r['preserved_before']=preservation();until=min(W,now()+10_000_000_000)
 for row in r['cases'][:3]:
  role=row['role'];v=row['initial'];pid=int(v['MainPID']);fd=os.pidfd_open(pid,os.O_NONBLOCK);identities[role]={'pid':pid,'fd':fd,'InvocationID':v['InvocationID'],'ControlGroup':v['ControlGroup']};assert v['InvocationID']and v['ControlGroup'].startswith('/system.slice/')and int(Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[1])==1;samples=[]
  while now()<until:
   assert not select.select([fd],[],[],0)[0]
   exe=Path(f'/proc/{pid}/exe');target=str(exe.readlink());digest=sha(exe);samples.append({'readlink':target,'sha256':digest});assert len(samples)<=1024
   if target==ROLES[role][1]and digest==M['files'][target.lstrip('/')]['sha256']:break
   time.sleep(.002)
  else:raise AssertionError('original combined executable readiness cutoff')
  for prefix in ['Uid:','Gid:']:assert next(line for line in Path(f'/proc/{pid}/status').read_text().splitlines()if line.startswith(prefix)).split()[1:]==['0']*4
  r.setdefault('initial_executable_samples',{})[role]=samples
 baseline=json.loads((P/'source/Qt50-baseline.json').read_text());rows=baseline['actual_guest_selected_libraries_and_plugins'];assert len(rows)==50 and all(sha(row['path'])==row['sha256']for row in rows);r['Qt50_unchanged']=True
 # Original Settings case intentionally restarts network only AFTER all3 coexistence.
 network_initial=installed_component.bind(command,min(W,now()+10_000_000_000),M['files'][ROLES['network'][1].lstrip('/')]['sha256']);assert network_initial['PID']==identities['network']['pid']and network_initial['InvocationID']==identities['network']['InvocationID']
 runtime=Path('/run/pokelinux-settings-development-component');assert not runtime.exists();runtime.mkdir(mode=0o700);os.chown(runtime,11050,11050);executable=runtime/'settings-network-card-service-test';source=P/'fixture-bin/settings-network-card-service-test';assert sha(source)==S['Settings_component']['sha256'];executable.write_bytes(source.read_bytes());executable.chmod(0o555);assert sha(executable)==S['Settings_component']['sha256'];env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QSG_RHI_BACKEND='software',XDG_RUNTIME_DIR=str(runtime))
 r['Settings_component']=installed_component.run(command,R,executable,env,network_initial,W,C,M['files'][ROLES['network'][1].lstrip('/')]['sha256']);power_same()
 # The original pair consumer retains its own true wait/EOF receipts and Stops.
 body=command(['/usr/bin/python3','-B',str(P/'source/power-consumer.py'),'--output',str(R/'power-observation.json'),'--until-ns',str(W),'--close-ns',str(C)],'unchanged-power-positive-consumer',seconds=660,absolute_until=W);power=json.loads((R/'power-observation.json').read_text());assert power['passed']and power['all_commands_actual_wait_EOF']and len(power['cases'])==2
 for row in power['cases']:assert row['initial']['MainPID']==str(identities[row['role']]['pid'])and row['initial']['InvocationID']==identities[row['role']]['InvocationID']
 assert power['battery_completed_collection_unavailable']['available']is False and power['brightness_absent']['observation']is None and power['brightness_recovered']['Client']['observation']is not None and power['whole_bundle_Stop']['both_cgroups_empty'];r['power_result_sha256']=sha(R/'power-observation.json')
 for role,i in identities.items():
  v=props(role);cg=Path('/sys/fs/cgroup')/i['ControlGroup'].lstrip('/');assert[v[k]for k in ['ActiveState','SubState','MainPID','Result','ExecMainCode','ExecMainStatus']]==['inactive','dead','0','success','1','0'];assert select.select([i['fd']],[],[],0)[0]and not Path(f"/proc/{i['pid']}").exists()and not Path(ROLES[role][2]).exists()and(not(cg/'cgroup.procs').exists()or not(cg/'cgroup.procs').read_text().strip())
 r['all_three_normal_PID1_Stops_closed']=True;r['serial_recovery_cases_distinct_from_initial_coexistence']=True;r['preserved_after']=preservation();assert command(['/usr/bin/pacman','-Q'],'original353-after')==before;functional_done=True;r['failure_kind']=None
except BaseException as error:r['error']=type(error).__name__+': '+str(error);r['traceback']=traceback.format_exc()
finally:
 if identities and not functional_done:
  try:r['failure_selected_unit_Stop']=command(['/usr/bin/systemctl','stop',*[v[0]for v in ROLES.values()]],'failure-selected-owner-Stop',seconds=10,absolute_until=C,cleanup=True)
  except BaseException as error:r['failure_Stop_error']=repr(error)
 if network_initial:os.close(network_initial['fd'])
 for i in identities.values():os.close(i['fd'])
 r['owned_command_receipts']=receipts;r['all_commands_actual_wait_EOF']=bool(receipts)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in receipts);r['completed_ns']=now();r['passed']=bool(functional_done and not blocked and not r.get('failure_Stop_error')and r['all_commands_actual_wait_EOF']and now()<C);a.output.write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'passed':r['passed'],'failure_kind':r['failure_kind'],'cases':[{'role':v['role'],'passed':v['passed']}for v in r['cases']],'error':r.get('error')}),flush=True);raise SystemExit(0 if r['passed']else 1)
