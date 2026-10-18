"""One current393 installed seven-service join: presence RED or serial cold cases."""
from pathlib import Path
import argparse,ctypes,hashlib,json,os,select,stat,sys,time,traceback
sys.dont_write_bytecode=True
from owned import Scope,now
from closure import close_owned
import installed_component
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--mode',choices=['check-only','run'],required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--until-ns',type=int,required=True)
p.add_argument('--close-ns',type=int,required=True)
a=p.parse_args()
R=Path('/results');P=Path('/run/pokelinux-development-inputs')
S=json.loads((R/'selection.json').read_text());W=a.until_ns;C=a.close_ns
assert a.output.parent==R and now()<W<C
ROLES={
 'battery':('pokelinux-battery.service','/usr/libexec/pokelinux/battery-service','/run/pokelinux-battery/snapshot.sock',0),
 'brightness':('pokelinux-brightness.service','/usr/libexec/pokelinux/brightness-service','/run/pokelinux-brightness/actor.sock',0),
 'network':('pokelinux-network-status.service','/usr/libexec/pokelinux/pokelinux-network-status','/run/pokelinux-network/snapshot.sock',0),
 'audio':('pokelinux-audio.service','/usr/libexec/pokelinux/pokelinux-audio','/run/pokelinux-audio/audio.sock',11040),
 'bus':('pokelinux-audio-bus.service','/usr/bin/dbus-daemon',None,11040),
 'pipewire':('pokelinux-audio-pipewire.service','/usr/bin/pipewire',None,11040),
 'wireplumber':('pokelinux-audio-wireplumber.service','/usr/bin/wireplumber',None,11040)}
ASSETS={
 'Settings':{'usr/libexec/pokelinux/pokelinux-settings':0o755,'usr/share/applications/org.omarchy.Settings.desktop':0o644},
 'Sounds':{'usr/libexec/pokelinux/pokelinux-sounds':0o755,'usr/share/applications/org.omarchy.Sounds.desktop':0o644}}
APPEARANCE='usr/share/pokelinux/appearance/PhoneAppearance.qml'
DEPLOYED={
 'etc/modules-load.d/pokelinux-backlight-vm.conf','etc/pokelinux/battery.conf','etc/pokelinux/brightness.conf',
 'usr/lib/systemd/system/pokelinux-battery.service','usr/lib/systemd/system/pokelinux-brightness.service','usr/lib/systemd/system/pokelinux-network-status.service',
 'usr/libexec/pokelinux/battery-service','usr/libexec/pokelinux/brightness-service','usr/libexec/pokelinux/pokelinux-network-status',
 'usr/libexec/pokelinux/pokelinux-settings','usr/libexec/pokelinux/pokelinux-sounds',
 'usr/share/applications/org.omarchy.Settings.desktop','usr/share/applications/org.omarchy.Sounds.desktop',APPEARANCE}
FIELDS='LoadState,UnitFileState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,Result,ExecMainCode,ExecMainStatus,NRestarts'
POWER=['battery','brightness'];AUDIO=['audio','bus','pipewire','wireplumber'];UNITS={v[0]:k for k,v in ROLES.items()}
owners=[];receipts=[];serial=0;blocked=False;identities={};retained=[];network_initial=None;functional_done=False;M=None;audio_case_admitted=False;remaining_Stop_attempted=False;audio_stop_until=None
r={'schema':1,'passed':False,'mode':a.mode,'scope':'Current393 seven-service installed join; no normal Main/window/Source/phone','original_until_ns':W,'original_close_ns':C,'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip(),'cases':[],'failure_kind':'setup','service_starts_before_joined_presence':False}
def sha(path):
 with Path(path).open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def pin(path):return {'bytes':Path(path).stat().st_size,'sha256':sha(path)}
def dead(fd):return bool(select.select([fd],[],[],0)[0])
def handles_closed(scope):
 return all(v['wait']is not None and v['eof']and v['log'].closed and v['child'].stdout.closed and(v['stderr_file']is None or v['stderr_file'].closed)and v['pidfd']is None for v in scope.children)
def command(argv,label,seconds=10,limit=262144,env=None,identity=None,absolute_until=None,cleanup=False,expected=(0,),reap_adopted=False):
 global serial,blocked
 assert not blocked and all(handles_closed(v)for v in owners),'incomplete prior command custody fences admission'
 serial+=1;scope=Scope(R);owners.append(scope);until=min(C if cleanup else W,now()+int(seconds*1e9),absolute_until if absolute_until is not None else W);name=f'joined-{serial:03d}-{label}.log'
 try:
  assert now()<until,'original command deadline before child admission'
  child=scope.start(argv,name,limit,env=env,identity=identity,cutoff=until);code=scope.finish(child,until);body=(R/name).read_text();assert code in expected,(label,code,body[-1000:]);return body
 finally:
  closure=close_owned(scope,C,reap_adopted=reap_adopted);receipts.extend(scope.receipts)
  r.setdefault('command_closure',[]).append({'label':label,'closure':closure,'handles_closed':handles_closed(scope)})
  if not closure['closed']or not handles_closed(scope):blocked=True
  assert closure['closed']and handles_closed(scope),(label,closure)
def props(role,cleanup=False,until=None):
 body=command(['/usr/bin/systemctl','show',ROLES[role][0],'--property='+FIELDS],role+'-properties',cleanup=cleanup,absolute_until=until,expected=(0,1));v=dict(line.split('=',1)for line in body.splitlines());assert 'LoadState'in v;return v
def active(v):return v['LoadState']=='loaded'and v['UnitFileState']=='enabled'and v['ActiveState']=='active'and v['SubState']=='running'and int(v['MainPID'])>1

def protected(relative,mode):
 f=Path('/')/relative;m=f.lstat();assert stat.S_ISREG(m.st_mode)and m.st_uid==m.st_gid==0 and stat.S_IMODE(m.st_mode)==mode and m.st_size>0
 if mode&0o111:
  with f.open('rb')as stream:head=stream.read(20)
  assert head[:6]==b'\x7fELF\x02\x01'and int.from_bytes(head[18:20],'little')==183
 return {'bytes':m.st_size,'mode':stat.S_IMODE(m.st_mode),'uid':m.st_uid,'gid':m.st_gid}
def exact_file(relative):
 row=M['files'][relative];f=Path('/')/relative;protected(relative,row['mode']);assert pin(f)=={k:row[k]for k in ['bytes','sha256']}
def presence():
 for role in ROLES:
  v=props(role);row={'role':role,'passed':False,'initial':v}
  try:assert active(v),'required installed enabled already-active PID1 service';row['passed']=True
  except AssertionError as error:row.update(failure_kind='installed-boundary-assertion',error=str(error))
  r['cases'].append(row)
 for app,files in ASSETS.items():
  row={'role':app+'-installed-files','passed':False,'files':{}}
  try:
   for relative,mode in {**files,APPEARANCE:0o644}.items():row['files'][relative]=protected(relative,mode)
   row['passed']=True
  except(FileNotFoundError,AssertionError)as error:row.update(failure_kind='installed-boundary-assertion',error=repr(error))
  r['cases'].append(row)
 assert all(v['passed']for v in r['cases']),'all seven enabled already-active services and protected Settings/Sounds files required'
 r['joined_presence_passed_before_any_start_or_Stop']=True

def bind(role,until,expected_previous=None):
 while now()<until:
  v=props(role,until=until)
  if active(v):break
  time.sleep(.01)
 else:raise AssertionError('original '+role+' PID1 admission cutoff')
 assert v['InvocationID']and v['ControlGroup'].startswith('/system.slice/')and '..'not in Path(v['ControlGroup']).parts and v['NRestarts']=='0'
 pid=int(v['MainPID']);fd=os.pidfd_open(pid,os.O_NONBLOCK);item={'role':role,'pid':pid,'fd':fd,'InvocationID':v['InvocationID'],'ControlGroup':v['ControlGroup']};retained.append(item)
 assert not dead(fd);uid=str(ROLES[role][3]);proc=Path('/proc')/str(pid)
 fields=(proc/'stat').read_text().rsplit(') ',1)[1].split();assert int(fields[1])==1;item['starttime']=fields[19]
 for prefix in ['Uid:','Gid:']:assert next(line for line in(proc/'status').read_text().splitlines()if line.startswith(prefix)).split()[1:]==[uid]*4
 assert(proc/'cgroup').read_text().strip()=='0::'+item['ControlGroup']
 samples=[]
 while now()<until:
  assert not dead(fd);exe=proc/'exe';target=str(exe.readlink());digest=sha(exe);samples.append({'readlink':target,'sha256':digest});assert len(samples)<=1024
  if target==ROLES[role][1]and digest==S['cohort_executables'][role]:break
  time.sleep(.002)
 else:raise AssertionError('original '+role+' executable readiness cutoff')
 if expected_previous:assert dead(expected_previous['fd'])and item['InvocationID']!=expected_previous['InvocationID'],'explicit restore must be a fresh invocation'
 assert now()<until,'original identity admission cutoff after executable hash'
 identities[role]=item;r.setdefault('identity_admissions',[]).append({**{k:v for k,v in item.items()if k!='fd'},'initial':v,'executable_samples':samples,'original_until_ns':until});return item

def unchanged(roles,label):
 facts={}
 for role in roles:
  item=identities[role];v=props(role);assert active(v)and v['MainPID']==str(item['pid'])and v['InvocationID']==item['InvocationID']and not dead(item['fd'])and v['NRestarts']=='0'
  facts[role]={'MainPID':v['MainPID'],'InvocationID':v['InvocationID'],'retained_Main_pidfd_alive':True}
 r.setdefault('unrelated_identity_checks',[]).append({'label':label,'observed_ns':now(),'roles':facts});return facts

def restore(roles,label):
 previous={role:identities[role]for role in roles};until=min(W,now()+30_000_000_000)
 for role,item in previous.items():
  v=props(role,until=until);assert v['ActiveState']=='inactive'and v['MainPID']=='0'and v['Result']=='success'and dead(item['fd'])
 command(['/usr/bin/systemctl','start',*[ROLES[role][0]for role in roles]],label+'-explicit-readonly-service-start',seconds=30,absolute_until=until)
 for role in roles:bind(role,until,previous[role])
 r.setdefault('explicit_readonly_restores',[]).append({'roles':roles,'original_until_ns':until,'fresh_invocations':{role:identities[role]['InvocationID']for role in roles},'effect_replay_requested':False})

def verify_stopped(role,until):
 item=identities[role];v=props(role,cleanup=True,until=until);allowed=[('1','0')]if role not in ['bus','pipewire','wireplumber']else[('1','0'),('2','15')]
 assert(v['ActiveState'],v['SubState'],v['MainPID'],v['Result'])==('inactive','dead','0','success')and(v['ExecMainCode'],v['ExecMainStatus'])in allowed
 cg=Path('/sys/fs/cgroup')/item['ControlGroup'].lstrip('/');assert not(cg/'cgroup.procs').exists()or not(cg/'cgroup.procs').read_text().split()
 assert dead(item['fd'])and not Path(f"/proc/{item['pid']}").exists()
 if ROLES[role][2]:assert not Path(ROLES[role][2]).exists()
 assert now()<until;return v

def final_stop():
 global remaining_Stop_attempted
 until=min(W,now()+30_000_000_000);descendants=[]
 try:
  for role in ['battery','brightness','network']:
   item=identities[role];cg=Path('/sys/fs/cgroup')/item['ControlGroup'].lstrip('/')
   if(cg/'cgroup.procs').exists():
    for raw in(cg/'cgroup.procs').read_text().split():
     pid=int(raw)
     if pid!=item['pid']:
      try:descendants.append((pid,os.pidfd_open(pid,os.O_NONBLOCK)))
      except ProcessLookupError:pass
  remaining_Stop_attempted=True
  command(['/usr/bin/systemctl','stop',*[ROLES[v][0]for v in ['battery','brightness','network']]],'final-remaining-three-PID1-Stop',seconds=30,absolute_until=until)
  r['remaining_three_original_Stop_until_ns']=until
  r['final_all_seven_PID1']=dict((role,verify_stopped(role,audio_stop_until if role in AUDIO else until))for role in ROLES)
  assert all(dead(fd)and not Path(f'/proc/{pid}').exists()for pid,fd in descendants)
  r['all_seven_normal_PID1_Stops_closed']=True;r['observed_final_descendants_dead']=len(descendants);r['PID1_Main_parent_wait_claimed']=False
 finally:
  for _,fd in descendants:os.close(fd)
try:
 assert os.getresuid()==os.getresgid()==(0,0,0)and os.uname().machine=='aarch64'and os.uname().release=='7.1.6-1-aarch64-ARCH'and Path('/proc/1/comm').read_text().strip()=='systemd'
 assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
 before=command(['/usr/bin/pacman','-Q'],'original393');assert before==(P/'source/expected-installed393.txt').read_text()and len(before.splitlines())==393
 r['original393_exact']=True;r['independent_only_loopback']=installed_component.onlylo()
 r['failure_kind']='installed-boundary-assertion';presence()
 if a.mode=='check-only':functional_done=True;r['failure_kind']=None
 else:
  r['failure_kind']='setup'
  assert S['providers_complete']and set(S['cohort_executables'])==set(ROLES)
  M=json.loads((P/'full-manifest.json').read_text());assert M['schema']==1 and set(M['files'])==DEPLOYED
  for relative in M['files']:exact_file(relative)
  for role in ROLES:assert len(S['cohort_executables'][role])==64
  for relative,key in [('fixture-bin/settings-network-card-service-test','Settings_component'),('source/power-consumer.py','power_consumer')]:assert pin(P/relative)==S[key]
  audio_entry=R/'source/component-entry.py';audio_assert=Path('/usr/libexec/pokelinux-fixture/preview-qml-assert')
  assert pin(audio_entry)==S['audio_component_source']and pin(audio_assert)==S['audio_assertion']
  baseline=json.loads((P/'source/Qt50-baseline.json').read_text());rows=baseline['actual_guest_selected_libraries_and_plugins'];assert len(rows)==50 and all(sha(row['path'])==row['sha256']for row in rows);r['Qt50_unchanged']=True
  until=min(W,now()+30_000_000_000)
  for role in ROLES:bind(role,until)
  r['all_seven_initially_PID1_owned']=True;r['failure_kind']='combined-runtime-assertion'
  # Reuse the existing network native consumer. Other six owners retain identity.
  unrelated=POWER+AUDIO;unchanged(unrelated,'before-network-local-Stops')
  network_initial=installed_component.bind(command,min(W,now()+10_000_000_000),S['cohort_executables']['network'])
  assert network_initial['PID']==identities['network']['pid']and network_initial['InvocationID']==identities['network']['InvocationID']
  runtime=Path('/run/pokelinux-settings-development-component');assert not runtime.exists();runtime.mkdir(mode=0o700);os.chown(runtime,11050,11050)
  executable=runtime/'settings-network-card-service-test';executable.write_bytes((P/'fixture-bin/settings-network-card-service-test').read_bytes());executable.chmod(0o555)
  env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QSG_RHI_BACKEND='software',XDG_RUNTIME_DIR=str(runtime))
  r['Settings_component']=installed_component.run(command,R,executable,env,network_initial,W,C,S['cohort_executables']['network']);os.close(network_initial['fd']);network_initial=None
  unchanged(unrelated,'after-network-local-Stops');restore(['network'],'after-network-case');unchanged(list(ROLES),'after-explicit-network-restore')
  # Inventory-only393 successor of the existing power consumer; all case predicates retained.
  unrelated=['network']+AUDIO;unchanged(unrelated,'before-power-local-Stops')
  command(['/usr/bin/python3','-B',str(P/'source/power-consumer.py'),'--output',str(R/'power-observation.json'),'--until-ns',str(W),'--close-ns',str(C)],'existing-power-positive-consumer',seconds=420,absolute_until=W)
  power=json.loads((R/'power-observation.json').read_text());assert power['passed']and power['all_commands_actual_wait_EOF']and len(power['cases'])==2
  for row in power['cases']:assert row['initial']['MainPID']==str(identities[row['role']]['pid'])and row['initial']['InvocationID']==identities[row['role']]['InvocationID']
  assert power['original393_exact']and power['battery_completed_collection_unavailable']['available']is False and power['brightness_absent']['observation']is None and power['brightness_recovered']['Client']['observation']is not None and power['whole_bundle_Stop']['both_cgroups_empty']
  r['power_result_sha256']=sha(R/'power-observation.json');unchanged(unrelated,'after-power-local-Stops');restore(POWER,'after-power-case');unchanged(list(ROLES),'after-explicit-power-restore')
  # The installed real-input/PCM controller owns its original ARM12/6 clocks and four-unit Stop.
  unrelated=POWER+['network'];unchanged(unrelated,'before-audio-local-Stop')
  env=dict(os.environ,PHASE_WORK_UNTIL_NS=str(W),PHASE_CLOSE_UNTIL_NS=str(C))
  audio_case_admitted=True
  command(['/usr/bin/python3','-B',str(audio_entry)],'existing-installed-real-QML-PCM',seconds=150,limit=1048576,env=env,absolute_until=W,reap_adopted=True)
  audio=json.loads((R/'runtime/result.json').read_text());assert audio['passed']and audio['cleanup_qualified']and audio['ready_for_compiled_assertion']
  audio_stop_until=audio['PID1_whole_stack_Stop']['original_until_ns'];assert type(audio_stop_until)is int
  audio_stop_until=min(W,audio_stop_until);r['audio_original_Stop_until_ns']=audio_stop_until
  assert r['command_closure'][-1]['closure']['wait_all_ECHILD']and all(handles_closed(v)for v in owners)
  r['audio_parent_custody_closed_before_native_assertion']=True
  command([str(audio_assert),str(R/'runtime/result.json')],'unchanged-native-audio-positive-after-cleanup',seconds=10,absolute_until=W)
  r['audio_native_assertion_exit']=0;r['audio_result_sha256']=sha(R/'runtime/result.json');unchanged(unrelated,'after-audio-local-Stop')
  for role in AUDIO:verify_stopped(role,audio_stop_until)
  final_stop()
  for relative in M['files']:exact_file(relative)
  assert command(['/usr/bin/pacman','-Q'],'original393-after')==before;r['original393_and_installed_assets_unchanged']=True
  functional_done=True;r['failure_kind']=None
except BaseException as error:r['error']=type(error).__name__+': '+str(error);r['traceback']=traceback.format_exc()
finally:
 if identities and not functional_done:
  # The audio controller and final Stop each own one original attempt. A
  # failed attempt is retained as failure; cleanup must not submit it again.
  cleanup_units=[]if remaining_Stop_attempted else[ROLES[v][0]for v in ['battery','brightness','network']]
  if not audio_case_admitted:cleanup_units.insert(0,'pokelinux-audio-backend.target')
  r['failure_cleanup_units']=cleanup_units
  r['failure_audio_Stop_not_retried_after_case_admission']=audio_case_admitted
  r['failure_remaining_Stop_not_retried_after_attempt']=remaining_Stop_attempted
  if cleanup_units:
   try:command(['/usr/bin/systemctl','stop',*cleanup_units],'failure-selected-cohort-Stop',seconds=30,absolute_until=C,cleanup=True)
   except BaseException as error:r['failure_Stop_error']=repr(error)
 if network_initial:os.close(network_initial['fd'])
 for item in retained:os.close(item['fd'])
 r['owned_command_receipts']=receipts;r['all_commands_actual_wait_EOF']=bool(receipts)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in receipts)
 r['all_command_handles_closed']=all(handles_closed(v)for v in owners);r['completed_ns']=now();r['passed']=bool(functional_done and not blocked and not r.get('failure_Stop_error')and r['all_commands_actual_wait_EOF']and r['all_command_handles_closed']and now()<C)
 a.output.write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'passed':r['passed'],'mode':a.mode,'failure_kind':r['failure_kind'],'cases':[{'role':v['role'],'passed':v['passed']}for v in r['cases']],'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['passed']else 1)
