#!/usr/bin/env python3
"""Actual original compositor to installed native Qt Messages. No simulated source."""
import array, hashlib, shlex, json, os, pwd, select, shutil, signal, socket, sqlite3, struct, subprocess, sys, time, traceback
from pathlib import Path
R=Path('/results');B=Path('/tmp/incoming-session-installed');B.mkdir();O=B/'evidence';O.mkdir()
selection=json.loads((R/'selection.json').read_text())
r={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'secure_presentation_qualified':False,'production_PAM_lock_qualified':False,'sample_budget_ns':100000000,'source_revived':False,'ui_queue_withdrawal_qualified':False}
children=[];logs=[];owned={};receiver=None;sourcefd=None;service=None;compositor=None;frames=[];control=None;peer=None
APP='org.omarchy.Messages';UID=11010
timing_channels=[];timings=[];guest_timeline=[]
def timing_pair(role):
 receive,send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET)
 receive.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1)
 receive.setsockopt(socket.SOL_SOCKET,socket.SO_RCVBUF,1048576)
 send.setsockopt(socket.SOL_SOCKET,socket.SO_SNDBUF,1048576)
 receive.setblocking(False);send.setblocking(False)
 timing_channels.append({'role':role,'receive':receive,'send':send,'pid':None})
 return receive,send
def pump_timings():
 for channel in timing_channels:
  if channel['pid'] is None:continue
  for _ in range(512):
   try:data,anc,flags,_=channel['receive'].recvmsg(256,socket.CMSG_SPACE(12),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
   except BlockingIOError:break
   if not data:break
   assert not flags & (socket.MSG_TRUNC|socket.MSG_CTRUNC)
   creds=[]
   for level,kind,body in anc:
    assert level==socket.SOL_SOCKET and kind==socket.SCM_CREDENTIALS
    creds.append(struct.unpack('3i',body))
   uid=1000 if channel['role']=='publisher' else 0
   assert creds==[(channel['pid'],uid,uid)],creds
   count=8 if channel['role']=='publisher' else 9
   assert len(data)==count*8
   values=list(struct.unpack('='+str(count)+'Q',data))
   assert values[0]==(2 if channel['role']=='publisher' else 1)
   assert len(timings)<100000,'bounded diagnostic retention exceeded'
   timings.append({'role':channel['role'],'guest_received_ns':now(),'values':values})
def diagnostic_sleep(seconds):
 until=now()+int(seconds*1e9)
 while now()<until:
  pump_timings();time.sleep(min(.004,max(0,(until-now())/1e9)))

runtime=Path('/run/pokelinux/messages');state=Path('/var/lib/pokelinux');install=Path('/usr/libexec/pokelinux');observe=Path('/run/joint-messages-observer')
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def pump():
 pump_timings()
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
 began=now()
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as s:
  s.settimeout(2);s.connect(str(runtime/'admin.sock'));s.send(json.dumps(v).encode());reply=json.loads(s.recv(4096));guest_timeline.append({'event':'admin','request':v,'began_ns':began,'ended_ns':now(),'reply':reply});return reply
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
   args=(path/'cmdline').read_bytes().split(b'\0');role='messages_read_worker' if b'--messages-read-worker' in args else 'messages_writer' if b'--messages-incoming-worker' in args or b'--messages-worker' in args else 'policy_storage_worker' if b'--storage-worker' in args else 'installed_Qt_app'
   owned[pid]={'fd':os.pidfd_open(pid,os.O_NONBLOCK),'role':role}
  except FileNotFoundError:pass

def capture(name):
 path=O/(name+'.png');capture_dir=Path('/run/user/1000/incoming-session-captures');capture_dir.mkdir(exist_ok=True);capture_dir.chmod(0o700);os.chown(capture_dir,1000,1000);temporary=capture_dir/path.name;command(['grim',str(temporary)],name+'-capture.log',env,True);shutil.copyfile(temporary,path);assert path.stat().st_size>1024;return {'file':path.name,'sha256':sha(path)}
def rows():
 with sqlite3.connect('file:'+str(state/'raw/receipts.sqlite3')+'?mode=ro',uri=True,timeout=0) as db:raw=db.execute('SELECT id,slot,source_incarnation,scope,ack,hex(raw) FROM receipts ORDER BY seq').fetchall()
 with sqlite3.connect('file:'+str(state/'inbox/inbox.sqlite3')+'?mode=ro',uri=True,timeout=0) as db:inbox=db.execute('SELECT id,slot,source_incarnation,state,body FROM records ORDER BY seq').fetchall()
 assert [v[0] for v in raw]==[v[0] for v in inbox]
 return {'raw':raw,'inbox':inbox}
def probe(*args):return command([str(peer_session.BASE/'target/aarch64-unknown-linux-gnu/debug/incoming-probe'),*map(str,args)],'probe-'+str(len(r['groups']))+'-'+str(now())+'.log')
def counters():
 values=[json.loads(probe('counters',slot)) for slot in range(2)];assert all(v[:2]==[1,1] and v[3:]==[0,0,0,0] for v in values);return values
def arrival(count,slot=0):
 probe('incoming',slot,0,'000008915155100000006201213141510005e8329bfd06',0)
 wait(lambda:admin({'op':'messages_status'})['incoming_completed']==count,4)
 v=rows();assert len(v['raw'])==count and all(x[3:5]==('test_backend','stored') for x in v['raw']);assert all(x[3:]==('text','hello') for x in v['inbox']);return v

def writer_identity():
 p=writer();args=Path(f'/proc/{p}/cmdline').read_bytes().split(b'\0')[:-1];assert args[1]==b'--messages-incoming-worker' and args[-2].decode()==peer.generation
 marker=state/'raw'/('incoming-attempt-'+peer.generation);assert marker.is_file();fds=[os.readlink(v) for v in Path(f'/proc/{p}/fd').iterdir() if v.exists()];assert any('receipts.sqlite3' in v for v in fds) and any('inbox.sqlite3' in v for v in fds) and any('binderfs/incoming' in v for v in fds)
 return {'pid':p,'worker_incarnation':args[-3].decode(),'generation':args[-2].decode(),'attempt_marker_sha256':sha(marker),'attempt_marker_count':len(list((state/'raw').glob('incoming-attempt-*'))),'owned_pidfd_retained':p in owned,'receipts_and_inbox_and_Binder_FDs_retained':True}
def assert_receiving_same(expected):
 assert writer_identity()==expected;assert admin({'op':'messages_status'})['receiving'];assert dict(peer.owned)['peer'].poll() is None;assert counters()
def read_incarnation(p):return Path(f'/proc/{p}/cmdline').read_bytes().split(b'\0')[-2].decode()
def hold_reader():
 p=worker();value={'pid':p,'incarnation':read_incarnation(p)};began=now();os.kill(p,signal.SIGSTOP);wait(lambda:Path(f'/proc/{p}/stat').read_text().split()[2] in ['T','t'],1);wait(lambda:admin({'op':'messages_status'})['busy'],1);began=now();assert admin({'op':'snapshot'})['ready'];return p,value,began

def wait_current_app_source():
 def current():
  events=source_events();failures=[v for v in events if v['event']=='source_failed']
  if failures:
   r['current_source_availability_failure']={'app_pid':pid,'source_failure':failures[-1],'snapshot':admin({'op':'snapshot'}),'messages':admin({'op':'messages_status'})};raise AssertionError('actual source expired; current Qt history/reopen is unqualified')
  fact=next((v for v in reversed(events) if v['event']=='session'),None)
  return fact if fact and fact['focus']=={'pid':pid,'uid':UID,'gid':UID} and not admin({'op':'snapshot'})['locked'] else None
 return wait(current,3)

def launch_installed():
 global window,pid
 guest_timeline.append({'event':'relaunch_requested','boottime_ns':now()})
 command(['python3','-c','import socket,json; s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET); s.connect('+repr(str(runtime/'launch-messages.sock'))+'); s.send(b\'{"op":"launch_messages"}\'); assert json.loads(s.recv(4096))=={"result":"ok"}'],'desktop-relaunch-'+str(now())+'.log',user=True)
 guest_timeline.append({'event':'relaunch_acknowledged','boottime_ns':now()})
 window=wait(lambda:next((v for v in json.loads(ctl('j/clients')) if v.get('title')=='Messages' and v.get('mapped')),None),90);pid=window['pid'];track();assert not window['xwayland'];assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010'

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
 for n in ['lifecycle-launcher','pokelinux-messages']:
  shutil.copyfile(R/'bins'/n,install/n);(install/n).chmod(0o755)
 shutil.copyfile(R/'bins/probe.so',B/'probe.so');(B/'probe.so').chmod(0o755)
 shutil.copyfile(R/'bins/diagnostic-adapter.so',B/'adapter.so');(B/'adapter.so').chmod(0o755)
 assert sha(B/'adapter.so')==selection['diagnostic_adapter_elf_sha256']
 r['diagnostic_adapter_elf_sha256']=sha(B/'adapter.so');shutil.copyfile(B/'adapter.so',O/'diagnostic-adapter.so')
 r['diagnostic_adapter_provenance']='byte-identical generated ELF retained by independent warm-focus-timing-arm-v1 against original installed headers; no source semantics change'
 publisher_receive,publisher_send=timing_pair('publisher')
 observer_receive,observer_send=timing_pair('observer')
 appearance=Path('/usr/share/pokelinux/appearance');appearance.mkdir(parents=True)
 for n in ['PhoneTheme.qml','ThemePalette.js']:shutil.copyfile(R/'source/native-app'/n,appearance/n)
 (B/'hypr.lua').write_text('hl.permission("/usr/bin/grim", "screencopy", "allow")\nhl.monitor({ output = "", mode = "320x240@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
 receive,send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET);receive.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);receive.setblocking(False);source=os.urandom(16).hex()
 env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','PATH':'/usr/local/bin:/usr/bin','POKELINUX_SESSION_FD':str(send.fileno()),'POKELINUX_SESSION_SOURCE':source,'POKELINUX_COLD_TIMING_FD':str(publisher_send.fileno())}
 compositor=start(['/usr/bin/Hyprland','--config',str(B/'hypr.lua')],'hyprland.log',env,True,(send.fileno(),publisher_send.fileno()));timing_channels[0]['pid']=compositor.pid;publisher_send.close();sourcefd=os.pidfd_open(compositor.pid,os.O_NONBLOCK);send.close()
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
 wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==warm.pid,20);diagnostic_sleep(1)
 command(['setfacl','-m','u:11010:--x','/run/user/1000'],'runtime-acl.log');command(['setfacl','-m','u:11010:rw-',str(wayland)],'wayland-acl.log')
 profile=B/'profile.json';profile.write_text(json.dumps({'apps':[{'app':APP,'uid':UID,'executable':str(install/'pokelinux-messages'),'args':['/var/lib/pokelinux-messages-app','/run/user/11010',str(wayland),'/home/alarm',str(runtime/'app.sock'),str(appearance),* []]}]}))

 # Only public generated synthetic peer helpers are staged here. Original Bionic
 # and libbinder_ndk stay in the private readonly runtime input mount.
 sys.path.insert(0,str(R/'source/rust-build/tools/native-messages-incoming-binder'))
 import peer_session
 peer_session.BASE=Path('/run/incoming-session-peer');peer_session.BASE.mkdir()
 peer_session.LOADER='/private/artifacts/runtime/apex/com.android.runtime/bin/linker64'
 peer_session.LIBRARIES='/private/artifacts/runtime/system/lib64:/private/artifacts/runtime/apex/com.android.runtime/lib64/bionic'
 for name in ['incoming-peer','property-info','property-init']:
  shutil.copyfile(R/'bins'/name,peer_session.BASE/name);(peer_session.BASE/name).chmod(0o755)
 peerbins=peer_session.BASE/'target/aarch64-unknown-linux-gnu/debug';peerbins.mkdir(parents=True)
 for name in ['incoming-manager','incoming-probe']:
  shutil.copyfile(R/'bins'/name,peerbins/name);(peerbins/name).chmod(0o755)
 Path('/dev/binderfs').mkdir();command(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],'binderfs.log')
 peer=peer_session.PeerSession(pwd.getpwnam('alarm'),O,'one-live-generation').start();r['incoming_source']=peer.evidence
 name='messages-session-cold-current-fixture';shutil.copyfile(R/'bins'/name,install/name);(install/name).chmod(0o755)
 args=[str(install/name),str(runtime),str(state/'policy'),str(profile),str(install/'lifecycle-launcher'),str(state/'raw'),str(state/'inbox'),peer.generation,'none']
 if selection['phase']=='green':
  args += [str(receive.fileno()),str(sourcefd),str(compositor.pid),'1000',source];fds=(receive.fileno(),sourcefd,observer_send.fileno())
 else:receiver=receive;fds=()
 service=start(args,'service.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1',NATIVE_COLD_TIMING_FD=str(observer_send.fileno())),fds=fds);timing_channels[1]['pid']=service.pid;observer_send.close()
 if selection['phase']=='green':receive.close()
 wait(lambda:admin({'op':'snapshot'}).get('ready'))
 wait(lambda:admin({'op':'messages_status'}).get('receiving'))
 wait(lambda:admin({'op':'messages_status'}).get('available'))
 owner=writer();initial=arrival(1);assert admin({'op':'snapshot'})['principals']==0
 record('actual_frozenV3_callback_commits_exact_raw_and_inbox_without_app',writer_pid=owner,generation=peer.generation,rows=initial,registration_counters=counters())
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'}
 command(['python3','-c','import socket,json; s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET); s.connect('+repr(str(runtime/'launch-messages.sock'))+'); s.send(b\'{"op":"launch_messages"}\'); assert json.loads(s.recv(4096))=={"result":"ok"}'],'desktop-launch.log',user=True)
 window=wait(lambda:next((v for v in json.loads(ctl('j/clients')) if v.get('title')=='Messages' and v.get('mapped')),None),90);pid=window['pid'];track();r['app']={k:window[k] for k in ['pid','mapped','xwayland','class','title']};assert not window['xwayland'];assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010';assert all(os.readlink(f'/proc/{p}/ns/{k}')==value for p in [compositor.pid,service.pid,pid] for k,value in r['namespaces'].items());r['setup_complete']=True
 if selection['phase']=='red':
  assert admin({'op':'snapshot'})['locked'];diagnostic_sleep(2);denied=capture('red-unknown-source-denied')
  assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'}
  wait(lambda:not admin({'op':'snapshot'})['locked']);diagnostic_sleep(2);exposed=capture('red-unknown-source-root-controls-incoming-history')
  assert exposed['sha256']!=denied['sha256'];assert writer()==owner and admin({'op':'messages_status'})['receiving'];assert rows()==initial
  r['expected_red']=True;r['red_detected']=True;r['signature']='incoming_current_core_SourceNone_manual_root_controls_publish_history_without_session_source';r['red_boundary']='desired NEW incoming/session composition must deny unknown source; prior incoming-only fixture was not a trusted-session implementation';r['production_screens_require_visual_review']=True
  record('desired_new_incoming_session_unknown_source_boundary',False,unknown_screen=denied,exposed_screen=exposed,actual_locked=False,exact_incoming_rows=initial,same_live_writer=owner)
  raise AssertionError('desired NEW boundary refuses unknown-source incoming history after manual root controls')

 assert admin({'op':'snapshot'})['locked'];diagnostic_sleep(1);unknown=capture('green-unknown-source-denied')
 assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'};diagnostic_sleep(1);assert admin({'op':'snapshot'})['locked'];manual=capture('green-unknown-root-controls-denied')
 record('normal_Qt_unknown_session_source_remains_denied_after_root_controls',unknown_screen=unknown,root_screen=manual)
 identity=writer_identity();r['writer_identity']=identity
 diagnostic_sleep(1);assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 wait(lambda:not admin({'op':'snapshot'})['locked'],3)
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3);assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID}
 diagnostic_sleep(1);wait_current_app_source();granted=capture('green-focused-independent-grant-history');assert granted['sha256']!=unknown['sha256'];assert_receiving_same(identity)
 record('normal_Qt_current_owned_focus_plus_independent_grant_exposes_actual_incoming_history',source=fact,screen=granted,exact_receipt_ids=[v[0] for v in rows()['raw']])
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':False})=={'result':'ok'};diagnostic_sleep(1);revoked=capture('green-grant-revoked');assert revoked['sha256']!=granted['sha256']
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'};diagnostic_sleep(1);restored=capture('green-grant-restored');assert restored['sha256']!=revoked['sha256']
 record('actual_focus_does_not_replace_revoked_read_grant',revoked_screen=revoked,restored_screen=restored)
 warmwindow=next(v for v in json.loads(ctl('j/clients')) if v['pid']==warm.pid)
 assert ctl('dispatch hl.dsp.focus({ window = '+json.dumps('address:'+warmwindow['address'])+' })').strip()=='ok'
 wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==warm.pid,3)
 background=arrival(2,slot=1);assert_receiving_same(identity);diagnostic_sleep(1);background_screen=capture('green-background-incoming-denied')
 record('actual_background_focus_denies_Qt_while_callback2_projects_same_live_source',rows=background,screen=background_screen,registration_counters=counters())
 refocus();wait_current_app_source();diagnostic_sleep(1);wait_current_app_source();refocused=capture('green-refocused-two-slot-history');assert refocused['sha256']!=background_screen['sha256']
 if selection['case']=='cold-reopen':
  held,read_identity,began=hold_reader();closed_pid=pid
  assert ctl('dispatch hl.dsp.window.close({ window = '+json.dumps('address:'+window['address'])+' })').strip()=='ok'
  wait(lambda:not Path(f'/proc/{closed_pid}').exists(),3);wait(lambda:not Path(f'/proc/{held}').exists(),3)
  elapsed=now()-began;assert elapsed<1_000_000_000,'close/reap must precede the held read absolute deadline'
  assert admin({'op':'snapshot'})['principals']==0;assert_receiving_same(identity)
  closed=arrival(3);assert closed['raw'][0][2]==closed['raw'][2][2]
  record('held_actual_readonly_close_reaps_reader_and_app_preserving_writer_generation_slotSID',readonly=read_identity,app_pid=closed_pid,close_reap_elapsed_ns=elapsed,rows=closed,writer=identity,registration_counters=counters())
  launch_installed();wait(lambda:admin({'op':'messages_status'})['available'],3);refocus();wait_current_app_source();diagnostic_sleep(1);wait_current_app_source();reopened=capture('green-reopened-same-receiver-history');fresh=worker();assert fresh!=held and read_incarnation(fresh)!=read_identity['incarnation'];assert_receiving_same(identity)
  record('fresh_normal_Qt_app_and_readonly_incarnation_read_same_live_receiver_history',new_app_pid=pid,new_readonly_pid=fresh,new_read_incarnation=read_incarnation(fresh),screen=reopened)
 else:
  refocus();wait_current_app_source();diagnostic_sleep(1);wait_current_app_source();warm_control=capture('diagnostic-warm-current-app-focus');assert_receiving_same(identity)
  record('separate_warm_focus_control_retains_actual_current_source_and_app',app_pid=pid,writer=identity,screen=warm_control)
 held,read_identity,began=hold_reader();guest_timeline.append({'event':'source_unload_requested','boottime_ns':now()});assert ctl('plugin unload '+str(B/'adapter.so')).strip()=='ok'
 failure=wait(lambda:next((v for v in source_events() if v['event']=='source_failed'),None),3);assert admin({'op':'snapshot'})['locked'];wait(lambda:not Path(f'/proc/{held}').exists(),3);assert_receiving_same(identity)
 lost=arrival(4,slot=1);assert lost['raw'][1][2]==lost['raw'][3][2]
 assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'};diagnostic_sleep(1);assert admin({'op':'snapshot'})['locked'];lost_screen=capture('green-terminal-session-loss-future-incoming-denied')
 record('terminal_session_unload_reaps_held_reader_but_callback4_keeps_same_Binder_source',source_failure=failure,readonly=read_identity,rows=lost,writer=identity,registration_counters=counters(),screen=lost_screen)
 r['last_live_counters']=counters();r['last_live_probe']=json.loads(probe('info'));assert r['last_live_probe']['generation']==peer.generation
 peer_process=dict(peer.owned)['peer'];guest_timeline.append({'event':'original_peer_death_requested','boottime_ns':now()});peer_process.kill();wait(lambda:peer_process.poll() is not None,3)
 dead=wait(lambda:(v if not (v:=admin({'op':'messages_status'}))['receiving'] and v['incoming_failure'] else None),4)
 wait(lambda:not Path(f'/proc/{owner}').exists(),3);assert rows()==lost
 record('actual_original_runtime_peer_death_exercises_retained_watch_after_read_and_session_loss',observed_status=dead,writer_reaped=owner,unchanged_exact_receipts=True,one_source_generation=peer.generation)
 r['production_screens_require_visual_review']=True
 r['passed']=all(v['passed'] for v in r['groups']);r['diagnostic_episode_complete']=True

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
 if peer is not None:
  try:
   peer.close()
   for name,process in peer.owned:r['cleanup'].append({'role':'synthetic_'+name,'pid':process.pid,'reaped':process.returncode is not None,'returncode':process.returncode})
  except BaseException as e:r.setdefault('cleanup_errors',[]).append(str(e))
 if receiver is not None:receiver.close()
 if sourcefd is not None:os.close(sourcefd)
 for f in logs:f.close()
 pump_timings()
 for channel in timing_channels:channel['receive'].close()
 (O/'timings.json').write_text(json.dumps(timings,separators=(',',':'))+'\n');(O/'guest-timeline.json').write_text(json.dumps(guest_timeline,indent=2)+'\n')
 r['timing_records']={role:sum(v['role']==role for v in timings) for role in ['publisher','observer']}
 r['selected_current_core_base']=selection['core_head'];r['reviewed_core_patch_sha256']=selection['immutable_patch_sha256'];r['patched_observer_sha256']=selection['patched_observer_sha256']
 r['diagnostic_scope']='immutable10ce core plus explicit reviewed Observer queue patch; actual incoming/current actor/normal Qt cold reopen then held readonly source unload and original runtime peer death; required Source stop/occupied delegated; diagnostic original adapter timing; unchanged100ms source and25ms publisher timer; no source revival'
 r['messages_children_reaped']={role:any(v['role']==role and v['reaped'] for v in r['cleanup']) for role in ['messages_writer','messages_read_worker']}
 r['source_events']=source_events() if (O/'service.log').exists() else [];r['baseline_frames']=frames;r['all_tracked_children_reaped']=not r.get('cleanup_errors') and all(v['reaped'] for v in r['cleanup']);r['passed']=r['passed'] and r['all_tracked_children_reaped']
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
