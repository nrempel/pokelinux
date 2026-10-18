"""One normal Settings/Hyprland/Factory graphical packet join; authored authority."""
import sys
sys.dont_write_bytecode=True
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
import ctypes,hashlib,json,os,re,select,shlex,shutil,signal,socket,stat,time,traceback
from owned import Scope,now
from closure import close_owned
import local_compositor,peer,settings_build,settings_readiness
R=Path('/results');S=json.loads((R/'selection.json').read_text());T=now();W=T+660_000_000_000;C=T+700_000_000_000
scope=Scope(R);serial=0;compositor=controller=seat=bus=None;touch=None;control_path=None;handled=set();windows={}
r={'passed':False,'selection_sha256':hashlib.sha256((R/'selection.json').read_bytes()).hexdigest(),'original_work_until_ns':W,'original_close_until_ns':C,
 'phone_accessed':False,'Source_auth_full_session_owner_selected':False,'normal_app_test_control_endpoint_selected':False,'capture_selected':True,'boundary_steps':[]}
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def tick():scope.pump();scope.check()
def run(argv,label,seconds=20,limit=1048576,env=None,user=None,until=None,allowed=(0,)):
 global serial
 serial+=1;end=min(W,now()+int(seconds*1e9),until or W);identity={}if user is None else{'user':user,'group':user,'extra_groups':[]}
 code,body=scope.run(argv,f'{serial:03d}-{label}',end,limit=limit,env=env,identity=identity);assert code in allowed,(label,code,body[-1500:]);return body.decode()
def start(argv,label,env=None,user=None):
 identity={}if user is None else{'user':user,'group':user,'extra_groups':[]}
 return scope.start(argv,label,4*1024**2,env=env,cutoff=W,wait_cutoff=C,identity=identity)
def wait(fn,end):
 while now()<end:
  tick();value=fn()
  if value:return value
  if compositor is not None:assert scope.live(compositor),'normal compositor exited'
  time.sleep(.005)
 raise TimeoutError('original bounded setup/observation cutoff')
def directory(p,uid=0,mode=0o755):
 p=Path(p);p.mkdir(parents=True,exist_ok=True);p.chmod(mode);os.chown(p,uid,uid);return p
def install(source,dest):
 dest=Path(dest);directory(dest.parent);shutil.copyfile(source,dest);dest.chmod(0o755);os.chown(dest,0,0);assert sha(source)==sha(dest);return dest
def control(request,end):
 assert now()<end;data=(request if request.startswith('j/')else '/'+request).encode();assert len(data)<4096
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as s:
  s.settimeout(max(.001,(end-now())/1e9));s.connect(str(control_path));cred=s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12)
  import struct
  assert struct.unpack('3i',cred)==(compositor['child'].pid,1000,1000)
  s.sendall(data);body=bytearray()
  while True:
   assert now()<end;s.settimeout(max(.001,(end-now())/1e9));chunk=s.recv(65536)
   if not chunk:break
   body.extend(chunk);assert len(body)<=262144
 assert now()<end
 return body.decode()
def dispatch(expression,end):
 before=now();reply=control('dispatch '+expression,end);row={'expression':expression,'raw_reply':reply,'before_ns':before,'after_ns':now(),'original_until_ns':end}
 r.setdefault('normal_control',[]).append(row);assert len(reply.encode())<=4096 and reply.strip()=='ok',row;return row
def qualify(pid,end):
 clients=json.loads(control('j/clients',end));matches=[x for x in clients if x.get('pid')==pid and x.get('mapped')]
 if not matches:return None
 assert len(matches)==1;v=matches[0];m=json.loads(control('j/monitors',end));assert len(m)==1;m=m[0]
 assert(m['width'],m['height'])==(1080,2424)and(m['physicalWidth'],m['physicalHeight'])==(65,146)and abs(m['scale']-2.625)<=.001
 assert m['dpmsStatus']and not m['disabled']and m['transform']==0
 r.setdefault('normal_window_precondition_observations',{})[str(pid)]={'pid':pid,'sampled_boottime_ns':now(),'original_until_ns':end,'actual_compositor_client':v,'actual_output':m}
 assert v['class']=='pokelinux-settings'and v['title']=='Settings'and not v['xwayland']and v['monitor']==m['id']
 x,y=v['at'];width,height=v['size'];assert abs(x-m['x'])<1 and abs(y-m['y'])<1 and abs(width-1080/2.625)<1 and abs(height-2424/2.625)<1
 assert sha(Path('/proc')/str(pid)/'exe')==S['providers']['settings']['sha256']
 status=(Path('/proc')/str(pid)/'status').read_text();assert all(next(l for l in status.splitlines()if l.startswith(k)).split()[1:]==['11050']*4 for k in ['Uid:','Gid:'])
 assert not next(l for l in status.splitlines()if l.startswith('Groups:')).split()[1:]
 assert all(int(next(l for l in status.splitlines()if l.startswith(k)).split()[1],16)==0 for k in ['CapEff:','CapPrm:','CapInh:','CapAmb:'])
 assert next(l for l in status.splitlines()if l.startswith('NoNewPrivs:')).split()[1]=='1'
 row={'pid':pid,'actual_compositor_client':v,'actual_output':m,'normal_QWindow_getters_sampled':False,'profile_scope':'external current compositor/output and identity-bound mapped normal window','proc_status':status}
 windows[pid]=row;return row
def focus(pid,end):
 v=wait(lambda:qualify(pid,end),end)['actual_compositor_client'];dispatch('hl.dsp.focus({window='+json.dumps('address:'+v['address'])+'})',end)
 wait(lambda:(lambda x:x if x.get('pid')==pid and x.get('address')==v['address']else None)(json.loads(control('j/activewindow',end))),end)
 return v
def tap(pid,which,end):
 from evdev import ecodes as E
 v=focus(pid,end);h=r['layout_metrics']['actual_Qt_Text_implicit_heights'];padding,gap,cgap=18,14,8
 # Exact current normal QML order before connection buttons: seven Texts + brightness Row.
 row_y=padding+h['16']+gap+h['14']+gap+h['12']+gap+h['10']+gap+56+gap+h['14']+gap+h['12']+gap+h['10']+gap
 width=(v['size'][0]-2*padding-cgap)/2
 lx=padding+width/2+(0 if which=='enable'else width+cgap);ly=row_y+28
 x=round((v['at'][0]+lx)*2.625);y=round((v['at'][1]+ly)*2.625)
 assert 0<x<1080 and 0<y<2424 and ly+28<v['size'][1]-padding
 ax=round(x*32767/1079);ay=round(y*32767/2423);before=now();assert before<end;events=[]
 def emit(t,c,v):n=touch.write(t,c,v);events.append({'type':t,'code':c,'value':v,'returned':n,'at_ns':now()})
 for c,z in [(E.ABS_MT_SLOT,0),(E.ABS_MT_TRACKING_ID,1 if which=='enable'else 2),(E.ABS_MT_POSITION_X,ax),(E.ABS_MT_POSITION_Y,ay),(E.ABS_X,ax),(E.ABS_Y,ay)]:emit(E.EV_ABS,c,z)
 emit(E.EV_KEY,E.BTN_TOUCH,1);emit(E.EV_SYN,E.SYN_REPORT,0);time.sleep(.1);assert now()<end
 emit(E.EV_ABS,E.ABS_MT_TRACKING_ID,-1);emit(E.EV_KEY,E.BTN_TOUCH,0);emit(E.EV_SYN,E.SYN_REPORT,0)
 row={'pid':pid,'operation':which,'before_ns':before,'after_ns':now(),'original_until_ns':end,'native_point':[x,y],'logical_point':[lx,ly],'derived_row_y':row_y,'actual_uinput_writes':events,'Qt_clicked_count_claimed':False}
 assert row['after_ns']<end;r.setdefault('normal_inputs',[]).append(row);return row
try:
 assert os.getuid()==0 and os.uname().machine=='x86_64'
 peer.call('prctl',36,1,0,0,0);subreaper=ctypes.c_int(-1);peer.call('prctl',37,ctypes.byref(subreaper),0,0,0);assert subreaper.value==1
 r['actual_guest_child_subreaper']=True
 run(['/usr/bin/uname','-a'],'kernel')
 assert hashlib.sha256(run(['/usr/bin/pacman','-Q'],'packages',limit=1048576).encode()).hexdigest()==S['installed_package_list_sha256']
 # Fail closed if an inherited owner/compositor is already active; never stop/retry a parked session.
 unintended=[]
 for p in Path('/proc').iterdir():
  if not p.name.isdigit():continue
  try:name=(p/'comm').read_text().strip();exe=os.readlink(p/'exe')
  except(FileNotFoundError,PermissionError,ProcessLookupError):continue
  if name=='Hyprland'or exe.endswith('/pokelinux-session-owner'):unintended.append({'pid':int(p.name),'comm':name,'exe':exe})
 assert not unintended,unintended;r['no_preexisting_session_owner_or_compositor']=True
 unit=run(['/usr/bin/systemctl','is-active','pokelinux-session-owner.service'],'preexisting-session-state',allowed=(3,4));assert unit.strip()in('inactive','unknown');r['preexisting_unit_state']=unit
 r['source_built_compositor_install']=local_compositor.install(R,S,run)
 for uid,name in [(11050,'settings-app')]:run(['/usr/bin/useradd','-u',str(uid),'-M','-s','/usr/bin/nologin',name],'useradd-settings')
 run(['/usr/bin/modprobe','virtio_gpu'],'GPU-module');run(['/usr/bin/modprobe','uinput'],'uinput-module');run(['/usr/bin/udevadm','settle'],'udev')
 for p in ['/usr/libexec/pokelinux','/usr/libexec/pokelinux-fixture','/etc/pokelinux-window-fixture','/run/pokelinux','/run/pokelinux/display']:directory(p)
 directory('/run/user/1000',1000,0o700);directory('/home/alarm',1000,0o700);directory('/run/user/1000/pixel10-profile',1000,0o700)
 assert 'settings' not in S['providers']
 generated_settings,settings_build_proof=settings_build.build_settings(R,S,run)
 S['providers']['settings']=generated_settings
 r['normal_Settings_generated_provider']=dict(generated_settings);r['normal_Settings_build']=settings_build_proof
 for name,row in S['providers'].items():
  source=R/row['relative_path'];assert source.stat().st_size==row['bytes']and sha(source)==row['sha256'];dest=install(source,'/usr/libexec/pokelinux-fixture/'+('pokelinux-settings'if name=='settings'else name))
  loader=run(['/usr/bin/ldd','-r',str(dest)],'loader-'+name);assert'not found'not in loader.lower()and'undefined symbol'not in loader.lower()
  row['guest_path']=str(dest)
 shared=directory('/usr/share/pokelinux/appearance');shutil.copyfile(R/'source/PhoneAppearance.qml',shared/'PhoneAppearance.qml');(shared/'PhoneAppearance.qml').chmod(0o644)
 # Explicit normal fallback: real appearance client endpoint unavailable, no fake theme/model property.
 assert not Path('/run/pokelinux/appearance.sock').exists();r['appearance']='normal unavailable-client fallback; no live theme-provider claim'
 run(['/usr/bin/pkg-config','--modversion','Qt6Core','Qt6Gui','Qt6Qml','Qt6Quick'],'Qt-versions')
 flags=shlex.split(run(['/usr/bin/pkg-config','--cflags','hyprland','Qt6Core'],'profile-SDK-flags'));libs=shlex.split(run(['/usr/bin/pkg-config','--libs','Qt6Core'],'profile-SDK-libs'))
 fix=Path('/usr/libexec/pokelinux-fixture')
 r['native_output_capture_build']=settings_readiness.build(R,run,fix)
 for src,name in [('physical-size-adapter.cpp','pixel10-physical-size-adapter.so'),('profile-ready-bootstrap.cpp','profile-ready-bootstrap.so')]:
  run(['/usr/bin/g++','-std=c++23','-shared','-fPIC','-O2','-Wall','-Wextra','-Werror',*flags,str(R/'source'/src),'-o',str(fix/name),*libs],'build-'+name,seconds=180)
 qflags=shlex.split(run(['/usr/bin/pkg-config','--cflags','--libs','Qt6Gui','Qt6Qml','Qt6Quick','wayland-client','libdrm'],'discovery-flags'))
 for src,name in [('qt-output-discovery.cpp','pixel10-output-discovery'),('layout-metrics.cpp','settings-layout-metrics')]:
  run(['/usr/bin/g++','-std=c++23','-O2','-fPIC','-Wall','-Wextra','-Werror',str(R/'source'/src),'-o',str(fix/name),*qflags],'build-'+name,seconds=180)
 profile=directory('/usr/share/pokelinux/display-fixture')
 shutil.copyfile(R/'source/pixel10.json',profile/'pixel10.json');shutil.copyfile(R/'source/public-api-pins.json',profile/'public-api-pins.json')
 # Bind the profile adapter to the actually installed local SDK/API cohort.
 (profile/'public-api-pins.json').write_text(json.dumps(r['source_built_compositor_install']['runtime_public_api']))
 env={'HOME':'/home/alarm','PATH':'/usr/bin','LANG':'C.UTF-8','XDG_RUNTIME_DIR':'/run/user/1000','XDG_SESSION_TYPE':'wayland','POKELINUX_DISPLAY_PROFILE':str(profile/'pixel10.json'),'POKELINUX_DISPLAY_PROFILE_RECEIPT':'/run/user/1000/pixel10-profile/applied.json'}
 bus=start(['/usr/bin/dbus-daemon','--session','--nofork','--print-address=1'],'desktop-bus.log',env,user=1000)
 env['DBUS_SESSION_BUS_ADDRESS']=wait(lambda:(R/'desktop-bus.log').read_text().splitlines()[0]if(R/'desktop-bus.log').exists()and(R/'desktop-bus.log').read_text().splitlines()else None,min(W,now()+5_000_000_000))
 seat=start(['/usr/bin/seatd','-g','alarm'],'seatd.log',env=dict(os.environ,SEATD_VTBOUND='0'));wait(lambda:Path('/run/seatd.sock').is_socket(),min(W,now()+5_000_000_000))
 config=Path('/etc/pokelinux-window-fixture/hyprland.lua');config.write_text('hl.permission("/usr/libexec/pokelinux-fixture/pixel10-raw-output-capture","screencopy","allow")\n'+'hl.monitor({ output="",mode="1080x2424@60",position="auto",scale=2.625 })\n'+'hl.permission([[/usr/libexec/pokelinux-fixture/pixel10-physical-size-adapter\\.so]],"plugin","allow")\n'+'hl.config({debug={disable_scale_checks=true},animations={enabled=false},decoration={blur={enabled=false},shadow={enabled=false}},misc={disable_hyprland_logo=true,disable_splash_rendering=true}})\n'+'hl.plugin.load("/usr/libexec/pokelinux-fixture/profile-ready-bootstrap.so")\n')
 compositor=start(['/usr/bin/Hyprland','--config',str(config)],'normal-compositor.log',env,user=1000)
 def ready_control():
  paths=list(Path('/run/user/1000/hypr').glob('*/.socket.sock'));assert len(paths)<=1
  return paths[0]if paths and Path('/run/user/1000/pixel10-profile/ordered-load.json').exists()else None
 control_path=wait(ready_control,min(W,now()+120_000_000_000));ordered=json.loads(Path('/run/user/1000/pixel10-profile/ordered-load.json').read_text());assert ordered['pid']==compositor['child'].pid and ordered['Source_FD_or_facts_accessed']is False
 waylands=list(Path('/run/user/1000').glob('wayland-[0-9]'));assert len(waylands)==1;wayland=waylands[0];env['WAYLAND_DISPLAY']=str(wayland)
 observed=Path('/run/user/1000/pixel10-profile/discovery.json');run([str(fix/'pixel10-output-discovery'),str(observed)],'actual-output-discovery',user=1000,env=env,seconds=30)
 r['actual_output_discovery']=json.loads(observed.read_text());outputs=r['actual_output_discovery']['wayland_outputs'];assert len(outputs)==1 and outputs[0]['current_mode']['width']==1080 and outputs[0]['current_mode']['height']==2424 and outputs[0]['physical_width_mm']==65 and outputs[0]['physical_height_mm']==146
 plugins=control('j/plugins',min(W,now()+5_000_000_000));assert'session-source'not in plugins and'audio-focus-observer'not in plugins;r['actual_plugins']=plugins
 layout_env=dict(os.environ,XDG_RUNTIME_DIR='/run/user/1000');r['layout_metrics']=json.loads(run([str(fix/'settings-layout-metrics')],'layout-metrics',user=1000,env=layout_env))
 from evdev import UInput,AbsInfo,ecodes as E
 caps={E.EV_KEY:[E.BTN_TOUCH],E.EV_ABS:[(E.ABS_X,AbsInfo(0,0,32767,0,0,0)),(E.ABS_Y,AbsInfo(0,0,32767,0,0,0)),(E.ABS_MT_SLOT,AbsInfo(0,0,9,0,0,0)),(E.ABS_MT_TRACKING_ID,AbsInfo(0,0,65535,0,0,0)),(E.ABS_MT_POSITION_X,AbsInfo(0,0,32767,0,0,0)),(E.ABS_MT_POSITION_Y,AbsInfo(0,0,32767,0,0,0))]}
 touch=UInput(caps,name='Pokelinux Settings connectivity touchscreen',input_props=[E.INPUT_PROP_DIRECT]);run(['/usr/bin/udevadm','settle'],'uinput-udev');wait(lambda:'pokelinux settings connectivity touchscreen'in control('j/devices',min(W,now()+1_000_000_000)).lower().replace('-',' '),min(W,now()+10_000_000_000))
 r['normal_control_visual_readiness']={}
 RT=now();CW=min(W,RT+60_000_000_000);CC=min(C,RT+70_000_000_000);out=R/'connectivity-case';argv=['/usr/bin/python3','-B',str(R/'source/controller.py')]
 for option,role in [('main','main'),('worker','worker'),('driver','driver'),('settings-app','settings'),('launcher','helper'),('storage','helper')]:argv+=['--'+option+'-elf',S['providers'][role]['guest_path']]
 argv+=['--display-helper',S['providers']['helper']['guest_path'],'--display-parent','/run/pokelinux/display','--wayland',str(wayland),'--compositor-pid',str(compositor['child'].pid),'--appearance','/run/pokelinux/appearance.sock','--theme-dir',str(shared),'--output',str(out),'--work-until-ns',str(CW),'--close-until-ns',str(CC)]
 ce=dict(os.environ,POKELINUX_CONNECTIVITY_ISOLATED='1');controller=scope.start(argv,'connectivity-controller.log',4*1024**2,env=ce,cutoff=CW,wait_cutoff=CC,identity={'preexec_fn':lambda:peer.reduce_caps(sum(1<<n for n in [0,1,5,6,7,8,12,21]))});G=out/'gates'
 while scope.live(controller):
  tick();assert now()<CW
  for ordinal in ['first','second']:
   source=G/(ordinal+'-window.json');key=ordinal+'-qualified'
   if source.exists()and key not in handled:
    row=json.loads(source.read_text());end=min(CW,now()+30_000_000_000)if ordinal=='first'else json.loads((G/'functional-clock.json').read_text())['until_ns']
    proof=wait(lambda:qualify(row['pid'],end),end)
    if ordinal=='second':
     readiness={};r['normal_control_visual_readiness']['second-disable']=readiness
     try:settings_readiness.observe(R,fix,run,env,row['pid'],True,end,qualify,focus,readiness,S['providers']['settings']['sha256'])
     except BaseException:r['classification']='VISUAL_SETUP_UNQUALIFIED';raise
    peer.publish(G,ordinal+'-window-qualified.json',proof);handled.add(key)
  if(G/'first-action-ready.json').exists()and'functional-clock'not in handled:
   ready=json.loads((G/'first-action-ready.json').read_text());end=min(CW,json.loads((G/'ready.json').read_text())['original_case_until_ns']);readiness={};r['normal_control_visual_readiness']['first-enable']=readiness
   try:settings_readiness.observe(R,fix,run,env,ready['pid'],False,end,qualify,focus,readiness,S['providers']['settings']['sha256'])
   except BaseException:r['classification']='VISUAL_SETUP_UNQUALIFIED';raise
   clock=now();peer.publish(G,'functional-clock.json',{'start_ns':clock,'until_ns':clock+12_000_000_000});handled.add('functional-clock')
  for action in ['enable','disable']:
   request=G/(action+'-input-request.json')
   if request.exists()and action not in handled:
    row=json.loads(request.read_text());receipt=tap(row['pid'],action,row['original_until_ns']);peer.publish(G,action+'-input-sent.json',receipt);handled.add(action)
  for ordinal in ['first','second']:
   name='close-'+ordinal;request=G/(name+'-request.json')
   if request.exists()and name not in handled:
    row=json.loads(request.read_text());end=row['original_until_ns'];v=focus(row['pid'],end);receipt=dispatch('hl.dsp.window.close({window='+json.dumps('address:'+v['address'])+'})',end);peer.publish(G,name+'-sent.json',receipt);handled.add(name)
  time.sleep(.002)
 assert scope.finish(controller,CC)==0
 case=json.loads((out/'result.json').read_text());assert case['passed']and case['custody_qualified'];r['packet_case']=case
 assert len(r['normal_inputs'])==2 and handled=={'first-qualified','second-qualified','functional-clock','enable','disable','close-first','close-second'}
 r['normal_window_proofs']=list(windows.values());r['Pixel10_profile']={'qualification':{'qualified':True,'scope':'external compositor/output and identity-bound mapped normal Settings windows; no direct app QWindow getter'}}
 r['boundary_steps']=[{'id':1,'passed':True,'normal_Enable_nonce1_normalClose_relaunch_nonce2_Disable_absence_quiet':True}]
 r['functional_passed']=True
except BaseException as e:r.update(error=repr(e),traceback=traceback.format_exc())
finally:
 if touch is not None:touch.close()
 for item in [controller,compositor,seat,bus]:
  if item is not None and scope.live(item):
   try:signal.pidfd_send_signal(item['pidfd'],signal.SIGTERM)
   except ProcessLookupError:pass
 r['normal_shutdown_codes']={}
 for item in [controller,compositor,seat,bus]:
  if item is not None:
   try:r['normal_shutdown_codes'][item['name']]=scope.finish(item,C)
   except BaseException as e:r.setdefault('shutdown_errors',[]).append(repr(e))
 r['closure']=close_owned(scope,C,reap_adopted=True);r['owned_receipts']=scope.receipts
 # Scope ECHILD is meaningful for orphaned descendants only with subreaping.
 r['guest_case_closure_qualified']=r.get('actual_guest_child_subreaper',False)and r['closure']['closed']and r['closure']['wait_all_ECHILD']and not r['closure']['adopted_actual_waits']
 nested=R/'connectivity-case/result.json';r['nested_controller_cleanup']={'record_present':nested.is_file()}
 if nested.is_file():
  try:
   assert nested.stat().st_size<=4*1024**2
   value=json.loads(nested.read_text());r['nested_controller_cleanup'].update({'sha256':sha(nested),'custody_qualified':value.get('custody_qualified',False),'services_closure':value.get('services_final_closure'),'driver_closure':value.get('driver_final_closure'),'deferred_parent_signals':value.get('deferred_parent_signals',[])})
  except BaseException as e:r['nested_controller_cleanup']['read_error']=repr(e)
 r['passed']=bool(r.get('functional_passed')and not r.get('shutdown_errors')and all(v in(0,-15)for v in r['normal_shutdown_codes'].values())and r['guest_case_closure_qualified']and now()<C)
 r['completed_boottime_ns']=now();(R/'result.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'passed':r['passed'],'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['passed']else 1)
