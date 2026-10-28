#!/usr/bin/env python3
"""One cold PID1-installed normal startup, genuine full shell and fixed Sounds launch."""
import array,math,ctypes,errno,hashlib,json,os,re,select,shlex,shutil,signal,socket,stat,struct,subprocess,sys,time,traceback
from pathlib import Path
sys.dont_write_bytecode=True
from owned import Scope,now
from systemd_units import Units,install_bundle
import display,theme,audio_config,startup_observer,installed,native_support,qt_native,local_compositor
R=Path('/results');S=json.loads((R/'selection.json').read_text());T0=now();WORK=T0+660_000_000_000;CLOSE=T0+700_000_000_000
scope=Scope(R);serial=0;expected={};watched={};call_offset=0;call_item=None;actor=None;app_pid=None;app_pidfd=None;service=None;compositor_pid=None;compositor_pidfd=None;compositor_log=None;broker=None;theme_shell=None;focus_layer=None;touch=None;startup=None;normal_argv_bound=False;normal_argv=None;profile_args=None;metadata_file=None;stderr_file=None;capture_dir=None;capture_mounted=False;owned_files=[]
RUN=installed.RUN;APP='org.omarchy.Sounds';FOCUS_LOG=Path('/run/user/1000/native-audio-focus.jsonl')
result={'schema':1,'passed':False,'scope':S['scope'],'selection_sha256':hashlib.sha256((R/'selection.json').read_bytes()).hexdigest(),'started_boottime_ns':T0,'original_work_until_ns':WORK,'original_closure_until_ns':CLOSE,'boundary_steps':[],'phase':'setup','phone_accessed':False,'hardware_qualified':False,'genuine_full_shell_executed':False,'auth_PAM_paths_selected':False,'desktop_menu_activation_exercised':False,'native_fixed_desktop_client_selected':True,'guest_authored_Source_snapshots_selected':False,'Source_channel_exclusively_owned_by_Rust':True}
assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
units=Units(scope,R,WORK,CLOSE,expected,S['selected_owner_unit'])
result.update(selected_owner_unit_test_variant=S['selected_owner_unit_test_variant'],manager_Stop_variant_selected=True,native_owner_IPC_Stop_selected=False,cold_installed_startup_slice_only=True,original_prepared_four_groups_rerun=False,authored_theme_host_selected=False,prepared_warmup_selected=False,manual_first_Source_load_selected=False)

def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def identity(user):
 assert user is None or type(user)is int,'numeric UID required; legacy bool is not a UID'
 if user==1000:return {'user':1000,'group':1000,'extra_groups':os.getgrouplist('alarm',1000)}
 return {'user':user,'group':user,'extra_groups':[]}if user else {}

def capture_bounds():
 for path,budget in [(metadata_file,8*2049),(stderr_file,4194304)]:
  if path is not None:assert path.stat().st_size<=budget,'accepted normal capture output budget exceeded'

def tick():scope.pump();scope.check();watch();capture_bounds()

def wait(check,seconds=20,before_timeout=None,absolute_cutoff=None):
 until=min(WORK,now()+int(seconds*1e9),absolute_cutoff if absolute_cutoff is not None else WORK);timeout_observed=False
 while now()<until:
  tick();value=check()
  if value:return value
  if before_timeout is not None and not timeout_observed and now()>=until-3_000_000_000:
   timeout_observed=True;before_timeout(until)
  time.sleep(.025)
 raise TimeoutError('original finite guest prerequisite cutoff')

def start(argv,label,env=None,user=None,stdin=None,allowed=(0,)):
 global serial
 assert len(scope.children)<384
 serial+=1;name=f'{serial:03d}-'+label;item=scope.start(argv,name,4194304,stdin=stdin,env=env,cutoff=WORK,wait_cutoff=CLOSE,identity=identity(user));expected[item['child'].pid]={'role':label,'allowed_actual_exit':list(allowed)};return item

def command(argv,label,env=None,user=None,seconds=30,limit=1048576,absolute_cutoff=None,allowed=(0,)):
 global serial
 assert len(scope.children)<384
 serial+=1;name=f'{serial:03d}-'+label;until=min(WORK,now()+int(seconds*1e9),absolute_cutoff if absolute_cutoff is not None else WORK);code,body=scope.run(argv,name,until,limit=limit,env=env,identity=identity(user));expected[scope.receipts[-1]['pid']]={'role':label,'allowed_actual_exit':list(allowed)};assert code in allowed,(label,code,body[-1200:]);watch();capture_bounds();return body.decode()

def live(item):return units.live(item)if 'unit'in item else scope.live(item)

def finish(item,seconds=10,permit_manager_stop=True):return units.finish(item,min(CLOSE,now()+int(seconds*1e9)),permit_manager_stop=permit_manager_stop)if 'unit'in item else scope.finish(item,min(CLOSE,now()+int(seconds*1e9)))

def stop(item,seconds=10,allowed=(0,-15)):
 if 'unit'in item:
  units.run(['/usr/bin/systemctl','stop',item['unit']],'explicit-stop-'+item['unit']+'.log',True);return None
 expected[item['child'].pid]['allowed_actual_exit']=list(allowed)
 if live(item):signal.pidfd_send_signal(item['pidfd'],signal.SIGTERM)
 code=finish(item,seconds);assert code in allowed,(item['name'],code);return code

def first_line(item):
 p=R/item['name'];lines=p.read_text(errors='replace').splitlines();return lines[0]if lines else None

def status():
 v=call({'op':'status'});assert v['ok']and v['peer_uid']==11041;return v

def call(request):
 global call_offset
 stream=call_item['child'].stdin;data=(json.dumps(request)+'\n').encode();assert len(data)<=4096;stream.write(data);stream.flush()
 def answer():
  global call_offset
  lines=(R/call_item['name']).read_text().splitlines()
  if len(lines)<=call_offset:return
  row=json.loads(lines[call_offset]);call_offset+=1;assert row['request']==request;assert row['reply_boottime_ns']<row['original_until_ns'];return row
 row=wait(answer,6);result.setdefault('privileged_fixture_requests',[]).append(row);return row['response']

def graph():
 text=command(['/usr/bin/pw-dump'],'independent-graph.json',env=dict(audio_env,PIPEWIRE_DEBUG='0'),user=11040,seconds=3);v=json.loads(text)
 nodes={x['id']:x for x in v if x['type'].endswith(':Node')};ports={x['id']:x for x in v if x['type'].endswith(':Port')};links=[]
 for x in v:
  if not x['type'].endswith(':Link'):continue
  i=x['info'];props=i.get('props',{});o=int(i.get('output-node-id',props.get('link.output.node',-1)));n=int(i.get('input-node-id',props.get('link.input.node',-1)))
  if o not in nodes or n not in nodes:continue
  a=nodes[o]['info']['props']['node.name'];b=nodes[n]['info']['props']['node.name']
  if a in ['source.media','source.call','source.microphone']:
   links.append({'id':x['id'],'source':a,'target':b,'state':i['state'],'source_state':nodes[o]['info']['state'],'target_state':nodes[n]['info']['state']})
 core=[x for x in v if x['type'].endswith(':Core')];assert len(core)==1
 clients=[]
 for x in v:
  props=x.get('info',{}).get('props',{})
  if x['type'].endswith(':Client')and props.get('application.process.binary')=='wireplumber':clients.append({'id':x['id'],'props':{k:props[k]for k in ['application.process.binary','application.process.id','pipewire.sec.pid','pipewire.sec.uid','pipewire.sec.gid']if k in props}})
 return {'cookie':core[0]['info']['cookie'],'wireplumber_clients':clients,'service_input_links':links,'node_ids':{x['info']['props']['node.name']:x['id']for x in nodes.values()},'raw_graph_sha256':hashlib.sha256(text.encode()).hexdigest()}

def wireplumber_ready(quiet=False):
 assert live(wireplumber),'owned WirePlumber exited before actual connection readiness'
 pid=wireplumber['child'].pid;proc=Path('/proc')/str(pid);text=(proc/'status').read_text()
 assert next(v for v in text.splitlines()if v.startswith('Uid:')).split()[1:]==['11040']*4
 assert next(v for v in text.splitlines()if v.startswith('Gid:')).split()[1:]==['11040']*4
 assert sha(proc/'exe')==S['original_tool_pins']['wireplumber']
 g=graph()
 if quiet:assert not g['service_input_links'],'service-source replay observed during fixture stack restart'
 matches=[c for c in g['wireplumber_clients']if int(c['props'].get('application.process.id',-1))==pid]
 if not matches:return
 assert len(matches)==1;props=matches[0]['props']
 for k,value in [('pipewire.sec.pid',pid),('pipewire.sec.uid',11040),('pipewire.sec.gid',11040)]:
  if k in props:assert int(props[k])==value
 return {'owned_PID':pid,'kernel_UIDs':[11040]*4,'kernel_GIDs':[11040]*4,'actual_exe_sha256':sha(proc/'exe'),'actual_original_graph_client':matches[0],'actual_security_fields_present':[k for k in props if k.startswith('pipewire.sec.')],'graph':g}

def matching(source,target):
 owner=status();g=graph();links=g['service_input_links'];want=[]if source is None else[(source,target)]
 if [(x['source'],x['target'])for x in links]!=want:return
 if any(x['state']!='active'or x['source_state']!='running'or x['target_state']!='running'for x in links):return
 if source is not None and not(owner['observed']['available']and owner['application']=='observed'):return
 return {'owner':owner,'independent_graph':g}

def pcm(stage,label):
 out=R/'pcm'/label;command(['/usr/bin/python3','/proposal/pcm_observer.py',stage,str(out)],'PCM-'+label+'.log',env=audio_env,user=11040,seconds=20,limit=1048576)
 v=json.loads((out/'result.json').read_text());assert v['passed']and v['classification']=='ACTUAL_PCM_ORACLES_PASS';return {'stage':stage,'result':str(out.relative_to(R))+'/result.json','sha256':sha(out/'result.json'),'measured_frames':sum(s['frames']for s in v['stats'].values()),'actual_stats':v['stats']}

def capture(label,until=None):
 def bounded(argv,name,**kwargs):return command(argv,name,absolute_cutoff=until,**kwargs)
 return display.capture_native(label,R,public_profile,profile_bins,bounded,desktop_env,qualification)

def pixels(capture):
 from measure import png_pixels
 # Independent Pillow decode of whole original native PNG, no transformed output.
 from PIL import Image
 with Image.open(R/capture['file'])as image:assert image.size==(1080,2424);rgb=image.convert('RGB');return rgb

def tap(point,number):
 from evdev import ecodes as E
 x,y=point;assert 0<x<1080 and 0<y<2424;x=round(x*32767/1079);y=round(y*32767/2423)
 for code,value in [(E.ABS_MT_SLOT,0),(E.ABS_MT_TRACKING_ID,number),(E.ABS_MT_POSITION_X,x),(E.ABS_MT_POSITION_Y,y),(E.ABS_X,x),(E.ABS_Y,y)]:touch.write(E.EV_ABS,code,value)
 touch.write(E.EV_KEY,E.BTN_TOUCH,1);touch.syn();time.sleep(.1);touch.write(E.EV_ABS,E.ABS_MT_TRACKING_ID,-1);touch.write(E.EV_KEY,E.BTN_TOUCH,0);touch.syn()
 result.setdefault('normal_input_events',[]).append({'actual_guest_uinput':True,'physical_native_point':point,'absolute_axes':[x,y],'tracking_id':number,'at_boottime_ns':now()})

def appearance():
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)as s:
  s.settimeout(2);s.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);s.connect('/run/pokelinux/appearance.sock');assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(broker['child'].pid,0,0)
  assert s.send(b'{"op":"snapshot"}')==17
  body,anc,flags,_=s.recvmsg(16384,socket.CMSG_SPACE(12));assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC)
  assert len(anc)==1 and anc[0][:2]==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS)and struct.unpack('3i',anc[0][2])==(broker['child'].pid,0,0)
  v=json.loads(body);assert v['schema']==3 and int(v['until_ns'])>now();return v

def color8(value):
 assert set(value)=={'r','g','b','a'}and value['a']==65535
 return tuple(round(value[k]/257)for k in ['r','g','b'])

def button_centres(frame,client,publication,count=2):
 image=pixels(frame);dpr=2.625;values=publication['appearance'];padding=values['style']['spacing_tokens']['panelPadding'];bg=color8(values['palette']['background'])
 x=round((client['at'][0]+padding)*dpr)+1;y0=round((client['at'][1]+25)*dpr);y1=min(2424,round((client['at'][1]+client['size'][1])*dpr));runs=[]
 for y in range(y0,y1):
  if image.getpixel((x,y))!=bg:
   if not runs or y>runs[-1][-1]+1:runs.append([y])
   else:runs[-1].append(y)
 candidates=[v for v in runs if abs(len(v)-56*dpr)<=5]
 assert len(candidates)==count,{'independent_vertical_button_edges':[(v[0],v[-1])for v in runs],'expected_visible_buttons':count}
 points=[[round((client['at'][0]+client['size'][0]/2)*dpr),(v[0]+v[-1])//2]for v in candidates]
 return points,{'actual_native_button_edge_runs':[[v[0],v[-1]]for v in candidates],'observed_panel_padding':padding}

def buttons_view(label,enabled,reconnect=False):
 until=min(WORK,now()+20_000_000_000);attempts=[]
 publication=appearance();muted=color8(publication['appearance']['palette']['muted']);fg=color8(publication['appearance']['palette']['foreground'])
 for attempt in range(1,9):
  assert now()<until
  frame=capture(label+'-attempt-'+str(attempt),until);client=window();assert client and client['address']==window_address
  image=pixels(frame);count=1 if reconnect else 2
  try:points,edges=button_centres(frame,client,publication,count)
  except AssertionError as error:
   attempts.append({'attempt':attempt,'capture':frame,'expected_button_count':count,'edge_mismatch':str(error),'observed_at_boottime_ns':now(),'original_until_ns':until});tick();time.sleep(.25);continue
  padding=publication['appearance']['style']['spacing_tokens']['panelPadding'];left=round((client['at'][0]+padding)*2.625)+8;right=round((client['at'][0]+client['size'][0]-padding)*2.625)-8;counts=[]
  for x,y in points:
   top=round(y-56*2.625/2)+8;bottom=round(y+56*2.625/2)-8
   counts.append({'muted_text_pixels':sum(image.getpixel((px,py))==muted for py in range(top,bottom)for px in range(left,right)),'enabled_foreground_pixels':sum(image.getpixel((px,py))==fg for py in range(top,bottom)for px in range(left,right))})
  matched=counts[0]['enabled_foreground_pixels']>=10 if reconnect else all(v['enabled_foreground_pixels']>=10 if enabled else v['muted_text_pixels']>=10 and v['enabled_foreground_pixels']==0 for v in counts)
  attempts.append({'attempt':attempt,'capture':frame,'actual_button_pixel_counts':counts,'muted_RGB':muted,'enabled_foreground_RGB':fg,'matched':matched,'observed_at_boottime_ns':now(),'original_until_ns':until});assert now()<until
  if matched:return {'attempts':attempts,'qualified_native_capture':frame,'native_route_points':[]if reconnect else points,'native_reconnect_point':points[0]if reconnect else None,'actual_button_edges':edges,'routes_enabled':enabled,'actual_visible_route_count':0 if reconnect else 2,'Reconnect_visible_and_enabled':reconnect,'text_visual_review_required':True}
  tick();time.sleep(.25)
 raise AssertionError('normal actual Qt button state not observed within original finite repaint cutoff')

def grant(allowed):
 assert admin({'op':'grant','app':APP,'permission':'media_route','allowed':allowed})=={'result':'ok'}

def live_fd(fd):return fd is not None and not select.select([fd],[],[],0)[0]
def compositor_live():return live_fd(compositor_pidfd)
def bind_app_arguments(pid):
 global normal_argv_bound,normal_argv
 proc=Path('/proc')/str(pid);raw=(proc/'cmdline').read_bytes();assert len(raw)<=8192;argv=[v.decode()for v in raw.rstrip(b'\0').split(b'\0')]
 direct=[str(installed.BIN/'pokelinux-sounds'),str(installed.FIX/'sounds-exec')]
 if argv and argv[0]in direct:args=argv[1:]
 elif argv[:5]==[str(installed.BIN/'pokelinux-session-owner'),'app-entry','11042',str(actor['child'].pid),str(installed.FIX/'sounds-exec')]:args=argv[5:]
 else:return
 assert len(args)==6 and args[:2]==profile_args[:2]and args[3:]==profile_args[3:]
 path=Path(args[2]);assert re.fullmatch(re.escape(str(installed.DISPLAY))+r'/[0-9a-f]{32}/wayland\.sock',str(path))
 folder=path.parent.lstat();node=path.lstat();assert stat.S_ISDIR(folder.st_mode)and folder.st_uid==0 and folder.st_gid==11042 and stat.S_IMODE(folder.st_mode)==0o710
 assert stat.S_ISSOCK(node.st_mode)and node.st_uid==0 and node.st_gid==11042 and stat.S_IMODE(node.st_mode)==0o660 and node.st_nlink==1
 normal_argv=[str(installed.BIN/'pokelinux-sounds'),*args];normal_argv_bound=True
 if startup is not None:startup.profile_args=normal_argv
 result['actual_per_app_display']={'pathname':str(path),'instance':path.parent.name,'instance_is_numeric_app_generation':False,'directory_UID':folder.st_uid,'directory_GID':folder.st_gid,'directory_mode':'0710','socket_UID':node.st_uid,'socket_GID':node.st_gid,'socket_mode':'0660','original_Wayland_socket_selected_for_app':False,'actual_same_PID_arguments':normal_argv}
def bind_compositor_log(proc,row):
 global compositor_log
 if compositor_log is not None or not live_fd(compositor_pidfd):return
 candidates=[]
 try:
  assert now()<WORK and (proc/'stat').read_text().split(') ',1)[1].split()[19]==row['starttime']
  root=Path('/run/user/1000/hypr');info=root.lstat();assert stat.S_ISDIR(info.st_mode)and info.st_uid==info.st_gid==1000
  with os.scandir(root)as entries:
   for index,entry in enumerate(entries):
    assert index<16 and now()<WORK,'bounded passive instance search exhausted'
    if not entry.is_dir(follow_symlinks=False)or not re.fullmatch(re.escape(S['source_built_compositor']['source_commit'])+r'_[0-9]{1,20}_[0-9]{1,10}',entry.name):continue
    folder=lock=log=None
    try:
     folder=os.open(entry.path,os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK)
     directory=os.fstat(folder);assert directory.st_uid==directory.st_gid==1000
     lock=os.open('hyprland.lock',os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=folder)
     info=os.fstat(lock);assert stat.S_ISREG(info.st_mode)and info.st_uid==info.st_gid==1000 and info.st_nlink==1 and info.st_size<=128
     raw=os.read(lock,129)
     if raw!=(str(compositor_pid)+'\n'+installed.WAYLAND.name+'\n').encode():continue
     log=os.open('hyprland.log',os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=folder)
     info=os.fstat(log);assert stat.S_ISREG(info.st_mode)and info.st_uid==info.st_gid==1000 and info.st_nlink==1
     candidates.append((log,{'path':str(root/entry.name/'hyprland.log'),'instance':entry.name,'PID':compositor_pid,'proc_starttime':row['starttime'],'lock_sha256':hashlib.sha256(raw).hexdigest(),'lock_PID_and_Wayland_match':True,'log_device':info.st_dev,'log_inode':info.st_ino,'bound_boottime_ns':now(),'retained_before_read':True,'readiness_or_Source_health_inferred':False}));log=None
    except FileNotFoundError:continue
    finally:
     for fd in [log,lock,folder]:
      if fd is not None:os.close(fd)
  assert len(candidates)<=1,'ambiguous live-PID instance lock'
  assert now()<WORK and live_fd(compositor_pidfd)and (proc/'stat').read_text().split(') ',1)[1].split()[19]==row['starttime']
  if candidates:
   fd,binding=candidates[0];stream=os.fdopen(fd,'rb');candidates.pop();compositor_log=stream;owned_files.append(stream);result['passive_owned_compositor_log_binding']=binding
 except (OSError,AssertionError,ValueError)as error:
  if len(row['observation_errors'])<8:row['observation_errors'].append({'observer':'passive-instance-log-binding','error':type(error).__name__,'errno':getattr(error,'errno',None)})
 finally:
  for fd,_ in candidates:os.close(fd)

def watch():
 global app_pid,app_pidfd,compositor_pid,compositor_pidfd
 observed_actor=actor if actor is not None else units.authenticated_pending_owner()
 parents={v['child'].pid for v in [service,observed_actor]if v is not None and live_fd(v['pidfd'])and Path('/proc/'+str(v['child'].pid)).exists()}
 for _ in range(3):
  more=set()
  for parent in parents:
   try:children=(Path('/proc')/str(parent)/'task'/str(parent)/'children').read_text().split()
   except FileNotFoundError:continue
   for child in children:
    pid=int(child);more.add(pid);proc=Path('/proc')/child
    if pid not in watched:
     try:fd=os.pidfd_open(pid,os.O_NONBLOCK)
     except (FileNotFoundError,ProcessLookupError):continue
     watched[pid]={'fd':fd,'starttime':None,'cmdline_sha256':None,'UIDs':[],'parent_when_observed':parent,'observed_at_boottime_ns':now(),'observation_errors':[]}
    row=watched[pid]
    if not live_fd(row['fd']):continue
    try:
     text=(proc/'status').read_text();uids=next(v for v in text.splitlines()if v.startswith('Uid:')).split()[1:];row['UIDs']=uids
     row['starttime']=(proc/'stat').read_text().split(') ',1)[1].split()[19];row['cmdline_sha256']=hashlib.sha256((proc/'cmdline').read_bytes()).hexdigest()
     if observed_actor is not None and parent==observed_actor['child'].pid:
      if compositor_pid is None and uids==['1000']*4 and os.readlink(proc/'exe')=='/usr/bin/Hyprland':
       compositor_pidfd=os.dup(row['fd']);compositor_pid=pid
       assert next(v for v in text.splitlines()if v.startswith('Gid:')).split()[1:]==['1000']*4 and not next(v for v in text.splitlines()if v.startswith('Groups:')).split()[1:]
       assert sha(proc/'exe')==S['source_built_compositor']['new_ELF_sha256']
       result['actual_owned_compositor']={'actual_UIDs':[1000]*4,'actual_GIDs':[1000]*4,'actual_supplementary_groups':[],'PID':pid,'observed_parent_PID':parent,'retained_external_pidfd':True,'actual_exe_sha256':sha(proc/'exe'),'actual_direct_parent_wait_claimed_by_guest':False,'owner_identity':observed_actor['identity'],'owner_readiness_inferred':False}
      if compositor_pid==pid:bind_compositor_log(proc,row)
     if actor is not None and parent==actor['child'].pid:
      if app_pid is None and uids==['11042']*4:
       app_pidfd=os.dup(row['fd']);app_pid=pid
       result['startup_app_pidfd_binding']={'app_PID':pid,'actor_PID':parent,'actual_UIDs':[11042]*4,'bound_boottime_ns':now(),'before_qualified_frame_prerequisite':True}
      if app_pid==pid and not normal_argv_bound:bind_app_arguments(pid)
    except OSError as error:
     if len(row['observation_errors'])<8:row['observation_errors'].append({'error':type(error).__name__,'errno':error.errno})
    if app_pid==pid and startup is not None and normal_argv_bound and not startup.rows:startup.record('first-owned11042-descendant',pid,app_pidfd,expected_starttime=row['starttime'])
  parents=more
 for row in watched.values():
  if not row.get('dead')and not live_fd(row['fd']):row['dead']=True;row['observed_dead_boottime_ns']=now()
 if FOCUS_LOG.exists():assert FOCUS_LOG.stat().st_size<=512*768,'bounded focus witness output exceeded'
units.passive_observer=watch
def control(request,absolute_cutoff=None):
 until=min(WORK,now()+5_000_000_000,absolute_cutoff if absolute_cutoff is not None else WORK)
 def remaining(sock):assert now()<until;sock.settimeout(max(.001,(until-now())/1e9))
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as sock:
  remaining(sock);sock.connect(str(hypr_control));assert struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(compositor_pid,1000,1000)
  remaining(sock);sock.sendall((request if request.startswith('j/')else'/'+request).encode());assert now()<until;body=bytearray()
  while True:
   remaining(sock);part=sock.recv(65536)
   if not part:break
   body.extend(part);assert len(body)<=131072
  assert now()<until;return body.decode()
def request_source_unload():
 # One authenticated full send is the request witness; a reply can race Source fencing.
 until=min(WORK,now()+5_000_000_000);payload=('/plugin unload '+str(installed.PLUGIN/'session-source.so')).encode()
 row={'started_boottime_ns':now(),'original_until_ns':until,'request':payload.decode(),'single_request_no_retry':True,'full_send_completed':False,'reply_EOF':False,'reply_error':None,'confirmed_plugin_unload_claimed':False};result['Source_unload_request']=row
 def remaining(sock):
  if now()>=until:raise TimeoutError('original5s unload reply observation cutoff')
  sock.settimeout(max(.001,(until-now())/1e9))
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as sock:
  remaining(sock);sock.connect(str(hypr_control));peer=struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer==(compositor_pid,1000,1000);row['actual_peer_PID_UID_GID']=list(peer)
  remaining(sock);sock.sendall(payload);row['full_send_completed']=True;row['full_send_completed_boottime_ns']=now();assert now()<until
  body=bytearray()
  try:
   while True:
    remaining(sock);part=sock.recv(65536)
    if not part:row['reply_EOF']=True;break
    body.extend(part);assert len(body)<=131072
  except OSError as error:row['reply_error']={'type':type(error).__name__,'errno':error.errno,'message':str(error)[:256]}
  row['reply']=body.decode(errors='replace');row['reply_observation_completed_boottime_ns']=now();row['reply_observation_within_original_cutoff']=now()<until
 assert row['full_send_completed']and row['full_send_completed_boottime_ns']<until
 return row
def window():
 assert app_pid is not None and live_fd(app_pidfd)and live(actor)and compositor_live()and live(service)and live(broker)
 rows=[v for v in json.loads(control('j/clients'))if v.get('pid')==app_pid and v.get('mapped')];assert len(rows)<=1
 if not rows:return
 v=rows[0];assert v['title']=='Sounds'and not v['xwayland'];return v
def seq(path,value,absolute_cutoff=None,observed_actor=None):
 selected_actor=actor if observed_actor is None else observed_actor
 until=min(WORK,now()+2_000_000_000,absolute_cutoff if absolute_cutoff is not None else WORK)
 def remaining(sock):assert now()<until;sock.settimeout(max(.001,(until-now())/1e9))
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)as sock:
  sock.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);remaining(sock);sock.connect(str(path));peer=struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert selected_actor is not None and peer==(selected_actor['child'].pid,0,0)
  packet=json.dumps(value,separators=(',',':')).encode();assert len(packet)<=4096;remaining(sock);assert sock.send(packet)==len(packet);remaining(sock)
  body,anc,flags,_=sock.recvmsg(4096,socket.CMSG_SPACE(12));assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC)and len(anc)==1 and anc[0][:2]==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS)and struct.unpack('3i',anc[0][2])==peer and now()<until
  answer=json.loads(body);result.setdefault('actual_root_actor_requests',[]).append({'request':value,'response':answer,'peer':peer,'at_boottime_ns':now(),'original_until_ns':until,'path':str(path)});return answer
def admin(value,absolute_cutoff=None,observed_actor=None):return seq(RUN/'admin.sock',value,absolute_cutoff=absolute_cutoff,observed_actor=observed_actor)
def quiet():
 v=matching(None,None)
 return v if v and v['owner']['desired']==[]else None
def native_focus(label):
 until=min(WORK,now()+5_000_000_000);value=json.loads(control('pokelinux_audio_focus',absolute_cutoff=until));assert now()<until and value['schema']==1 and value['boottime_ns']<=now()
 result.setdefault('actual_native_focus_queries',[]).append({'label':label,'value':value,'observed_at_boottime_ns':now(),'original_until_ns':until,'not_a_replacement_Source_packet':True});return value
def app_focus():
 value=native_focus('wait-owned-app-focus')
 return value if value['pid']==app_pid and value['uid']==value['gid']==11042 and value['native_window_eligible']and value['peer_pidfd_live']and value['peer_pidfd_cloexec']else None
def layer_focus():
 value=native_focus('wait-exclusive-layer-null-native-focus')
 return value if value['pid']==focus_layer['child'].pid and value['uid']==value['gid']==1000 and value['keyboard_surface_present']and not value['native_window_eligible']and value['peer_pidfd_live']else None
def focus_interval(before,after):
 assert FOCUS_LOG.stat().st_size<=512*768;lines=FOCUS_LOG.read_bytes().splitlines();assert len(lines)<=512 and all(len(line)+1<=768 for line in lines);rows=[json.loads(line)for line in lines]
 assert [r['sequence']for r in rows]==list(range(1,len(rows)+1));selected=[r for r in rows if before['sequence']<=r['sequence']<=after['sequence']];assert selected and selected[0]==before and selected[-1]==after
 assert all(r['pid']==focus_layer['child'].pid and r['uid']==r['gid']==1000 and r['keyboard_surface_present']and not r['native_window_eligible']and r['peer_pidfd_live']for r in selected)
 return {'rows':selected,'native_focus_null_through_actual_touch':True,'Rust_Source_channel_received_by_guest':False,'scope':'original compositor query/event witness uses the same native-window predicate as selected Source adapter; actual Rust Source acceptance follows selected owner/authority and observed UI/effects'}

def start_tone(label,quiet=False):
 p=R/'call-tone.f32le'
 if not p.exists():
  with p.open('wb')as f:
   cycle=array.array('f',(0.25*math.sin(2*math.pi*880*i/48000)for i in range(48000))).tobytes()
   for _ in range(300):f.write(cycle)
  p.chmod(0o444)
 assert p.stat().st_size==300*48000*4 and sys.byteorder=='little'
 stream=p.open('rb');owned_files.append(stream)
 args=['/usr/bin/pw-cat','--playback','--raw','--format','f32','--rate','48000','--channels','1','--channel-map','MONO','--target','0','-P','{"node.name":"source.call","node.autoconnect":false,"node.dont-reconnect":true}','-']
 item=start(args,'call-tone-'+label+'.log',env=audio_env,user=11040,stdin=stream,allowed=(0,1,-15))
 def found():
  v=graph()
  if quiet:assert not v['service_input_links'],'service-source replay during fixture tone setup'
  return v['node_ids'].get('source.call')
 node=wait(found,5)
 command(['/usr/bin/pw-cli','set-param',str(node),'PortConfig','{ direction = Output mode = dsp monitor = false format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }'],'call-port-format.log',env=audio_env,user=11040,seconds=3)
 receipt={'frequency':880,'amplitude':.25,'frames':300*48000,'format':'F32LE','rate':48000,'channels':1,'path':str(p),'sha256':sha(p),'actual_child_pid':item['child'].pid,'argv':args,'finite_duration_seconds':300,'not_production_PCM_broker':True}
 (R/'tone-source.json').write_text(json.dumps(receipt,indent=2)+'\n');(R/'tone-source.json').chmod(0o444);result.setdefault('actual_call_tone_sources',[]).append(receipt);return item

# Private fixed ABI: little-endian 64-byte header + 128 24-byte numeric records.
EVENT_FILE=Path('/run/user/1000/pixel10-profile/startup-events.bin')
EVENT_HEADER=struct.Struct('<8sIIIIQQ3Q');EVENT_RECORD=struct.Struct('<QQII')
EVENT_BYTES=4096;EVENT_CAPACITY=128

def prepare_startup_events():
 assert now()<WORK
 # Check the actual longest enclosing mount, not merely a /run pathname.
 with Path('/proc/self/mountinfo').open('rb')as stream:mountinfo=stream.read(131073)
 assert len(mountinfo)<=131072
 mounts=[]
 for line in mountinfo.decode().splitlines():
  left,right=line.split(' - ',1);mount=left.split()[4]
  if str(EVENT_FILE)==mount or str(EVENT_FILE).startswith(mount.rstrip('/')+'/'):mounts.append((len(mount),right.split()[0]))
 assert mounts and max(mounts)[1]=='tmpfs','startup event file requires actual tmpfs'
 parent=EVENT_FILE.parent.lstat();assert stat.S_ISDIR(parent.st_mode)and parent.st_uid==parent.st_gid==1000 and stat.S_IMODE(parent.st_mode)==0o700
 header=EVENT_HEADER.pack(b'PLSEVT01',1,64,24,EVENT_CAPACITY,0,0,0,0,0);data=header+bytes(EVENT_BYTES-len(header))
 fd=os.open(EVENT_FILE,os.O_CREAT|os.O_EXCL|os.O_RDWR|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK,0o600)
 try:
  os.fchown(fd,1000,1000);os.fchmod(fd,0o600);os.ftruncate(fd,EVENT_BYTES)
  assert os.write(fd,data)==EVENT_BYTES
  info=os.fstat(fd);assert stat.S_ISREG(info.st_mode)and info.st_nlink==1 and info.st_size==EVENT_BYTES
 finally:os.close(fd)
 assert now()<WORK
 result['startup_event_file']={'path':str(EVENT_FILE),'device':info.st_dev,'inode':info.st_ino,'bytes':EVENT_BYTES,'UID':info.st_uid,'GID':info.st_gid,'mode':oct(stat.S_IMODE(info.st_mode)),'initial_sha256':hashlib.sha256(data).hexdigest(),'actual_enclosing_filesystem':'tmpfs','prepared_before_native_owner':True,'schema':1,'capacity':EVENT_CAPACITY}

def collect_startup_events(row,until):
 observed={'attempted':False,'original_until_ns':min(until,WORK),'only_after_retained_compositor_pidfd_dead':True,'inflight_or_cause_inference_permitted':False,'absence_deductions_disallowed':True};row['startup_event_witness']=observed;fd=None
 try:
  if compositor_pidfd is None:observed['not_attempted_reason']='no_retained_compositor_pidfd';return
  if now()>=min(until,WORK):observed['not_attempted_reason']='original_observation_or_work_cutoff_elapsed';return
  if live_fd(compositor_pidfd):observed['not_attempted_reason']='retained_compositor_pidfd_still_live';return
  observed['retained_compositor_pidfd_dead_observed_at_boottime_ns']=now()
  allocated=result.get('startup_event_file')
  if allocated is None:observed['not_attempted_reason']='file_not_prepared';return
  observed['attempted']=True;fd=os.open(EVENT_FILE,os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK);before=os.fstat(fd)
  assert stat.S_ISREG(before.st_mode)and before.st_uid==before.st_gid==1000 and stat.S_IMODE(before.st_mode)==0o600 and before.st_nlink==1 and before.st_size==EVENT_BYTES
  assert (before.st_dev,before.st_ino)==(allocated['device'],allocated['inode']),'same preallocated file required'
  assert now()<min(until,WORK);raw=os.read(fd,EVENT_BYTES+1);after=os.fstat(fd);assert len(raw)==EVENT_BYTES and (before.st_size,before.st_mtime_ns)==(after.st_size,after.st_mtime_ns)
  assert now()<min(until,WORK);target=R/'startup-events-after-death.bin'
  with target.open('xb')as saved:saved.write(raw)
  observed.update({'raw_file':target.name,'raw_bytes':len(raw),'raw_sha256':hashlib.sha256(raw).hexdigest(),'source_device':before.st_dev,'source_inode':before.st_ino,'regular_file_not_PIPE_EOF':True,'uncommitted_tail_ignored':True})
  magic,version,header_bytes,record_bytes,capacity,count,flags,*reserved=EVENT_HEADER.unpack_from(raw)
  observed.update({'committed_count':count,'flags':flags,'overflow':bool(flags&1),'clock_error':bool(flags&2),'retired':bool(flags&4),'unknown_flag_bits':flags&~7})
  assert (magic,version,header_bytes,record_bytes,capacity)==(b'PLSEVT01',1,64,24,EVENT_CAPACITY)and reserved==[0,0,0]and count<=EVENT_CAPACITY
  names={1:'witness_armed',2:'config.preReload',3:'config.reloaded',4:'config.props_refreshed'};records=[];last_stamp=0
  for index in range(count):
   ordinal,stamp,kind,detail=EVENT_RECORD.unpack_from(raw,64+index*24)
   assert ordinal==index+1 and stamp>=last_stamp and kind in names and(detail in [0,1]if kind==4 else detail==0)
   records.append({'ordinal':ordinal,'boottime_ns':stamp,'kind':kind,'event':names[kind],'execd_as_scheduled':bool(detail)if kind==4 else None});last_stamp=stamp
  armed=sum(record['kind']==1 for record in records);assert armed<=1 and(not armed or records[0]['kind']==1)
  observed.update({'records':records,'witness_armed':armed==1,'missing_witness_armed':armed==0,'decoded_committed_prefix_valid':True,'uncommitted_tail_nonzero':any(raw[64+count*24:64+EVENT_CAPACITY*24]),'absence_deductions_disallowed':bool(flags)or not armed,'completion_absence_does_not_establish_inflight':True})
 except BaseException as error:observed.update({'error':type(error).__name__,'errno':getattr(error,'errno',None),'detail':str(error)[:512],'absence_deductions_disallowed':True})
 finally:
  if fd is not None:os.close(fd)
  observed['completed_at_boottime_ns']=now();observed['within_original_observation_and_work_cutoffs']=now()<until and now()<WORK

def failure_observations():
 until=min(CLOSE,now()+2_000_000_000);row={'observed_at_boottime_ns':now(),'original_observation_until_ns':until,'before_native_owner_Stop':True,'compositor_PID':compositor_pid,'retained_compositor_pidfd_present':compositor_pidfd is not None,'existing_logs':[],'Source_health_not_inferred_from_admin_ready_or_Stop':True};result['failure_before_cleanup']=row
 if compositor_pidfd is not None:row['retained_compositor_pidfd_live']=live_fd(compositor_pidfd)
 if compositor_pid is not None:row['compositor_proc_absent']=not Path('/proc/'+str(compositor_pid)).exists()
 collect_startup_events(row,until)
 observation={'before_native_owner_Stop':True,'original_until_ns':min(until,WORK),'attempted':False,'Source_observed_is_historical_not_current_health':True,'first_failure_timestamp_is_actor_observation_not_cause':True};row['root_admin_Source_observation']=observation
 observed_actor=actor if actor is not None else units.authenticated_pending_owner()
 if actor is None and observed_actor is not None:observation['authenticated_owner_identity_before_notify_ready']=observed_actor['identity'];observation['owner_Readiness_not_promoted']=True
 try:
  if observed_actor is None:observation['not_attempted_reason']='no_authenticated_owned_actor'
  elif now()>=min(until,WORK):observation['not_attempted_reason']='original_observation_or_work_cutoff_elapsed'
  elif not live(observed_actor):observation['not_attempted_reason']='same_owned_actor_not_live'
  else:
   observation['attempted']=True;observation['request_started_at_boottime_ns']=now();value=admin({'op':'snapshot'},absolute_cutoff=until,observed_actor=observed_actor)
   observation['reply']=value;observation['reply_observed_at_boottime_ns']=now();observation['authenticated_peer_PID_UID_GID']=result['actual_root_actor_requests'][-1]['peer']
   assert type(value)is dict and value['result']=='snapshot'and type(value['source'])is dict
   observation['source']=value['source'];observation['retained_without_normalization']=True
 except BaseException as error:observation.update({'error':type(error).__name__,'errno':getattr(error,'errno',None),'detail':str(error)[:512]})
 observation['completed_at_boottime_ns']=now();observation['within_original_observation_and_work_cutoffs']=now()<until and now()<WORK
 paths=[];control_path=globals().get('hypr_control')
 if compositor_log is not None:paths.append((Path(result['passive_owned_compositor_log_binding']['path']),'failure-Hyprland-existing-instance.log'))
 elif isinstance(control_path,Path)and control_path.parent.parent==Path('/run/user/1000/hypr'):paths.append((control_path.parent/'hyprland.log','failure-Hyprland-existing-instance.log'))
 if compositor_pid is not None:paths.append((Path('/home/alarm/.cache/hyprland')/('hyprlandCrashReport'+str(compositor_pid)+'.txt'),'failure-Hyprland-existing-owned-PID-crash.txt'))
 assert len(paths)<=2
 for source,name in paths:
  log={'source':str(source),'output':name,'maximum_retained_bytes':262144,'regular_file_not_PIPE_EOF':True};row['existing_logs'].append(log);fd=None
  try:
   assert now()<until
   retained=compositor_log is not None and name=='failure-Hyprland-existing-instance.log'
   fd=os.dup(compositor_log.fileno())if retained else os.open(source,os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW|os.O_NONBLOCK)
   before=os.fstat(fd);assert stat.S_ISREG(before.st_mode)and before.st_uid==1000,'expected owned UID1000 regular log'
   log['retained_live_PID_lock_bound_descriptor']=retained
   if retained:
    binding=result['passive_owned_compositor_log_binding'];assert(before.st_dev,before.st_ino)==(binding['log_device'],binding['log_inode']);log['original_live_binding']=binding
   data=bytearray()
   while len(data)<262144:
    assert now()<until;chunk=os.read(fd,min(65536,262144-len(data)))
    if not chunk:break
    data.extend(chunk)
   after=os.fstat(fd);target=R/name
   with target.open('xb')as out:out.write(data)
   log.update({'bytes':len(data),'sha256':sha(target),'source_device':before.st_dev,'source_inode':before.st_ino,'source_UID':before.st_uid,'source_GID':before.st_gid,'source_mode':oct(stat.S_IMODE(before.st_mode)),'source_bytes_before':before.st_size,'source_bytes_after':after.st_size,'source_mtime_ns_before':before.st_mtime_ns,'source_mtime_ns_after':after.st_mtime_ns,'retained_prefix_truncated_or_growing':max(before.st_size,after.st_size)>len(data),'PIPE_EOF_claimed':False})
  except BaseException as error:log.update({'error':type(error).__name__,'errno':getattr(error,'errno',None),'detail':str(error)[:512]})
  finally:
   if fd is not None:os.close(fd)
 row['completed_at_boottime_ns']=now();row['within_original_observation_and_guest_cutoffs']=now()<until and now()<CLOSE

try:
 result['kernel']=command(['/usr/bin/uname','-a'],'kernel.log');assert sha('/usr/bin/Hyprland')==S['expected_hyprland_elf_sha256']
 packages=command(['/usr/bin/pacman','-Q'],'installed-packages.log');assert hashlib.sha256(packages.encode()).hexdigest()==S['installed_package_list_sha256']
 result['native_guest_preflight']=native_support.preflight(R,S,command)
 # Signed367 preflight above is preserved; only now install the separate local cohort.
 result['source_built_compositor_install']=local_compositor.install(R,S,command)
 normal,buildproof=qt_native.build(command,R);result['current_native_Qt_build']=buildproof;result['current_epoch_Qt_build_or_test_executed']=True;result['prior_ARM_Qt_results_relabelled_as_native']=False
 for uid,name in [(11040,'sounds-service'),(11041,'sounds-call-fixture'),(11042,'sounds-app')]:command(['/usr/bin/useradd','-u',str(uid),'-M','-s','/usr/bin/nologin',name],'useradd-'+name+'.log')
 command(['/usr/bin/usermod','-a','-G','video,input','alarm'],'desktop-support-groups.log');command(['/usr/bin/modprobe','virtio_gpu'],'GPU-module.log');command(['/usr/bin/modprobe','uinput'],'uinput-module.log');command(['/usr/bin/udevadm','settle'],'udev-settle.log')
 for path,uid in [('/run/user/1000',1000),('/run/user/11042',11042),('/var/lib/pokelinux-sounds',11042)]:installed.directory(path,0o700,uid)
 Path('/home/alarm').chmod(0o700);os.chown(R,11040,11040);(R/'pcm').mkdir(mode=0o700);os.chown(R/'pcm',11040,11040)
 for path in [installed.BIN,installed.FIX,installed.CONFIG,installed.RUN,installed.DISPLAY,installed.POLICY]:installed.directory(path,0o700 if path==installed.POLICY else 0o755)
 for name in ['native-appearance','pokelinux-audio','pokelinux-sounds']:installed.program(normal if name=='pokelinux-sounds'else R/'bins'/name,installed.BIN/name)
 shutil.copyfile(R/'source/call-client.py',installed.FIX/'call-client.py');(installed.FIX/'call-client.py').chmod(0o444)
 proposal=installed.directory('/proposal')
 for name in ['pcm_observer.py','ORACLES.json']:shutil.copyfile(R/'source/observer'/name,proposal/name);(proposal/name).chmod(0o444)
 public_profile=installed.directory('/usr/share/pokelinux/display-fixture')
 for name in ['pixel10.json','measure.py']:shutil.copyfile(R/'source'/name,public_profile/name);(public_profile/name).chmod(0o444)
 installed.document(public_profile/'public-api-pins.json',result['source_built_compositor_install']['runtime_public_api'])
 profile_bins=installed.FIX
 result['current_native_display_helper_build']=native_support.build_display_helpers(R,command,profile_bins)
 original=Path('/usr/share/pokelinux/omarchy');theme_host=original/'shell'
 desktop_env={'HOME':'/home/alarm','USER':'alarm','LOGNAME':'alarm','LANG':'C.UTF-8','PATH':'/usr/bin','XDG_RUNTIME_DIR':'/run/user/1000','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','POKELINUX_DISPLAY_PROFILE':str(public_profile/'pixel10.json'),'POKELINUX_DISPLAY_PROFILE_RECEIPT':'/run/user/1000/pixel10-profile/applied.json'}
 display.prepare_display_receipts()
 prepare_startup_events()
 desktop_bus=start(['/usr/bin/dbus-daemon','--session','--nofork','--address=unix:path=/run/user/1000/bus','--print-address=1'],'desktop-private-bus.log',env=desktop_env,user=1000,allowed=(0,-15));desktop_env['DBUS_SESSION_BUS_ADDRESS']=wait(lambda:first_line(desktop_bus),5)
 # Source bootstrap deliberately keeps groups[]; only this private fixture seatd socket admits gid1000.
 seatd=start(['/usr/bin/seatd','-g','alarm'],'seatd-gid1000.log',env=dict(os.environ,SEATD_VTBOUND='0'),allowed=(0,-15))
 result['new_native_plugin_build']=installed.compile_plugins(R,command)
 discovery_source=R/'source/qt-output-discovery.cpp';discovery_before=sha(discovery_source)
 discovery_flags=shlex.split(command(['/usr/bin/pkg-config','--cflags','--libs','Qt6Gui','wayland-client','libdrm'],'output-discovery-SDK-flags.log',seconds=5))
 discovery_probe=profile_bins/'pixel10-output-discovery'
 discovery_argv=['/usr/bin/g++','-std=c++23','-Wall','-Wextra','-Werror','-O2','-g0','-fPIC',str(discovery_source),*discovery_flags,'-o',str(discovery_probe)]
 command(discovery_argv,'compile-nonpresenting-output-discovery.log',seconds=180,limit=1048576);assert sha(discovery_source)==discovery_before;discovery_probe.chmod(0o755)
 result['new_native_output_discovery_build']={'source_sha256':discovery_before,'output_sha256':sha(discovery_probe),'argv':discovery_argv,'compiled_before_owner_spawn':True,'runtime_qualification_inherited':False}
 root,private,control_dir,profile,audio_env=audio_config.configure();audio_socket=control_dir/'audio.sock'
 private_bus=start(['/usr/bin/dbus-daemon','--session','--nofork','--address=unix:path=/tmp/pokelinux-native-audio/private/bus','--print-address=1'],'audio-private-bus.log',env=audio_env,user=11040,allowed=(0,-15));audio_env['DBUS_SESSION_BUS_ADDRESS']=wait(lambda:first_line(private_bus),5)
 pipewire=start(['/usr/bin/pipewire','-c',str(root/'config/pipewire.conf')],'private-PipeWire.log',env=audio_env,user=11040,allowed=(0,-15));wait(lambda:graph()['node_ids'].get('source.media'),10)
 tone=start_tone('backend-catalog-only');wireplumber=start(['/usr/bin/wireplumber','--profile','native-audio-test'],'private-WirePlumber.log',env=audio_env,user=11040,allowed=(0,-15));result['actual_WirePlumber_initial_connection']=wait(wireplumber_ready,10)
 # Dedicated output tmpfs; reserve cleanup before fallible admission/post-command hooks.
 capture_dir=installed.directory(installed.HOME/'captures',0o700,11042);capture_mounted=True
 command(['/usr/bin/mount','-t','tmpfs','-o','size=8388608,mode=0700,uid=11042,gid=11042,nosuid,nodev,noexec','sounds-capture-files',str(capture_dir)],'capture-tmpfs-mount.log',seconds=5)
 mountlines=[line for line in Path('/proc/self/mountinfo').read_text().splitlines()if line.split(' - ',1)[0].split()[4]==str(capture_dir)];assert len(mountlines)==1;left,right=mountlines[0].split(' - ',1)
 assert right.split()[0]=='tmpfs'and {'rw','nosuid','nodev','noexec'}<=set(left.split()[5].split(','));vfs=os.statvfs(capture_dir);st=capture_dir.lstat();assert vfs.f_blocks*vfs.f_frsize==8388608 and st.st_uid==st.st_gid==11042 and stat.S_IMODE(st.st_mode)==0o700 and st.st_dev!=installed.HOME.stat().st_dev
 result['capture_tmpfs']={'mountinfo_line':mountlines[0],'aggregate_hard_bytes':8388608,'UID':11042,'GID':11042,'mode':'0700','HOME_on_distinct_overlay_device':True,'accepted_metadata_bytes':16392,'accepted_stderr_bytes':4194304,'global_RLIMIT_FSIZE_override_selected':False}
 metadata_file=capture_dir/'normal-qt-window.jsonl';stderr_file=capture_dir/'normal-qt.stderr'
 for path in [metadata_file,stderr_file]:path.touch(exist_ok=False);path.chmod(0o600);os.chown(path,11042,11042)
 installed.deploy(R,desktop_env,audio_socket,normal)  # existing binaries/directories; operational selection below is explicit.
 result['actual_Root_installed_bundle']=install_bundle(R,command)
 config=installed.CONFIG/'session.json';selection=json.loads(config.read_text());app_profile=json.loads(Path(selection['profiles']).read_text());profile_args=app_profile['apps'][0]['args'];normal_argv=[str(installed.BIN/'pokelinux-sounds'),*profile_args]
 result['actual_installed_config']={'path':str(config),'sha256':sha(config),'selection':selection,'profile':app_profile,'private_capture_instrumentation_only':True,'production_profile_argument_count':6}
 result['phase']='cold-installed-target-startup'
 units.start()
 specifications=[('pokelinux-normal-audio.service',11040,str(installed.BIN/'pokelinux-audio'),[str(installed.BIN/'pokelinux-audio'),str(audio_socket),str(profile),'11041','0']),('pokelinux-normal-appearance.service',0,str(installed.BIN/'native-appearance'),[str(installed.BIN/'native-appearance'),'--serve','/run/pokelinux/appearance.sock',str(installed.CONFIG/'appearance.json')]),('pokelinux-session-owner.service',0,str(installed.BIN/'pokelinux-session-owner'),[str(installed.BIN/'pokelinux-session-owner'),str(config)]),('pokelinux-omarchy-shell.service',1000,str(Path('/usr/bin/quickshell').resolve()),['/usr/bin/quickshell','-n','-p',str(theme_host)])]
 def required_units_started():
  global service,broker,actor,theme_shell
  items=[]
  for variable,spec in zip(['service','broker','actor','theme_shell'],specifications):
   launcher=({'exe':'/usr/libexec/pokelinux/presentation-environment','argv':['/usr/libexec/pokelinux/presentation-environment','launch'],'sha256':'8bab91fd79d54f96fe18af58053d7c907d5417c0ddfca5e8517171db791c6301'} if variable=='theme_shell' else None)
   item=units.bind(*spec,launcher=launcher);items.append(item)
   if item is not None:globals()[variable]=item
  return items if all(items)else None
 service,broker,actor,theme_shell=wait(required_units_started,120)
 result['actual_installed_PID1_identities']={item['unit']:item['identity']for item in [service,broker,actor,theme_shell]}
 result['theme_provisioning_PID1_completion']=units.completed_theme()
 wait(lambda:audio_socket.is_socket(),10)
 call_item=start(['/usr/bin/python3',str(installed.FIX/'call-client.py'),str(audio_socket),str(service['child'].pid),str(os.getpid())],'privileged-readonly-call-client.log',user=11041,stdin=subprocess.PIPE)
 ready=wait(lambda:first_line(call_item),5);ready=json.loads(ready);assert ready=={'ready':True,'uid':11041,'server_pid':service['child'].pid};call_offset=1;result['actual_readonly_call_client_ready']=ready
 wait(lambda:status()['observed']['available'],10);assert wait(quiet,10)

 def compositor_ready():
  assert all(live(item)for item in [actor,service,broker,theme_shell]),'required installed unit exited during cold startup'
  if compositor_pid is None:return
  assert compositor_live(),'owned compositor died during cold startup'
  if not installed.WAYLAND.is_socket():return
  controls=list(Path('/run/user/1000/hypr').glob('*/.socket.sock'));matches=[]
  for path in controls:
   try:
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as probe:
     probe.settimeout(.05);probe.connect(str(path));peer=struct.unpack('3i',probe.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
    if peer==(compositor_pid,1000,1000):matches.append(path)
   except OSError:continue
  assert len(matches)<=1
  if not matches:return
  ordered=Path('/run/user/1000/pixel10-profile/ordered-load.json')
  try:
   with ordered.open('rb')as stream:
    info=os.fstat(stream.fileno());assert stat.S_ISREG(info.st_mode)and info.st_uid==1000 and stat.S_IMODE(info.st_mode)==0o600 and info.st_size<=4096
    raw=stream.read(4097);assert len(raw)<=4096;value=json.loads(raw)
  except(FileNotFoundError,json.JSONDecodeError):return
  assert type(value)is dict and set(value)=={'schema','private_fixture_only','pid','uid','attempted_before_load','ready_callback_boottime_ns','physical_load_completed_boottime_ns','fixed_plugin','actual_applied_receipt','actual_applied_receipt_bytes','synchronous_original_config_loader','Source_FD_or_facts_accessed'}
  assert type(value['schema'])is int and value['schema']==1 and value['private_fixture_only']is True
  assert type(value['pid'])is int and value['pid']==compositor_pid and type(value['uid'])is int and value['uid']==1000
  assert value['attempted_before_load']is True and value['synchronous_original_config_loader']is True and value['Source_FD_or_facts_accessed']is False
  assert type(value['ready_callback_boottime_ns'])is int and type(value['physical_load_completed_boottime_ns'])is int and 0<value['ready_callback_boottime_ns']<=value['physical_load_completed_boottime_ns']<=now()
  assert value['fixed_plugin']==str(installed.FIX/'pixel10-physical-size-adapter.so')and value['actual_applied_receipt']==str(ordered.parent/'applied.json')
  assert type(value['actual_applied_receipt_bytes'])is int and 0<value['actual_applied_receipt_bytes']<=65536
  return matches[0]
 hypr_control=wait(compositor_ready,120);desktop_env.update(WAYLAND_DISPLAY=installed.WAYLAND.name,HYPRLAND_INSTANCE_SIGNATURE=hypr_control.parent.name)
 # Source autoload is solely the unchanged selected installed Lua. No warmup or manual load.
 def policy_service_ready():
  assert live(actor)and compositor_live()
  if not(RUN/'admin.sock').is_socket():return
  value=admin({'op':'snapshot'});return value if value['ready']and not value['locked']and not value['degraded']else None
 result['actual_policy_service_ready']=wait(policy_service_ready,120)
 def cold_source_healthy():
  assert all(live(item)for item in [actor,service,broker,theme_shell])and compositor_live(),'required installed process exited before real Source health'
  value=admin({'op':'snapshot'});result.setdefault('first_authenticated_cold_Source_snapshot',value)
  assert value['source']['first_failure']is None,('actual cold Source failure',value)
  return value if value['source']['state']=='observed'else None
 result['actual_cold_Source_healthy']=wait(cold_source_healthy,120)
 result['Source_observation_scope']={'real_Rust_observer_selected':True,'initial_Source_cap_ns':120_000_000_000,'bootstrap_receive_cap_ns':5_000_000_000,'sample_budget_ns':100_000_000,'wire_frames_mirrored_by_guest':False,'independent_focus_witness':'actual native compositor predicate plus actor-gated UI/effect; no authored Source snapshots'}
 display_discovery=display.discover_display(R,public_profile,profile_bins,command,desktop_env,result,compositor_pid)
 result['phase']='cold-full-shell-and-Source'
 assert all(live(item)for item in [actor,service,broker,theme_shell])
 ping=command(['/usr/bin/quickshell','ipc','-n','-p',str(theme_host),'call','shell','ping'],'genuine-full-shell-ping.log',env=desktop_env,user=1000,seconds=5)
 assert ping.strip()=='ok'
 def full_shell_surface():
  members=set(units.collect_members(theme_shell));value=json.loads(control('j/layers'));matches=[]
  def visit(v):
   if isinstance(v,dict):
    if type(v.get('pid'))is int and v['pid']in members:matches.append(v)
    for child in v.values():visit(child)
   elif isinstance(v,list):
    for child in v:visit(child)
  visit(value);return matches or None
 shell_surfaces=wait(full_shell_surface,10);result['genuine_full_shell_executed']=True
 result['genuine_full_shell']={'upstream_base_files':196,'namespace_only_modified_QML_files':54,'genuine_theme_asset_files':40,'actual_unit_MainPID':theme_shell['child'].pid,'same_Wayland_path':str(installed.WAYLAND),'authenticated_compositor_layers_query_PID':compositor_pid,'actual_layer_rows_with_PID_in_shell_unit_cgroup':shell_surfaces,'literal_Wayland_socket_client_peer_inspection_claimed':False,'actual_shell_ping':ping.strip(),'palette_snapshot_or_live_theme_qualified':False,'startup_unit_environment_unchanged':False,'manager_main_PID_is_not_guest_child':True}
 published=appearance();assert published['available']
 denied='import os,socket,json,sys;assert os.getuid()==11042 and os.getgroups()==[];s=socket.socket(socket.AF_UNIX);s.settimeout(2);s.connect(sys.argv[1]);s.sendall((json.dumps({"op":"acquire","role":"media","route":"media-a7"})+"\\n").encode());v=json.loads(s.makefile("rb").readline(65537));assert v=={"ok":False,"error":"denied"},v;print(json.dumps(v))'
 result['actual11042_direct_audio_denied']=command(['/usr/bin/python3','-c',denied,str(audio_socket)],'direct-app-audio-denial.log',user=11042,seconds=3)
 no_original='import os,socket,sys,errno;assert os.getuid()==11042 and os.getgroups()==[];s=socket.socket(socket.AF_UNIX);s.settimeout(2)\ntry:s.connect(sys.argv[1]);raise AssertionError("original Wayland unexpectedly reachable")\nexcept PermissionError as e:assert e.errno==errno.EACCES;print("original_Wayland_EACCES")'
 result['actual11042_original_Wayland_denied']=command(['/usr/bin/python3','-c',no_original,str(installed.WAYLAND)],'original-Wayland-denied.log',user=11042,seconds=3)
 startup=startup_observer.StartupObserver(R,WORK,actor['child'].pid,normal_argv,control)
 client=installed.BIN/'sounds-launch';desktop_file=Path('/usr/share/applications/org.omarchy.Sounds.desktop');assert desktop_file.read_text().split('Exec=',1)[1].splitlines()[0]==str(client)+' '+str(RUN/'launch-sounds.sock')
 command([str(client),str(RUN/'launch-sounds.sock')],'installed-UID1000-native-desktop-client.log',env=desktop_env,user=1000,seconds=10)
 # Independent actual registry boundary on the already-owned app listener; no Source or app permissions are supplied.
 wait(lambda:normal_argv_bound and app_pid is not None and live_fd(app_pidfd),10)
 registry_probe=installed.program(R/'source/logical-output-registry-probe.py',installed.FIX/'logical-output-registry-probe.py')
 trusted_registry=json.loads(command(['/usr/bin/python3',str(registry_probe),str(installed.WAYLAND),'--bind-allowed'],'logical-output-trusted-registry.json',env={'PATH':'/usr/bin','LANG':'C.UTF-8'},user=1000,seconds=5))
 app_registry=json.loads(command(['/usr/bin/python3',str(registry_probe),result['actual_per_app_display']['pathname'],'--bind-allowed'],'logical-output-owned-app-registry.json',env={'PATH':'/usr/bin','LANG':'C.UTF-8'},user=11042,seconds=5))
 trusted_interfaces={v['interface']for v in trusted_registry['globals']};app_interfaces={v['interface']for v in app_registry['globals']}
 forbidden={'ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1','zwp_virtual_keyboard_manager_v1','zwlr_layer_shell_v1'}
 snapshot=admin({'op':'snapshot'})
 logical={'trusted':trusted_registry,'app':app_registry,'per_app_listener':result['actual_per_app_display'],'source_snapshot_after_registry':snapshot,'required_logical_output':'zxdg_output_manager_v1','forbidden_privileged_globals':sorted(forbidden),'privileged_globals_still_hidden':not bool(forbidden&app_interfaces),'logical_output_advertised_to_app':'zxdg_output_manager_v1'in app_interfaces,'positive_Sounds_Pixel_metadata_still_required':True}
 result['actual_logical_output_boundary']=logical
 assert trusted_registry['uid']==trusted_registry['gid']==1000 and app_registry['uid']==app_registry['gid']==11042
 assert trusted_registry['allowed_bound']==app_registry['allowed_bound']==['wl_compositor','xdg_wm_base']
 assert {'wl_output','wp_fractional_scale_manager_v1','zxdg_output_manager_v1'}<=trusted_interfaces
 assert {'wl_output','wp_fractional_scale_manager_v1'}<=app_interfaces and not forbidden&app_interfaces
 assert snapshot['source']['state']=='observed'and snapshot['source']['first_failure']is None
 assert logical['logical_output_advertised_to_app'],'actual owned app listener omits required logical output metadata'
 def actor_metadata():
  capture_bounds();text=metadata_file.read_text()
  if not text or not text.endswith('\n'):return
  lines=text.splitlines();assert len(lines)<=8 and all(len(line.encode())<=2048 for line in lines)
  value=json.loads(lines[-1]);assert value.get('kind')=='sounds_window_metadata'and value['schema']==1 and value['title']=='Sounds'
  if not(value['window']['visible']is True and value['window']['exposed']is True and abs(value['window']['dpr']-2.625)<=.001):return
  assert value['window']['visible']is True and value['window']['exposed']is True and abs(value['window']['dpr']-2.625)<=.001,'latest normal app frame metadata incompatible'
  return value
 def before_metadata_timeout(until):startup.record('before-original-frame-cutoff',app_pid,app_pidfd,absolute_cutoff=until,expected_starttime=watched.get(app_pid,{}).get('starttime'))
 frame_until=min(WORK,now()+30_000_000_000)
 observed=wait(actor_metadata,30,before_timeout=before_metadata_timeout,absolute_cutoff=frame_until);assert app_pid==observed['pid']and app_pidfd is not None and normal_argv_bound;proc=Path('/proc')/str(app_pid)
 startup.record('first-qualified-frame',app_pid,app_pidfd,expected_starttime=watched[app_pid]['starttime']);assert sha(proc/'exe')==sha(normal)
 text=(proc/'status').read_text();assert next(v for v in text.splitlines()if v.startswith('Uid:')).split()[1:]==next(v for v in text.splitlines()if v.startswith('Gid:')).split()[1:]==['11042']*4 and not next(v for v in text.splitlines()if v.startswith('Groups:')).split()[1:]
 assert int((proc/'stat').read_text().split(') ',1)[1].split()[1])==actor['child'].pid
 window_address=wait(window,10)['address'];assert admin({'op':'snapshot'})['principals']==1
 result['actual_normal_window_metadata']=observed;result['actual_actor_launch']={'app_PID':app_pid,'observed_parent_PID':actor['child'].pid,'kernel_UIDs':[11042]*4,'kernel_GIDs':[11042]*4,'supplementary_groups':[],'single_actor_launch_epoch':True,'numeric_generation_exported':False,'normal_ELF_sha256':sha(proc/'exe'),'fixed_desktop_client_executed_as_UID':1000,'desktop_menu_activation_claimed':False}
 assert now()<frame_until and actor_metadata()==observed,'normal frame changed before qualification'
 qualification=display.qualify_app(public_profile,display_discovery,observed,control,result,frame_until)
 assert now()<frame_until and actor_metadata()==observed,'normal frame changed during qualification'
 result['phase']='cold-fixed-Sounds-window-focus-and-quiet-PCM'
 focused=wait(app_focus,10);initial=wait(quiet,10);initial_PCM=pcm('released_silence','cold-installed-initial-no-grant')
 frame=capture('Sounds-cold-installed-no-grant')
 final_source=admin({'op':'snapshot'});assert final_source['source']['state']=='observed'and final_source['source']['first_failure']is None
 assert all(live(item)for item in [actor,service,broker,theme_shell])and compositor_live()and live_fd(app_pidfd)
 result['boundary_steps']=[{'id':1,'name':'cold_installed_PID1_full_shell_Source_fixed_Sounds_Pixel_focus_no_grant_quiet_PCM','passed':True,'native_focus':focused,'owner_and_graph':initial,'PCM':initial_PCM,'native_capture':frame,'final_authenticated_source_snapshot':final_source}]
 result['passed']=True
except BaseException as error:
 result['error']=type(error).__name__+': '+str(error);result['failure_classification']='SETUP_FAILURE'if result['phase']=='setup'else'ACTUAL_INSTALLED_BOUNDARY_FAILURE';result['traceback']=traceback.format_exc()
 try:failure_observations()
 except BaseException as observation_error:result['failure_observation_error']={'error':type(observation_error).__name__,'errno':getattr(observation_error,'errno',None),'detail':str(observation_error)[:512]}
 print(result['traceback'],flush=True)
finally:
 cleanup=[]
 if startup is not None:result['passive_startup_observations']={'file':startup.path.name,'sha256':sha(startup.path),'bytes':startup.bytes,'snapshot_count':len(startup.rows),'snapshots':startup.rows,'normal_metadata_or_authority_channel':False}
 # Preserve the existing authenticated independent-service Stop evidence before
 # an explicit owner manager stop can propagate through target dependencies.
 if service is not None and live(service)and call_item is not None and live(call_item):
  try:assert call({'op':'stop'})['ok']and finish(service,permit_manager_stop=False)==0;result['authenticated_audio_stop_PID1_exit0']=True
  except BaseException as error:cleanup.append('audio-stop: '+repr(error))
 if call_item is not None:
  try:
   if call_item['child'].stdin and not call_item['child'].stdin.closed:call_item['child'].stdin.close()
   assert finish(call_item)==0
  except BaseException as error:cleanup.append('readonly-call-client: '+repr(error))
 if broker is not None and live(broker):
  try:
   with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)as sock:sock.settimeout(2);sock.connect('/run/pokelinux/appearance-control.sock');assert struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(broker['child'].pid,0,0);sock.send(b'{"op":"stop"}');assert json.loads(sock.recv(4096))=={'result':'stopping'}
   assert finish(broker,permit_manager_stop=False)==0;result['authenticated_appearance_stop_PID1_exit0']=True
  except BaseException as error:cleanup.append('appearance-stop: '+repr(error))
 manager_owner=actor if actor is not None else units.authenticated_pending_owner()
 if manager_owner is not None and live(manager_owner):
  try:
   until=min(CLOSE,now()+10_000_000_000);before=units.show(manager_owner['unit'],True,absolute_cutoff=until)
   assert before['MainPID']==manager_owner['child'].pid and before['InvocationID']==manager_owner['identity']['InvocationID']and live(manager_owner)
   stop_record={'trigger':'actual-systemctl-stop-selected-owner','retained_MainPID':before['MainPID'],'retained_InvocationID':before['InvocationID'],'actual_pre_request_PID1':before,'original_until_ns':until,'request_started_boottime_ns':now(),'actual_after_notify_Ready':before['ActiveState']=='active'and before['SubState']=='running','full_startup_functional_slice_previously_passed':result['passed'],'native_IPC_ACK_claimed':False,'raw_signal_syscall_observed':False,'attribution':'actual selected Type/ExecStop/KillMode unit bytes and PID1 systemctl stop path'};result['manager_owner_Stop']=stop_record
   units.run(['/usr/bin/systemctl','stop',manager_owner['unit']],'manager-stop-retained-owner.log',True,absolute_cutoff=until)
   stop_record['actual_command_receipt']=dict(scope.receipts[-1]);assert stop_record['actual_command_receipt']['returncode']==0
   assert units.finish(manager_owner,until,permit_manager_stop=False)==0
   final=manager_owner['observed_closure'];stop_record['actual_final_observation']=final;stop_record['completed_boottime_ns']=now()
   assert final['process_retirement_proven']and final['main_process_exit0']and final['overall_unit_operational_success']and final['PID1_exit']['MainPID']==final['PID1_exit']['ControlPID']==0 and not final['cgroup_members']and now()<until
   result['owner_PID1_exit0']=True;result['manager_owner_Stop_after_Ready_qualified']=bool(stop_record['actual_after_notify_Ready']and stop_record['full_startup_functional_slice_previously_passed'])
  except BaseException as error:cleanup.append('manager-owner-stop: '+repr(error))
 else:
  result['manager_owner_Stop_not_attempted']={'reason':'no_authenticated_live_retained_Main','full_startup_functional_slice_previously_passed':result['passed'],'after_Ready_manager_Stop_qualified':False}
  if result['passed']:cleanup.append('manager-owner-stop prerequisite: retained Main not live')
 for item in [focus_layer,theme_shell]:
  if item is not None:
   try:stop(item,allowed=(0,-15))
   except BaseException as error:cleanup.append('owned-Wayland-client: '+repr(error))
 for label,pid,fd in [('app',app_pid,app_pidfd),('Source_compositor',compositor_pid,compositor_pidfd)]:
  if fd is not None:
   try:result['external_'+label+'_closure']={'PID':pid,'retained_pidfd_dead':not live_fd(fd),'proc_absent':not Path('/proc/'+str(pid)).exists(),'native_parent_wait_not_relabelled_as_guest_wait':True};os.close(fd)
   except BaseException as error:cleanup.append(label+'-pidfd: '+repr(error))
 for source,dest,limit in [(metadata_file,R/'normal-qt-window.jsonl',16392),(stderr_file,R/'normal-qt.stderr',4194304),(FOCUS_LOG,R/'actual-native-focus.jsonl',512*768)]:
  try:
   if source is not None and source.exists():
    assert source.stat().st_size<=limit;shutil.copyfile(source,dest);result.setdefault('preserved_regular_files',[]).append({'file':dest.name,'sha256':sha(dest),'bytes':dest.stat().st_size,'budget_bytes':limit,'PIPE_EOF_claimed':False})
  except BaseException as error:cleanup.append('regular-capture: '+repr(error))
 if touch is not None:touch.close()
 result['installed_unit_closure']=units.close_all();cleanup.extend(result['installed_unit_closure']['cleanup_errors'])
 for item in reversed(scope.children):
  if item['wait']is not None:continue
  try:stop(item,allowed=tuple(expected.get(item['child'].pid,{'allowed_actual_exit':[]})['allowed_actual_exit']))
  except BaseException as error:cleanup.append('owned-stop: '+repr(error))
 if capture_mounted:
  try:
   serial+=1;name=f'{serial:03d}-capture-tmpfs-unmount.log';until=min(CLOSE,now()+5_000_000_000);code,_=scope.run(['/usr/bin/umount',str(capture_dir)],name,until,limit=65536);expected[scope.receipts[-1]['pid']]={'role':'capture-tmpfs-unmount','allowed_actual_exit':[0]};assert code==0;capture_mounted=False;result['capture_tmpfs_unmount_actual_wait0']=True
  except BaseException as error:cleanup.append('capture-tmpfs-unmount: '+repr(error))
 cleanup.extend(scope.close(CLOSE))
 for file in owned_files:file.close()
 adopted=[]
 while now()<CLOSE:
  try:pid,status_code=os.waitpid(-1,os.WNOHANG)
  except ChildProcessError:break
  if pid:adopted.append({'pid':pid,'actual_guest_subreaper_wait_exit':os.waitstatus_to_exitcode(status_code),'at_boottime_ns':now()})
  else:time.sleep(.01)
 else:cleanup.append('guest original closure cutoff without ECHILD')
 observed=[]
 for pid,row in watched.items():
  dead=not live_fd(row['fd']);absent=not Path('/proc/'+str(pid)).exists();os.close(row['fd']);observed.append({k:v for k,v in row.items()if k!='fd'}|{'pid':pid,'retained_pidfd_dead':dead,'proc_absent':absent,'native_parent_actual_wait_not_independently_relabelled':True})
 result.update({'cleanup_errors':cleanup,'owned_guest_process_receipts':scope.receipts,'expected_direct_child_exits':expected,'observed_native_service_descendants':observed,'actual_guest_subreaper_waits':adopted,'all_direct_children_actual_wait_and_EOF':len(scope.receipts)==len(scope.children)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and v['returncode']in expected.get(v['pid'],{'allowed_actual_exit':[]})['allowed_actual_exit']for v in scope.receipts),'observed_descendant_pidfds_closed':all(v['retained_pidfd_dead']and v['proc_absent']for v in observed)})
 # Standard cloud-init power_state owns poweroff only after bootstrap/runcmd returns.
 result['shutdown_receipts']=[];result['poweroff_custody']='normal cloud-init power_state after runcmd returns; boot independently requires QMP guest shutdown and actual QEMU wait0'
 result['completed_boottime_ns']=now();result['guest_case_closure_qualified']=not cleanup and result['all_direct_children_actual_wait_and_EOF']and result['observed_descendant_pidfds_closed']and now()<CLOSE
 result['passed']=bool(result['passed']and result['installed_unit_closure']['all_unit_operational_results_success']and result['guest_case_closure_qualified']and result.get('owner_PID1_exit0')and result.get('manager_owner_Stop_after_Ready_qualified')and result.get('authenticated_audio_stop_PID1_exit0')and result.get('authenticated_appearance_stop_PID1_exit0')and result.get('capture_tmpfs_unmount_actual_wait0')and result.get('external_app_closure',{}).get('retained_pidfd_dead')and result.get('external_Source_compositor_closure',{}).get('retained_pidfd_dead'))
 (R/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'passed':result['passed'],'boundary_steps':[v['id']for v in result['boundary_steps']],'error':result.get('error')}),flush=True);os.sync()
