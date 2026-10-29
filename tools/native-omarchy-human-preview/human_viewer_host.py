"""One bounded human Unix viewer session; no internal client, retry or capture."""
import hashlib,json,os,select,signal,stat
from pathlib import Path
from owned import now
from viewer_protocol import read,publish

class HumanViewerOwner:
 def __init__(self,scope,root,selection,qemu,xvfb,work,close,qmp_progress,query_mice):
  self.scope=scope;self.root=Path(root);self.selection=selection;self.qemu=qemu;self.xvfb=xvfb;self.work=work;self.close=close;self.qmp_progress=qmp_progress;self.query_mice=query_mice
  self.sha=hashlib.sha256((self.root/'selection.json').read_bytes()).hexdigest();self.directory=self.root/'viewer-input';self.directory.mkdir(mode=0o700)
  self.setup=None;self.server=None;self.host_ready=False;self.retired=False;self.passed=False;self.client_observed=False;self.stop_requested=False;self.ending=None;self.dwell_until=None;self.no_client_until=None
  self.record={'schema':1,'selection_sha256':self.sha,'human_connection_workflow_qualified':False,'remote_event_queues_drained_claim':False,'internal_RFB_client_selected':False,'original_host_work_until_ns':work,'original_host_close_until_ns':close}
  assert not os.environ.get('LD_LIBRARY_PATH')and not os.environ.get('LD_PRELOAD')
  manifest=json.loads((self.root/'source/viewer-bundle.json').read_bytes());assert hashlib.sha256((self.root/'source/viewer-bundle.json').read_bytes()).hexdigest()=='2e78905a047d5df43dcae3a7c8ace9ce7827a3d567db8b43d7732a1f5abfae8c'
  base=Path('/viewer');assert{str(p.relative_to(base))for p in base.rglob('*')}=={r['path']for r in manifest['files']}
  for row in manifest['files']:
   p=base/row['path'];assert oct(stat.S_IMODE(p.lstat().st_mode))==row['mode']
   if row['kind']=='regular':assert p.stat().st_size==row['bytes']and hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256']
   elif row['kind']=='symlink':assert p.is_symlink()and os.readlink(p)==row['target']
   else:assert p.is_dir()
 def tcp(self,label):
  rows={n:Path('/proc/net/'+n).read_text()for n in['tcp','tcp6']};assert all(len(v.splitlines())==1 for v in rows.values());self.record.setdefault('no_TCP_observations',[]).append({'label':label,'host_ns':now(),'tables':rows})
 def check(self):
  self.scope.pump();self.scope.check();assert now()<self.close
  assert self.scope.live(self.qemu)and self.scope.live(self.xvfb),'human display dependency exited'
 def prepare(self,request):
  assert request=={'schema':1,'selection_sha256':self.sha}and not self.host_ready and self.setup is None and now()<self.work
  actual=self.query_mice();until=min(self.work,now()+10_000_000_000)
  argv=['/usr/bin/python3','/results/source/viewer_viewport.py','--prepare-native-once','--qemu-pid',str(self.qemu['child'].pid),'--xvfb-pid',str(self.xvfb['child'].pid),'--parent-pid',str(os.getpid()),'--until-host-ns',str(until)]
  self.setup=self.scope.start(argv,'human-window-setup.log',65536,cutoff=until,wait_cutoff=until);assert self.scope.finish(self.setup,until)==0
  value=json.loads((self.root/'human-window-setup.log').read_bytes());assert now()<until and value['owner_PID']==self.qemu['child'].pid and value['explicit_native_window_setup']['request_count']==1 and value['explicit_native_window_setup']['after_tuple']==[1080,2424,0,24,0,0]
  self.record.update(actual_query_mice=actual,explicit_native_window_setup=value)
  publish(self.directory/'host-ready.json',{'schema':1,'selection_sha256':self.sha,'actual_query_mice':actual,'setup':value,'actual_setup_wait':self.setup['wait']});self.host_ready=True
 def start_server(self,request):
  assert self.host_ready and self.server is None and request['schema']==1 and request['selection_sha256']==self.sha
  duration=request['requested_dwell_ns'];assert type(duration)is int and 0<duration<=300_000_000_000 and type(request['guest_until_ns'])is int
  received=now();assert received<self.work;self.dwell_until=min(self.work,received+duration);self.record.update(guest_ready_received_host_ns=received,guest_requested_dwell_ns=duration,guest_until_ns_recorded_local_only=request['guest_until_ns'],host_dwell_until_ns=self.dwell_until,guest_absolute_clock_compared_to_host=False)
  path=self.root/'viewer.sock';assert not path.exists()
  env=dict(os.environ,DISPLAY=':97',LD_LIBRARY_PATH='/viewer/usr/lib/x86_64-linux-gnu');env.pop('WAYLAND_DISPLAY',None);env.pop('XDG_SESSION_TYPE',None)
  argv=['/viewer/usr/bin/x11vnc','-norc','-display',':97','-noxdamage','-unixsock','/results/viewer.sock','-rfbport','0','-rfbportv6','0','-no6','-noipv6','-nolookup','-nopw','-once']
  self.server=self.scope.start(argv,'human-x11vnc.log',65536,env=env,cutoff=self.dwell_until,wait_cutoff=self.close)
  until=min(self.dwell_until,received+10_000_000_000)
  while not path.is_socket():
   self.check();self.qmp_progress();assert now()<until and self.scope.live(self.server);select.select([],[],[],min(.005,max(0,(until-now())/1e9)))
  st=path.lstat();assert stat.S_ISSOCK(st.st_mode)and st.st_uid==st.st_gid==1000
  self.no_client_until=min(self.dwell_until,now()+60_000_000_000);self.record.update(socket_UID=st.st_uid,socket_GID=st.st_gid,socket_mode=oct(stat.S_IMODE(st.st_mode)),no_host_chmod_selected=True,host_no_client_until_ns=self.no_client_until)
  self.tcp('listener-ready')
  publish(self.directory/'human-ready.json',{'schema':1,'selection_sha256':self.sha,'socket_inside_container':'/results/viewer.sock','host_socket_path':self.selection['human_preview']['host_case_dir']+'/viewer.sock','host_dwell_until_ns':self.dwell_until,'host_no_client_until_ns':self.no_client_until,'human_connection_workflow_qualified':False})
 def retire(self,reason):
  assert not self.retired;self.ending=reason
  if self.server is not None:
   if self.scope.live(self.server):
    self.stop_requested=True;self.record['owned_server_stop_requested_host_ns']=now();signal.pidfd_send_signal(self.server['pidfd'],signal.SIGTERM)
   code=self.scope.finish(self.server,self.close)
   expected=code in(0,2,-signal.SIGTERM)if self.stop_requested else code==0 and self.client_observed
   assert self.server['wait']['actual_direct_parent_wait']and self.server['wait']['stdout_EOF']and self.server['wait']['wait_before_original_cutoff']and self.server['wait']['EOF_before_original_cutoff']
  else:code=None;expected=False
  self.passed=bool(expected and self.host_ready and self.setup is not None and self.setup['wait']['returncode']==0 and self.dwell_until is not None)
  self.record.update(ending=reason,client_transport_observed=self.client_observed,owned_server_stop_requested=self.stop_requested,server_actual_exit=code,operational_success=self.passed,completed_host_ns=now())
  publish(self.directory/'retired.json',{'schema':1,'selection_sha256':self.sha,'no_future_host_writes':True,'remote_event_queues_drained_claim':False,'operational_success':self.passed,'ending':reason,'client_transport_observed':self.client_observed,'server_actual_wait':self.server['wait']if self.server is not None else None,'window_setup_actual_wait':self.setup['wait']if self.setup is not None else None,'published_host_ns':now()});self.retired=True
 def pump(self):
  if self.retired:return
  self.check();ending=read(self.directory/'retire.request.json')
  if ending is not None:
   assert ending['schema']==1 and ending['selection_sha256']==self.sha and ending['reason']=='guest-ending';self.retire('guest-ending');return
  if not self.host_ready:
   request=read(self.directory/'mice.request.json')
   if request is not None:self.prepare(request)
  if self.host_ready and self.server is None:
   request=read(self.directory/'guest-ready.json')
   if request is not None:self.start_server(request)
  if self.server is not None:
   self.scope.pump();self.scope.check();text=(self.root/'human-x11vnc.log').read_bytes();assert len(text)<=65536
   if not self.client_observed and b'incr accepted_client='in text:
    self.client_observed=True;self.record['client_transport_first_observed_host_ns']=now();self.record['client_observation_is_log_hint_not_human_input_qualification']=True;self.tcp('first-client-observed')
   if self.server['child'].poll()is not None:self.retire('client-disconnected'if self.client_observed else'server-exited-without-client-observation')
   elif now()>=self.dwell_until:self.retire('host-dwell-expired')
   elif not self.client_observed and now()>=self.no_client_until:self.retire('no-client-observed-timeout')
