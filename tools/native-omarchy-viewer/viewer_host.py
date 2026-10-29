"""Boot-owned fixture children; only boot's retained QMP callback queries mice."""
import hashlib,json,os,select,signal,stat,time
from pathlib import Path
from owned import now
from viewer_protocol import read,publish

class ViewerOwner:
 def __init__(self,scope,root,selection,qemu,xvfb,work,close,qmp_progress,query_mice):
  self.scope=scope;self.root=Path(root);self.selection=selection;self.sha=hashlib.sha256((self.root/'selection.json').read_bytes()).hexdigest();self.qemu=qemu;self.xvfb=xvfb;self.work=work;self.close=close;self.qmp_progress=qmp_progress;self.query_mice=query_mice;self.retired=False;self.passed=False;self.mice_done=False;self.server=None;self.worker=None;self.setup=None
  self.directory=self.root/'viewer-input';self.directory.mkdir(mode=0o700);self.record={'schema':1,'selection_sha256':self.sha,'remote_event_queues_drained_claim':False}
  manifest=json.loads((self.root/'source/viewer-bundle.json').read_bytes());assert hashlib.sha256((self.root/'source/viewer-bundle.json').read_bytes()).hexdigest()=='2e78905a047d5df43dcae3a7c8ace9ce7827a3d567db8b43d7732a1f5abfae8c'
  assert not os.environ.get('LD_LIBRARY_PATH')and not os.environ.get('LD_PRELOAD')
  assert {str(p.relative_to('/viewer'))for p in Path('/viewer').rglob('*')}=={r['path']for r in manifest['files']}
  for row in manifest['files']:
   p=Path('/viewer')/row['path'];assert oct(stat.S_IMODE(p.lstat().st_mode))==row['mode']
   if row['kind']=='regular':assert p.stat().st_size==row['bytes']and hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256']
   elif row['kind']=='symlink':assert p.is_symlink()and os.readlink(p)==row['target']
   else:assert p.is_dir()
  env=dict(os.environ,DISPLAY=':97',LD_LIBRARY_PATH='/viewer/usr/lib/x86_64-linux-gnu');env.pop('WAYLAND_DISPLAY',None);env.pop('XDG_SESSION_TYPE',None)
  args=['/viewer/usr/bin/x11vnc','-norc','-display',':97','-noxdamage','-unixsock','/results/viewer.sock','-rfbport','0','-rfbportv6','0','-no6','-noipv6','-nolookup','-nopw','-once']
  self.server=scope.start(args,'viewer-x11vnc.log',65536,env=env,cutoff=work,wait_cutoff=close);until=min(work,now()+10_000_000_000)
  while not(self.root/'viewer.sock').is_socket():
   scope.pump();scope.check();qmp_progress();assert now()<until and all(scope.live(v)for v in [qemu,xvfb,self.server]);select.select([],[],[],.005)
  p=self.root/'viewer.sock';st=p.lstat();assert stat.S_ISSOCK(st.st_mode)and st.st_uid==st.st_gid==1000;os.chmod(p,0o600)
  args=['/usr/bin/python3','/results/source/viewer_worker.py','--qemu-pid',str(qemu['child'].pid),'--xvfb-pid',str(xvfb['child'].pid),'--server-pid',str(self.server['child'].pid),'--parent-pid',str(os.getpid()),'--work-until',str(work),'--close-until',str(close),'--selection-sha',self.sha]
  self.worker=scope.start(args,'viewer-worker.log',262144,env=dict(os.environ,DISPLAY=':97'),cutoff=work,wait_cutoff=close)
 def pump(self):
  if not self.mice_done and not self.retired:
   request=read(self.directory/'mice.request.json')
   if request is not None and self.scope.live(self.worker):
    assert request=={'schema':1,'selection_sha256':self.sha};self.mice_done=True;actual=self.query_mice();self.record['actual_query_mice']=actual
    until=min(self.work,now()+10_000_000_000)
    argv=['/usr/bin/python3','/results/source/viewer_viewport.py','--prepare-native-once','--qemu-pid',str(self.qemu['child'].pid),'--xvfb-pid',str(self.xvfb['child'].pid),'--parent-pid',str(os.getpid()),'--until-host-ns',str(until)]
    self.setup=self.scope.start(argv,'viewer-window-setup.log',65536,cutoff=until,wait_cutoff=until);assert self.scope.finish(self.setup,until)==0
    setup=json.loads((self.root/'viewer-window-setup.log').read_bytes());assert now()<until and setup['owner_PID']==self.qemu['child'].pid and setup['explicit_native_window_setup']['request_count']==1 and setup['explicit_native_window_setup']['after_tuple']==[1080,2424,0,24,0,0]
    self.record['explicit_native_window_setup']=setup
    publish(self.directory/'window-setup.json',{'schema':1,'selection_sha256':self.sha,'setup':setup,'actual_boot_parent_wait':self.setup['wait']})
    publish(self.directory/'mice.reply.json',{'schema':1,'selection_sha256':self.sha,'actual_query_mice':actual,'query_owned_by_boot_QMP':True,'observed_host_ns':now()})
  if self.worker is not None and self.worker['child'].poll()is not None and not self.retired:
   worker_code=self.scope.finish(self.worker,self.close)
   if worker_code!=0 and self.scope.live(self.server):
    self.record['failed_worker_server_disposal_signal']='SIGTERM';signal.pidfd_send_signal(self.server['pidfd'],signal.SIGTERM)
   server_code=self.scope.finish(self.server,self.close);path=self.root/'viewer-worker-result.json';assert path.is_file()and path.stat().st_size<=262144;worker_result=json.loads(path.read_bytes())
   tables={n:Path('/proc/net/'+n).read_text()for n in ['tcp','tcp6']};no_tcp=all(len(v.splitlines())==1 for v in tables.values())
   self.passed=bool(worker_code==server_code==0 and worker_result['passed']and self.mice_done and self.setup is not None and self.setup['wait']['returncode']==0 and no_tcp);self.record.update(worker_actual_exit=worker_code,server_actual_exit=server_code,worker_result_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),worker_result=worker_result,post_retirement_TCP=tables,operational_success=self.passed)
   value={'schema':1,'selection_sha256':self.sha,'no_future_host_writes':True,'remote_event_queues_drained_claim':False,'worker_operational_success':self.passed,'worker_actual_wait':self.worker['wait'],'server_actual_wait':self.server['wait'],'window_setup_actual_wait':self.setup['wait']if self.setup is not None else None,'published_host_ns':now()};publish(self.directory/'retired.json',value);self.retired=True
