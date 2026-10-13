#!/usr/bin/env python3
"""Actual original compositor to installed native Qt Messages. No simulated source."""
import array, hashlib, json, os, select, shutil, signal, socket, sqlite3, struct, subprocess, time, traceback
from pathlib import Path
R=Path('/results');B=Path('/tmp/display-cancel-admission');B.mkdir();O=B/'evidence';O.mkdir()
selection=json.loads((R/'selection.json').read_text())
r={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'secure_presentation_qualified':False,'production_PAM_lock_qualified':False,'sample_budget_ns':100000000,'source_revived':False,'ui_queue_withdrawal_qualified':False}
children=[];logs=[];owned={};receiver=None;sourcefd=None;service=None;compositor=None;frames=[];control=None;servicefd=None;active_rpc=None;held_sdk=None
APP='org.omarchy.Messages';UID=11010
runtime=Path('/run/pokelinux/messages');state=Path('/var/lib/pokelinux');install=Path('/usr/libexec/pokelinux');observe=Path('/run/joint-messages-observer')
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def pump():
 if receiver is None:return
 for _ in range(64):
  try:data,anc,flags,_=receiver.recvmsg(1024,socket.CMSG_SPACE(12)+socket.CMSG_SPACE(4),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
  except BlockingIOError:return
  if not data:return
  rights=[];creds=[]
  try:
   for level,kind,body in anc:
    if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
     a=array.array('i');a.frombytes(body[:len(body)-len(body)%a.itemsize]);rights.extend(a)
    elif level==socket.SOL_SOCKET and kind==socket.SCM_CREDENTIALS:creds.append(struct.unpack('3i',body))
    else:raise AssertionError('unexpected ancillary')
   f=json.loads(data);assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC);assert creds==[(compositor.pid,1000,1000)];assert len(rights)==int(f['focus'] is not None)
   if rights:assert os.readlink(f'/proc/self/fd/{rights[0]}')=='anon_inode:[pidfd]'
   frames.append(f)
  finally:
   for fd in rights:os.close(fd)
def wait(fn,seconds=20,optional=False):
 until=now()+int(seconds*1e9)
 while now()<until:
  pump()
  try:
   if v:=fn():return v
  except (FileNotFoundError,ConnectionRefusedError):pass
  time.sleep(.004)
 if optional:return None
 raise TimeoutError('bounded joint condition')
def start(args,name,env=None,user=False,fds=(),stdin=None):
 log=(O/name).open('w');logs.append(log);opts={'user':UID,'group':UID,'extra_groups':[]} if user==UID else {'user':1000,'group':1000,'extra_groups':os.getgrouplist('alarm',1000)} if user else {}
 p=subprocess.Popen(args,stdout=log,stderr=subprocess.STDOUT,env=env,start_new_session=True,pass_fds=fds,stdin=stdin,**opts);children.append((p,name));return p
def finish(p,name,force=True):
 entry={'role':name,'pid':p.pid,'signals':[]}
 if p.poll() is None and force:
  for sig,budget in [(signal.SIGCONT,.01),(signal.SIGTERM,3),(signal.SIGKILL,2)]:
   try:os.killpg(p.pid,sig);entry['signals'].append(sig.name)
   except ProcessLookupError:pass
   if sig==signal.SIGCONT:continue
   if wait(lambda:p.poll() is not None,budget,True):break
 p.wait(timeout=1);entry.update({'reaped':True,'returncode':p.returncode});r['cleanup'].append(entry)
def command(args,name,env=None,user=False,seconds=30):
 p=start(args,name,env,user);wait(lambda:p.poll() is not None,seconds);finish(p,name,False);text=(O/name).read_text();assert p.returncode==0,(args,text[-2000:]);return text
def ctl(request):
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
  s.settimeout(.05);s.connect(str(control));assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(compositor.pid,1000,1000);s.setblocking(False);data=(request if request.startswith('j/') else '/'+request).encode();sent=0;reply=b'';until=now()+15_000_000_000
  while sent<len(data):
   pump()
   try:sent+=s.send(data[sent:])
   except BlockingIOError:time.sleep(.004)
   assert now()<until
  while now()<until:
   pump()
   try:chunk=s.recv(65536)
   except BlockingIOError:time.sleep(.004);continue
   if not chunk:return reply.decode()
   reply+=chunk;assert len(reply)<131072
 raise TimeoutError('control')
def admin(v):
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as s:
  s.settimeout(2);s.connect(str(runtime/'admin.sock'));assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(service.pid,0,0);s.send(json.dumps(v).encode());return json.loads(s.recv(4096))
def source_events():
 p=O/'service.log'
 return [v for line in p.read_text().splitlines() if line.startswith('{') and (v:=json.loads(line)).get('event')]
def worker():
 track()
 return next(pid for pid,v in owned.items() if v['role']=='messages_read_worker' and Path(f'/proc/{pid}').exists())
def writer():
 track()
 return next(pid for pid,v in owned.items() if v['role']=='messages_writer' and Path(f'/proc/{pid}').exists())
def refocus():
 assert ctl('dispatch hl.dsp.focus({ window = '+json.dumps('address:'+window['address'])+' })').strip()=='ok'
 wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==pid,10)
def record(name,passed=True,**v):r['groups'].append({'id':name,'passed':passed,**v});print(('PASS ' if passed else 'FAIL ')+name,flush=True)
def track():
 if service is None:return
 for path in Path('/proc').iterdir():
  if not path.name.isdecimal():continue
  try:
   if int((path/'stat').read_text().split()[3])!=service.pid:continue
   pid=int(path.name)
   if pid in owned:continue
   args=(path/'cmdline').read_bytes().split(b'\0');role='display_context_worker' if b'--display-context-worker' in args else 'messages_read_worker' if b'--messages-read-worker' in args else 'messages_writer' if b'--messages-worker' in args else 'policy_storage_worker' if b'--storage-worker' in args else 'installed_Qt_app'
   owned[pid]={'fd':os.pidfd_open(pid,os.O_NONBLOCK),'role':role}
  except FileNotFoundError:pass
sequence=0
def snapshot():
 global sequence
 sequence+=1;temp=observe/'control.tmp';temp.write_text(json.dumps({'sequence':sequence}));os.chown(temp,UID,UID);temp.replace(observe/'control.json');f=observe/f'window-{sequence}.json';v=wait(lambda:json.loads(f.read_text()) if f.exists() else None,8);r['last_ui']=v;return v
def ui_denied():
 v=snapshot();return v if v['accessDenied'] and v['bodyCount']==0 and v['sourceBodyCount']==0 else None
def ui_read():
 v=snapshot();return v if v['literalFixture'] and v['plain'] and v['sourceBodyCount']>0 else None
class ProductionComplete(Exception):pass
def capture(name):
 path=O/(name+'.png');capture_dir=Path('/run/user/1000/joint-captures');capture_dir.mkdir(exist_ok=True);capture_dir.chmod(0o700);os.chown(capture_dir,1000,1000);temporary=capture_dir/path.name;command(['grim',str(temporary)],name+'-capture.log',env,True);assert temporary.is_file() and not temporary.is_symlink();shutil.copyfile(temporary,path);assert path.stat().st_size>1024;return {'file':path.name,'sha256':sha(path)}
def independent_registry(path,name,forged=None,uid=UID):
 args=['python3',str(R/'source/registry-probe.py'),str(path),'--bind-allowed']
 if forged:args += ['--forged-name',str(forged['name']),'--forged-interface',forged['interface']]
 return json.loads(command(args,name,user=True if uid==1000 else uid))
def direct_path_as_app(path):
 script='import json,socket; s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)\ntry:\n s.connect('+repr(str(path))+')\nexcept OSError as e:\n print(json.dumps({"connected":False,"errno":e.errno}))\nelse:\n print(json.dumps({"connected":True}))\n'
 return json.loads(command(['python3','-c',script],'direct-original-app-connect.log',user=UID))
def sdk_worker():
 track()
 return next(pid_ for pid_,v in owned.items() if v['role']=='display_context_worker' and Path(f'/proc/{pid_}').exists())
def policy_worker():
 track()
 return next(pid_ for pid_,v in owned.items() if v['role']=='policy_storage_worker' and Path(f'/proc/{pid_}').exists())
def protected_realm_checks():
 original=independent_registry(wayland,'trusted-original-registry-for-global-IDs.log',uid=1000)
 globals_={v['interface']:v for v in original['globals']}
 direct=direct_path_as_app(wayland);assert direct.get('errno')==13 and not direct['connected'],direct
 record('normal_actor_owned_app_has_no_original_runtime_or_socket_access',actual=direct)
 filtered=independent_registry(app_listener,'same-app-UID-filtered-registry.log')
 forbidden={'ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1','zwp_virtual_keyboard_manager_v1','zwlr_layer_shell_v1'}
 assert not forbidden&{v['interface'] for v in filtered['globals']}
 record('actual_actor_app_realm_admits_normal_globals_and_hides_privileged_globals',registry=filtered)
 for interface in ['ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1']:
  denied=independent_registry(app_listener,'filtered-bind-'+interface+'.log',globals_[interface]);assert denied['forged_bind_error'] is not None and not denied['forged_bind_completed'],denied
  record('actual_actor_app_realm_refuses_original_privileged_global_ID_'+interface,error=denied['forged_bind_error'],actual_global=globals_[interface])
def live_pidfd(pid_):
 return Path(f'/proc/{pid_}').exists() and not select.select([owned[pid_]['fd']],[],[],0)[0]
def current_service_health(expected_writer,expected_sdk):
 snapshot_=admin({'op':'snapshot'});display=next((v for v in reversed(source_events()) if v['event']=='display'),None)
 assert service.poll() is None and not select.select([servicefd],[],[],0)[0] and not any(v['event']=='source_failed' for v in source_events())
 assert snapshot_['ready'] and not snapshot_['locked']
 assert display and display['state']=='Ready' and display['admission'] and display['occupied'] and display['metadata_scope']=='service_epoch'
 assert live_pidfd(expected_writer) and live_pidfd(expected_sdk)
 assert writer()==expected_writer and sdk_worker()==expected_sdk
 return {'actor_snapshot':snapshot_,'display':display,'writer_pid':expected_writer,'sdk_pid':expected_sdk,'service_pidfd_fdinfo':Path(f'/proc/self/fdinfo/{servicefd}').read_text(),'SDK_pidfd_fdinfo':Path(f'/proc/self/fdinfo/{owned[expected_sdk]["fd"]}').read_text(),'writer_pidfd_fdinfo':Path(f'/proc/self/fdinfo/{owned[expected_writer]["fd"]}').read_text(),'source_current_probe':json.loads(ctl('nativebuildprobe')),'observed_at_ns':now()}
def root_history():
 result={}
 for name,query in [('raw','SELECT count(*) FROM receipts'),('inbox','SELECT count(*) FROM records')]:
  path=state/name/('receipts.sqlite3' if name=='raw' else 'inbox.sqlite3')
  connection=sqlite3.connect('file:'+str(path)+'?mode=ro',uri=True,timeout=0)
  try:
   connection.execute('PRAGMA query_only=ON');result[name]={'rows':connection.execute(query).fetchone()[0],'uid':os.stat(path).st_uid,'mode':oct(os.stat(path).st_mode&0o777)}
  finally:connection.close()
 return result
def initializing_sdk():
 latest=next((v for v in reversed(source_events()) if v['event'] in ['display','display_spawned']),None)
 assert latest and latest['state']=='Initializing' and latest['occupied'] and not latest.get('admission',False),latest
 assert latest['worker_pid']==held_sdk and live_pidfd(held_sdk)
 assert not any(v['event']=='display' and v['state']=='Ready' for v in source_events()),'SDK Ready won the hold race: unselected'
 assert Path(f'/proc/{held_sdk}/stat').read_text().split()[2] in ['T','t']
 assert service.poll() is None and not select.select([servicefd],[],[],0)[0]
 assert not any(v['event']=='source_failed' for v in source_events())
 return latest
def fresh_actual_source_focus(window_,previous_revision):
 invoked=now();assert ctl('dispatch hl.dsp.focus({ window = '+json.dumps('address:'+window_['address'])+' })').strip()=='ok'
 event=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and v['revision']>previous_revision and (v.get('focus') or {}).get('pid')==window_['pid']),None),1)
 assert event['sampled_at_ns']>=invoked and event['until']==event['sampled_at_ns']+100000000
 assert event['focus']=={'pid':window_['pid'],'uid':1000,'gid':1000}
 initializing_sdk();return event
def root_rpc(request,name,budget_ns=1_000_000_000):
 global active_rpc
 connection=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);connection.settimeout(.05);connection.connect(str(runtime/'admin.sock'))
 assert struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(service.pid,0,0)
 connection.setblocking(False);active_rpc=connection;started=now();until=started+budget_ns;packet=json.dumps(request).encode();sent=False
 while now()<until:
  if not sent:
   try:assert connection.send(packet)==len(packet);sent=True
   except BlockingIOError:pass
  try:data=connection.recv(4096)
  except BlockingIOError:time.sleep(.0005);continue
  assert data;reply=json.loads(data);ended=now();connection.close();active_rpc=None
  value={'request':request,'reply':reply,'start_ns':started,'reply_ns':ended,'elapsed_ns':ended-started,'nonblocking_socket':True,'caller_pid':os.getpid(),'caller_uid':os.getuid(),'server_pid':service.pid}
  (O/name).write_text(json.dumps(value,indent=2)+'\n');return value
 raise TimeoutError('owned nonblocking Root RPC original budget')
def initialization_checks():
 initializing_sdk();assert admin({'op':'snapshot'})['ready'];assert admin({'op':'snapshot'})['principals']==0
 began=now();assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 first=wait(lambda:next((v for v in source_events() if v['event']=='session'),None),1)
 assert first['sampled_at_ns']>=began;initializing_sdk()
 second=fresh_actual_source_focus(warm1_window,first['revision'])
 third=fresh_actual_source_focus(warm2_window,second['revision'])
 wait(lambda:now()>=third['sampled_at_ns']+120000000,.5)
 initializing_sdk();assert admin({'op':'snapshot'})['ready']
 record('actual_current_original_Source_updates_and_ready_policy_while_SDK_Initializing',first=first,second=second,third=third,display=initializing_sdk(),actor_snapshot=admin({'op':'snapshot'}))
 connection=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);connection.settimeout(.05);connection.connect(str(runtime/'admin.sock'));assert struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(service.pid,0,0);connection.setblocking(False)
 request={'op':'launch','app':APP};packet=json.dumps(request).encode();started=now();until=started+1000000000
 while True:
  try:assert connection.send(packet)==len(packet);break
  except BlockingIOError:
   assert now()<until;time.sleep(.0005)
 sent=now();connection.shutdown(socket.SHUT_RDWR);connection.close();closed=now()
 launched={'request':request,'request_sent_ns':sent,'caller_shutdown_close_ns':closed,'caller_pid':os.getpid(),'caller_uid':os.getuid(),'reply_read':False,'nonblocking_socket':True,'server_pid':service.pid}
 (O/'Root-canceled-launch-before-SDK-Ready.json').write_text(json.dumps(launched,indent=2)+'\n')
 snap=root_rpc({'op':'snapshot'},'Root-Snapshot-after-canceled-launch.json')
 assert snap['reply']['ready'] and snap['reply']['principals']==1 and snap['elapsed_ns']<250000000
 track();app_pid=next(pid_ for pid_,v in owned.items() if v['role']=='installed_Qt_app' and live_pidfd(pid_))
 status=wait(lambda:Path(f'/proc/{app_pid}/status').read_text() if Path(f'/proc/{app_pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010' else None,1)
 assert os.readlink(f'/proc/{app_pid}/exe')==str(install/'pokelinux-messages')
 assert sha(f'/proc/{app_pid}/exe')==selection['normal_qt_elf_sha256']
 assert int(status.split('PPid:\t')[1].split()[0])==service.pid
 display=initializing_sdk();r['actual_premature_launch']={'rpc':launched,'snapshot':snap,'display':display,'app_pid':app_pid,'app_uid':11010,'app_pidfd_fdinfo':Path(f'/proc/self/fdinfo/{owned[app_pid]["fd"]}').read_text(),'app_cmdline':Path(f'/proc/{app_pid}/cmdline').read_bytes().split(b'\0')[:-1]}
 r['actual_premature_launch']['app_cmdline']=[v.decode() for v in r['actual_premature_launch']['app_cmdline']]
 record('future_canceled_pending_app_context_launch_never_registers_an_app',passed=False,actual=r['actual_premature_launch'],desired_no_registered_app_after_pending_client_EOF=True)
 r['desired_boundary_red']=True;r['red_detected']=True;r['passed']=False;r['prerequisites_qualified']=True;r['setup_complete']=True;r['source_failed_before_desired_boundary']=False
 record('actual_Snapshot_remains_responsive_while_initializing_SDK_held',rpc=snap)
 stop=root_rpc({'op':'stop'},'Root-Stop-with-initializing-SDK-held.json');assert stop['reply']=={'result':'ok'} and stop['elapsed_ns']<250000000
 r['whole_Stop_ACK']=stop;record('actual_Stop_remains_responsive_while_initializing_SDK_held',rpc=stop)
 wait(lambda:service.poll() is not None,3)
 assert bool(select.select([owned[held_sdk]['fd']],[],[],0)[0]) and not Path(f'/proc/{held_sdk}').exists()
 assert bool(select.select([owned[app_pid]['fd']],[],[],0)[0]) and not Path(f'/proc/{app_pid}').exists()
 record('whole_Stop_cleans_actual_held_SDK_and_registered_app',sdk_pid=held_sdk,app_pid=app_pid,service_returncode=service.returncode,evidence_kind='actual direct service wait plus helper pidfd death/proc absence; helper parent waits not independently syscall traced')
 r['boundary_scope']='New per-app pending-launch cancellation requirement: after genuine request then client shutdown/EOF, old synchronous service-epoch actor retains real Qt principal while SDK remains Initializing. No retroactive old-completed-launch rollback, mapped Qt or local-history claim.'
try:
 assert os.getuid()==0 and os.uname().machine=='aarch64';assert os.uname().release=='7.1.6-1-aarch64-ARCH';assert sorted(p.name for p in Path('/sys/class/net').iterdir())==['lo']
 for n,d in selection['files'].items():assert sha(R/n)==d,n
 assert sha('/usr/bin/Hyprland')==selection['expected_hyprland_elf_sha256'];r['kernel']={k:getattr(os.uname(),k) for k in ['release','machine']};r['hyprland_sha256']=sha('/usr/bin/Hyprland')
 r['packages']=command(['pacman','-Q','hyprland','quickshell','qt6-base','qt6-declarative','qt6-wayland'],'packages.log')
 # Pin a single shared procfs and PID/user namespace for this isolated fixture.
 r['namespaces']={k:os.readlink('/proc/self/ns/'+k) for k in ['pid','user','mnt']};r['procfs_mount']=next(line for line in Path('/proc/self/mountinfo').read_text().splitlines() if ' /proc ' in line and ' - proc ' in line)
 command(['modprobe','virtio_gpu'],'modprobe.log');command(['udevadm','settle'],'udev.log');command(['usermod','-a','-G','video,input','alarm'],'groups.log');command(['useradd','-u',str(UID),'-M','-s','/usr/bin/nologin','messages-app'],'app-user.log')
 command(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'],'runtime.log');start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
 for d,mode in [(runtime,0o755),(state,0o700),(state/'policy',0o700),(install,0o755),(observe,0o700),(Path('/run/user/11010'),0o700),(Path('/var/lib/pokelinux-messages-app'),0o700)]:d.mkdir(parents=True,exist_ok=True);d.chmod(mode)
 for d in [observe,Path('/run/user/11010'),Path('/var/lib/pokelinux-messages-app')]:os.chown(d,UID,UID)
 for n in ['messages-service','lifecycle-launcher','pokelinux-messages-observer','pokelinux-messages']:
  shutil.copyfile(R/'bins'/n,install/n);(install/n).chmod(0o755)
 for n in ['adapter.so','probe.so','lock-client']:shutil.copyfile(R/'bins'/n,B/n);(B/n).chmod(0o755)
 appearance=Path('/usr/share/pokelinux/appearance');appearance.mkdir(parents=True)
 for n in ['PhoneTheme.qml','ThemePalette.js']:shutil.copyfile(R/'source/native-app'/n,appearance/n)
 (B/'hypr.lua').write_text('hl.permission("/usr/bin/grim", "screencopy", "allow")\nhl.monitor({ output = "", mode = "320x240@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
 receive,send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET);receive.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);receive.setblocking(False);source=os.urandom(16).hex()
 env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','PATH':'/usr/local/bin:/usr/bin','POKELINUX_SESSION_FD':str(send.fileno()),'POKELINUX_SESSION_SOURCE':source}
 compositor=start(['/usr/bin/Hyprland','--config',str(B/'hypr.lua')],'hyprland.log',env,True,(send.fileno(),));sourcefd=os.pidfd_open(compositor.pid,os.O_NONBLOCK);send.close()
 wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),120);control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),120);env.update(WAYLAND_DISPLAY=wayland.name,HYPRLAND_INSTANCE_SIGNATURE=control.parent.name);r['source']={'pid':compositor.pid,'uid':1000,'gid':1000,'incarnation':source,'owned_launcher_pidfd_retained':True,'pidfd_fileno':sourcefd,'pidfd_fdinfo':Path(f'/proc/self/fdinfo/{sourcefd}').read_text()}
 assert ctl('plugin load '+str(B/'probe.so')).strip()=='ok'
 # Finish cold graphics before the first adapter load; no source restart or deadline reset.
 layerdir=B/'layer';layerdir.mkdir()
 (layerdir/'shell.qml').write_text('''import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Io
Scope {
 PanelWindow { id: layer; anchors { top: true; left: true; right: true } implicitHeight: 80; color: "#223344"; WlrLayershell.layer: WlrLayer.Overlay; WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive; Text { text: "Joint layer fixture"; color: "white" } }
 IpcHandler { target: "joint-layer"; function show(): void { layer.visible = true } function hide(): void { layer.visible = false } }
}
''')
 layer=start(['quickshell','-n','-p',str(layerdir)],'layer.log',env,True)
 wait(lambda:json.loads(ctl('nativebuildprobe'))['type']==3,30)
 finish(layer,'layer.log')

 warm=start(['foot','--app-id=joint-render-warmup','sleep','180'],'warmup.log',env,True)
 wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==warm.pid,20);time.sleep(1)
 warm1_window=next(v for v in json.loads(ctl('j/clients')) if v.get('pid')==warm.pid and v.get('mapped'))
 warm2=start(['foot','--app-id=init-second-render-warmup','sleep','180'],'warmup-second.log',env,True);wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==warm2.pid,20);time.sleep(1);warm2_window=next(v for v in json.loads(ctl('j/clients')) if v.get('pid')==warm2.pid and v.get('mapped'))
 if selection['test_case']=='direct-realm-composition-red':
  command(['setfacl','-m','u:11010:--x','/run/user/1000'],'runtime-acl.log');command(['setfacl','-m','u:11010:rw-',str(wayland)],'wayland-acl.log')
 else:
  listener_parent=Path('/run/pokelinux-display');listener_parent.mkdir();listener_parent.chmod(0o750);os.chown(listener_parent,0,UID);app_listener=listener_parent/'messages-wayland.sock'
 profile=B/'profile.json';profile.write_text(json.dumps({'apps':[{'app':APP,'uid':UID,'executable':str(install/('pokelinux-messages' if selection.get('normal_production_qt') else 'pokelinux-messages-observer')),'args':['/var/lib/pokelinux-messages-app','/run/user/11010',str(wayland if selection['test_case']=='direct-realm-composition-red' else app_listener),'/home/alarm',str(runtime/'app.sock'),str(appearance),* ([] if selection.get('normal_production_qt') else ['--isolated-observe',str(observe)])]}]}))
 command([str(R/'bins/messages-seed-fixture'),str(state/'raw'),'--renderer-text'],'seed.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'))
 args=[str(install/'messages-service'),str(runtime),str(state/'policy'),str(profile),str(install/'lifecycle-launcher'),str(state/'raw'),str(state/'inbox')]
 if selection['phase']=='green':
  fixture_name=selection['fixture_binary'];shutil.copyfile(R/'bins'/fixture_name,install/fixture_name);(install/fixture_name).chmod(0o755);args[0]=str(install/fixture_name);args += [str(receive.fileno()),str(sourcefd),str(compositor.pid),'1000',source]
  if selection['test_case']!='direct-realm-composition-red':args += [str(wayland),str(app_listener)]
  fds=(receive.fileno(),sourcefd)
 else:receiver=receive;fds=()
 service=start(args,'service.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),fds=fds);servicefd=os.pidfd_open(service.pid,os.O_NONBLOCK)
 spawned_until=now()+1000000000
 while True:
  spawned=next((v for v in source_events() if v['event']=='display_spawned'),None)
  if spawned:break
  assert now()<spawned_until,'No actual display_spawned observed'
  time.sleep(.00025)
 held_sdk=spawned['worker_pid'];sdkfd=os.pidfd_open(held_sdk,os.O_NONBLOCK);owned[held_sdk]={'fd':sdkfd,'role':'display_context_worker'}
 signal.pidfd_send_signal(sdkfd,signal.SIGSTOP);wait(lambda:Path(f'/proc/{held_sdk}/stat').read_text().split()[2] in ['T','t'],.5);initializing_sdk()
 if selection['phase']=='green':receive.close()
 wait(lambda:admin({'op':'snapshot'}).get('ready'))
 def read_available():
  track();status=admin({'op':'messages_status'});r['last_messages_status']=status;return status.get('available')
 # The new shared core requires this exact owned reader selector. Observe the
 # selected old executable on the actual ARM kernel before correcting dispatch.
 fixture_source=R/'source/rust-build/tools/native-session-installed-app/fixture/src/bin/service.rs'
 core_source=R/'source/rust-build/native-messages/src/worker.rs'
 if selection['test_case']=='direct-realm-composition-red' and fixture_source.exists() and b'--messages-read-worker' in core_source.read_bytes() and b'--messages-read-worker' not in fixture_source.read_bytes():
  probe_child=start([str(install/'messages-session-installed-fixture'),'--messages-read-worker',str(runtime/'messages-read.sock'),str(state/'raw'),str(state/'inbox'),str(service.pid),os.urandom(16).hex()],'missing-reader-selector.log',{})
  wait(lambda:probe_child.poll() is not None,3);finish(probe_child,'missing-reader-selector.log',False)
  assert probe_child.returncode==1 and 'PermissionDenied' in (O/'missing-reader-selector.log').read_text()
  assert wait(read_available,2,True) is None
  owner=writer();assert Path(f'/proc/{owner}').exists();assert admin({'op':'snapshot'})['ready']
  r['expected_red']=True;r['red_detected']=True;r['signature']='current_core_old_fixture_rejects_readonly_bootstrap_selector'
  r['reader_selector_probe']={'selected_fixture_sha256':sha(install/'messages-session-installed-fixture'),'returncode':probe_child.returncode,'observed_error':'PermissionDenied','live_writer_pid':owner,'lifecycle_ready':True,'readonly_available':False}
  raise AssertionError('desired new-core readonly bootstrap must become available under the selected fixture')
 wait(read_available)

 initialization_checks();raise ProductionComplete()
except ProductionComplete:pass
except BaseException as e:r['error']=str(e);r['traceback']=traceback.format_exc();print(r['traceback'],flush=True)
finally:
 if active_rpc is not None:active_rpc.close()
 if service is not None and service.poll() is None:
  try:
   if not r.get('whole_Stop_ACK'):admin({'op':'stop'})
   wait(lambda:service.poll() is not None,5)
  except BaseException as e:r.setdefault('cleanup_errors',[]).append(str(e))
 for p,name in reversed(children):
  if any(v['pid']==p.pid and v['reaped'] for v in r['cleanup']):continue
  try:finish(p,name)
  except BaseException as e:r.setdefault('cleanup_errors',[]).append(str(e))
 for pid,v in owned.items():
  dead=bool(select.select([v['fd']],[],[],0)[0]);absent=not Path(f'/proc/{pid}').exists();r['cleanup'].append({'role':v['role'],'pid':pid,'owned_pidfd_death_observed':dead,'proc_absent':absent,'reaped':dead and absent,'evidence_kind':'pidfd_death_and_proc_absence; app parent reap inferred from trusted compiled Actor and live-parent kernel absence'});os.close(v['fd'])
 if receiver is not None:receiver.close()
 if sourcefd is not None:os.close(sourcefd)
 if servicefd is not None:os.close(servicefd)
 for f in logs:f.close()
 r['combined_messages_children_reaped']={role:any(v['role']==role and v['reaped'] for v in r['cleanup']) for role in ['messages_writer','messages_read_worker']}
 r['source_events']=source_events() if (O/'service.log').exists() else [];r['baseline_frames']=frames;r['all_tracked_children_finally_absent_or_directly_waited']=not r.get('cleanup_errors') and all(v['reaped'] for v in r['cleanup']);r['passed']=r['passed'] and r['all_tracked_children_finally_absent_or_directly_waited']
 for p in observe.glob('*'):
  if p.suffix in ['.json','.png']:shutil.copyfile(p,O/p.name)
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
