#!/usr/bin/env python3
"""Owned native PAM component fixture; never export credential material."""
import ctypes,errno,hashlib,json,os,platform,pwd,re,resource,secrets,select,shutil,signal,stat,struct,subprocess,time
from pathlib import Path
R=Path('/results');P=Path('/workspace');I=Path('/opt/pokelinux-native-pam')
result={'schema':1,'passed':False,'scope':'native fixed PAM worker actual ARM private-pipe component only',
 'cases':[],'direct_waits':[],'adopted_waits':[],'setup_prerequisites_complete':False,
 'credentials_exported':False,'raw_worker_stdio_exported':False,'Original_Quickshell_executed':False,
 'PAM_helper_wait_syscalls_independently_traced':False,'generic_descendant_cgroup_or_cancel_qualified':False,
 'actor_authentication_unlock_or_restoration_qualified':False,'phone_accessed':False}
owned={};handles=[];outer=time.monotonic()+80
libc=ctypes.CDLL(None,use_errno=True)
class FixtureError(Exception):pass
def require(value,code):
 if not value:raise FixtureError(code)
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def budget(seconds):
 value=min(seconds,outer-time.monotonic());require(value>0,'fixture_deadline');return value
def metadata(path):
 p=Path(path);s=p.lstat();return {'path':str(p),'uid':s.st_uid,'gid':s.st_gid,
  'mode':oct(stat.S_IMODE(s.st_mode)),'directory':stat.S_ISDIR(s.st_mode),'symlink':stat.S_ISLNK(s.st_mode)}
def trusted(path):
 row=metadata(path);result.setdefault('trusted_ancestry',[]).append(row)
 require(row['uid']==0 and row['directory'] and not row['symlink'] and not(Path(path).lstat().st_mode&0o022),'untrusted_ancestor')
def living(fd):return not select.select([fd],[],[],0)[0]
def process(pid):
 base=Path('/proc')/str(pid);text=(base/'stat').read_text();fields=text[text.rfind(')')+2:].split()
 values={line.split(':',1)[0]:line.split(':',1)[1].strip() for line in (base/'status').read_text().splitlines() if ':' in line}
 return {'pid':pid,'parent':int(fields[1]),'state':fields[0],'start_ticks':int(fields[19]),
  'uids':list(map(int,values['Uid'].split())),'gids':list(map(int,values['Gid'].split())),
  'groups':list(map(int,values.get('Groups','').split())),
  'capabilities':{key:values[key] for key in ['CapInh','CapPrm','CapEff','CapBnd','CapAmb']},
  'NoNewPrivs':int(values['NoNewPrivs']),'proc_stat_owner_uid':(base/'stat').stat().st_uid,
  'thread_count':len(list((base/'task').iterdir()))}
def children():return [int(x) for x in Path(f'/proc/self/task/{os.getpid()}/children').read_text().split()]
def mapped(pid):
 text=Path(f'/proc/{pid}/maps').read_text();require(len(text)<=262144,'maps_bound');names=set()
 for line in text.splitlines():
  fields=line.split(None,5)
  if len(fields)==6 and fields[5].startswith('/'):
   require(not fields[5].endswith(' (deleted)'),'deleted_mapping');names.add(fields[5])
 require(len(names)<=128,'mapped_file_count')
 rows=[]
 for name in sorted(names):
  require(name.startswith('/usr/lib/') or name.startswith('/lib/') or name==str(I/'pam-worker'),'unexpected_mapping_path')
  path=Path(name).resolve(strict=True)
  for parent in [path.parent,*path.parent.parents]:
   s=parent.lstat();require(s.st_uid==0 and stat.S_ISDIR(s.st_mode) and not(s.st_mode&0o022),'mapped_ancestor')
  s=path.stat();require(stat.S_ISREG(s.st_mode) and s.st_uid==0 and not(s.st_mode&0o022),'mapped_file_trust')
  rows.append({'path':name,'resolved':str(path),'sha256':sha(path),'uid':s.st_uid,'gid':s.st_gid,'mode':oct(stat.S_IMODE(s.st_mode))})
 return {'actual_maps':text,'mapped_files':rows}
def direct(argv,label,data=None):
 child=subprocess.Popen(argv,stdin=subprocess.PIPE if data is not None else subprocess.DEVNULL,
  stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
 row={'role':label,'pid':child.pid,'argv':argv,'started_BOOTTIME_ns':now()};result['direct_waits'].append(row)
 try:out,err=child.communicate(input=data,timeout=budget(10))
 except subprocess.TimeoutExpired:
  child.kill();out,err=child.communicate(timeout=3);row['timed_out']=True
 row.update(actual_direct_wait=True,exit=child.returncode,stdout_bytes=len(out),stderr_bytes=len(err),completed_BOOTTIME_ns=now())
 # No provisioning/module/helper raw output or digest enters retained evidence.
 require(child.returncode==0 and not row.get('timed_out'),'credential_provisioning_failed')
def io_budget(until):
 remaining=min((until-now())/1e9,outer-time.monotonic());require(remaining>0,'conversation_deadline');return remaining
def read_exact(child,fd,until,count,row):
 data=bytearray()
 while len(data)<count:
  io_budget(until)
  if select.select([child.stdout],[],[],min(.05,io_budget(until)))[0]:
   chunk=os.read(child.stdout.fileno(),count-len(data));require(bool(chunk),'unexpected_worker_EOF')
   data.extend(chunk);row['worker_protocol_output_bytes']+=len(chunk)
   require(row['worker_protocol_output_bytes']<=128,'worker_output_bound')
  elif not living(fd):raise FixtureError('worker_died_before_reply')
 return bytes(data)
def write_exact(child,until,data):
 offset=0
 while offset<len(data):
  io_budget(until)
  if select.select([],[child.stdin],[],min(.05,io_budget(until)))[1]:
   try:count=os.write(child.stdin.fileno(),data[offset:])
   except BlockingIOError:continue
   require(count>0,'private_pipe_write');offset+=count
def role(child,fd,start,row,boundary):
 before=process(child.pid);require(living(fd),'worker_dead_at_role_capture')
 require(before['parent']==os.getpid() and before['start_ticks']==start,'worker_generation_parent')
 require(before['uids']==before['gids']==[1000]*4 and before['groups']==[],'worker_fixed_credentials')
 require(int(before['capabilities']['CapBnd'],16)==0x20000000 and all(int(before['capabilities'][key],16)==0 for key in ['CapInh','CapPrm','CapEff','CapAmb']),'worker_capabilities_not_expected_audit_only_bounding')
 require(before['NoNewPrivs']==0 and before['proc_stat_owner_uid']==0,'worker_protected_privilege_policy')
 require(before['thread_count']==1,'worker_single_thread_role')
 require(Path(f'/proc/{child.pid}/cmdline').read_bytes().split(b'\0')[:-1]==
         [str(I/'pam-worker').encode(),b'--parent',str(os.getpid()).encode()],'fixed_trusted_parent_invocation')
 require(sha(f'/proc/{child.pid}/exe')==selected['worker_ELF_sha256'],'actual_worker_executable')
 require(os.readlink(f'/proc/{child.pid}/fd/0')==row['private_pipe_links']['worker_stdin'] and
         os.readlink(f'/proc/{child.pid}/fd/1')==row['private_pipe_links']['worker_stdout'],'private_pipe_identity')
 stdio_flags={}
 for number in range(3):
  info=Path(f'/proc/{child.pid}/fdinfo/{number}').read_text()
  flags=int(next(line.split(':',1)[1].strip() for line in info.splitlines() if line.startswith('flags:')),8)
  require(flags&os.O_CLOEXEC,'worker_stdio_not_CLOEXEC');stdio_flags[str(number)]=oct(flags)
 core=resource.prlimit(child.pid,resource.RLIMIT_CORE);require(core==(0,0),'worker_core_limit')
 maps=mapped(child.pid);after=process(child.pid)
 require(before['start_ticks']==after['start_ticks']==start and after['parent']==os.getpid() and living(fd),'role_capture_generation_changed')
 held=now();require(held+100_000_000<row['until_BOOTTIME_ns'],'role_hold_precondition_deadline')
 while now()<held+100_000_000:
  require(living(fd),'worker_died_while_response_withheld');time.sleep(.01)
 require(living(fd),'worker_died_after_role_hold')
 observation={'boundary':boundary,'before':before,'after':after,'maps':maps,'actual_stdio_flags':stdio_flags,
  'actual_RLIMIT_CORE':list(core),'actual_open_FD_numbers':sorted(int(path.name) for path in Path(f'/proc/{child.pid}/fd').iterdir()),
  'withheld_started_BOOTTIME_ns':held,'withheld_completed_BOOTTIME_ns':now(),
  'secret_response_written_before_capture':False,'proc_stat_root_owner_observed':True,
  'numeric_dumpable0_scope':'source-checked PR_GET_DUMPABLE before Ready plus guest policy0; proc owner is separately observed'}
 row['role_observations'].append(observation);return observation
def run_case(name,credential,expected):
 until=now()+30_000_000_000
 child=subprocess.Popen([str(I/'pam-worker'),'--parent',str(os.getpid())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,
  env={'PATH':'/usr/bin:/bin','LC_ALL':'C.UTF-8'},start_new_session=True)
 fd=os.pidfd_open(child.pid);owned[child.pid]=(child,fd);handles.append(fd)
 start=process(child.pid)['start_ticks'];os.set_blocking(child.stdin.fileno(),False);os.set_blocking(child.stdout.fileno(),False)
 row={'case':name,'expected_terminal_code':expected,'pid':child.pid,'start_ticks':start,'parent':os.getpid(),
  'until_BOOTTIME_ns':until,'deadline_seconds':30,'private_pipe_links':{
   'worker_stdin':os.readlink(f'/proc/self/fd/{child.stdin.fileno()}'),
   'worker_stdout':os.readlink(f'/proc/self/fd/{child.stdout.fileno()}')},
  'worker_protocol_output_bytes':0,'typed_records':[],'role_observations':[],'hidden_prompts':0,
  'secret_response_frames_sent':0,'passed':False,'actual_direct_wait':False}
 result['cases'].append(row)
 try:
  write_exact(child,until,b'PKPA'+struct.pack('<Q',until))
  require(read_exact(child,fd,until,1,row)==b'\x10','missing_private_Ready')
  row['typed_records'].append({'event':'Ready','observed_BOOTTIME_ns':now()});sequence=0
  while True:
   kind=read_exact(child,fd,until,1,row)
   if kind==b'\x11':
    number=struct.unpack('<H',read_exact(child,fd,until,2,row))[0]
    require(number==sequence+1 and number<=32,'hidden_prompt_sequence');sequence=number
    row['hidden_prompts']+=1;row['typed_records'].append({'event':'HiddenPrompt','sequence':number,'observed_BOOTTIME_ns':now()})
    observed=role(child,fd,start,row,'hidden_prompt_before_secret_response')
    names={Path(item['resolved']).name for item in observed['maps']['mapped_files']}
    require({'libpam.so.0.85.1','pam_unix.so'}<=names,'actual_PAM_mapping_before_response')
    encoded=credential.encode();require(0<len(encoded)<=512 and b'\0' not in encoded,'fixture_response_bounds')
    write_exact(child,until,b'\x20'+struct.pack('<HH',number,len(encoded))+encoded)
    row['secret_response_frames_sent']+=1
   elif kind==b'\x30':
    code=read_exact(child,fd,until,1,row)[0];require(code in (0,1,2),'unknown_terminal_code')
    row['typed_records'].append({'event':'TerminalCandidate','code':code,'observed_BOOTTIME_ns':now()})
    if not row['role_observations']:role(child,fd,start,row,'terminal_before_ACK_no_hidden_prompt')
    require(living(fd),'terminal_candidate_worker_not_live')
    write_exact(child,until,b'\x40');row['typed_records'].append({'event':'ParentACK','observed_BOOTTIME_ns':now()})
    status=child.wait(timeout=min(3,io_budget(until)));row.update(actual_direct_wait=True,actual_wait_exit=status)
    require(status==0,'terminal_candidate_without_wait0')
    require(child.stdout.read()==b'','extra_worker_output_after_terminal')
    row['worker_pidfd_dead']=not living(fd);row['worker_proc_absent']=not Path(f'/proc/{child.pid}').exists()
    require(row['worker_pidfd_dead'] and row['worker_proc_absent'],'worker_death_absence')
    row['terminal_code_after_wait0']=code;row['candidate_buffered_until_actual_wait0']=True
    row['passed']=code==expected and row['hidden_prompts']>=1 and row['secret_response_frames_sent']==row['hidden_prompts']
    row['closed_missing_prompt_baseline_observed']=code==2 and row['hidden_prompts']==0 and row['secret_response_frames_sent']==0
    break
   else:raise FixtureError('unexpected_worker_record_kind')
 finally:
  if child.poll() is None:signal.pidfd_send_signal(fd,signal.SIGKILL)
  row['actual_wait_exit']=child.wait(timeout=3);row['actual_direct_wait']=True
  row['worker_pidfd_dead']=not living(fd);row['worker_proc_absent']=not Path(f'/proc/{child.pid}').exists()
  child.stdin.close();child.stdout.close()
def cleanup():
 for child,fd in owned.values():
  if child.poll() is None:signal.pidfd_send_signal(fd,signal.SIGKILL)
  child.wait(timeout=3)
 deadline=time.monotonic()+3
 while True:
  for pid in children():
   try:
    fd=os.pidfd_open(pid);signal.pidfd_send_signal(fd,signal.SIGKILL);os.close(fd)
   except ProcessLookupError:pass
  try:pid,status=os.waitpid(-1,os.WNOHANG)
  except ChildProcessError:result['final_wait_all_ECHILD']=True;break
  if pid:result['adopted_waits'].append({'pid':pid,'status':status,'actual_waitpid':True,'proc_absent':not Path(f'/proc/{pid}').exists()})
  else:
   require(time.monotonic()<deadline,'owned_descendant_cleanup_deadline');time.sleep(.01)
 require(not result['adopted_waits'],'unexpected_adopted_child')
try:
 resource.setrlimit(resource.RLIMIT_CORE,(0,0));require(libc.prctl(36,1,0,0,0)==0,'subreaper_setup')
 selected=json.loads((R/'selection.json').read_text());require(platform.release()=='7.1.6-1-aarch64-ARCH','wrong_kernel')
 result['kernel_release']=platform.release();parent=process(os.getpid());result['fixture_parent_role']=parent
 require(os.getuid()==0 and parent['NoNewPrivs']==0 and int(parent['capabilities']['CapEff'],16)&(1<<19),'root_fixture_role')
 require(int(parent['capabilities']['CapInh'],16)==0,'fixture_inheritable_capabilities')
 for name,digest in selected['workspace_files'].items():require(sha(P/name)==digest,'workspace_hash')
 pins=json.loads((P/'selected-package-stack-pins.json').read_text())
 for item in pins['files']:
  target=Path(item['path']);require(sha(target)==item['sha256'],'installed_stack_hash')
  row=metadata(target);result.setdefault('selected_package_stack_metadata',[]).append(row)
  require(row['uid']==0 and not(target.stat().st_mode&0o022),'installed_stack_ownership')
 require(stat.S_IMODE(Path('/usr/bin/unix_chkpwd').stat().st_mode)==0o6755,'original_helper_mode')
 for path in ('/','/etc','/etc/pam.d','/run','/opt'):trusted(path)
 policy=Path('/proc/sys/fs/suid_dumpable');prior=policy.read_text().strip()
 result['guest_native_suid_dumpable_policy']={'actual_prior':prior,'intended_after':'0','host_policy_changed':False,
  'reason':'explicit native protected credential-transition prerequisite, not original unchanged global policy'}
 require(prior in ('0','1','2'),'unknown_guest_suid_dumpable')
 result['guest_native_suid_dumpable_policy']['actual_write_bytes']=policy.write_text('0\n')
 after=policy.read_text().strip();result['guest_native_suid_dumpable_policy']['actual_after']=after;require(after=='0','native_policy_write_verify')
 service=Path('/etc/pam.d/omarchy-lock-password')
 try:
  service.lstat();result['original_service_present_before_install']=True
  raise FixtureError('expected_original_service_missing_baseline')
 except FileNotFoundError as error:
  result['original_service_present_before_install']=False
  result['original_service_missing_lstat_errno']=error.errno;require(error.errno==errno.ENOENT,'service_absence_errno')
 service.write_bytes((P/'omarchy-lock-password').read_bytes());os.chown(service,0,0);service.chmod(0o644)
 result['installed_original_service_sha256']=sha(service);result['installed_original_service_metadata']=metadata(service)
 I.mkdir(mode=0o755);os.chown(I,0,0);trusted(I)
 shutil.copyfile(P/'pam-worker',I/'pam-worker');os.chown(I/'pam-worker',0,0);(I/'pam-worker').chmod(0o755)
 require(sha(I/'pam-worker')==selected['worker_ELF_sha256'],'installed_worker_ELF');result['installed_worker_metadata']=metadata(I/'pam-worker')
 account=pwd.getpwuid(1000);username=account.pw_name;reverse=pwd.getpwnam(username)
 require(re.fullmatch('[a-z_][a-z0-9_-]*',username) is not None and account.pw_gid==1000 and reverse.pw_uid==reverse.pw_gid==1000,'fixed_guest_account')
 result['guest_fixed_account']={'uid':1000,'resolved_same_fixed_account':True}
 secret=secrets.token_urlsafe(32);wrong=secrets.token_urlsafe(32);require(secret!=wrong,'fixture_credential_collision')
 direct(['/usr/bin/chpasswd'],'guest_only_credential_provisioning',data=(username+':'+secret+'\n').encode())
 result['credential_provisioned_only_through_owned_stdin_pipe']=True;result['setup_prerequisites_complete']=True
 run_case('wrong',wrong,1);run_case('correct',secret,0)
 for item in pins['files']:require(sha(item['path'])==item['sha256'],'installed_stack_post_hash')
 for name,digest in selected['workspace_files'].items():require(sha(P/name)==digest,'workspace_post_hash')
 result['selected_package_and_workspace_closure_before_after']=True
 result['passed']=len(result['cases'])==2 and all(row['passed'] for row in result['cases'])
 result['classification']='ACTUAL_NATIVE_PAM_WRONG_DENIED_CORRECT_AUTHENTICATED' if result['passed'] else 'NATIVE_PAM_CONTRACT_FAILED'
except BaseException as error:
 result['error_type']=type(error).__name__
 if isinstance(error,FixtureError):result['fixture_error_code']=error.args[0]
 result['passed']=False
finally:
 try:cleanup()
 except BaseException as error:result['cleanup_error_type']=type(error).__name__;result['passed']=False
 result['qualified_missing_hidden_prompt_RED']=bool(result.get('setup_prerequisites_complete') and
  result.get('selected_package_and_workspace_closure_before_after') and len(result['cases'])==2 and
  all(row.get('closed_missing_prompt_baseline_observed') and row.get('actual_direct_wait') and row.get('actual_wait_exit')==0 and
      row.get('worker_pidfd_dead') and row.get('worker_proc_absent') and row['role_observations'] for row in result['cases']) and
  result.get('final_wait_all_ECHILD') and not result['adopted_waits'] and not result.get('cleanup_error_type') and not result.get('error_type'))
 result['actual_native_PAM_authentication_qualified']=bool(result['passed'])
 if result['qualified_missing_hidden_prompt_RED']:result['classification']='QUALIFIED_NATIVE_CLOSED_UNAVAILABLE_MISSING_HIDDEN_PROMPT_RED'
 for fd in handles:os.close(fd)
 with (R/'result.json').open('x') as output:json.dump(result,output,indent=2);output.write('\n')
 print('NATIVE_PAM_COMPONENT_TERMINAL '+json.dumps({'passed':result['passed'],'qualified_missing_hidden_prompt_RED':result['qualified_missing_hidden_prompt_RED'],'error_type':result.get('error_type'),'fixture_error_code':result.get('fixture_error_code')}),flush=True)
 # Exact existing pixel-sim.service owns its ExecStopPost poweroff; no detached helper.
