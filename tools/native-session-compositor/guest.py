#!/usr/bin/env python3
"""Owned actual compositor packets; facts observation only, never app authorization."""
import array,fcntl,grp,re,hashlib,json,os,platform,select,shlex,shutil,signal,socket,struct,subprocess,time,traceback
from pathlib import Path
R=Path('/results');S=R/'source';B=Path('/tmp/session-compositor-fixture')
B.mkdir();O=B/'evidence';O.mkdir();(O/'bins').mkdir()
LEASE=100_000_000;MAX_FRAME=1024
r={'passed':False,'red_detected':False,'architecture':platform.machine(),'hardware_verified':False,'trusted_compositor_profile_qualified':False,'actor_or_authorization_integration_qualified':False,'groups':[],'epochs':[],'cleanup':[],'rust_observer_executed':False,'evidence_io':'guest-local /tmp; copied to results only after all tracked child reaps','fixture_monitor_mode':'320x240@60','fixture_graphics_backend':'virtio-gpu-gl under host software GL with TCG'}
processes=[];logs=[];epochs=[]
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def record(name,**data):r['groups'].append({'id':name,'passed':True,**data});print('PASS '+name,flush=True)
def start(args,name,env=None,user=False,pass_fds=(),stdin=None,output_path=None):
 f=(output_path or O/name).open('w');logs.append(f)
 options={'user':1000,'group':1000,'extra_groups':os.getgrouplist('alarm',1000)} if user else {}
 p=subprocess.Popen(args,stdout=f,stderr=subprocess.STDOUT,env=env,start_new_session=True,pass_fds=pass_fds,stdin=stdin,**options)
 processes.append((p,name));return p

def finish(p,name,force=True):
 result={'pid':p.pid,'name':name,'initial_returncode':p.poll(),'signals':[],'reaped':False}
 if p.poll() is None and force:
  for sig,budget in [(signal.SIGCONT,.05),(signal.SIGTERM,3),(signal.SIGKILL,2)]:
   try:os.killpg(p.pid,sig);result['signals'].append(sig.name)
   except ProcessLookupError:pass
   if sig==signal.SIGCONT:continue
   deadline=now()+int(budget*1e9)
   while p.poll() is None and now()<deadline:
    for epoch in epochs:
     if epoch.pidfd>=0 and (epoch.rust or epoch.receive is not None):epoch.pump()
    time.sleep(.004)
   if p.poll() is not None:break
 try:result['returncode']=p.wait(timeout=.1);result['reaped']=True
 except subprocess.TimeoutExpired:result['returncode']=None
 r['cleanup'].append(result)
 if not result['reaped']:raise RuntimeError('owned child not reaped '+name)
 return result

def wait(fn,seconds=20,pump=None):
 end=now()+int(seconds*1e9)
 while now()<end:
  if pump:pump()
  value=fn()
  if value:return value
  time.sleep(.004)
 raise TimeoutError('bounded compositor fixture condition')

def command(args,name,env=None,user=False,pump=None,timeout=60):
 p=start(args,name,env,user)
 wait(lambda:p.poll() is not None,timeout,pump)
 finish(p,name,False)
 text=(O/name).read_text()
 if p.returncode:raise RuntimeError(repr(args)+' failed\n'+text[-2000:])
 return text

def live(fd):
 p=select.poll();p.register(fd,select.POLLIN);return not p.poll(0)

class Epoch:
 def __init__(self,label,rust=False):
  self.label=label;self.rust=rust;self.warmup=None;self.events=[];self.frames=[];self.current=None;self.closed=False;self.failure=None;self.line=0;self.sequence=0;self.current_peer_fd=None;self.focus_peer_qualified=False
  self.last_pump_ns=None;self.maximum_pump_gap_ns=0;self.output_closed_at_ns=None
  self.source=os.urandom(16).hex();receive,send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET)
  receive.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);receive.setblocking(False)
  if label=='rust-pressure':send.setsockopt(socket.SOL_SOCKET,socket.SO_SNDBUF,4096)
  self.send_buffer=send.getsockopt(socket.SOL_SOCKET,socket.SO_SNDBUF)
  self.creation=struct.unpack('3i',receive.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert self.creation==(os.getpid(),0,0)
  self.output_fd=send.fileno();st=os.fstat(self.output_fd);self.output_identity=(st.st_dev,st.st_ino)
  env=dict(baseenv,POKELINUX_SESSION_FD=str(self.output_fd),POKELINUX_SESSION_SOURCE=self.source)
  self.child=start(['/usr/bin/Hyprland','--config',str(B/'hypr.lua')],label+'-hyprland.log',env,True,(self.output_fd,))
  self.pidfd=os.pidfd_open(self.child.pid,0);send.close();self.receive=receive
  assert live(self.pidfd) and self.child.pid!=self.creation[0]
  self.meta={'label':label,'receiver_kind':'actual_Rust_Observer' if rust else 'authored_Python_packet_recorder','compositor_pid':self.child.pid,'compositor_uid':1000,'source':self.source,'socketpair_creation_credentials':{'pid':self.creation[0],'uid':0,'gid':0},'source_pidfd_dev':os.fstat(self.pidfd).st_dev,'source_pidfd_inode':os.fstat(self.pidfd).st_ino,'inherited_output_fd':self.output_fd,'inherited_output_identity':list(self.output_identity),'socket_send_buffer_bytes':self.send_buffer}
  r['epochs'].append(self.meta);epochs.append(self)
  if rust:
   self.observer_log=B/(label+'-observer.jsonl');self.meta['observer_stdout_location']='guest-local /tmp; copied only after reap'
   self.observer=start([str(B/'session-observer-fixture'),str(receive.fileno()),str(self.pidfd),str(self.child.pid),'1000',self.source],label+'-observer.jsonl',pass_fds=(receive.fileno(),self.pidfd),output_path=self.observer_log)
   receive.close();self.receive=None;r['rust_observer_executed']=True
   wait(lambda:any(e.get('event')=='ready' for e in self.events),10,self.pump)
  self.wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),120,self.pump)
  self.control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),120,self.pump)
  self.env=dict(baseenv,WAYLAND_DISPLAY=self.wayland.name,HYPRLAND_INSTANCE_SIGNATURE=self.control.parent.name)
  self.meta['running_compositor_elf_sha256']=sha(f'/proc/{self.child.pid}/exe')
  assert self.meta['running_compositor_elf_sha256']==selection['expected_hyprland_elf_sha256']
  assert self.cmd(['hyprctl','plugin','load',str(B/'probe.so')],label+'-probe-load.log').strip()=='ok'
  if rust:
   self.warmup=start(['foot','--app-id=session-compositor-warmup','sleep','180'],label+'-warmup-foot.log',self.env,True)
   wait(lambda:json.loads(self.cmd(['hyprctl','nativebuildprobe'],label+'-warmup-probe.log'))['pid']==self.warmup.pid,10,self.pump)
   ready_after=now()+750_000_000;wait(lambda:now()>=ready_after,2,self.pump)
   self.cmd(['hyprctl','nativebuildclearfocus'],label+'-warmup-clear-focus.log')
   ready_after=now()+250_000_000;wait(lambda:now()>=ready_after,1,self.pump)
   assert not self.frames and not self.failure
   self.meta['render_preparation']={'owned_warmup_window_pid':self.warmup.pid,'completed_before_first_adapter_load_at_ns':now(),'wire_frames_before_preparation':0,'source_restarted_or_revived':False}
  assert self.cmd(['hyprctl','plugin','load',str(B/'adapter.so')],label+'-adapter-load.log').strip()=='ok'
  wait(lambda:len(self.frames)>0,10,self.pump)
  assert self.frames[-1]['focus'] is None and self.frames[-1]['locked'] is False
  assert self.output_still_open()
 def cmd(self,args,name):
  assert args[0]=='hyprctl'
  values=args[1:];json_mode='-j' in values;values=[v for v in values if v!='-j']
  request=(('j/' if json_mode else '/')+' '.join(values)).encode()
  assert len(request)<=2048
  deadline=now()+15_000_000_000
  with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as ctl:
   ctl.settimeout(.01);ctl.connect(str(self.control));ctl.setblocking(False)
   assert struct.unpack('3i',ctl.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(self.child.pid,1000,1000)
   sent=0;reply=bytearray()
   while sent<len(request):
    assert now()<deadline
    self.pump()
    try:sent+=ctl.send(request[sent:])
    except BlockingIOError:time.sleep(.004)
   while now()<deadline:
    self.pump()
    try:data=ctl.recv(4096)
    except BlockingIOError:time.sleep(.004);continue
    if not data:
     text=reply.decode();(O/name).write_text(text)
     if text.startswith('error:'):raise RuntimeError('owned control request failed '+text)
     return text
    reply.extend(data);assert len(reply)<=65536
  raise TimeoutError('owned compositor control deadline')
 def accept(self,frame):
  assert set(frame)=={'source','sequence','sampled_at_ns','locked','lock_completed','focus','locker'}
  assert frame['source']==self.source and type(frame['sequence']) is int and frame['sequence']>self.sequence
  if not self.rust:assert frame['sequence']==self.sequence+1
  assert type(frame['sampled_at_ns']) is int and 0<=frame['sampled_at_ns']<=now()
  assert type(frame['locked']) is bool and type(frame['lock_completed']) is bool
  assert not frame['lock_completed'] or frame['locked']
  assert not frame['locked'] or frame['focus'] is None
  locker=frame['locker'];assert type(locker) is dict and locker.get('state') in ('unavailable','mapped')
  if locker['state']=='unavailable':assert set(locker)=={'state'}
  else:
   assert frame['locked'] and set(locker)=={'state','pid','uid','gid'}
   assert all(type(locker[k]) is int and locker[k]>=0 for k in ('pid','uid','gid')) and 0<locker['pid']<=2147483647
  if frame['focus'] is not None:
   f=frame['focus'];assert set(f)=={'pid','uid','gid'}
   assert all(type(x) is int and x>=0 for x in f.values()) and 0<f['pid']<=2147483647
  self.sequence=frame['sequence'];self.current=frame;self.frames.append(frame)
  assert len(self.frames)<=10000
 def pump(self):
  observed=now()
  if self.last_pump_ns is not None:self.maximum_pump_gap_ns=max(self.maximum_pump_gap_ns,observed-self.last_pump_ns)
  self.last_pump_ns=observed
  if self.output_closed_at_ns is None and not self.output_still_open():self.output_closed_at_ns=observed
  if self.rust:
   p=self.observer_log
   if not p.exists():return
   lines=p.read_text().splitlines(keepends=True)
   while self.line<len(lines) and lines[self.line].endswith('\n'):
    event=json.loads(lines[self.line]);self.line+=1;event['observed_at_ns']=now();self.events.append(event)
    if event['event']=='sample':
     frame=event['frame'];assert event['has_owned_focus']==(frame['focus'] is not None)
     self.focus_peer_qualified=event['focused_peer_matches_metadata'] is True and event['focused_peer_cloexec'] is True
     if frame['focus'] is not None:assert self.focus_peer_qualified,'Rust focused metadata lacks live retained CLOEXEC peer'
     assert event['has_owned_locker']==(frame['locker']['state']=='mapped')
     if event['has_owned_locker']:assert event['locker_peer_matches_metadata'] is True and event['locker_peer_cloexec'] is True
     self.accept(frame)
    elif event['event']=='failed':
     assert event['positive_state'] is False
     event['last_accepted_sampled_at_ns']=self.current['sampled_at_ns'] if self.current else None
     event['last_accepted_age_when_observed_ns']=now()-self.current['sampled_at_ns'] if self.current else None
     event['source_output_open_when_observed']=self.output_still_open()
     self.failure=event;self.closed=True;self.current=None
   return
  if self.closed or self.receive is None:return
  for _ in range(64):
   try:data,anc,flags,_=self.receive.recvmsg(MAX_FRAME,socket.CMSG_SPACE(12)+socket.CMSG_SPACE(4),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
   except BlockingIOError:return
   if not data:self.closed=True;self.current=None;self.events.append({'event':'eof','at_ns':now()});return
   credentials=[];rights=[]
   try:
    unexpected=False
    for level,kind,body in anc:
     if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
      received=array.array('i');received.frombytes(body[:len(body)-len(body)%received.itemsize]);rights.extend(received)
     elif level==socket.SOL_SOCKET and kind==socket.SCM_CREDENTIALS:credentials.append(struct.unpack('3i',body))
     else:unexpected=True
    assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC) and not unexpected
    assert credentials==[(self.child.pid,1000,1000)],credentials
    frame=json.loads(data)
    mapped_locker=frame['locker']['state']=='mapped'
    metadata=frame['focus'] if frame['focus'] is not None else (frame['locker'] if mapped_locker else None)
    assert len(rights)==(1 if metadata is not None else 0),'actual focused or mapped-locker metadata requires exactly one pidfd'
    self.focus_peer_qualified=False
    peer_qualified=False
    if rights:
     owned=rights[0];assert os.readlink(f'/proc/self/fd/{owned}')=='anon_inode:[pidfd]'
     assert fcntl.fcntl(owned,fcntl.F_GETFD)&fcntl.FD_CLOEXEC
     text=Path(f'/proc/self/fdinfo/{owned}').read_text();peer_pid=int(next(line.split()[1] for line in text.splitlines() if line.startswith('Pid:')))
     peer_qualified=live(owned) and peer_pid==metadata['pid']
     assert peer_qualified
     self.focus_peer_qualified=peer_qualified and frame['focus'] is not None
    self.events.append({'event':'packet','received_at_ns':now(),'credentials':list(credentials[0]),'bytes':len(data),'frame':frame,'retained_peer_live_matches_metadata':peer_qualified})
    self.accept(frame)
    if self.current_peer_fd is not None:os.close(self.current_peer_fd);self.current_peer_fd=None
    if rights:self.current_peer_fd=rights.pop()
   finally:
    for owned in rights:os.close(owned)
  raise RuntimeError('packet recorder bounded drain exhausted')
 def state(self):
  self.pump()
  if self.closed or not live(self.pidfd) or self.current is None:return None
  return self.current if now()<self.current['sampled_at_ns']+LEASE else None
 def output_still_open(self):
  try:st=os.stat(f'/proc/{self.child.pid}/fd/{self.output_fd}');return (st.st_dev,st.st_ino)==self.output_identity
  except FileNotFoundError:return False
 def save(self):
  self.meta.update({'last_sequence':self.sequence,'closed':self.closed,'failure':self.failure,'recorded_frames':len(self.frames),'source_live_at_save':live(self.pidfd),'output_closed_first_observed_at_ns':self.output_closed_at_ns,'maximum_parent_pump_gap_ns':self.maximum_pump_gap_ns})
  (O/(self.label+'-packets.jsonl')).write_text(''.join(json.dumps(e,separators=(',',':'))+'\n' for e in self.events))
 def stop(self):
  self.pump();self.save()
  if self.receive is not None:self.receive.close();self.receive=None
  if self.current_peer_fd is not None:os.close(self.current_peer_fd);self.current_peer_fd=None
  if self.rust:
   finish(self.observer,self.label+'-observer.jsonl');self.pump();shutil.copyfile(self.observer_log,O/(self.label+'-observer.jsonl'))
  finish(self.child,self.label+'-hyprland.log')
  if self.warmup is not None:finish(self.warmup,self.label+'-warmup-foot.log')
  os.close(self.pidfd);self.pidfd=-1
  wait(lambda:not any(p.is_socket() for p in Path('/run/user/1000').glob('wayland-*')),10)

try:
 selection=json.loads((R/'selection.json').read_text())
 for name,digest in selection['files'].items():assert sha(R/name)==digest,(name,digest)
 r['selection']=selection;r['kernel']=command(['uname','-a'],'kernel.log');r['interfaces']=sorted(p.name for p in Path('/sys/class/net').iterdir());assert r['interfaces']==['lo']
 r['hyprland_elf_sha256']=sha('/usr/bin/Hyprland');assert r['hyprland_elf_sha256']==selection['expected_hyprland_elf_sha256']
 r['packages']=command(['pacman','-Q','hyprland','gcc','binutils','pkgconf','wayland','wayland-protocols','quickshell','foot'],'packages.log')
 command(['readelf','-n','/usr/bin/Hyprland'],'hyprland-elf-notes.log');command(['nm','-D','--defined-only','--demangle','/usr/bin/Hyprland'],'hyprland-symbols.log')
 headers={str(p):sha(p) for p in sorted(Path('/usr/include/hyprland').rglob('*')) if p.is_file()};(O/'headers.json').write_text(json.dumps(headers,indent=2)+'\n')
 shutil.copyfile(S/'adapter.cpp',B/'adapter.cpp');shutil.copyfile(R/'bins/probe.so',B/'probe.so');(B/'probe.so').chmod(0o755);assert sha(B/'probe.so')==selection['expected_probe_sha256']
 cflags=shlex.split(command(['pkg-config','--cflags','hyprland'],'pkgconfig.log'))
 r['adapter_compile_command']=['g++','-std=c++23','-fPIC','-shared','-O2','-g',*cflags,str(B/'adapter.cpp'),'-o',str(B/'adapter.so')]
 command(r['adapter_compile_command'],'adapter-build.log',timeout=420);shutil.copyfile(B/'adapter.so',O/'bins/adapter.so');r['adapter_elf_sha256']=sha(B/'adapter.so');command(['readelf','-n',str(B/'adapter.so')],'adapter-elf-notes.log')
 if selection['observer_selected_path']:
  shutil.copyfile(R/'bins/session-observer-fixture',B/'session-observer-fixture');(B/'session-observer-fixture').chmod(0o755);command(['readelf','-n',str(B/'session-observer-fixture')],'observer-elf-notes.log')
 xml=Path('/usr/share/wayland-protocols/staging/ext-session-lock/ext-session-lock-v1.xml');shutil.copyfile(xml,O/'session-lock-v1.xml')
 client=(S/'lock-client-original.c').read_text().replace('lock = ext_session_lock_manager_v1_lock(manager);','fprintf(stderr,"CONNECTED pid=%d\\n",getpid()); fflush(stderr);\n    char trigger[32]; if (!fgets(trigger,sizeof(trigger),stdin)) return 4;\n    lock = ext_session_lock_manager_v1_lock(manager);')
 (B/'lock-client.c').write_text(client);shutil.copyfile(B/'lock-client.c',O/'lock-client.c')
 command(['wayland-scanner','client-header',str(xml),str(B/'session-lock-client.h')],'lock-client-header.log');command(['wayland-scanner','private-code',str(xml),str(B/'session-lock-protocol.c')],'lock-client-code.log')
 command(['gcc','-O2',str(B/'lock-client.c'),str(B/'session-lock-protocol.c'),'-lwayland-client','-o',str(B/'lock-client')],'lock-client-build.log')
 for name in ['session-lock-client.h','session-lock-protocol.c','lock-client']:shutil.copyfile(B/name,O/name)
 r['lock_client_elf_sha256']=sha(B/'lock-client');command(['readelf','-n',str(B/'lock-client')],'lock-client-elf-notes.log')
 command(['modprobe','virtio_gpu'],'modprobe.log');command(['udevadm','settle'],'udev.log');command(['usermod','-a','-G','video,input','alarm'],'groups.log');command(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'],'runtime.log')
 start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
 (B/'hypr.lua').write_text('hl.monitor({ output = "", mode = "320x240@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
 baseenv={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','PATH':'/usr/local/bin:/usr/bin'}
 record('native_adapter_exact_package_build',adapter_sha256=r['adapter_elf_sha256'])
 if selection.get('owned_kernel_test_binaries'):
  r['owned_kernel_tests']={}
  for name,expected in [('kernel-lib-tests',5),('kernel-pidfd-tests',7),('kernel-socket-tests',2),('observer-process-tests',26)]:
   shutil.copyfile(R/'bins'/name,B/name);(B/name).chmod(0o755)
   output=command([str(B/name),'--test-threads=1','--nocapture'],name+'.log',timeout=120)
   reports=re.findall(r'test result: ok[.] (\d+) passed; (\d+) failed;',output)
   assert reports and tuple(map(int,reports[-1]))==(expected,0),reports
   r['owned_kernel_tests'][name]={'passed':expected,'failed':0,'elf_sha256':sha(B/name)}
   record('actual_ARM_kernel_'+name,passed_tests=expected)

 def foot(e,label):
  p=start(['foot','--app-id=session-compositor-test','sleep','180'],label+'-foot.log',e.env,True)
  frame=wait(lambda:(f if (f:=e.state()) and f['focus'] is not None else None),20,e.pump)
  assert frame['focus']['pid']==p.pid and frame['focus']['uid']==1000 and frame['focus']['gid']==1000
  appfd=os.pidfd_open(p.pid,0);assert live(appfd) and e.focus_peer_qualified;os.close(appfd)
  return p,frame
 def locker(e,label):
  p=start([str(B/'lock-client')],label+'-locker.log',e.env,True,stdin=subprocess.PIPE)
  wait(lambda:'CONNECTED' in (O/(label+'-locker.log')).read_text(),10,e.pump)
  e.pump();trigger=now();p.stdin.write(b'lock\n');p.stdin.flush();return p,trigger
 def focus_cases(e,label):
  p,f=foot(e,label);record(label+'_actual_native_focus_owned_pidfd_matches_child',focus=f['focus'])
  e.cmd(['hyprctl','nativebuildclearfocus'],label+'-clear-focus.log');wait(lambda:(f if (f:=e.state()) and f['focus'] is None else None),10,e.pump);record(label+'_central_null_focus')
  e.cmd(['hyprctl','dispatch','hl.dsp.focus({ window = "class:session-compositor-test" })'],label+'-refocus.log');wait(lambda:(f if (f:=e.state()) and f['focus'] is not None else None),10,e.pump)
  layerdir=B/(label+'-layer');layerdir.mkdir();(layerdir/'shell.qml').write_text('import QtQuick\nimport Quickshell\nimport Quickshell.Wayland\nScope { PanelWindow { anchors { top: true; left: true; right: true } implicitHeight: 80; color: "#223344"; WlrLayershell.layer: WlrLayer.Overlay; WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive; Text { text: "Session layer fixture"; color: "white" } } }\n')
  layer=start(['quickshell','-n','-p',str(layerdir)],label+'-layer.log',e.env,True)
  wait(lambda:(f if (f:=e.state()) and f['focus'] is None else None),10,e.pump)
  active=json.loads(e.cmd(['hyprctl','-j','activewindow'],label+'-active-window.log'));probe=json.loads(e.cmd(['hyprctl','nativebuildprobe'],label+'-layer-probe.log'))
  assert active['pid']==p.pid and probe['type']==3 and probe['pid']!=p.pid
  record(label+'_exclusive_layer_fences_actual_focus_while_activewindow_stays',active_pid=p.pid,layer_pid=probe['pid'])
  finish(layer,label+'-layer.log');wait(lambda:(f if (f:=e.state()) and f['focus'] is not None else None),10,e.pump);return p
 e=Epoch('python-admission');record('actual_compositor_child_SCM_credentials_override_root_socketpair_creation',source_pid=e.child.pid,creator_pid=e.creation[0])
 app=focus_cases(e,'python');before=len(e.frames);lock,trigger=locker(e,'python-first-lock')
 wait(lambda:bool(e.frames) and e.frames[-1]['locked'],10,e.pump)
 admission=[f for f in e.frames[before:] if f['sampled_at_ns']>=trigger and f['focus'] is None]
 assert admission,'no actual admission-transition packet'
 r['first_admission_frame']=admission[0];r['admission_trigger_ns']=trigger
 if not admission[0]['locked']:
  r['red_detected']=True;r['groups'].append({'id':'first_actual_admission_packet_locked_without_focus','passed':False,'frame':admission[0]});raise AssertionError('ACTUAL_ADMISSION_RED: first null-focus admission packet claimed unlocked')
 record('first_actual_admission_packet_locked_without_focus',frame=admission[0])
 finish(lock,'python-first-lock-locker.log');time.sleep(.05);e.pump();assert e.state()['locked'] and not e.state()['lock_completed'];record('actual_locker_death_retains_immediate_denial')
 restored,_=locker(e,'python-restored-lock');wait(lambda:'LOCKED' in (O/'python-restored-lock-locker.log').read_text(),10,e.pump);wait(lambda:(f if (f:=e.state()) and f['locked'] and f['lock_completed'] else None),2,e.pump);record('actual_restore_fallback_completion_separate_from_presentation')
 os.kill(restored.pid,signal.SIGUSR1);wait(lambda:(f if (f:=e.state()) and not f['locked'] and f['focus'] is not None else None),10,e.pump);finish(restored,'python-restored-lock-locker.log');record('actual_protocol_unlock_reconciles_native_focus')
 force,_=locker(e,'python-force-lock');wait(lambda:(f if (f:=e.state()) and f['locked'] else None),10,e.pump);e.cmd(['hyprctl','nativebuildforceunlock'],'python-force-unlock.log');wait(lambda:(f if (f:=e.state()) and not f['locked'] and f['focus'] is not None else None),10,e.pump);finish(force,'python-force-lock-locker.log');record('actual_force_unlock_reconciles_native_focus')
 finish(app,'python-foot.log');e.stop()
 if selection['observer_selected_path']:
  e=Epoch('rust-live',True);app=focus_cases(e,'rust');e.cmd(['hyprctl','plugin','unload',str(B/'adapter.so')],'rust-unload.log');wait(lambda:e.failure,10,e.pump);assert live(e.pidfd) and not e.output_still_open();record('actual_Rust_Observer_adapter_unload_closes_source_and_denies',failure=e.failure);finish(app,'rust-foot.log');e.stop()
  e=Epoch('rust-stopped',True);app,f=foot(e,'rust-stopped');sample=f['sampled_at_ns'];os.kill(e.child.pid,signal.SIGSTOP);wait(lambda:e.failure,3,e.pump);assert live(e.pidfd) and now()>=sample+LEASE;record('actual_stopped_compositor_Rust_Observer_absolute_BOOTTIME_expiry',failure=e.failure,source_still_live=True,elapsed_ns=now()-sample);finish(app,'rust-stopped-foot.log');e.stop()
  e=Epoch('rust-pressure',True);app,f=foot(e,'rust-pressure');assert e.output_still_open();os.kill(e.observer.pid,signal.SIGSTOP);wait(lambda:not e.output_still_open(),3);assert live(e.pidfd);os.kill(e.observer.pid,signal.SIGCONT);wait(lambda:e.failure,3,e.pump);assert not e.output_still_open();record('actual_adapter_output_backpressure_terminal_closure_and_Rust_denial',failure=e.failure,source_still_live=True);finish(app,'rust-pressure-foot.log');e.stop()
  e=Epoch('rust-death',True);app,f=foot(e,'rust-death');finish(e.child,'rust-death-hyprland.log');wait(lambda:e.failure,3,e.pump);assert not live(e.pidfd);record('actual_compositor_death_Rust_Observer_fences_source',failure=e.failure);finish(app,'rust-death-foot.log');e.stop()
 r['passed']=True
except Exception:
 r['error']=traceback.format_exc()
 if 'ACTUAL_OWNED_FOCUS_RED' in r['error']:
  r['red_detected']=True;r['groups'].append({'id':'actual_focused_metadata_requires_owned_pidfd','passed':False})
 print(r['error'],flush=True)
finally:
 if Path('/tmp/native-session-build-probe.ndjson').exists():shutil.copyfile('/tmp/native-session-build-probe.ndjson',O/'original-probe-events.jsonl')
 for e in epochs:
  if e.pidfd>=0:
   try:e.pump();e.save()
   except Exception:pass
   if e.receive is not None:e.receive.close();e.receive=None
   if e.current_peer_fd is not None:os.close(e.current_peer_fd);e.current_peer_fd=None
   os.close(e.pidfd);e.pidfd=-1
 for p,name in reversed(processes):
  if not any(x['pid']==p.pid and x['reaped'] for x in r['cleanup']):
   try:finish(p,name)
   except Exception as error:r['passed']=False;r.setdefault('cleanup_errors',[]).append(str(error))
 for e in epochs:
  if e.rust and e.observer.poll() is not None and e.observer_log.exists():shutil.copyfile(e.observer_log,O/(e.label+'-observer.jsonl'))
 for f in logs:f.close()
 r['all_tracked_children_reaped']=all(x['reaped'] for x in r['cleanup'])
 r['artifact_hashes']={str(p.relative_to(O)):sha(p) for p in sorted(O.iterdir()) if p.is_file() and p.name not in ['result.json','serial.log','host.log']}
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n')
 shutil.copytree(O,R,dirs_exist_ok=True)
 subprocess.run(['systemctl','poweroff'],timeout=20)
