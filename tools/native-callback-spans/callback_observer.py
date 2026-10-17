"""Optional fixed callback-ring custody. No Source socket, readiness, child or retry."""
import hashlib,json,os,select,stat,time
from pathlib import Path
import callback_span_consumer as reader

FILE=Path('/run/user/1000/pokelinux-callback-spans.bin')
FILE_BYTES=82048
ASSESSMENT_BYTES=1024*1024
OWNER_UNIT='pokelinux-session-owner.service'

def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def dead(fd):return fd is not None and bool(select.select([fd],[],[],0)[0])
def metadata(info):
 return {'device':info.st_dev,'inode':info.st_ino,'bytes':info.st_size,'UID':info.st_uid,'GID':info.st_gid,'mode':stat.S_IMODE(info.st_mode),'links':info.st_nlink,'mtime_ns':info.st_mtime_ns,'ctime_ns':info.st_ctime_ns}
def exact(info):
 assert stat.S_ISREG(info.st_mode)and info.st_nlink==1 and info.st_uid==info.st_gid==1000 and stat.S_IMODE(info.st_mode)==0o600 and info.st_size==FILE_BYTES

def source_stamps(requests,owner):
 rows=[];seen=set()
 for index,request in enumerate(requests):
  if len(rows)==16:break
  if request.get('path')!='/run/pokelinux/session/admin.sock' or tuple(request.get('peer',()))!=(owner['PID'],0,0):continue
  reply=request.get('response',{});source=reply.get('source')if isinstance(reply,dict)else None;accepted=source.get('last_accepted')if isinstance(source,dict)else None
  if not isinstance(accepted,dict):continue
  sequence=accepted.get('sequence');stamp=accepted.get('sampled_at_ns')
  if type(sequence)is not int or not 0<sequence<1<<64 or type(stamp)is not int or not 0<=stamp<1<<64 or(sequence,stamp)in seen:continue
  seen.add((sequence,stamp));rows.append({'existing_request_index':index,'authenticated_peer_PID_UID_GID':list(request['peer']),'owner_InvocationID':owner['InvocationID'],'Source_sequence':sequence,'sampled_at_ns':stamp,'reply_observed_at_boottime_ns':request['at_boottime_ns']})
 return rows

class CallbackObserver:
 def __init__(self,root,close_until):
  self.root=Path(root);self.close_until=close_until;self.fd=None;self.writer_fd=None;self.owner_fd=None;self.owner=None;self.writer=None;self.prepared=None;self.bind_attempted=False;self.work_until=None
  self.row={'schema':1,'optional_diagnostic_only':True,'runtime_qualified':False,'functional_result_or_readiness_changed_by_trace':False,'Source_channel_accessed':False,'source_stamp_limit':16,'fixed_file_bytes':FILE_BYTES,'assessment_byte_cap':ASSESSMENT_BYTES,'read_while_writer_live_permitted':False,'readiness_or_cause_inference_permitted':False}
 def failure(self,stage,error):
  self.row.setdefault('errors',[]).append({'stage':stage,'error':type(error).__name__,'errno':getattr(error,'errno',None),'detail':str(error)[:512]});self.row['available']=False
 def prepare(self,manager_started,work_until):
  fd=None;readfd=None;self.work_until=work_until
  try:
   assert not manager_started and now()<work_until
   assert FILE.parent==FILE.parent.resolve(strict=True)
   parent=FILE.parent.lstat();assert stat.S_ISDIR(parent.st_mode)and parent.st_uid==parent.st_gid==1000 and stat.S_IMODE(parent.st_mode)==0o700
   with Path('/proc/self/mountinfo').open('rb')as stream:raw=stream.read(131073)
   assert len(raw)<=131072;mounts=[]
   for line in raw.decode().splitlines():
    left,right=line.split(' - ',1);mount=left.split()[4]
    if str(FILE).startswith(mount.rstrip('/')+'/'):mounts.append((len(mount),right.split()[0]))
   assert mounts and max(mounts)[1]=='tmpfs'
   fd=os.open(FILE,os.O_CREAT|os.O_EXCL|os.O_RDWR|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK,0o600)
   os.fchown(fd,1000,1000);os.fchmod(fd,0o600);os.ftruncate(fd,FILE_BYTES);zero=bytes(FILE_BYTES);assert os.pwrite(fd,zero,0)==FILE_BYTES
   info=os.fstat(fd);exact(info)
   readfd=os.open(FILE,os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK);again=os.fstat(readfd);exact(again);assert metadata(again)==metadata(info)
   assert now()<work_until
   self.fd=readfd;readfd=None;self.prepared=metadata(info);self.row['file_preparation']={'path':str(FILE),**self.prepared,'zero_bytes_written':FILE_BYTES,'initial_zero_sha256':hashlib.sha256(zero).hexdigest(),'actual_enclosing_filesystem':'tmpfs','retained_readonly_fd_before_manager_start':True,'prepared_at_boottime_ns':now()}
  except BaseException as error:self.failure('prepare',error)
  finally:
   for value in [fd,readfd]:
    if value is not None:
     try:os.close(value)
     except OSError as error:self.failure('prepare-close',error)
 def bind_writer(self,pid,pidfd,owner,owner_pidfd,starttime):
  if self.fd is None or self.bind_attempted:return
  self.bind_attempted=True
  try:
   assert self.work_until is not None and now()<self.work_until
   self.writer_fd=os.dup(pidfd);self.owner_fd=os.dup(owner_pidfd)
   assert not dead(self.writer_fd)and not dead(self.owner_fd)
   proc=Path('/proc')/str(pid);text=(proc/'status').read_text();fields={line.split(':',1)[0]:line.split(':',1)[1].split()for line in text.splitlines()if ':'in line}
   assert fields['Pid']==fields['Tgid']==[str(pid)]and fields['Uid']==fields['Gid']==['1000']*4 and not fields['Groups']
   task=(proc/'task'/str(pid)/'status').read_text();taskfields={line.split(':',1)[0]:line.split(':',1)[1].split()for line in task.splitlines()if ':'in line};assert taskfields['Pid']==taskfields['Tgid']==[str(pid)]
   assert(proc/'stat').read_text().rsplit(') ',1)[1].split()[19]==starttime
   current=FILE.lstat();exact(current);assert(current.st_dev,current.st_ino)==(self.prepared['device'],self.prepared['inode'])
   assert not dead(self.writer_fd)and not dead(self.owner_fd)
   self.owner=dict(owner);self.writer=pid;self.row['writer_binding']={'PID':pid,'expected_main_TID':pid,'actual_proc_Pid_equals_Tgid':True,'actual_main_task_Pid_equals_Tgid':True,'proc_starttime':starttime,'owner_identity':self.owner,'retained_duplicate_writer_and_owner_pidfds':True,'bound_at_boottime_ns':now(),'live_file_content_read':False}
  except BaseException as error:self.failure('bind-writer',error)
 def capture(self,closure,requests):
  try:
   until=min(self.close_until,now()+2_000_000_000);self.row['original_capture_until_ns']=until
   if self.fd is None or self.writer is None:self.row['not_attempted_reason']='no_retained_file_and_authenticated_main_writer';return
   assert now()<until
   if not dead(self.writer_fd)or not dead(self.owner_fd):self.row['not_attempted_reason']='retained_writer_or_owner_pidfd_still_live';return
   states=closure.get('unit_closures',[]);assert len(states)==4 and {row['unit']for row in states}=={OWNER_UNIT,'pokelinux-normal-audio.service','pokelinux-normal-appearance.service','pokelinux-omarchy-shell.service'};matching=[row for row in states if row['unit']==OWNER_UNIT and row.get('startup_identity')==self.owner]
   if len(matching)!=1:self.row['not_attempted_reason']='missing_matching_owner_retirement';return
   retired=lambda row:row['final_PID1_state']['MainPID']==row['final_PID1_state']['ControlPID']==0 and not row['remaining_cgroup_members']
   if not matching[0]['retained_pidfd_dead']or not all(retired(row)for row in states)or not states or not all(row['retained_pidfd_dead']for row in closure.get('member_pidfds',[])):
    self.row['not_attempted_reason']='existing_unit_or_member_retirement_incomplete';return
   self.row['retirement']={'retained_writer_pidfd_dead':True,'retained_owner_pidfd_dead':True,'matching_owner_InvocationID':self.owner['InvocationID'],'existing_selected_unit_cgroups_empty':True,'observed_at_boottime_ns':now(),'no_additional_stop_signal_wait_or_command':True}
   before=os.fstat(self.fd);exact(before);assert(before.st_dev,before.st_ino)==(self.prepared['device'],self.prepared['inode'])
   assert now()<until;raw=os.pread(self.fd,FILE_BYTES,0);after=os.fstat(self.fd);exact(after);assert len(raw)==FILE_BYTES and metadata(before)==metadata(after)and now()<until
   target=self.root/'callback-spans-after-retirement.bin'
   with target.open('xb')as stream:stream.write(raw)
   self.row['snapshot']={'file':target.name,'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),'retained_descriptor_metadata_before':metadata(before),'retained_descriptor_metadata_after':metadata(after),'exact_retained_fd_pread_only_after_retirement':True,'regular_file_not_PIPE_EOF':True}
   provenance=source_stamps(requests,self.owner);self.row['authenticated_existing_Source_stamps']=provenance
   assessment=reader.assess(raw,self.writer,self.writer,[row['sampled_at_ns']for row in provenance]);assert now()<until
   data=(json.dumps(assessment,sort_keys=True,separators=(',',':'))+'\n').encode();assert len(data)<=ASSESSMENT_BYTES
   target=self.root/'callback-spans-assessment.json'
   with target.open('xb')as stream:stream.write(data)
   self.row['assessment']={'file':target.name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'frozen_reader_v2':True,'writer_PID_equals_main_TID':assessment['writer_pid']==assessment['writer_tid']==self.writer,'status_flags':assessment['status_flags'],'sequence_saturated':assessment['sequence_saturated'],'committed_record_count':assessment['committed_record_count'],'overflow_observed':assessment['overflow_observed'],'partial_final_reservation':assessment['partial_final_reservation'],'unmatched_entry_count':len(assessment['unmatched_entries']),'completed_span_count':len(assessment['completed_spans'])}
   assert now()<until;self.row['available']=not bool(self.row.get('errors'))
  except BaseException as error:self.failure('capture',error)
  finally:
   try:self.row['capture_completed_at_boottime_ns']=now()
   except BaseException as error:self.failure('capture-completion-clock',error)
 def close(self):
  for name in ['fd','writer_fd','owner_fd']:
   value=getattr(self,name)
   if value is not None:
    try:os.close(value)
    except OSError as error:self.failure('close-'+name,error)
    finally:setattr(self,name,None)
  self.row['owned_descriptor_close_attempts_completed']=True


# Finite proc-metadata observer; external native identity authentication is required.
import re

class FDIdentityObserver:
 SPACING_NS=50_000_000
 MAX_FDS=64
 MAX_BYTES=524288
 def __init__(self,work_until_ns):
  self.until=work_until_ns;self.pid=None;self.pidfd=None;self.starttime=None;self.bound=False;self.disarmed=False
  self.record={'schema':1,'optional_diagnostic_only':True,'Source_channel_accessed':False,'fd_lifetime_continuity_proven':False,'external_native_identity_authentication_required':True,'created_boottime_ns':None,'first_trigger_boottime_ns':None,'second_due_boottime_ns':None,'photos':[],'errors':[]}
  try:self.record['created_boottime_ns']=now()
  except BaseException as error:self._error(self.record,error);self.disarmed=True
 def _error(self,row,error):
  if len(row['errors'])<8:row['errors'].append({'error':type(error).__name__,'errno':getattr(error,'errno',None)})
 def _active(self):
  if now()>=self.until:raise TimeoutError('original WORK expired')
 def _raw(self,path,maximum):
  self._active()
  with Path(path).open('rb')as stream:data=stream.read(maximum+1)
  self._active();return data
 def _identity(self):
  self._active();assert not dead(self.pidfd),'borrowed pidfd dead'
  raw=self._raw(Path('/proc/self/fdinfo')/str(self.pidfd),4096);assert len(raw)<=4096
  assert re.search(rb'^Pid:\s*'+str(self.pid).encode()+rb'\s*$',raw,re.M),'borrowed pidfd targets different PID'
  raw=self._raw(Path('/proc')/str(self.pid)/'stat',8192);assert len(raw)<=8192
  actual=raw.decode().rsplit(') ',1)[1].split()[19];assert actual==self.starttime,'proc starttime mismatch'
  assert not dead(self.pidfd);return actual
 @staticmethod
 def _stat(info):
  return {'device':info.st_dev,'inode':info.st_ino,'rdev':info.st_rdev,'type':stat.S_IFMT(info.st_mode),'mode':stat.S_IMODE(info.st_mode),'UID':info.st_uid,'GID':info.st_gid}
 def _fdrow(self,fd):
  self._active();proc=Path('/proc')/str(self.pid);path=proc/'fd'/str(fd)
  target=os.readlink(path);encoded=os.fsencode(target);assert len(encoded)<=4096
  before=self._stat(os.stat(path));raw=self._raw(proc/'fdinfo'/str(fd),4096)
  after=self._stat(os.stat(path));target_after=os.readlink(path);assert before==after and target==target_after,'descriptor metadata changed during read'
  allowed=re.fullmatch(r'(?:anon_inode:\[[A-Za-z0-9_]+\]|(?:pipe|socket):\[[0-9]+\]|/dev/dri/(?:card[0-9]+|renderD[0-9]+)|/dev/input/event[0-9]+)',target)is not None
  return {'fd':fd,'target':target if allowed else None,'target_sha256':hashlib.sha256(encoded).hexdigest(),'stat':before,'fdinfo':raw[:4096].decode('ascii'),'fdinfo_truncated':len(raw)>4096,'stable_during_read':True}
 def _photo(self,ordinal):
  photo={'ordinal':ordinal,'PID':self.pid,'started_boottime_ns':None,'completed_boottime_ns':None,'proc_starttime_before':None,'proc_starttime_after':None,'pidfd_live_before':False,'pidfd_live_after':False,'proc_identity_verified':False,'fd_limit_exceeded':False,'rows':[],'errors':[]};self.record['photos'].append(photo)
  try:
   photo['started_boottime_ns']=now();self._active();photo['proc_starttime_before']=self._identity();photo['pidfd_live_before']=True
   names=[]
   with os.scandir(Path('/proc')/str(self.pid)/'fd')as entries:
    for entry in entries:
     self._active();assert entry.name.isdecimal();names.append(int(entry.name))
     if len(names)==self.MAX_FDS+1:break
   photo['fd_limit_exceeded']=len(names)>self.MAX_FDS
   for fd in sorted(names[:self.MAX_FDS]):
    self._active()
    try:row=self._fdrow(fd)
    except BaseException as error:row={'fd':fd,'error':type(error).__name__,'errno':getattr(error,'errno',None),'stable_during_read':False}
    photo['rows'].append(row)
    # Reserve8KiB for both finite photo epilogues and bounded error summaries.
    if len(json.dumps(self.record,separators=(',',':'),sort_keys=True).encode())>self.MAX_BYTES-8192:
     photo['rows'].pop();photo['byte_limit_exceeded']=True;self.record['byte_limit_exceeded']=True;break
   photo['proc_starttime_after']=self._identity();photo['pidfd_live_after']=True;photo['proc_identity_verified']=True
  except BaseException as error:
   photo['rows'].clear();self._error(photo,error);self.disarmed=True
  finally:
   try:photo['completed_boottime_ns']=now()
   except BaseException as error:self._error(photo,error);self.disarmed=True
 def bind_main(self,pid,borrowed_pidfd,expected_starttime):
  if self.bound or self.disarmed:return
  self.bound=True
  try:
   self._active();assert type(pid)is int and pid>0 and type(expected_starttime)is str and expected_starttime.isdecimal()
   self.pid=pid;self.pidfd=borrowed_pidfd;self.starttime=expected_starttime
   trigger=now();self.record['first_trigger_boottime_ns']=trigger;self.record['second_due_boottime_ns']=trigger+self.SPACING_NS
   self._photo(1)
  except BaseException as error:self._error(self.record,error);self.disarmed=True
 def watch(self):
  if not self.bound or self.disarmed or len(self.record['photos'])>=2:return
  try:
   self._active()
   if now()<self.record['second_due_boottime_ns']:return
   self._identity();self._photo(2)
  except BaseException as error:self._error(self.record,error);self.disarmed=True
