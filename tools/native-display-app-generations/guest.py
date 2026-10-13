#!/usr/bin/env python3
"""Actual original compositor to installed native Qt Messages. No simulated source."""
import array, hashlib, json, os, select, shutil, signal, socket, sqlite3, struct, subprocess, time, traceback
from pathlib import Path
R=Path('/results');B=Path('/tmp/display-app-generation');B.mkdir();O=B/'evidence';O.mkdir()
selection=json.loads((R/'selection.json').read_text())
r={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'secure_presentation_qualified':False,'production_PAM_lock_qualified':False,'sample_budget_ns':100000000,'source_revived':False,'ui_queue_withdrawal_qualified':False}
children=[];logs=[];owned={};receiver=None;sourcefd=None;service=None;compositor=None;frames=[];control=None;servicefd=None
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
def app_generation_checks():
 unknown=capture('normal-unknown-source-denied');assert admin({'op':'snapshot'})['locked']
 record('normal_Qt_unknown_source_denied_on_service_epoch_realm',screen=unknown)
 ctx=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='display' and v['state']=='Ready'),None),5)
 assert ctx['metadata_scope']=='service_epoch' and ctx['admission'] and ctx['occupied']
 direct=direct_path_as_app(wayland);assert direct=={'connected':False,'errno':13},direct
 filtered=independent_registry(app_listener,'normal-app-realm-allowed-registry.log')
 assert filtered['uid']==filtered['gid']==UID and filtered['allowed_bound']==['wl_compositor','xdg_wm_base']
 forbidden={'ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1','zwp_virtual_keyboard_manager_v1','zwlr_layer_shell_v1'}
 assert not forbidden&{v['interface'] for v in filtered['globals']}
 record('actual_filtered_normal_app_realm_hides_privileged_globals_and_denies_original_path',registry=filtered,direct_original=direct)
 time.sleep(1);r['render_preparation_completed_before_first_adapter_load_ns']=now();assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 if not wait(lambda:not admin({'op':'snapshot'})['locked'],3,True):
  r['availability_unqualified']=True;raise AssertionError('Original100ms Source availability gate prevents prerequisite qualification')
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3)
 assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID}
 owner=writer();sdk=sdk_worker();health=current_service_health(owner,sdk);assert health['actor_snapshot']['principals']==1 and health['source_current_probe']['pid']==pid
 time.sleep(1);granted=capture('normal-current-focus-grant-conversation');health=current_service_health(owner,sdk)
 assert granted['sha256']!=unknown['sha256'];r['actual_focus_and_grant_admission_qualified']=True;r['normal_conversation_requires_visual_review']=True
 record('actual_current_focus_independent_grant_and_normal_conversation',source=fact,health=health,screen=granted)
 before=root_history();assert before['raw']['rows']>0 and before['inbox']['rows']>0
 r['retained_app_identity']={'pid':pid,'pidfd_fileno':owned[pid]['fd'],'pidfd_fdinfo':Path(f'/proc/self/fdinfo/{owned[pid]["fd"]}').read_text(),'owned_by_actor_service_pid':service.pid,'uid':UID,'gid':UID,'window_address':window['address'],'original_parent_pid':int(Path(f'/proc/{pid}/status').read_text().split('PPid:\t')[1].split()[0])};assert r['retained_app_identity']['original_parent_pid']==service.pid
 close_invoked=now();assert ctl('dispatch hl.dsp.window.close({ window = '+json.dumps('address:'+window['address'])+' })').strip()=='ok'
 wait(lambda:bool(select.select([owned[pid]['fd']],[],[],0)[0]) and not Path(f'/proc/{pid}').exists(),3)
 wait(lambda:admin({'op':'snapshot'})['principals']==0,3);app_absent=now()
 parent_reap={'app_pid':pid,'original_parent_pid':service.pid,'actor_principals':0,'app_pidfd_death_observed':True,'app_proc_absent':True,'original_parent_pidfd_living':not bool(select.select([servicefd],[],[],0)[0]),'evidence_kind':'inferred from trusted compiled Actor Child removal after try_wait plus kernel absence while original service parent remains alive','independent_syscall_trace':False}
 assert parent_reap['original_parent_pidfd_living']
 after_health=current_service_health(owner,sdk);assert after_health['actor_snapshot']['principals']==0
 assert after_health['source_current_probe']['pid']!=pid
 record('graceful_exact_normal_Qt_close_is_actually_waited_by_actor_without_source_or_writer_loss',app= r['retained_app_identity'],parent_reap=parent_reap,close_invoked_at_ns=close_invoked,app_reap_observed_at_ns=app_absent,health=after_health)
 # The separate probe is not a launched app principal and carries no retained app handle.
 wait(lambda:admin({'op':'messages_status'}).get('available') and not admin({'op':'messages_status'}).get('busy'),3)
 boundary=json.loads(command(['python3',str(R/'source/boundary-probe.py'),'--runtime',str(runtime),'--state',str(state)],'same-UID-without-principal-data-boundary.log',user=UID))
 assert boundary['uid']==boundary['gid']==UID and boundary['pid']!=pid
 assert boundary['app_read']=={'reply':{'result':'denied'}},boundary
 assert boundary['admin_snapshot'].get('errno')==13,boundary
 assert all(not v['readable'] and v['errno']==13 for v in boundary['raw_history'].values()),boundary
 after=root_history();assert after==before,{'before':before,'after':after}
 record('old_display_metadata_gives_no_principal_admin_or_Root_raw_history_authority',actual_same_UID_probe=boundary,Root_history_before=before,Root_history_after=after)
 future=independent_registry(app_listener,'same-UID-future-old-listener-registry.log');observed=now()
 assert future['uid']==future['gid']==UID and future['pid']!=pid and future['allowed_bound']==['wl_compositor','xdg_wm_base']
 latest=current_service_health(owner,sdk);assert latest['actor_snapshot']['principals']==0
 record('future_per_app_display_generation_withdraws_old_listener_after_exact_app_reap',passed=False,actual_allowed_registry=future,actual_reaped_app_pid=pid,actual_sdk_pid=sdk,old_listener=str(app_listener),post_reap_probe_at_ns=observed,post_reap_health=latest,desired_future_connection_refused=True,existing_metadata_scope='service_epoch')
 r['source_failed_before_desired_boundary']=False;r['desired_boundary_red']=True;r['red_detected']=True;r['passed']=False;r['boundary_scope']='New per-app generation admission requirement is absent; selected service-epoch context deliberately survives normal app reap. This does not retroactively invalidate its documented service-epoch contract.'
 r['app_parent_reap_inference']=parent_reap;r['Root_history_boundary']=boundary;r['normal_conversation_requires_visual_review']=True
 r['prerequisites_qualified']=True
 record('post_reap_same_live_original_Source_SDK_and_writer_preserved',health=latest)
def production_checks():return app_generation_checks()
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

 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'}
 command(['python3','-c','import socket,json; s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET); s.connect('+repr(str(runtime/'launch-messages.sock'))+'); s.send(b\'{"op":"launch_messages"}\'); assert json.loads(s.recv(4096))=={"result":"ok"}'],'desktop-launch.log',user=True)
 window=wait(lambda:next((v for v in json.loads(ctl('j/clients')) if v.get('title')=='Messages' and v.get('mapped')),None),90);pid=window['pid'];track();r['app']={k:window[k] for k in ['pid','mapped','xwayland','class','title']};assert not window['xwayland'];assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010';assert all(os.readlink(f'/proc/{p}/ns/{k}')==value for p in [compositor.pid,service.pid,pid] for k,value in r['namespaces'].items());r['setup_complete']=True
 production_checks();raise ProductionComplete()
except ProductionComplete:pass
except BaseException as e:r['error']=str(e);r['traceback']=traceback.format_exc();print(r['traceback'],flush=True)
finally:
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
