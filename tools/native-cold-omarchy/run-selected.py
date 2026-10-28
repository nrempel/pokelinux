#!/usr/bin/env python3
"""Run one admitted native x86 composition; own every CLI, both PIPE EOFs and exact container identity."""
import argparse,fcntl,hashlib,json,os,select,signal,stat,subprocess,sys,time
from pathlib import Path
sys.dont_write_bytecode=True
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
MAIN=Path('/home/ubuntu/Projects/pokelinux');STOP=[]
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def load(path,limit=8*1024*1024):
 p=Path(path);assert 0<p.stat().st_size<=limit;return json.loads(p.read_text())
def relative(value):
 p=Path(value);assert p.parts and not p.is_absolute() and '..' not in p.parts;return p
def stop(signum,_frame):STOP.append(signum)  # Defer exit until post-spawn child custody is retained.
class Children:
 def __init__(self,root):self.root=root;self.items=[];self.receipts=[]
 def start(self,argv,role,until,limit=1048576):
  assert now()<until and len(self.items)<256
  logs={k:(self.root/(role+'.'+k)).open('xb',buffering=0) for k in ['stdout','stderr']}
  try:p=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,close_fds=True,start_new_session=True)
  except BaseException:
   for f in logs.values():f.close()
   raise
  item={'p':p,'role':role,'argv':argv,'until':until,'logs':logs,'eof':{},'counts':{'stdout':0,'stderr':0},'limit':limit,'wait':None,'pidfd':None,'ready':False,'started':now()};self.items.append(item)
  for k in logs:os.set_blocking(getattr(p,k).fileno(),False)
  item['ready']=True;item['pidfd']=os.pidfd_open(p.pid,os.O_NONBLOCK);return item
 def pump(self):
  for item in self.items:
   if not item['ready']:continue
   for k in item['logs']:
    if k in item['eof']:continue
    try:data=os.read(getattr(item['p'],k).fileno(),65536)
    except (BlockingIOError,InterruptedError):continue
    if not data:item['eof'][k]=now();continue
    old=item['counts'][k];item['counts'][k]+=len(data);room=max(0,item['limit']-old);item['logs'][k].write(data[:room])
 def check(self):
  assert not STOP,'deferred host signal '+str(STOP)
  assert all(n<=item['limit'] for item in self.items for n in item['counts'].values()),'bounded CLI output overflow'
 def finish(self,item,until,check=True):
  until=min(until,item['until']);p=item['p']
  while p.poll() is None:
   self.pump()
   if check:self.check()
   assert now()<until,item['role']+' original actual-wait cutoff'
   time.sleep(.005)
  if item['wait'] is None:
   code=p.wait(timeout=0);stamp=now();item['wait']={'role':item['role'],'pid':p.pid,'argv':item['argv'],'returncode':code,'actual_direct_parent_wait':True,'waited_at_boottime_ns':stamp,'original_cutoff_ns':until,'wait_before_original_cutoff':stamp<until};self.receipts.append(item['wait'])
  while len(item['eof'])<2 and now()<until:
   self.pump()
   if check:self.check()
   time.sleep(.001)
  self.complete(item)
  assert item['wait']['wait_before_original_cutoff'] and item['wait']['both_EOF_before_original_cutoff'],item['role']+' late/missing actual wait or EOF'
  return item['wait']['returncode']
 def complete(self,item):
  if item['wait'] is not None:item['wait'].update({'output_bytes':dict(item['counts']),'output_limit_per_stream':item['limit'],'stdout_EOF':'stdout' in item['eof'],'stderr_EOF':'stderr' in item['eof'],'EOF_at_boottime_ns':dict(item['eof']),'both_EOF_before_original_cutoff':len(item['eof'])==2 and max(item['eof'].values())<item['wait']['original_cutoff_ns'],'output_within_bound':all(n<=item['limit'] for n in item['counts'].values())})
 def run(self,argv,role,until,limit=1048576,check=True):
  item=self.start(argv,role,until,limit);code=self.finish(item,until,check=check);return code,{k:(self.root/(role+'.'+k)).read_text() for k in item['logs']}
 def close(self,original_until):
  errors=[]
  for item in reversed(self.items):
   p=item['p']
   try:
    if item['wait'] is None:
     if p.poll() is None:
      if item['pidfd'] is not None:signal.pidfd_send_signal(item['pidfd'],signal.SIGKILL)
      else:p.kill()
     # Failure-only emergency disposal has no power to qualify a late original receipt.
     emergency=now()+5_000_000_000
     while p.poll() is None and now()<emergency:self.pump();time.sleep(.005)
     code=p.wait(timeout=max(.001,(emergency-now())/1e9));stamp=now();cutoff=min(original_until,item['until']);item['wait']={'role':item['role'],'pid':p.pid,'argv':item['argv'],'returncode':code,'actual_direct_parent_wait':True,'waited_at_boottime_ns':stamp,'original_cutoff_ns':cutoff,'wait_before_original_cutoff':stamp<cutoff,'failure_disposal':True};self.receipts.append(item['wait'])
    emergency=now()+1_000_000_000
    while len(item['eof'])<2 and now()<emergency:self.pump();time.sleep(.001)
    self.complete(item)
    assert len(item['eof'])==2,item['role']+' failure disposal missing PIPE EOF'
   except BaseException as error:errors.append(item['role']+': '+type(error).__name__+': '+str(error))
   finally:
    for k,f in item['logs'].items():f.close();getattr(p,k).close()
    if item['pidfd'] is not None:os.close(item['pidfd'])
  return errors

def missing(code,body,identity):return code==1 and any(v+identity.lower() in body['stderr'].lower() for v in ['no such object: ','no such container: '])
def main():
 a=argparse.ArgumentParser(description=__doc__);a.add_argument('--selection-dir',required=True,type=Path);a.add_argument('--selection-sha256',required=True);a.add_argument('--command-sha256',required=True);x=a.parse_args()
 R=x.selection_dir.resolve(strict=True);assert R.parent==Path('/tmp') and R.name.startswith('pokelinux-native-x86-composition-')
 assert sha(R/'selection.json')==x.selection_sha256 and sha(R/'command.json')==x.command_sha256
 S=load(R/'selection.json');command=load(R/'command.json');assert S['phase']=='native-x86-installed-Source-Sounds-composition' and S['network']=='none'
 assert sha(__file__)==S['files']['source/run-selected.py'] and not S['vendor_or_private_inputs_selected'] and not S['phone_accessed'] and S['host_devices_attached']==['/dev/kvm'] and not S['automatic_retries_selected']
 assert command[:len(DOCKER)]==DOCKER and '--privileged' not in command and command.count('--device')==1 and command[command.index('--device')+1]=='/dev/kvm:/dev/kvm:rw'
 assert command[command.index('--network')+1]=='none' and command[command.index('--cidfile')+1]==str(R/'owned-container-id')
 assert S['container_cpus']==16 and command[command.index('--cpus')+1]=='16'and S['container_memory']==command[command.index('--memory')+1]=='10g'and S['guest_cpus']==6 and S['guest_memory_mib']==6144
 assert S['runtime_image']=='sha256:3b6891447ad0490585badb0cbbac5658238b9dc61bfa57e64a4cbad050be8f1f'
 admission=load(R/'source/proofs/runtime-Root-qualification.json');assert admission['qualified_derived_runtime_image']and admission['actual_image_id']==S['runtime_image']and not admission['runtime_guest_boot_qualified']
 authenticated=load(R/'source/proofs/cloud-authentication.json');assert authenticated['passed']and authenticated['checksum_and_image_signatures_valid']and authenticated['stock_qcow2_no_external_backing_or_data_file']and authenticated['actual_image_sha256']==S['stock_cloud_image_sha256']
 assert command[command.index('--user')+1]=='1000:1000'and command.count('--group-add')==1 and command[command.index('--group-add')+1]=='993'and command[command.index('--cap-drop')+1]=='ALL'and command[command.index('--security-opt')+1]=='no-new-privileges'
 assert command.count('-v')==4 and S['host_devices_attached']==['/dev/kvm']and S['automatic_retries_selected']is False
 assert S['native_guest_admission']['qualified']is True and S['native_guest_admission']['architecture']=='x86_64'
 provision=load(R/'source/proofs/native-provision-admission.json');assert provision==S['native_guest_admission']
 assert provision['coherent_offline_transaction_qualified']is True and provision['package_signatures_qualified']is True and provision['Hyprland_binary_source_binding_qualified']is True
 mounts=[command[i+1]for i,v in enumerate(command[:-1])if v=='-v'];assert set(mounts)=={str(R)+':/results:rw',S['stock_cloud_image_path']+':/vm/stock-cloud.qcow2:ro',S['provisioned_layer_path']+':/vm/provisioned-layer.qcow2:ro',S['theme_archive_host_path']+':/assets/omarchy-public.tar.gz:ro'}
 name=command[command.index('--name')+1];assert name=='pokelinux-native-x86-composition-'+S['label'] and command[-3:]==[S['runtime_image'],'python3','/results/source/boot.py']
 assert all(not (R/n).exists() for n in ['execution.json','result.json','bootstrap-result.json','boot-result.json','console.log','owned-container-id','output.lock'])
 lock=Path(S['shared_input_lock']);assert lock.is_absolute()and lock.is_file()and str(lock)==provision['input_read_lock_path']
 T0=now();UNTIL=T0+int(S['outer_deadline_seconds'])*1_000_000_000;WORK_UNTIL=UNTIL-30_000_000_000
 record={'schema':1,'scope':S['scope'],'passed':False,'selection_sha256':x.selection_sha256,'command_sha256':x.command_sha256,'runner_sha256':sha(__file__),'started_at_boottime_ns':T0,'original_outer_until_ns':UNTIL,'original_work_until_ns':WORK_UNTIL,'phone_accessed':False,'hardware_qualified':False,'native_x86_KVM_cold_installed_startup_slice_only':True,'prior_ARM_functional_result_inherited':False,'ownership_observations':[]}
 children=Children(R);cli=None;cid=None;live=False;daemon=None;error=None;counter=0
 cpu_pidfd=None;cpu_binding=None;cpu_next=0;cpu_bytes=0
 cpu_record={'scope':'Read-only exact inspected container cgroup2 CPU counters, not Source or causal evidence','resource_variant':S['resource_variant'],'records':[],'maximum_records':64,'maximum_encoded_record_bytes':131072,'maximum_stat_file_bytes':1024,'maximum_max_file_bytes':128,'maximum_proc_file_bytes':4096,'sample_interval_ns':1000000000,'baseline8_counter_control_available':False,'new_children_or_workers':False,'quota_cause_proven':False};record['container_CPU_observations']=cpu_record
 def kernel_read(path,maximum):
  fd=os.open(path,os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK)
  try:
   raw=os.read(fd,maximum+1);assert len(raw)<=maximum,'bounded kernel file overflow';return raw.decode('ascii')
  finally:os.close(fd)
 def cpu_capture(label,force=False):
  nonlocal cpu_next,cpu_bytes,cpu_binding
  stamp=now()
  if cpu_pidfd is None or(len(cpu_record['records'])>=64):cpu_record['capture_capped']=len(cpu_record['records'])>=64;return
  if not force and stamp<cpu_next:return
  cpu_next=stamp+1000000000;entry={'label':label,'entered_boottime_ns':stamp,'original_work_until_ns':WORK_UNTIL,'exact_owned_CID':cid,'inspected_container_init_PID':record['container_init_pid'],'qualified_identity':False}
  try:
   assert stamp<WORK_UNTIL and not select.select([cpu_pidfd],[],[],0)[0],'inspected container init already exited'
   proc=Path('/proc')/str(record['container_init_pid']);before=kernel_read(proc/'stat',4096).rsplit(') ',1)[1].split()[19];membership=kernel_read(proc/'cgroup',4096)
   lines=membership.splitlines();assert len(lines)==1 and lines[0].startswith('0::/');group=Path(lines[0][3:]);assert group.is_absolute()and '..'not in group.parts and group.parts[-1]=='docker-'+cid+'.scope'
   root=Path('/sys/fs/cgroup');assert(root/'cgroup.controllers').is_file();directory=root.joinpath(*group.parts[1:]);assert directory.resolve(strict=True)==directory
   info=directory.stat();assert stat.S_ISDIR(info.st_mode)and info.st_dev==root.stat().st_dev
   identity={'proc_starttime':before,'cgroup_v2_membership':membership,'resolved_cgroup_path':str(directory),'cgroup_device':info.st_dev,'cgroup_inode':info.st_ino,'host_cgroup_mount_device':root.stat().st_dev}
   if cpu_binding is None:cpu_binding=identity
   assert cpu_binding==identity,'owned init or cgroup identity changed'
   actual_max=kernel_read(directory/'cpu.max',128);actual_stat=kernel_read(directory/'cpu.stat',1024)
   assert kernel_read(proc/'stat',4096).rsplit(') ',1)[1].split()[19]==before and kernel_read(proc/'cgroup',4096)==membership and not select.select([cpu_pidfd],[],[],0)[0],'owned init changed/exited during CPU read'
   counters={}
   for line in actual_stat.splitlines():
    key,value=line.split();assert key not in counters and value.isdecimal();counters[key]=int(value)
   maximum=actual_max.split();assert len(maximum)==2 and maximum[1].isdecimal();period=int(maximum[1]);assert period>0
   entry.update(identity=identity,cpu_max_raw=actual_max,cpu_stat_raw=actual_stat,cpu_stat_counters=counters,actual_period_usec=period,quota_matches_selected16=(maximum[0].isdecimal()and int(maximum[0])==16*period),qualified_identity=True)
  except (OSError,AssertionError,ValueError,IndexError)as failure:entry['observation_error']={'type':type(failure).__name__,'errno':getattr(failure,'errno',None),'detail':str(failure)[:256]}
  entry['returned_boottime_ns']=now();entry['returned_before_original_work_cutoff']=entry['returned_boottime_ns']<WORK_UNTIL;entry['process_wait_or_PIPE_EOF_claimed']=False
  encoded=len(json.dumps(entry,separators=(',',':')).encode())
  if cpu_bytes+encoded>131072:cpu_record['capture_capped']=True;return
  cpu_bytes+=encoded;cpu_record['records'].append(entry);cpu_record['encoded_record_bytes']=cpu_bytes
 def helper(args,role,until=WORK_UNTIL,check=True):
  nonlocal counter
  counter+=1;return children.run(DOCKER+args,str(counter).zfill(3)+'-'+role,until,limit=131072,check=check)
 def closure():
  for p,h in S['files'].items():assert sha(R/relative(p))==h,p
  for p,h in S['external_inputs'].items():assert sha(p)==h,p
  for name,row in S['cold_installed_bundle']['files'].items():
   path=R/'deployment/root'/relative(name);assert path.stat().st_size==row['bytes']and path.stat().st_mode&0o777==row['mode']and sha(path)==row['sha256'],name
  assert sha(R/'selection.json')==x.selection_sha256 and sha(R/'command.json')==x.command_sha256
  assert now()<UNTIL,'input closure observed beyond original outer cutoff'
 def published():
  nonlocal cid
  p=R/'owned-container-id'
  if not p.exists():return
  body=p.read_bytes();assert len(body)<=128;v=body.decode('ascii').strip()
  if len(v)!=64 or any(c not in '0123456789abcdef' for c in v):return
  if cid is None:cid=v;record['exact_owned_container_id']=v
  assert cid==v,'exact owned CID changed'
 def inspect(phase,until):
  assert cid is not None
  code,body=helper(['container','inspect','--format','{"id":{{json .Id}},"name":{{json .Name}},"image":{{json .Config.Image}},"state":{{json .State}}}',cid],'inspect-'+phase,until)
  stamp=now();row={'phase':phase,'id':cid,'returncode':code,'stdout':body['stdout'],'stderr':body['stderr'],'completed_at_boottime_ns':stamp,'original_cutoff_ns':until,'completed_before_original_cutoff':stamp<until};record['ownership_observations'].append(row);assert stamp<until
  if code==0:
   v=json.loads(body['stdout']);assert v['id']==cid and v['name']=='/'+name and v['image']==S['runtime_image'];return code,body,v['state']
  return code,body,None
 for sig in [signal.SIGINT,signal.SIGTERM]:signal.signal(sig,stop)
 with (R/'output.lock').open('xb') as outlock,lock.open('rb') as inlock:
  fcntl.flock(outlock,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(inlock,fcntl.LOCK_SH|fcntl.LOCK_NB);record['shared_readonly_input_lock_held']=record['per_label_exclusive_lock_held']=True
  try:
   closure();code,b=helper(['image','inspect','--format','{{.Id}}',S['runtime_image']],'image');assert code==0 and b['stdout'].strip()==S['runtime_image']
   code,b=helper(['info','--format','{{.ID}}'],'daemon-before');assert code==0;daemon=b['stdout'].strip();assert daemon;record['docker_daemon_id_before']=daemon
   code,b=helper(['container','inspect',name],'label-absent-before');assert missing(code,b,name)
   registration_until=min(WORK_UNTIL,now()+30_000_000_000);record['original_registration_until_ns']=registration_until
   cli=children.start(command,'container-cli',WORK_UNTIL,limit=8388608);record['owned_container_CLI_pid']=cli['p'].pid
   while cli['p'].poll() is None:
    children.pump();children.check();published();assert now()<WORK_UNTIL,'original owned container work deadline'
    if not live:
     assert now()<registration_until,'original exact-CID registration cutoff'
     if cid is not None:
      code,b,state=inspect('live-registration',registration_until)
      if state and state['Running'] and state['Pid']>1:
       live=True;record['live_owned_identity_confirmed']=True;record['container_init_pid']=state['Pid']
       try:cpu_pidfd=os.pidfd_open(state['Pid'],os.O_NONBLOCK);cpu_capture('after-exact-live-CID-inspection',True)
       except OSError as failure:cpu_record['pidfd_open_error']={'errno':failure.errno,'detail':str(failure)[:256]}
      elif state and state['Status'] in ['exited','dead']:raise RuntimeError('owned container terminal before live receipt')
    if live:cpu_capture('existing-host-loop')
    time.sleep(.02)
   published();record['actual_CLI_wait_exit']=children.finish(cli,WORK_UNTIL);assert record['actual_CLI_wait_exit']==0 and live
  except BaseException as failure:error=type(failure).__name__+': '+str(failure)
  finally:
   if error:record['error']=error
   if cpu_pidfd is not None:
    os.close(cpu_pidfd);cpu_pidfd=None;cpu_record['retained_init_pidfd_closed']=True
   try:published()
   except BaseException as failure:record['CID_publication_error']=str(failure)
   if cli is not None and cli['p'].poll() is None and cid is not None:
    try:code,b=helper(['kill',cid],'failure-exact-CID-kill',UNTIL,check=False);record['failure_exact_CID_kill']={'returncode':code,**b}
    except BaseException as failure:record['failure_exact_CID_kill_error']=str(failure)
   # Reap all retained helpers and the exact foreground Docker CLI before claiming CID absence.
   record['host_cleanup_errors']=children.close(UNTIL)
   post=Children(R)
   def final_helper(args,role):return post.run(DOCKER+args,'final-'+role,UNTIL,limit=131072,check=False)
   try:
    assert cid is not None and cli is not None and cli['wait'] is not None,'unresolved original owned CLI/CID'
    absent=False
    for index in range(128):
     code,b=final_helper(['container','inspect',cid],'CID-'+str(index));stamp=now();assert stamp<UNTIL
     record['ownership_observations'].append({'phase':'exact-owned-CID-after-actual-CLI-wait','id':cid,'returncode':code,**b,'completed_at_boottime_ns':stamp,'original_cutoff_ns':UNTIL,'completed_before_original_cutoff':stamp<UNTIL})
     if missing(code,b,cid):absent=True;break
     time.sleep(.1)
    record['owned_container_absence_qualified']=absent
    if not absent:record['unresolved_owned_container_handle']=cid
    assert absent
    code,b=final_helper(['info','--format','{{.ID}}'],'daemon-after');record['docker_daemon_id_after']=b['stdout'].strip();assert code==0 and record['docker_daemon_id_after']==daemon;record['same_daemon']=True
    closure();record['all_original_and_selected_inputs_unchanged']=True
    guest=load(R/'result.json');boot=load(R/'boot-result.json');bootstrap=load(R/'bootstrap-result.json');record['guest_result_sha256']=sha(R/'result.json');record['boot_result_sha256']=sha(R/'boot-result.json');record['bootstrap_result_sha256']=sha(R/'bootstrap-result.json')
    assert bootstrap['passed']and bootstrap['selection_sha256']==x.selection_sha256 and bootstrap['actual_resuid']==bootstrap['actual_resgid']==[0,0,0]and bootstrap['actual_uname_m']=='x86_64'and bootstrap['actual_early_runtime_time_wait_mask']['passed']
    assert guest['selection_sha256']==x.selection_sha256 and guest['guest_case_closure_qualified']
    record['guest_functional_passed']=guest['passed']
    if guest['passed']:
     assert guest['cold_installed_startup_slice_only']and not guest['original_prepared_four_groups_rerun']and not guest['authored_theme_host_selected']and not guest['prepared_warmup_selected']and not guest['manual_first_Source_load_selected']
     assert [v['id']for v in guest['boundary_steps']]==[1]and guest['boundary_steps'][0]['passed']
     assert guest['theme_provisioning_PID1_completion']['cgroup_empty']
     assert guest['genuine_full_shell_executed']and len(guest['actual_Root_installed_bundle']['files'])==len(S['cold_installed_bundle']['files']) and len(guest['actual_installed_PID1_identities'])==4
     assert not guest['installed_unit_closure']['cleanup_errors']and guest['installed_unit_closure']['service_waits_not_relabelled_as_guest_waits']and guest['installed_unit_closure']['all_unit_operational_results_success']
     qt=guest['current_native_Qt_build'];assert qt['actual_audio_total_including_setup_cleanup']==32 and qt['actual_audio_behavior_cases']==30 and len(qt['actual_appearance_cases'])==29
     assert all(v['ELF_machine']==62 for v in qt['binaries'].values())and guest['prior_ARM_Qt_results_relabelled_as_native']is False
     assert len(list((R/'pcm').glob('*/result.json')))==1 and guest['boundary_steps'][0]['PCM']['measured_frames']==48000
    assert sha(R/'seed-user-data')==boot['seed']['user_data_sha256']and sha(R/'seed-meta-data')==boot['seed']['meta_data_sha256']
    user_data=(R/'seed-user-data').read_text();assert user_data.startswith('#cloud-config\n');cloud=json.loads(user_data.split('\n',1)[1]);assert cloud['users']==[]and cloud['package_update']is False and cloud['package_upgrade']is False
    assert cloud['runcmd']==[['/usr/bin/python3','/usr/local/libexec/pokelinux-native-composition/cloud-bootstrap.py','--selection-sha256',x.selection_sha256,'--instance-id',S['label']]]
    assert boot['actual_guest_result_sha256']==sha(R/'result.json')and boot['actual_bootstrap_result_sha256']==sha(R/'bootstrap-result.json')and bootstrap['actual_guest_result_sha256']==sha(R/'result.json')
    assert boot['passed'] and boot['QEMU_actual_wait_exit']==0 and len(boot['owned_process_receipts'])==5 and boot['actual_QMP_KVM']['query_kvm_enabled']and boot['actual_QMP_KVM']['query_kvm_present']and boot['serial_guest_identity_and_result_hash_match']and boot['actual_QMP_KVM']['actual_guest_SHUTDOWN']['value']['data']=={'guest':True,'reason':'guest-shutdown'}and boot['actual_QMP_KVM']['socket_EOF_before_original_cutoff']
    assert all(v['returncode']==0 and v['actual_direct_parent_wait'] and v['stdout_EOF'] and v['wait_before_original_cutoff'] and v['EOF_before_original_cutoff'] for v in boot['owned_process_receipts'])
    record['nested_boot_and_guest_closure_qualified']=True
   except BaseException as failure:
    record['closure_error']=type(failure).__name__+': '+str(failure)
    if not record.get('owned_container_absence_qualified',False):record['unresolved_owned_container_handle']=cid or name
   finally:record['host_cleanup_errors']+=post.close(UNTIL);record['host_process_receipts']=children.receipts+post.receipts;record['completed_at_boottime_ns']=now()
   allclosed=len(record['host_process_receipts'])==len(children.items)+len(post.items) and all(v['actual_direct_parent_wait'] and v['wait_before_original_cutoff'] and v['stdout_EOF'] and v['stderr_EOF'] and v['both_EOF_before_original_cutoff'] and v['output_within_bound'] for v in record['host_process_receipts'])
   record['deferred_signals']=list(STOP)
   record['passed']=not STOP and error is None and 'closure_error' not in record and not record['host_cleanup_errors'] and allclosed and record.get('all_original_and_selected_inputs_unchanged',False) and record.get('owned_container_absence_qualified',False) and record.get('same_daemon',False) and record.get('nested_boot_and_guest_closure_qualified',False) and record.get('guest_functional_passed',False) and now()<UNTIL
   record['native_KVM_cold_installed_startup_slice_qualified']=record['passed'];record['original_prepared_four_groups_or_live_theme_qualified']=False
   (R/'execution.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'execution':str(R/'execution.json'),'sha256':sha(R/'execution.json'),'passed':record['passed']}),flush=True)
 if not record['passed']:raise SystemExit(1)
if __name__=='__main__':main()
