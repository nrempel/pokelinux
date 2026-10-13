#!/usr/bin/env python3
"""Actual original compositor to installed native Qt Messages. No simulated source."""
import array, hashlib, json, os, select, shutil, signal, socket, struct, subprocess, time, traceback
from pathlib import Path
R=Path('/results');B=Path('/tmp/joint-installed');B.mkdir();O=B/'evidence';O.mkdir()
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
 log=(O/name).open('w');logs.append(log);opts={'user':1000,'group':1000,'extra_groups':os.getgrouplist('alarm',1000)} if user else {}
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
 return next(pid for pid,v in owned.items() if v['role']=='messages_worker' and Path(f'/proc/{pid}').exists())
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
   args=(path/'cmdline').read_bytes().split(b'\0');role='messages_worker' if b'--messages-worker' in args else 'policy_storage_worker' if b'--storage-worker' in args else 'installed_Qt_app'
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
def production_checks():
 assert admin({'op':'snapshot'})['locked'];unknown=capture('production-unknown');record('normal_production_ELF_installed_UID11010_window_unknown_source',screen=unknown)
 assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'};time.sleep(1);assert admin({'op':'snapshot'})['locked'];manual=capture('production-unknown-root-controls');record('normal_production_unknown_source_root_controls_remain_denied',screen=manual)
 if selection.get('prepare_lock_before_first_source'):
  preparation_locker=start([str(B/'lock-client')],'preparation-locker.log',env,True,stdin=subprocess.PIPE)
  wait(lambda:'CONNECTED' in (O/'preparation-locker.log').read_text(),10);preparation_locker.stdin.write(b'lock\n');preparation_locker.stdin.flush();wait(lambda:json.loads(ctl('nativebuildprobe'))['locked'],3);time.sleep(1);ctl('nativebuildforceunlock');finish(preparation_locker,'preparation-locker.log');refocus();r['actual_lock_preparation_before_first_source_frame_ns']=now()
 time.sleep(1);r['render_preparation_completed_before_first_adapter_load_ns']=now();assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok';wait(lambda:not admin({'op':'snapshot'})['locked'],3)
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3);assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID};time.sleep(1);granted=capture('production-granted');assert granted['sha256']!=unknown['sha256'];record('normal_production_focused_owned_app_plus_independent_grant',source=fact,screen=granted)
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':False})=={'result':'ok'};time.sleep(1.5);revoked=capture('production-grant-revoked');assert revoked['sha256']!=granted['sha256'];record('normal_production_current_grant_revocation_changes_actual_view',screen=revoked)
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'};time.sleep(1.5);restored=capture('production-grant-restored');assert restored['sha256']!=revoked['sha256'];assert json.loads(ctl('nativebuildprobe'))['pid']==pid;record('normal_production_same_window_restores_local_history_after_new_grant',screen=restored)
 held=worker();os.kill(held,signal.SIGSTOP);wait(lambda:admin({'op':'messages_status'}).get('busy'),2);assert ctl('plugin unload '+str(B/'adapter.so')).strip()=='ok';failure=wait(lambda:next((v for v in source_events() if v['event']=='source_failed'),None),3);assert admin({'op':'snapshot'})['locked'];time.sleep(1.5);lost=capture('production-source-unloaded');assert lost['sha256']!=restored['sha256'];record('normal_production_source_loss_fences_held_read_and_changes_actual_view',source=failure,screen=lost)
 try:os.kill(held,signal.SIGCONT)
 except ProcessLookupError:pass
 wait(lambda:not Path(f'/proc/{held}').exists(),3);track();r['production_screens_require_visual_review']=True;r['passed']=True
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
 command(['setfacl','-m','u:11010:--x','/run/user/1000'],'runtime-acl.log');command(['setfacl','-m','u:11010:rw-',str(wayland)],'wayland-acl.log')
 profile=B/'profile.json';profile.write_text(json.dumps({'apps':[{'app':APP,'uid':UID,'executable':str(install/('pokelinux-messages' if selection.get('normal_production_qt') else 'pokelinux-messages-observer')),'args':['/var/lib/pokelinux-messages-app','/run/user/11010',str(wayland),'/home/alarm',str(runtime/'app.sock'),str(appearance),* ([] if selection.get('normal_production_qt') else ['--isolated-observe',str(observe)])]}]}))
 command([str(R/'bins/messages-seed-fixture'),str(state/'raw'),'--renderer-text'],'seed.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'))
 args=[str(install/'messages-service'),str(runtime),str(state/'policy'),str(profile),str(install/'lifecycle-launcher'),str(state/'raw'),str(state/'inbox')]
 if selection['phase']=='green':
  shutil.copyfile(R/'bins/messages-session-installed-fixture',install/'messages-session-installed-fixture');(install/'messages-session-installed-fixture').chmod(0o755);args[0]=str(install/'messages-session-installed-fixture');args += [str(receive.fileno()),str(sourcefd),str(compositor.pid),'1000',source];fds=(receive.fileno(),sourcefd)
 else:receiver=receive;fds=()
 service=start(args,'service.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),fds=fds)
 if selection['phase']=='green':receive.close()
 wait(lambda:admin({'op':'snapshot'}).get('ready'));wait(lambda:admin({'op':'messages_status'}).get('available'))
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'}
 command(['python3','-c','import socket,json; s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET); s.connect('+repr(str(runtime/'launch-messages.sock'))+'); s.send(b\'{"op":"launch_messages"}\'); assert json.loads(s.recv(4096))=={"result":"ok"}'],'desktop-launch.log',user=True)
 window=wait(lambda:next((v for v in json.loads(ctl('j/clients')) if v.get('title')=='Messages' and v.get('mapped')),None),90);pid=window['pid'];track();r['app']={k:window[k] for k in ['pid','mapped','xwayland','class','title']};assert not window['xwayland'];assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010';assert all(os.readlink(f'/proc/{p}/ns/{k}')==value for p in [compositor.pid,service.pid,pid] for k,value in r['namespaces'].items());r['setup_complete']=True
 if selection.get('normal_production_qt'):
  production_checks();raise ProductionComplete()
 wait(ui_denied,15);record('unknown_source_actual_Qt_denied_before_first_adapter_load')
 assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'}
 time.sleep(1)
 denied=wait(ui_denied,3,True)
 if denied:record('root_controls_cannot_replace_unknown_source')
 else:
  leaked=wait(ui_read,10);record('root_controls_cannot_replace_unknown_source',False,ui=leaked);r['red_detected']=True
 if selection['phase']=='red':
  assert admin({'op':'locked','locked':True})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':True})=={'result':'ok'};wait(ui_denied,10)
 if selection.get('prepare_lock_before_first_source'):
  preparation_locker=start([str(B/'lock-client')],'preparation-locker.log',env,True,stdin=subprocess.PIPE)
  wait(lambda:'CONNECTED' in (O/'preparation-locker.log').read_text(),10);preparation_locker.stdin.write(b'lock\n');preparation_locker.stdin.flush();wait(lambda:json.loads(ctl('nativebuildprobe'))['locked'],3);time.sleep(1);ctl('nativebuildforceunlock');finish(preparation_locker,'preparation-locker.log');refocus();r['actual_lock_preparation_before_first_source_frame_ns']=now()
 time.sleep(1);r['render_preparation_completed_before_first_adapter_load_ns']=now();assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 probe=wait(lambda:(v if (v:=json.loads(ctl('nativebuildprobe')))['pid']==pid else None),10);r['actual_focus_probe']=probe
 if selection['phase']=='red':
  f=wait(lambda:next((f for f in reversed(frames) if f['focus'] and f['focus']['pid']==pid),None),5);r['baseline_actual_focused_owned_wire']=f
  v=wait(ui_read,3,True);record('actual_compositor_focus_plus_independent_grant_admits_installed_Qt',bool(v),ui=v or snapshot());assert not v;r['red_detected']=True
 else:
  v=wait(ui_read,20);record('actual_compositor_focus_plus_independent_grant_admits_installed_Qt',ui=v)
  assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':False})=={'result':'ok'}
  wait(ui_denied,10);record('actual_focus_cannot_replace_revoked_independent_grant')
  assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'};wait(ui_read,10)
  ctl('nativebuildclearfocus');wait(ui_denied,10)
  active=json.loads(ctl('j/activewindow'));probe=json.loads(ctl('nativebuildprobe'));assert active['pid']==pid and not probe['eligible'];record('actual_central_null_focus_clears_Qt_while_activewindow_remembers_app',probe=probe)
  refocus();wait(ui_read,10)
  layer=start(['quickshell','-n','-p',str(layerdir)],'layer-live.log',env,True)
  wait(lambda:json.loads(ctl('nativebuildprobe'))['type']==3,10);wait(ui_denied,10)
  active=json.loads(ctl('j/activewindow'));probe=json.loads(ctl('nativebuildprobe'));assert active['pid']==pid and probe['pid']==layer.pid and not probe['eligible'];record('actual_exclusive_layer_focus_clears_Qt_with_remembered_activewindow',probe=probe)
  finish(layer,'layer-live.log');refocus();wait(ui_read,10)
  for op,key in [('locked','locked'),('background','background')]:
   request={'op':op,key:True}
   if op=='background':request['app']=APP
   assert admin(request)=={'result':'ok'};wait(ui_denied,10);assert json.loads(ctl('nativebuildprobe'))['pid']==pid;record('source_facts_cannot_clear_root_'+op+'_denial_intent')
   request[key]=False;assert admin(request)=={'result':'ok'};wait(ui_read,10)
  if not selection.get('skip_lock_group'):
   # Connect and prepare the actual protocol client before holding storage work.
   locker=start([str(B/'lock-client')],'locker.log',env,True,stdin=subprocess.PIPE)
   wait(lambda:'CONNECTED' in (O/'locker.log').read_text(),10)
   held=worker();os.kill(held,signal.SIGSTOP);wait(lambda:Path(f'/proc/{held}/stat').read_text().split()[2] in ['T','t'],3)
   wait(lambda:admin({'op':'messages_status'}).get('busy'),2)
   began=now();snapshot_reply=admin({'op':'snapshot'});elapsed=now()-began;assert snapshot_reply['ready'] and elapsed<500000000;record('held_real_Messages_storage_keeps_joint_actor_responsive',elapsed_ns=elapsed,worker_pid=held)
   locker.stdin.write(b'lock\n');locker.stdin.flush();wait(lambda:admin({'op':'snapshot'}).get('locked'),2)
   lock_fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and v.get('locked')),None),2);assert lock_fact['focus'] is None;record('actual_protocol_lock_fences_current_actor_while_Qt_readback_is_suppressed',source=lock_fact);r['locked_UI_readback_qualified']=False
   try:os.kill(held,signal.SIGCONT)
   except ProcessLookupError:pass
   wait(lambda:not admin({'op':'messages_status'}).get('busy'),3);wait(lambda:admin({'op':'messages_status'}).get('available'),5);track();record('held_real_read_completion_retains_owned_worker_and_locked_authority',worker_pid=held,worker_still_owned=Path(f'/proc/{held}').exists(),locked=admin({'op':'snapshot'})['locked'])
   finish(locker,'locker.log');assert json.loads(ctl('nativebuildprobe'))['locked'];assert admin({'op':'snapshot'})['locked'];record('actual_locker_death_retains_joint_denial')
   # This explicit test-only force unlock qualifies actor ingress, not PAM or presentation.
   assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':False})=={'result':'ok'};ctl('nativebuildforceunlock');refocus();wait(ui_denied,10);record('revoked_held_read_cannot_restore_Qt_local_history_after_protocol_unlock');assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'};wait(ui_read,15);record('original_force_unlock_reconciles_actual_app_without_supplying_grant')
  kind=selection.get('source_loss_case','unload');loss_at=now()
  if kind=='unload':assert ctl('plugin unload '+str(B/'adapter.so')).strip()=='ok'
  elif kind=='stopped':os.kill(compositor.pid,signal.SIGSTOP)
  else:finish(compositor,'hyprland.log')
  failure=wait(lambda:next((v for v in source_events() if v['event']=='source_failed'),None),3);assert admin({'op':'snapshot'})['locked'];record('actual_'+kind+'_source_terminally_fences_current_actor',failure=failure)
  if kind=='stopped':
   assert failure['observed_at_ns']>=failure['last_sampled_at_ns']+100000000;os.kill(compositor.pid,signal.SIGCONT)
  if kind!='death':
   wait(ui_denied,10)
   assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'};time.sleep(.5);wait(ui_denied,5);assert admin({'op':'snapshot'})['locked'];record('current_source_loss_cannot_be_revived_by_root_controls_or_continued_compositor')
  else:
   wait(lambda:not Path(f'/proc/{pid}').exists(),5);record('actual_compositor_death_closes_and_reaps_installed_Qt_window')
  r['passed']=all(v['passed'] for v in r['groups'])
except ProductionComplete:pass
except BaseException as e:r['error']=str(e);r['traceback']=traceback.format_exc();print(r['traceback'],flush=True)
finally:
 if service is not None and service.poll() is None:
  try:admin({'op':'stop'});wait(lambda:service.poll() is not None,5)
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
 r['source_events']=source_events() if (O/'service.log').exists() else [];r['baseline_frames']=frames;r['all_tracked_children_reaped']=not r.get('cleanup_errors') and all(v['reaped'] for v in r['cleanup']);r['passed']=r['passed'] and r['all_tracked_children_reaped']
 for p in observe.glob('*'):
  if p.suffix in ['.json','.png']:shutil.copyfile(p,O/p.name)
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
