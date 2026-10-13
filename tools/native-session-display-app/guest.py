#!/usr/bin/env python3
"""Actual original compositor to installed native Qt Messages. No simulated source."""
import array, hashlib, json, os, select, shutil, signal, socket, struct, subprocess, time, traceback
from pathlib import Path
R=Path('/results');B=Path('/tmp/joint-display-app');B.mkdir();O=B/'evidence';O.mkdir()
selection=json.loads((R/'selection.json').read_text())
r={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'secure_presentation_qualified':False,'production_PAM_lock_qualified':False,'sample_budget_ns':100000000,'source_revived':False,'ui_queue_withdrawal_qualified':False}
children=[];logs=[];owned={};receiver=None;sourcefd=None;service=None;compositor=None;frames=[];control=None
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
  s.settimeout(2);s.connect(str(runtime/'admin.sock'));s.send(json.dumps(v).encode());return json.loads(s.recv(4096))
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
def composite_checks():
 unknown=capture('normal-composite-before-source');assert admin({'op':'snapshot'})['locked']
 record('actual_composite_actor_normal_Qt_unknown_source_denied',screen=unknown)
 ctx=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='display' and v['state']=='Ready'),None),5)
 assert ctx['admission'] and ctx['occupied']
 protected_realm_checks()
 time.sleep(1);r['render_preparation_completed_before_first_adapter_load_ns']=now();assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 if not wait(lambda:not admin({'op':'snapshot'})['locked'],3,True):
  r['availability_unqualified']=True;raise AssertionError('Original100ms source availability gate: composite normal app not admitted')
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3);assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID}
 assert not any(v['event']=='source_failed' for v in source_events())
 time.sleep(1);granted=capture('normal-composite-focus-grant-conversation');assert granted['sha256']!=unknown['sha256']
 writer_pid=writer();reader_pid=worker();sdk=sdk_worker();assert ctx['worker_pid']==sdk
 record('actual_composite_source_plus_independent_grant_admits_normal_conversation',source=fact,display=ctx,screen=granted,writer_pid=writer_pid,readonly_pid=reader_pid,sdk_pid=sdk)
 r['actual_focus_and_grant_admission_qualified']=True;r['normal_conversation_requires_visual_review']=True;r['composite_sdk_pid']=sdk
 if selection['test_case'].endswith('-green'):
  assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':False})=={'result':'ok'};time.sleep(1.5)
  revoked=capture('normal-composite-independent-grant-revoked');assert revoked['sha256']!=granted['sha256']
  assert json.loads(ctl('nativebuildprobe'))['pid']==pid
  record('actual_filtered_source_focus_cannot_replace_revoked_independent_grant',screen=revoked)
  assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'};time.sleep(1.5)
  restored=capture('normal-composite-independent-grant-restored');assert restored['sha256']!=revoked['sha256']
  assert json.loads(ctl('nativebuildprobe'))['pid']==pid and not any(v['event']=='source_failed' for v in source_events())
  record('same_filtered_normal_app_restores_local_conversation_with_new_current_grant',screen=restored)
  granted=restored
 if selection['test_case'] in ['composite-stop-resource-red','composite-held-policy-stop-green']:
  policy=policy_worker();signal.pidfd_send_signal(owned[policy]['fd'],signal.SIGSTOP)
  wait(lambda:Path(f'/proc/{policy}/stat').read_text().split()[2] in ['T','t'],1)
  began=now();snapshot_=admin({'op':'snapshot'});elapsed=now()-began;assert snapshot_['ready'] and elapsed<250000000
  record('actual_held_idle_policy_worker_keeps_composite_actor_Snapshot_responsive',policy_pid=policy,elapsed_ns=elapsed,snapshot=snapshot_)
  began=now();ack=admin({'op':'stop'});ack_ns=now();assert ack=={'result':'ok'} and ack_ns-began<250000000
  r['whole_Stop_ACK']={'accepted':ack,'acknowledged_at_ns':ack_ns,'elapsed_ns':ack_ns-began,'completion_claim':False}
  record('actual_held_idle_policy_worker_keeps_AdminStop_request_acceptance_responsive',observed=r['whole_Stop_ACK'])
  if selection['test_case']=='composite-stop-resource-red':
   wait(lambda:now()>=ack_ns+250000000,1)
   sdk_living=not bool(select.select([owned[sdk]['fd']],[],[],0)[0]);assert sdk_living
   future=independent_registry(app_listener,'future-filtered-registry-after-StopACK-budget.log');observed_ns=now()
   assert future['allowed_bound']==['wl_compositor','xdg_wm_base'];assert not select.select([owned[sdk]['fd']],[],[],0)[0]
   record('Actor_Stop_withdraws_outside_SDK_context_within_250ms_of_ACK',passed=False,stop_ack_ns=ack_ns,observed_after_ns=observed_ns,elapsed_after_ACK_ns=observed_ns-ack_ns,actual_sdk_pid=sdk,actual_sdk_pidfd_living=True,future_filtered_registry=future,desired_future_connection_refused=True)
   r['red_detected']=True;r['passed']=False;r['stage2_scope']='Poll-only Source trait plus outside-actor Supervisor; missing new owned-child shutdown integration before core hook'
  else:
   # The observation window includes actual original-server close processing.
   refusal=None;until=ack_ns+250000000
   while now()<until:
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as connection:
     connection.settimeout(.02)
     try:connection.connect(str(app_listener))
     except OSError as error:
      if error.errno in [2,111]:refusal={'errno':error.errno,'observed_at_ns':now()};break
    time.sleep(.002)
   assert refusal and refusal['observed_at_ns']<=until,{'refusal':refusal,'stop_ack_ns':ack_ns,'now_ns':now()}
   record('Actor_Stop_withdraws_SDK_listener_within_observed_250ms_of_ACK',stop_ack_ns=ack_ns,actual_refusal=refusal,elapsed_after_ACK_ns=refusal['observed_at_ns']-ack_ns,server_processing_observation_not_remote_ACK=True)
   wait(lambda:bool(select.select([owned[sdk]['fd']],[],[],0)[0]) and not Path(f'/proc/{sdk}').exists(),2)
   record('Actor_Stop_retains_SDK_ownership_until_actual_reap',sdk_pid=sdk,owned_pidfd_dead=True,proc_absent=True)
   exited=wait(lambda:service.poll() is not None,2);r['whole_Stop_completed_at_ns']=now();assert r['whole_Stop_completed_at_ns']-ack_ns<2000000000
   record('whole_Actor_Stop_shared_original_two_second_cleanup_completes',elapsed_after_ACK_ns=r['whole_Stop_completed_at_ns']-ack_ns,service_returncode=service.returncode)
   r['passed']=True
 elif selection['test_case']=='composite-held-sdk-loss-green':
  signal.pidfd_send_signal(owned[sdk]['fd'],signal.SIGSTOP);wait(lambda:Path(f'/proc/{sdk}/stat').read_text().split()[2] in ['T','t'],1)
  signal.pidfd_send_signal(owned[reader_pid]['fd'],signal.SIGSTOP);wait(lambda:Path(f'/proc/{reader_pid}/stat').read_text().split()[2] in ['T','t'],1)
  wait(lambda:admin({'op':'messages_status'}).get('busy'),2)
  began=now();snapshot_=admin({'op':'snapshot'});elapsed=now()-began;assert snapshot_['ready'] and not snapshot_['locked'] and elapsed<250000000
  record('actual_held_SDK_and_readonly_workers_keep_current_actor_responsive',snapshot=snapshot_,elapsed_ns=elapsed,sdk_pid=sdk,readonly_pid=reader_pid,writer_pid=writer_pid)
  signal.pidfd_send_signal(owned[sdk]['fd'],signal.SIGKILL)
  failure=wait(lambda:next((v for v in source_events() if v['event']=='source_failed'),None),2)
  assert admin({'op':'snapshot'})['locked'];assert not select.select([owned[writer_pid]['fd']],[],[],0)[0]
  wait(lambda:bool(select.select([owned[sdk]['fd']],[],[],0)[0]) and not Path(f'/proc/{sdk}').exists(),2)
  wait(lambda:bool(select.select([owned[reader_pid]['fd']],[],[],0)[0]) and not Path(f'/proc/{reader_pid}').exists(),2)
  time.sleep(1.5);denied=capture('normal-composite-held-SDK-death-denied');assert denied['sha256']!=granted['sha256'];assert Path(f'/proc/{pid}').exists()
  assert not select.select([owned[writer_pid]['fd']],[],[],0)[0]
  record('actual_SDK_death_fences_current_reads_and_reaps_held_readonly_while_writer_alive',source_failure=failure,screen=denied,sdk_pid=sdk,held_readonly_pid=reader_pid,writer_pid=writer_pid,writer_alive=True,normal_app_alive=True,actual_SDK_reaped=True,actual_readonly_reaped=True)
  r['normal_denial_requires_visual_review']=True;r['passed']=True
 else:raise ValueError('unknown selected composite case')
def production_checks():
 if selection['test_case']!='direct-realm-composition-red':return composite_checks()
 unknown=capture('normal-current-actor-before-source');assert admin({'op':'snapshot'})['locked']
 record('actual_current_actor_and_normal_Qt_installed_window_unknown_source',screen=unknown)
 time.sleep(1);r['render_preparation_completed_before_first_adapter_load_ns']=now()
 assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 unlocked=wait(lambda:not admin({'op':'snapshot'})['locked'],3,True)
 if not unlocked:
  r['availability_unqualified']=True;raise AssertionError('Original100ms source availability gate: cannot qualify desired composition RED')
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3)
 assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID}
 assert not any(v['event']=='source_failed' for v in source_events())
 time.sleep(1);granted=capture('normal-current-actor-focus-grant-conversation')
 assert granted['sha256']!=unknown['sha256']
 writer_pid=writer();reader_pid=worker();assert writer_pid!=reader_pid
 record('actual_current_focus_independent_grant_and_normal_conversation_capture',source=fact,screen=granted,writer_pid=writer_pid,reader_pid=reader_pid)
 r['normal_conversation_requires_visual_review']=True;r['actual_focus_and_grant_admission_qualified']=True
 connect=direct_path_as_app(wayland);assert connect['connected']
 record('actor_owned_app_realm_cannot_connect_original_Wayland_path',passed=False,actual=connect,desired_errno=13)
 registry=independent_registry(wayland,'same-app-UID-original-registry.log');assert registry['uid']==registry['gid']==UID
 present={v['interface']:v for v in registry['globals']}
 for interface in ['ext_session_lock_manager_v1','wp_security_context_manager_v1']:
  assert interface in present
  record('actor_owned_app_realm_hides_privileged_global_'+interface,passed=False,actual_original_global=present[interface])
  bound=independent_registry(wayland,'same-app-UID-bind-'+interface+'.log',present[interface])
  assert bound['forged_bind_completed'] is True and bound['forged_bind_error'] is None,bound
  record('actor_owned_app_realm_refuses_privileged_bind_'+interface,passed=False,actual=bound)
 assert not any(v['event']=='source_failed' for v in source_events()),'Source expired during composition RED qualification'
 r['red_detected']=True;r['passed']=False;r['stage1_scope']='Actual current actor/read-split data plus normal Qt on direct original realm; missing filtered app-realm composition'

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
 wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),120);control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),120);env.update(WAYLAND_DISPLAY=wayland.name,HYPRLAND_INSTANCE_SIGNATURE=control.parent.name);r['source']={'pid':compositor.pid,'uid':1000,'gid':1000,'incarnation':source,'owned_launcher_pidfd_retained':True}
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
 service=start(args,'service.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),fds=fds)
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
  dead=bool(select.select([v['fd']],[],[],0)[0]);absent=not Path(f'/proc/{pid}').exists();r['cleanup'].append({'role':v['role'],'pid':pid,'owned_pidfd_death_observed':dead,'proc_absent':absent,'reaped':dead and absent});os.close(v['fd'])
 if receiver is not None:receiver.close()
 if sourcefd is not None:os.close(sourcefd)
 for f in logs:f.close()
 r['combined_messages_children_reaped']={role:any(v['role']==role and v['reaped'] for v in r['cleanup']) for role in ['messages_writer','messages_read_worker']}
 r['source_events']=source_events() if (O/'service.log').exists() else [];r['baseline_frames']=frames;r['all_tracked_children_reaped']=not r.get('cleanup_errors') and all(v['reaped'] for v in r['cleanup']);r['passed']=r['passed'] and r['all_tracked_children_reaped']
 for p in observe.glob('*'):
  if p.suffix in ['.json','.png']:shutil.copyfile(p,O/p.name)
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
