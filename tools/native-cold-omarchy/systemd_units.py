"""One installed target, actual PID1 identity and bounded journal/cgroup custody.

Only the systemctl/journalctl command processes have guest direct waits/PIPE EOFs.
Service exit status is PID1 metadata plus retained pidfd and cgroup observations.
"""
import configparser,ctypes,hashlib,json,os,re,select,shlex,stat,time
from pathlib import Path
from types import SimpleNamespace
from owned import now

TARGET='pokelinux-normal-session.target'
NAMES=['pokelinux-normal-audio.service','pokelinux-normal-appearance.service','pokelinux-session-owner.service','pokelinux-omarchy-shell.service','pokelinux-omarchy-theme.service']
PROPS=['Id','Type','ActiveState','SubState','Result','MainPID','ControlPID','ExecMainPID','ExecMainCode','ExecMainStatus','InvocationID','ControlGroup','NRestarts','Restart','KillMode','LoadState','ConditionResult','Job','ExecStop','NotifyAccess']

def live_fd(fd):return fd is not None and not select.select([fd],[],[],0)[0]

def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def install_bundle(R,command):
 source=R/'deployment';manifest=source/'selected-profile.json';document=json.loads(manifest.read_text());files=document['files'];assert files==json.loads((R/'selection.json').read_text())['cold_installed_bundle']['files']
 stage=Path('/run/pokelinux-cold-selected-stage');stage.mkdir(mode=0o700)
 command(['/usr/bin/python3',str(R/'source/install_deployment.py'),'--root',str(source/'root'),'--manifest',str(manifest),'--destdir',str(stage)],'validate-and-stage-selected-profile.log',seconds=10)
 rows={}
 # The helper is build-only. This explicit Root deployment step copies only its
 # completely verified selected regular files, with no unit activation here.
 for name,row in files.items():
  relative=Path(name);assert not relative.is_absolute()and '..'not in relative.parts
  p=stage/relative;info=p.lstat();assert stat.S_ISREG(info.st_mode)and info.st_uid==info.st_gid==0 and info.st_nlink==1
  assert info.st_size==row['bytes']and sha(p)==row['sha256']and stat.S_IMODE(info.st_mode)==row['mode']
  target=Path('/')/relative
  for parent in reversed([target.parent,*target.parent.parents]):
   if parent.exists():assert parent.is_dir()and not parent.is_symlink()
   else:parent.mkdir(mode=0o755)
  assert not target.is_symlink()
  fd=os.open(target,os.O_WRONLY|os.O_CREAT|os.O_TRUNC|os.O_CLOEXEC|os.O_NOFOLLOW,0o600)
  try:
   data=p.read_bytes();assert len(data)==row['bytes']and hashlib.sha256(data).hexdigest()==row['sha256'];view=memoryview(data)
   while view:n=os.write(fd,view);assert n>0;view=view[n:]
   os.fchmod(fd,row['mode']);os.fchown(fd,0,0);os.fsync(fd)
  finally:os.close(fd)
  check=target.lstat();assert stat.S_ISREG(check.st_mode)and check.st_uid==check.st_gid==0 and check.st_size==row['bytes']and stat.S_IMODE(check.st_mode)==row['mode']and sha(target)==row['sha256']
  rows[name]={'bytes':check.st_size,'sha256':row['sha256'],'mode':row['mode'],'UID':0,'GID':0}
 unit=Path('/usr/lib/systemd/system/pokelinux-session-owner.service');data=(R/'source/pokelinux-session-owner.service').read_bytes();unit.write_bytes(data);unit.chmod(0o644);os.chown(unit,0,0)
 selected=json.loads((R/'selection.json').read_text())['selected_owner_unit'];assert len(data)==selected['bytes']and sha(unit)==selected['sha256']
 return {'selected_manifest_sha256':sha(manifest),'files':rows,'core_owner_unit_sha256':sha(unit),'guest_root_ownership_checked':True,'unit_activation_performed_by_copy':False}

class UnitReferences:
 """One owned system-bus connection; method returns are not process waits."""
 class Error(ctypes.Structure):
  _fields_=[('name',ctypes.c_char_p),('message',ctypes.c_char_p),('_need_free',ctypes.c_int)]
 def __init__(self):
  self.bus=ctypes.c_void_p();self.held=[];self.calls=[];self.closed=False;self.setup=[]
  path=Path('/usr/lib/libsystemd.so.0');self.library={'path':str(path.resolve(strict=True)),'sha256':sha(path)};self.lib=ctypes.CDLL(str(path))
  ptr=ctypes.c_void_p;ptrptr=ctypes.POINTER(ptr);char=ctypes.c_char_p;errorptr=ctypes.POINTER(self.Error)
  for name,args,result in [('sd_bus_new',[ptrptr],ctypes.c_int),('sd_bus_set_address',[ptr,char],ctypes.c_int),('sd_bus_set_bus_client',[ptr,ctypes.c_int],ctypes.c_int),('sd_bus_set_method_call_timeout',[ptr,ctypes.c_uint64],ctypes.c_int),('sd_bus_start',[ptr],ctypes.c_int),('sd_bus_close',[ptr],None),('sd_bus_unref',[ptr],ptr),('sd_bus_message_unref',[ptr],ptr),('sd_bus_error_free',[errorptr],None)]:
   function=getattr(self.lib,name);function.argtypes=args;function.restype=result
  self.lib.sd_bus_call_method.argtypes=[ptr,char,char,char,char,errorptr,ptrptr,char];self.lib.sd_bus_call_method.restype=ctypes.c_int
 def connect(self,until):
  # Configuration/start is asynchronous; authentication/Hello is completed by
  # the first bounded method call on this same existing system-bus connection.
  assert now()<until and stat.S_ISSOCK(Path('/run/dbus/system_bus_socket').stat().st_mode)
  for name,args in [('sd_bus_new',(ctypes.byref(self.bus),)),('sd_bus_set_address',(self.bus,b'unix:path=/run/dbus/system_bus_socket')),('sd_bus_set_bus_client',(self.bus,1)),('sd_bus_set_method_call_timeout',(self.bus,max(1,(until-now())//1000))),('sd_bus_start',(self.bus,))]:
   enter=now();assert enter<until;result=getattr(self.lib,name)(*args);self.setup.append({'function':name,'actual_return':result,'entered_boottime_ns':enter,'returned_boottime_ns':now(),'original_until_ns':until});assert result>=0 and now()<until,self.setup[-1]
 def call(self,method,name,until):
  assert method in('RefUnit','UnrefUnit')and name in NAMES and self.bus.value and not self.closed
  enter=now();assert enter<until;micros=max(1,(until-enter)//1000);error=self.Error();reply=ctypes.c_void_p()
  timeout_result=self.lib.sd_bus_set_method_call_timeout(self.bus,micros);assert timeout_result>=0
  try:
   result=self.lib.sd_bus_call_method(self.bus,b'org.freedesktop.systemd1',b'/org/freedesktop/systemd1',b'org.freedesktop.systemd1.Manager',method.encode(),ctypes.byref(error),ctypes.byref(reply),b's',ctypes.c_char_p(name.encode()))
   returned=now();row={'method':method,'unit':name,'actual_return':result,'error_name':error.name.decode(errors='replace')if error.name else None,'error_message':error.message.decode(errors='replace')if error.message else None,'entered_boottime_ns':enter,'returned_boottime_ns':returned,'original_until_ns':until,'selected_timeout_usec':micros,'same_owned_connection':True,'process_wait_or_PIPE_EOF_claimed':False};self.calls.append(row)
   if result>=0:
    if method=='RefUnit':self.held.append(name)
    else:self.held.remove(name)
   assert result>=0 and returned<until,row
  finally:
   if reply.value:self.lib.sd_bus_message_unref(reply)
   self.lib.sd_bus_error_free(ctypes.byref(error))
 def close(self):
  if self.bus.value:self.lib.sd_bus_close(self.bus);self.lib.sd_bus_unref(self.bus);self.bus=ctypes.c_void_p()
  self.closed=True

class Units:
 def __init__(self,scope,output,work,close,expected,selected_owner_unit):
  self.selected_owner_unit=selected_owner_unit;self.scope=scope;self.output=Path(output);self.work=work;self.close=close;self.expected=expected;self.serial=0;self.items={};self.start_sent=False;self.observations=[];self.members={};self.journals=[];self.errors=[];self.exited_before_pidfd_bind=[];self.references=None;self.passive_observer=None;self.pending_launchers={};self.theme_completion=None
 def run(self,argv,label,closing=False,limit=131072,allowed=(0,),absolute_cutoff=None):
  self.serial+=1;assert self.serial<=192;until=min(self.close if closing else self.work,now()+5_000_000_000,absolute_cutoff if absolute_cutoff is not None else self.close);name='unit-'+str(self.serial).zfill(3)+'-'+label
  code,body=self.scope.run(argv,name,until,limit=limit);self.expected[self.scope.receipts[-1]['pid']]={'role':label,'allowed_actual_exit':list(allowed)};assert code in allowed,(label,code,body[-1024:]);return body.decode()
 def show(self,name,closing=False,absolute_cutoff=None):
  assert name in NAMES or name==TARGET
  body=self.run(['/usr/bin/systemctl','show',name,'--no-pager','--property='+','.join(PROPS)],'show-'+name+'.log',closing,absolute_cutoff=absolute_cutoff)
  row=dict(v.split('=',1)for v in body.splitlines()if'='in v);assert row['Id']==name
  for key in ['MainPID','ControlPID','ExecMainPID','ExecMainCode','ExecMainStatus','NRestarts']:row[key]=int(row[key])
  row['observed_boottime_ns']=now();self.observations.append(row);assert len(self.observations)<=192
  if self.passive_observer is not None:self.passive_observer()
  return row
 def start(self):
  assert not self.start_sent
  for name in NAMES:
   before=self.show(name);assert before['MainPID']==0 and before['ActiveState']in('inactive','failed')
  self.run(['/usr/bin/systemctl','daemon-reload'],'daemon-reload.log')
  self.references=UnitReferences();self.references.connect(min(self.work,now()+5_000_000_000))
  for name in NAMES:self.reference_call('RefUnit',name,self.work)
  self.start_sent=True
  self.run(['/usr/bin/systemctl','start','--no-block',TARGET],'start-selected-target-once.log')
 def reference_call(self,method,name,deadline):
  self.serial+=1;assert self.serial<=192
  self.references.call(method,name,min(deadline,now()+5_000_000_000))
 def completed_theme(self):
  name='pokelinux-omarchy-theme.service';row=self.show(name)
  assert row['Type']=='oneshot' and row['ActiveState']=='active' and row['SubState']=='exited'
  assert row['Result']=='success' and row['MainPID']==row['ControlPID']==0
  assert row['ExecMainPID']>0 and row['ExecMainCode']==1 and row['ExecMainStatus']==0
  assert row['NRestarts']==0 and len(row['InvocationID'])==32 and not self.collect_members({'unit':name})
  self.theme_completion=row
  return {'PID1_completed_invocation':row,'cgroup_empty':True,'guest_waitpid_or_PIPE_EOF_claimed':False,'graphical_readiness_inferred':False}
 def bind(self,name,uid,exe,argv,launcher=None):
  row=self.show(name)
  # A selected same-PID exec launcher is startup, never accepted shell readiness.
  if launcher is not None and row['MainPID'] and name not in self.items:
   pid=row['MainPID'];pending=self.pending_launchers.get(name)
   if pending is None:
    pending={'pid':pid,'pidfd':os.pidfd_open(pid,os.O_NONBLOCK),'InvocationID':row['InvocationID'],'observed_wrapper':False}
    self.pending_launchers[name]=pending
   assert pending['pid']==pid and pending['InvocationID']==row['InvocationID'] and live_fd(pending['pidfd'])
   process=Path('/proc')/str(pid);before=os.readlink(process/'exe')
   actual=[v.decode()for v in(process/'cmdline').read_bytes().rstrip(b'\0').split(b'\0')]
   after=os.readlink(process/'exe')
   assert before in (launcher['exe'],exe) and after in (launcher['exe'],exe)
   if before!=after:return None  # Same retained process crossing exec; no admission.
   if before==launcher['exe']:
    image_hash=sha(process/'exe');hashed_image=os.readlink(process/'exe')
    assert hashed_image in (launcher['exe'],exe)
    if hashed_image!=launcher['exe']:return None  # Exec crossed the hash read.
    assert actual==launcher['argv'] and image_hash==launcher['sha256']
    status=(process/'status').read_text()
    assert next(v for v in status.splitlines()if v.startswith('PPid:')).split()[1]=='1'
    for key in ('Uid:','Gid:'):assert next(v for v in status.splitlines()if v.startswith(key)).split()[1:]==[str(uid)]*4
    assert row['NRestarts']==0 and row['ControlGroup']=='/system.slice/'+name and live_fd(pending['pidfd'])
    pending['observed_wrapper']=True;self.collect_members({'unit':name});return None
  if name in self.items:
   item=self.items[name];assert row['MainPID']==item['child'].pid and row['InvocationID']==item['startup']['InvocationID']and row['NRestarts']==0 and self.live(item),('selected unit identity changed or exited',row)
   assert row['ActiveState']in('active','activating'),('selected unit no longer starting/running',row)
   return item if row['ActiveState']=='active'and row['SubState']=='running'else None
  if row['MainPID']==0:
   assert not row['ExecMainPID']and row['ActiveState']!='failed'and row['LoadState']=='loaded',('selected unit exited or failed',row)
   # A queued start has not evaluated conditions or created an invocation yet.
   assert row['ConditionResult']=='yes'or(bool(row['Job'])and not row['InvocationID']),('selected unit neither queued nor condition-qualified',row)
   return None
  owner_activating=name=='pokelinux-session-owner.service'and row['Type']=='notify'and row['ActiveState']=='activating'and row['SubState']=='start'
  if not(row['ActiveState']=='active'and row['SubState']=='running')and not owner_activating:return None
  pid=row['MainPID']
  try:fd=os.pidfd_open(pid,os.O_NONBLOCK)
  except ProcessLookupError as error:
   after=self.show(name);observed_exit=after['ExecMainPID']==pid and after['MainPID']==0 and after['ExecMainCode']!=0
   self.exited_before_pidfd_bind.append({'unit':name,'observed_MainPID':pid,'before_pidfd_open':row,'actual_pidfd_open_errno':error.errno,'after_ESRCH_PID1_state':after,'PID1_exit_observed':observed_exit,'retained_pidfd_present':False,'kernel_UID_argv_identity_qualified':False,'retry_or_replacement_attempted':False})
   raise RuntimeError('INSTALLED_UNIT_EXITED_BEFORE_PIDFD_BIND'if observed_exit else'INSTALLED_UNIT_PIDFD_ESRCH_WITHOUT_TERMINAL_PID1_STATUS')from error
  item={'unit':name,'child':SimpleNamespace(pid=pid),'pidfd':fd,'name':name,'startup':row,'argv':argv,'observed_closure':None};self.items[name]=item
  expected_type='exec';expected_kill='control-group'
  if name=='pokelinux-session-owner.service':
   installed_unit=Path('/usr/lib/systemd/system')/name;info=installed_unit.lstat()
   assert stat.S_ISREG(info.st_mode)and info.st_size==self.selected_owner_unit['bytes']and sha(installed_unit)==self.selected_owner_unit['sha256']
   unit_config=configparser.ConfigParser(interpolation=None,strict=True);unit_config.read_string(installed_unit.read_text());selected=unit_config['Service'];expected_type=selected['Type'];assert expected_type in('exec','notify')
   expected_kill=selected['KillMode'];assert expected_kill in('control-group','mixed')
   if expected_type=='notify':assert selected['NotifyAccess']=='main'and row['NotifyAccess']=='main'
   stop=selected.get('ExecStop','').strip();actual_stop=row.get('ExecStop','')
   if stop:
    tokens=shlex.split(stop);assert tokens and re.search(r'path='+re.escape(tokens[0])+r'\s*;',actual_stop)and re.search(r'argv\[\]='+re.escape(stop)+r'\s*;',actual_stop),('selected ExecStop differs',stop,actual_stop)
   else:assert not actual_stop,('unexpected actual ExecStop',actual_stop)
   item['selected_unit_contract']={'Type':expected_type,'KillMode':expected_kill,'ExecStop_source':stop,'ExecStop_actual_raw':actual_stop}
  assert row['Type']==expected_type and row['Restart']=='no'and row['KillMode']==expected_kill and row['NRestarts']==0
  assert len(row['InvocationID'])==32 and all(c in'0123456789abcdef'for c in row['InvocationID'])
  assert row['ControlGroup']=='/system.slice/'+name and live_fd(fd)
  p=Path('/proc')/str(pid);text=(p/'status').read_text();assert next(v for v in text.splitlines()if v.startswith('PPid:')).split()[1]=='1'
  assert next(v for v in text.splitlines()if v.startswith('Uid:')).split()[1:]==[str(uid)]*4
  assert next(v for v in text.splitlines()if v.startswith('Gid:')).split()[1:]==[str(uid)]*4
  actual=[v.decode()for v in(p/'cmdline').read_bytes().rstrip(b'\0').split(b'\0')];assert actual==argv,(name,actual,argv)
  assert os.readlink(p/'exe')==exe
  item['identity']={'PID':pid,'UID':uid,'GID':uid,'PPid':1,'exe':exe,'exe_sha256':sha(p/'exe'),'argv':actual,'proc_starttime':(p/'stat').read_text().split(') ',1)[1].split()[19],'InvocationID':row['InvocationID'],'ControlGroup':row['ControlGroup'],'NRestarts':0,'observed_while_activating':owner_activating,'startup_success_inferred_from_identity':False,'guest_direct_parent_wait_claimed':False}
  again=self.show(name);assert again['MainPID']==pid and again['InvocationID']==row['InvocationID']and again['NRestarts']==0 and live_fd(fd),('unit changed during identity capture',again)
  assert pid in self.collect_members(item),'actual MainPID absent from exact unit cgroup'
  return item if again['ActiveState']=='active'and again['SubState']=='running'else None
 def authenticated_pending_owner(self):
  item=self.items.get('pokelinux-session-owner.service')
  return item if item is not None and item.get('identity')is not None else None
 def matching_exit(self,item):
  # Collected/reset manager defaults cannot become an exit0 observation.
  matches=[row for row in self.observations if row['Id']==item['unit']and row['InvocationID']==item['startup']['InvocationID']and row['ExecMainPID']==item['child'].pid and row['ExecMainCode']!=0]
  return matches[-1]if matches else None
 def collect_members(self,item):
  group='/system.slice/'+item['unit'];root=Path('/sys/fs/cgroup')/group.lstrip('/');assert root.parent==Path('/sys/fs/cgroup/system.slice')
  if not root.exists():return []
  files=[root/'cgroup.procs'];files.extend(p/'cgroup.procs'for p in root.rglob('*')if p.is_dir());assert len(files)<=32
  pids=set()
  for file in files:
   body=file.read_bytes();assert len(body)<=16384;pids.update(int(v)for v in body.split())
  assert len(pids)<=128
  for pid in pids:
   if pid in self.members:continue
   try:fd=os.pidfd_open(pid,os.O_NONBLOCK)
   except ProcessLookupError:continue
   self.members[pid]={'fd':fd,'PID':pid,'unit':item['unit'],'observed_boottime_ns':now()}
  return sorted(pids)
 def live(self,item):
  self.collect_members(item);return live_fd(item['pidfd'])
 def finish(self,item,until,permit_manager_stop=True):
  # Native Stop may finish without a manager stop. Wait only under original close.
  while live_fd(item['pidfd']):
   assert now()<until;self.scope.pump();self.scope.check();time.sleep(.005)
  row=self.show(item['unit'],closing=True,absolute_cutoff=until);members=self.collect_members(item)
  manager_fallback_needed=bool(row['MainPID']or row['ControlPID']or members)
  manager_fallback=manager_fallback_needed and permit_manager_stop
  if manager_fallback:
   self.run(['/usr/bin/systemctl','stop',item['unit']],'stop-'+item['unit']+'.log',True,absolute_cutoff=until);row=self.show(item['unit'],True,absolute_cutoff=until);members=self.collect_members(item)
  retired=row['MainPID']==row['ControlPID']==0 and not members and not live_fd(item['pidfd'])
  while not retired and not permit_manager_stop:
   assert now()<until;self.scope.pump();self.scope.check();time.sleep(.025)
   row=self.show(item['unit'],closing=True,absolute_cutoff=until);members=self.collect_members(item)
   retired=row['MainPID']==row['ControlPID']==0 and not members and not live_fd(item['pidfd'])
  exit_observation=self.matching_exit(item);main_exit0=exit_observation is not None and exit_observation['ExecMainCode']==1 and exit_observation['ExecMainStatus']==0
  closure={'startup':item['identity'],'PID1_exit':row,'matching_invocation_exit_observation':exit_observation,'retained_pidfd_dead':not live_fd(item['pidfd']),'cgroup_members':members,'process_retirement_proven':retired,'main_process_exit0':main_exit0,'overall_unit_Result':row['Result'],'overall_unit_operational_success':row['Result']=='success','ExecStop_raw':row.get('ExecStop'),'service_guest_waitpid_or_PIPE_EOF_claimed':False,'manager_stop_fallback_used':manager_fallback,'manager_stop_fallback_permitted':permit_manager_stop,'manager_stop_fallback_needed':manager_fallback_needed,'native_cleanup_inferred':False};item['observed_closure']=closure
  assert retired and main_exit0 and row['Result']=='success'and now()<until,closure
  # Main exit0 proves this process result only. Overall unit health is separately
  # mandatory for the cold success aggregate; ExecStop failures stay failures.
  return 0
 def diagnostics(self):
  for name in NAMES:
   try:
    row=self.show(name,True);inv=self.items.get(name,{}).get('startup',{}).get('InvocationID')or row.get('InvocationID')
    args=['/usr/bin/journalctl','--no-pager','--output=short-monotonic','--lines=256','--unit='+name]
    if inv:args.append('_SYSTEMD_INVOCATION_ID='+inv)
    body=self.run(args,'journal-'+name+'.log',True,limit=1048576);self.journals.append({'unit':name,'InvocationID':inv or None,'bytes':len(body.encode()),'sha256':hashlib.sha256(body.encode()).hexdigest(),'journal_snapshot_not_service_PIPE_EOF':True,'status':row})
   except BaseException as e:self.errors.append('diagnostics '+name+': '+repr(e))
 def close_all(self):
  self.diagnostics()
  if self.start_sent:
   # Capture matching-invocation exit facts before the explicit target stop.
   # Manager auto-propagation/collection can still erase fields: preserve unknown.
   for name,item in self.items.items():
    try:
     self.run(['/usr/bin/systemctl','stop',name],'stop-bound-'+name+'.log',True)
     self.show(name,True)
    except BaseException as e:self.errors.append('bound-unit-stop '+name+': '+repr(e))
   try:self.run(['/usr/bin/systemctl','stop',TARGET,*NAMES],'stop-selected-target-and-exact-units.log',True)
   except BaseException as e:self.errors.append('target-stop: '+repr(e))
  states=[]
  for name in NAMES:
   try:
    row=self.show(name,True);item=self.items.get(name);members=self.collect_members(item if item else {'unit':name})
    assert row['MainPID']==row['ControlPID']==0 and not members
    if item:assert not live_fd(item['pidfd'])and row['ExecMainPID']in(0,item['child'].pid)
    states.append({'unit':name,'startup_identity':item.get('identity')if item else None,'native_Stop_finish_observation':item.get('observed_closure')if item else None,'matching_invocation_exit_observation':self.matching_exit(item)if item else (self.theme_completion if name=='pokelinux-omarchy-theme.service' else None),'selected_unit_contract':item.get('selected_unit_contract')if item else None,'final_PID1_state':row,'retained_pidfd_dead':not live_fd(item['pidfd'])if item else None,'remaining_cgroup_members':members,'service_guest_waitpid_or_PIPE_EOF_claimed':False})
   except BaseException as e:self.errors.append('unit-close '+name+': '+repr(e))
  # Read all matching invocation/status/cgroup facts before releasing ANY ref.
  reference_result=None
  if self.references is not None:
   try:
    for name in list(reversed(self.references.held)):
     try:self.reference_call('UnrefUnit',name,self.close)
     except BaseException as error:self.errors.append('unit-unref '+name+': '+repr(error))
   finally:
    unresolved=list(self.references.held);self.references.close()
    reference_result={'library':self.references.library,'setup':self.references.setup,'calls':self.references.calls,'unresolved_explicit_UnrefUnit':unresolved,'connection_closed_without_flush':self.references.closed,'references_dropped_after_final_status_and_cgroup_capture':True,'service_wait_or_PIPE_EOF_claimed':False}
    if unresolved:self.errors.append('unit refs released only by connection close: '+repr(unresolved))
  member_rows=[]
  for pid,row in self.members.items():
   dead=not live_fd(row['fd']);member_rows.append({k:v for k,v in row.items()if k!='fd'}|{'retained_pidfd_dead':dead,'proc_absent':not Path('/proc',str(pid)).exists()});os.close(row['fd'])
   if not dead:self.errors.append('live cgroup member '+str(pid))
  for pending in self.pending_launchers.values():
   if live_fd(pending['pidfd']):self.errors.append('live selected startup launcher '+str(pending['pid']))
   os.close(pending['pidfd'])
  for item in self.items.values():os.close(item['pidfd'])
  return {'unit_metadata_references':reference_result,'start_target_once':self.start_sent,'observations':self.observations,'unit_closures':states,'all_unit_operational_results_success':len(states)==len(NAMES)and all(v['final_PID1_state']['Result']=='success'and v['matching_invocation_exit_observation']is not None and v['matching_invocation_exit_observation']['Result']=='success'for v in states),'exited_before_pidfd_bind':self.exited_before_pidfd_bind,'member_pidfds':member_rows,'journals':self.journals,'cleanup_errors':self.errors,'systemctl_and_journalctl_direct_waits_in_original_Scope':True,'service_waits_not_relabelled_as_guest_waits':True,'readiness_not_inferred_from_ActiveState':True}
