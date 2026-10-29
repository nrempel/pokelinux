"""Guest request helper and genuinely read-only input-device identification."""
import array,fcntl,json,os,select,stat,struct,time
from pathlib import Path
from owned import now
from viewer_protocol import ACTIONS,read,publish,digest

def ioctl_read(fd,nr,size):
 data=bytearray(size);fcntl.ioctl(fd,(2<<30)|(size<<16)|(ord('E')<<8)|nr,data,True);return bytes(data)

def normalized(value):return ' '.join(value.lower().replace('-',' ').split())

class GuestInput:
 def __init__(self,root,selection_sha,work,close,tick,cleanup_tick):
  self.root=Path(root);self.sha=selection_sha;self.work=work;self.close=close;self.tick=tick;self.cleanup_tick=cleanup_tick;self.seq=0;self.failed=False;self.retire_sent=False;self.fds=[];self.actions=[];self.devices=None
 def wait(self,path,until,cleanup=False):
  while now()<until:
   (self.cleanup_tick if cleanup else self.tick)()
   if not cleanup:
    retired=read(self.root/'retired.json')
    assert retired is None or retired.get('worker_operational_success')is True,'host viewer worker retired with failure'
   v=read(path)
   if v is not None:assert now()<until;return v
   select.select([],[],[],min(.01,max(0,(until-now())/1e9)))
  raise TimeoutError('original guest viewer handoff cutoff')
 def identify(self):
  found={}
  for entry in Path('/sys/class/input').glob('event*'):
   base=entry/'device';name=(base/'name').read_text().strip();real=str(base.resolve())
   kind='tablet'if'virtio'in real and normalized(name)=='qemu virtio tablet'else'keyboard'if'serio'in real and normalized(name)=='at translated set 2 keyboard'else None
   if not kind:continue
   assert kind not in found
   path=Path('/dev/input')/entry.name;fd=os.open(path,os.O_RDONLY|os.O_NONBLOCK|os.O_CLOEXEC|os.O_NOFOLLOW);self.fds.append(fd);st=os.fstat(fd);assert stat.S_ISCHR(st.st_mode)and fcntl.fcntl(fd,fcntl.F_GETFL)&os.O_ACCMODE==os.O_RDONLY
   keys=ioctl_read(fd,0x21,128);has=lambda code:bool(keys[code//8]&(1<<(code%8)))
   expected_driver='virtio_input'if kind=='tablet'else'atkbd';drivers=[str((parent/'driver').resolve())for parent in [base.resolve(),*base.resolve().parents]if(parent/'driver').is_symlink()];matches=[v for v in drivers if Path(v).name==expected_driver];assert matches
   row={'kind':kind,'name':name,'event_path':str(path),'sysfs_path':real,'driver_path':matches[0],'rdev':st.st_rdev,'inode':st.st_ino,'open_access':'O_RDONLY','fd':fd}
   if kind=='tablet':
    assert has(272);row['ABS_X']=list(struct.unpack('6i',ioctl_read(fd,0x40,24)));row['ABS_Y']=list(struct.unpack('6i',ioctl_read(fd,0x41,24)));assert row['ABS_X'][1]==row['ABS_Y'][1]==0 and min(row['ABS_X'][2],row['ABS_Y'][2])>0
   else:assert has(125)and has(17)
   found[kind]=row
  assert set(found)=={'tablet','keyboard'};self.devices=found;return self.device_record()
 def device_record(self):return {k:{n:v for n,v in row.items()if n!='fd'}for k,row in self.devices.items()}
 def verify_devices(self,compositor_devices):
  assert self.devices is not None
  for row in self.devices.values():
   st=os.fstat(row['fd']);path=Path(row['event_path']);assert(st.st_rdev,st.st_ino)==(row['rdev'],row['inode'])and(path.stat().st_rdev,path.stat().st_ino)==(st.st_rdev,st.st_ino)and str((Path('/sys/class/input')/path.name/'device').resolve())==row['sysfs_path']
   assert fcntl.fcntl(row['fd'],fcntl.F_GETFL)&os.O_ACCMODE==os.O_RDONLY
  names=[]
  def visit(value):
   if isinstance(value,dict):
    if isinstance(value.get('name'),str):names.append(normalized(value['name']))
    for item in value.values():visit(item)
   elif isinstance(value,list):
    for item in value:visit(item)
  visit(compositor_devices)
  assert all(normalized(row['name'])in names for row in self.devices.values())
  return {'read_only_kernel_devices':self.device_record(),'actual_compositor_devices':compositor_devices}
 def preflight(self,until):
  publish(self.root/'mice.request.json',{'schema':1,'selection_sha256':self.sha})
  mice=self.wait(self.root/'mice.reply.json',until);assert mice['selection_sha256']==self.sha
  match=[v for v in mice['actual_query_mice']if v.get('absolute')is True and v.get('current')is True and'virtio tablet'in v.get('name','').lower()];assert len(match)==1
  connected=self.wait(self.root/'connected.json',until);assert connected['selection_sha256']==self.sha and connected['connected']is True
  return {'boot_owned_QMP_mice':mice,'RFB_connection':connected,'read_only_kernel_devices':self.device_record()}
 def perform(self,point,until):
  assert not self.failed and self.seq<7 and now()<until<=self.work;self.seq+=1;action=ACTIONS[self.seq-1];assert(point is None)==(action=='close-1')
  request={'schema':1,'selection_sha256':self.sha,'seq':self.seq,'action':action,'point':point,'guest_until_ns':until};prefix=f'{self.seq:02d}'
  try:
   publish(self.root/(prefix+'.request.json'),request);ready=self.wait(self.root/(prefix+'.ready.json'),until);assert ready['seq']==self.seq and ready['request_sha256']==digest(request)
   sampled=now();remaining=until-sampled;assert remaining>100_000_000
   publish(self.root/(prefix+'.commit.json'),{'schema':1,'selection_sha256':self.sha,'seq':self.seq,'request_sha256':digest(request),'host_ready_anchor_ns':ready['host_ready_anchor_ns'],'guest_sample_ns':sampled,'guest_until_ns':until,'remaining_ns':remaining})
   done=self.wait(self.root/(prefix+'.done.json'),until);assert done['seq']==self.seq and done['request_sha256']==digest(request);self.actions.append({'request':request,'ready':ready,'commit_guest_sample_ns':sampled,'completion_guest_ns':now(),'done':done})
   assert done['outcome']=='sent_once'and now()<until;return self.actions[-1]
  except BaseException:
   self.failed=True
   if not(self.root/'abort.json').exists():publish(self.root/'abort.json',{'schema':1,'selection_sha256':self.sha,'failed_seq':self.seq})
   raise
 def retire(self,until):
  assert until<=self.close
  if not self.retire_sent:
   publish(self.root/'retire.request.json',{'schema':1,'selection_sha256':self.sha,'completed_actions':self.seq,'action_failed':self.failed});self.retire_sent=True
  value=self.wait(self.root/'retired.json',until,cleanup=True);assert value['selection_sha256']==self.sha and value['no_future_host_writes']is True and value['remote_event_queues_drained_claim']is False
  return value
 def close_devices(self):
  for fd in self.fds:os.close(fd)
  self.fds=[]
