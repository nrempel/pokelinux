#!/usr/bin/env python3
"""Actual original compositor to installed native Qt Messages. Diagnostic source copy; no simulated Source facts."""
import array, hashlib, json, os, select, shutil, signal, socket, sqlite3, struct, subprocess, time, traceback
from pathlib import Path
R=Path('/results');B=Path('/tmp/per-app-context-diagnostic');B.mkdir();O=B/'evidence';O.mkdir()
selection=json.loads((R/'selection.json').read_text())
r={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'secure_presentation_qualified':False,'production_PAM_lock_qualified':False,'sample_budget_ns':100000000,'source_revived':False,'ui_queue_withdrawal_qualified':False}
children=[];logs=[];owned={};receiver=None;sourcefd=None;service=None;compositor=None;frames=[];control=None;servicefd=None;active_rpc=None;held_sdk=None;trace_socket=None;trace_records=[];trace_errors=[]
APP='org.omarchy.Messages';UID=11010
runtime=Path('/run/pokelinux/messages');state=Path('/var/lib/pokelinux');install=Path('/usr/libexec/pokelinux');observe=Path('/run/joint-messages-observer')
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def trace_pump():
 if trace_socket is None:return
 for _ in range(32):
  try:data,anc,flags,_=trace_socket.recvmsg(112,socket.CMSG_SPACE(12)+socket.CMSG_SPACE(4),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
  except BlockingIOError:return
  rights=[]
  try:
   creds=[]
   for level,kind,body in anc:
    if level==socket.SOL_SOCKET and kind==socket.SCM_CREDENTIALS:creds.append(struct.unpack('3i',body))
    elif level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
     a=array.array('i');a.frombytes(body[:len(body)-len(body)%a.itemsize]);rights.extend(a)
    else:raise AssertionError('Unexpected diagnostic ancillary')
   assert service is not None and creds==[(service.pid,0,0)]
   assert not rights and not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC) and len(data)==112
   fields=struct.unpack('<8sHHI6Q6I3Q',data);assert fields[0]==b'PKSDIAG1' and fields[1]==1
   keys=['category','shape_flags','recorded_boottime_ns','previous_sequence','previous_sampled_at_ns','sequence','sampled_at_ns','examination_boottime_ns','drain_count','raw_errno_bits','packet_bytes','rights_messages','rights_handles','recv_flags','attempted','sent_before_record','dropped_before_record']
   record=dict(zip(keys,fields[2:]));record.update(authenticated_kernel_credentials=creds[0],callback_boottime_ns=now(),record_hex=data.hex())
   assert record['attempted']==record['sent_before_record']+record['dropped_before_record']+1
   assert not trace_records or record['attempted']>trace_records[-1]['attempted']
   trace_records.append(record)
  except BaseException as error:trace_errors.append(str(error))
  finally:
   for fd in rights:os.close(fd)
def pump():
 trace_pump()
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
def source_health(expected_writer):
 snapshot_=admin({'op':'snapshot'});assert snapshot_['ready'] and not snapshot_['locked']
 assert service.poll() is None and not select.select([servicefd],[],[],0)[0]
 assert compositor.poll() is None and not select.select([sourcefd],[],[],0)[0]
 assert not any(v['event']=='source_failed' for v in source_events())
 assert writer()==expected_writer and live_pidfd(expected_writer)
 return {'actor_snapshot':snapshot_,'writer_pid':expected_writer,'current_original_focus':json.loads(ctl('nativebuildprobe')),'observed_at_ns':now()}
def app_context(path):
 return next((v for v in reversed(source_events()) if v['event']=='app_context' and v['listener']==str(path)),None)
def app_arguments(app_pid):
 commandline=Path(f'/proc/{app_pid}/cmdline').read_bytes().split(b'\0')[:-1];arguments=[v.decode() for v in commandline]
 assert len(arguments)==7 and arguments[0]==str(install/'pokelinux-messages'),{'app_pid':app_pid,'arguments':arguments,'source_events':source_events()[-5:]}
 assert arguments[1:3]==profile_args[:2] and arguments[4:]==profile_args[3:]
 path=Path(arguments[3]);assert path.is_relative_to(context_parent) and path!=Path(profile_args[2])
 return path,arguments
def launch_rpc(begin_only=False):
 connection=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);connection.settimeout(.05);connection.connect(str(runtime/'admin.sock'))
 assert struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(service.pid,0,0)
 connection.setblocking(False);started=now();until=started+2000000000;packet=json.dumps({'op':'launch','app':APP}).encode()
 while True:
  try:assert connection.send(packet)==len(packet);break
  except BlockingIOError:assert now()<until;time.sleep(.0005)
 if begin_only:return connection,started,until
 while now()<until:
  try:reply=connection.recv(4096)
  except BlockingIOError:time.sleep(.0005);continue
  assert json.loads(reply)=={'result':'ok'};connection.close();return {'start_ns':started,'reply_ns':now(),'reply':'ok'}
 connection.close();raise TimeoutError('original two-second launch client budget')
def find_normal_window():
 return wait(lambda:next((v for v in json.loads(ctl('j/clients')) if v.get('title')=='Messages' and v.get('mapped')),None),90)
def root_history():
 result={}
 for name,query in [('raw','SELECT count(*) FROM receipts'),('inbox','SELECT count(*) FROM records')]:
  path=state/name/('receipts.sqlite3' if name=='raw' else 'inbox.sqlite3')
  connection=sqlite3.connect('file:'+str(path)+'?mode=ro',uri=True,timeout=0)
  try:
   connection.execute('PRAGMA query_only=ON');result[name]={'rows':connection.execute(query).fetchone()[0],'uid':os.stat(path).st_uid,'mode':oct(os.stat(path).st_mode&0o777)}
  finally:connection.close()
 return result
def first_generation():
 global window,pid,app_listener
 reply=launch_rpc();window=find_normal_window();pid=window['pid'];track();app_listener,args=app_arguments(pid)
 context=wait(lambda:app_context(app_listener),2)
 assert context['state']=='Ready' and context['occupied'] and context['metadata_scope']=='app_generation'
 assert context['admission'] and live_pidfd(context['worker_pid'])
 assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010'
 r['setup_complete']=True
 unknown=capture('first-generation-unknown-Source-denied');assert admin({'op':'snapshot'})['locked']
 filtered=independent_registry(app_listener,'first-generation-filtered-registry.log');assert filtered['allowed_bound']==['wl_compositor','xdg_wm_base']
 forbidden={'ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1','zwp_virtual_keyboard_manager_v1','zwlr_layer_shell_v1'}
 assert not forbidden&{v['interface'] for v in filtered['globals']}
 direct=direct_path_as_app(wayland);assert direct.get('errno')==13 and not direct['connected']
 time.sleep(1);assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok'
 wait(lambda:not admin({'op':'snapshot'})['locked'],3)
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3)
 assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID}
 owner=writer();source_health(owner);time.sleep(1);granted=capture('first-generation-current-focus-grant-history');source_health(owner)
 assert granted['sha256']!=unknown['sha256'];r['actual_focus_and_grant_admission_qualified']=True
 record('fresh_app_context_ready_before_actual_normal_Qt_and_current_independent_grant_history',context=context,arguments=args,launch=reply,source=fact,denied_screen=unknown,history_screen=granted)
 return {'pid':pid,'path':str(app_listener),'instance':context['instance'],'sdk_pid':context['worker_pid'],'writer_pid':owner,'window':window}
def lifetime_checks():
 global window,pid,app_listener
 first=first_generation();before=root_history();closed=now()
 assert ctl('dispatch hl.dsp.window.close({ window = '+json.dumps('address:'+window['address'])+' })').strip()=='ok'
 wait(lambda:bool(select.select([owned[first['pid']]['fd']],[],[],0)[0]) and not Path(f'/proc/{first["pid"]}').exists(),3)
 wait(lambda:admin({'op':'snapshot'})['principals']==0,3)
 wait(lambda:not Path(first['path']).exists(),2)
 refusal=json.loads(command(['python3',str(R/'source/green-connect-probe.py'),first['path']],'old-generation-future-connect-refused.log',user=UID))
 assert not refusal['connected'] and refusal['errno'] in [2,111] and refusal['uid']==UID
 wait(lambda:bool(select.select([owned[first['sdk_pid']]['fd']],[],[],0)[0]) and not Path(f'/proc/{first["sdk_pid"]}').exists(),2)
 retired=wait(lambda:app_context(Path(first['path'])) if app_context(Path(first['path'])) and not app_context(Path(first['path']))['occupied'] else None,2)
 health=source_health(first['writer_pid']);assert health['actor_snapshot']['principals']==0
 record('exact_app_reap_withdraws_old_path_and_retires_SDK_without_Source_or_writer_poison',first=first,close_invoked_ns=closed,observed_ns=now(),actual_refusal=refusal,retired_context=retired,health=health,parent_reap_evidence='inferred from trusted compiled actor and native SDK occupancy through actual wait plus kernel absence; not syscall-traced')
 wait(lambda:admin({'op':'messages_status'}).get('available') and not admin({'op':'messages_status'}).get('busy'),3)
 data=json.loads(command(['python3',str(R/'source/boundary-probe.py'),'--runtime',str(runtime),'--state',str(state)],'post-retirement-same-UID-data-denied.log',user=UID))
 assert data['app_read']=={'reply':{'result':'denied'}} and data['admin_snapshot'].get('errno')==13
 assert all(not v['readable'] and v['errno']==13 for v in data['raw_history'].values());assert root_history()==before
 record('separate_same_UID_after_reap_gets_no_principal_raw_or_admin_authority',actual=data,history_counts=before)
 reply=launch_rpc();window=find_normal_window();pid=window['pid'];track();app_listener,args=app_arguments(pid);context=wait(lambda:app_context(app_listener),2)
 assert context['state']=='Ready' and context['occupied'] and context['metadata_scope']=='app_generation'
 assert context['instance']!=first['instance'] and str(app_listener)!=first['path'] and context['worker_pid']!=first['sdk_pid'] and pid!=first['pid']
 assert live_pidfd(context['worker_pid']);source_health(first['writer_pid'])
 wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==pid,3)
 fact=wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==pid),None),3);assert fact['focus']=={'pid':pid,'uid':UID,'gid':UID}
 time.sleep(1);screen=capture('second-generation-new-current-focus-existing-grant-history');health=source_health(first['writer_pid'])
 assert health['actor_snapshot']['principals']==1 and root_history()==before and not Path(first['path']).exists()
 record('explicit_relaunch_gets_distinct_context_path_SDK_instance_and_owned_focus_same_writer',context=context,new_app_pid=pid,arguments=args,launch=reply,source=fact,health=health,screen=screen)
 r['generations']=[first,{'pid':pid,'path':str(app_listener),'instance':context['instance'],'sdk_pid':context['worker_pid'],'writer_pid':first['writer_pid']}];r['production_screens_require_visual_review']=True;r['prerequisites_qualified']=True;r['passed']=True
 r['boundary_scope']='Actual new per-app path/SDK/instance retirement and explicit relaunch; accepted old clients are not promised disconnected. Original100ms Source preserved, no automatic Source/SDK retry.'
def start_healthy_source():
 assert ctl('plugin load '+str(B/'adapter.so')).strip()=='ok';wait(lambda:not admin({'op':'snapshot'})['locked'],3)
 for target in [warm,warm_second,warm]:
  assert ctl('dispatch hl.dsp.focus({ window = '+json.dumps('pid:'+str(target.pid))+' })').strip()=='ok'
  wait(lambda:next((v for v in reversed(source_events()) if v['event']=='session' and (v.get('focus') or {}).get('pid')==target.pid),None),2)
  time.sleep(.12)
 events=[v for v in source_events() if v['event']=='session' and v.get('focus')]
 assert len({v['revision'] for v in events})>=3
 return events
def cancel_checks():
 global active_rpc,held_sdk
 # Actual first Source lease starts only after all ordinary warm graphics complete.
 source_updates=start_healthy_source()
 owner=writer();source_health(owner);connection,started,until=launch_rpc(True);active_rpc=connection
 end=now()+300000000
 while True:
  context=next((v for v in source_events() if v['event']=='app_context' and v['state']=='Preparing'),None)
  if context:break
  assert now()<end;time.sleep(.00025)
 held_sdk=context['worker_pid'];fd=os.pidfd_open(held_sdk,os.O_NONBLOCK);owned[held_sdk]={'fd':fd,'role':'display_context_worker'};signal.pidfd_send_signal(fd,signal.SIGSTOP)
 wait(lambda:Path(f'/proc/{held_sdk}/stat').read_text().split()[2] in ['T','t'],.5)
 assert not any(v['event']=='app_context' and v['instance']==context['instance'] and v['state']=='Ready' for v in source_events()),'Ready race is unselected'
 assert context['sdk_state']=='Initializing' and context['occupied'] and not context['admission']
 assert now()<context['until_ns'],'Preparation expired before cancellation setup; unselected'
 snapshot_start=now();snapshot_=admin({'op':'snapshot'});snapshot_finished=now();assert snapshot_['principals']==0;source_health(owner)
 track();assert not any(v['role']=='installed_Qt_app' for v in owned.values()),'Actual app spawned while SDK Preparing'
 try:queued_reply=connection.recv(4096)
 except BlockingIOError:queued_reply=None
 assert queued_reply is None,{'premature_launch_reply':repr(queued_reply)}
 current_context=app_context(Path(context['listener']))
 assert current_context['state']=='Preparing' and current_context['sdk_state']=='Initializing' and live_pidfd(held_sdk)
 assert now()<context['until_ns'],'Preparation expired during preconditions; unselected'
 canceled=now();connection.shutdown(socket.SHUT_RDWR);connection.close();active_rpc=None
 try:signal.pidfd_send_signal(fd,signal.SIGCONT);continued=now()
 except ProcessLookupError:continued=None
 wait(lambda:bool(select.select([fd],[],[],0)[0]) and not Path(f'/proc/{held_sdk}').exists(),2)
 wait(lambda:not Path(context['listener']).exists(),2);assert admin({'op':'snapshot'})['principals']==0
 track();assert not any(v['role']=='installed_Qt_app' for v in owned.values()),'Canceled preparing launch created actual app'
 assert not any(v.get('title')=='Messages' for v in json.loads(ctl('j/clients')))
 retired=wait(lambda:app_context(Path(context['listener'])) if app_context(Path(context['listener'])) and not app_context(Path(context['listener']))['occupied'] else None,2)
 record('actual_client_EOF_before_SDK_release_cannot_create_app_or_publish_success',context=context,launch_started_ns=started,client_RPC_original_until_ns=until,client_EOF_ns=canceled,real_SIGCONT_ns=continued,retired=retired,source_health=source_health(owner),source_updates=source_updates,actual_principals=0,pending_snapshot={'start_ns':snapshot_start,'finish_ns':snapshot_finished,'response':snapshot_},pending_reply_before_EOF=None)
 # Explicit successor only after observed actual helper retirement. Source never reloads.
 reply=launch_rpc();window=find_normal_window();app_pid=window['pid'];track();path,args=app_arguments(app_pid);next_context=wait(lambda:app_context(path),2)
 assert next_context['instance']!=context['instance'] and next_context['worker_pid']!=held_sdk and str(path)!=context['listener'] and next_context['state']=='Ready'
 source_health(owner);record('fresh_explicit_launch_after_canceled_context_reap_uses_new_instance',new_app_pid=app_pid,context=next_context,arguments=args,launch=reply,source_health=source_health(owner))
 r['setup_complete']=True;r['prerequisites_qualified']=True;r['passed']=True;r['boundary_scope']='Actual canceled preparing launch and real SDK release race; no app before actual retirement, then explicit successor. No Source reset or deadline extension.'
def held_initialization_stop_checks():
 global active_rpc,held_sdk
 source_updates=start_healthy_source();owner=writer();health=source_health(owner)
 connection,started,until=launch_rpc(True);active_rpc=connection;end=now()+300000000
 while True:
  context=next((v for v in source_events() if v['event']=='app_context' and v['state']=='Preparing'),None)
  if context:break
  assert now()<end;time.sleep(.00025)
 held_sdk=context['worker_pid'];fd=os.pidfd_open(held_sdk,os.O_NONBLOCK);owned[held_sdk]={'fd':fd,'role':'display_context_worker'};signal.pidfd_send_signal(fd,signal.SIGSTOP)
 wait(lambda:Path(f'/proc/{held_sdk}/stat').read_text().split()[2] in ['T','t'],.5)
 assert not any(v['event']=='app_context' and v['instance']==context['instance'] and v['state']=='Ready' for v in source_events()),'Ready race is unselected'
 assert context['sdk_state']=='Initializing' and not context['admission'] and now()<context['until_ns']
 snapshot_start=now();snapshot_=admin({'op':'snapshot'});snapshot_finished=now();assert snapshot_['principals']==0
 track();assert not any(v['role']=='installed_Qt_app' for v in owned.values())
 try:queued_reply=connection.recv(4096)
 except BlockingIOError:queued_reply=None
 assert queued_reply is None,{'premature_launch_reply':repr(queued_reply)}
 health=source_health(owner);assert now()<context['until_ns'],'Preparation expired before Stop; unselected'
 r['setup_complete']=True;r['prerequisites_qualified']=True
 stop_start=now();stop_reply=admin({'op':'stop'});stop_finished=now();assert stop_reply=={'result':'ok'}
 r['whole_Stop_ACK']=stop_reply
 wait(lambda:service.poll() is not None,2);service_exit=now();assert service.returncode==0
 connection.close();active_rpc=None
 assert select.select([fd],[],[],0)[0] and not Path(f'/proc/{held_sdk}').exists()
 assert not Path(context['listener']).exists()
 retired=app_context(Path(context['listener']));assert retired and not retired['occupied']
 record('held_actual_SDK_Preparing_has_no_app_success_and_Snapshot_Stop_remain_responsive',context=context,source_updates=source_updates,health_before_Stop=health,launch_started_ns=started,client_RPC_original_until_ns=until,pending_reply=None,snapshot={'start_ns':snapshot_start,'finish_ns':snapshot_finished,'response':snapshot_},stop={'start_ns':stop_start,'reply_ns':stop_finished,'response':stop_reply,'service_exit_observed_ns':service_exit},retired_context=retired)
 r['passed']=True;r['boundary_scope']='Actual held Preparing SDK with responsive authenticated Snapshot/Stop, no actual Qt or queued launch reply, original deadlines unchanged. SIGSTOP does not imply uninterruptible cleanup; SDK is killed and occupied clears through trusted parent wait.'
def production_checks():
 if selection['case']=='lifetime':return lifetime_checks()
 if selection['case']=='cancel':return cancel_checks()
 if selection['case']=='held-init-stop':return held_initialization_stop_checks()
 raise ValueError('unknown new production case')
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
 if selection['case'] in ['cancel','held-init-stop']:
  warm_second=start(['foot','--app-id=joint-render-warmup-second','sleep','180'],'warmup-second.log',env,True)
  wait(lambda:json.loads(ctl('nativebuildprobe'))['pid']==warm_second.pid,20);time.sleep(1)
 context_parent=Path('/run/pokelinux-display');context_parent.mkdir();context_parent.chmod(0o711)
 profile_args=['/var/lib/pokelinux-messages-app','/run/user/11010',str(context_parent/'required-fresh-app-socket'),'/home/alarm',str(runtime/'app.sock'),str(appearance)]
 profile=B/'profile.json';profile.write_text(json.dumps({'apps':[{'app':APP,'uid':UID,'executable':str(install/'pokelinux-messages'),'args':profile_args}]}))
 command([str(R/'bins/messages-seed-fixture'),str(state/'raw'),'--renderer-text'],'seed.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'))
 args=[str(install/'messages-service'),str(runtime),str(state/'policy'),str(profile),str(install/'lifecycle-launcher'),str(state/'raw'),str(state/'inbox')]
 if selection['phase']=='diagnostic':
  fixture_name=selection['fixture_binary'];shutil.copyfile(R/'bins'/fixture_name,install/fixture_name);(install/fixture_name).chmod(0o755);args[0]=str(install/fixture_name);args += [str(receive.fileno()),str(sourcefd),str(compositor.pid),'1000',source]
  if selection['test_case']!='direct-realm-composition-red':args += [str(wayland),str(context_parent)]
  fds=(receive.fileno(),sourcefd)
 else:receiver=receive;fds=()
 trace_directory=Path('/run/pokelinux-diagnostic');trace_directory.mkdir();trace_directory.chmod(0o700)
 trace_path=trace_directory/'source-trace.sock';trace_socket=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM);trace_socket.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);trace_socket.setblocking(False);trace_socket.bind(str(trace_path));trace_path.chmod(0o600)
 service=start(args,'service.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1',POKELINUX_SESSION_DIAGNOSTIC_TRACE=str(trace_path)),fds=fds);servicefd=os.pidfd_open(service.pid,os.O_NONBLOCK)
 if selection['phase']=='diagnostic':receive.close()
 wait(lambda:admin({'op':'snapshot'}).get('ready'))
 def read_available():
  track();status=admin({'op':'messages_status'});r['last_messages_status']=status;return status.get('available')
 wait(read_available)
 assert admin({'op':'grant','app':APP,'permission':'messages_read','allowed':True})=={'result':'ok'}
 production_checks();raise ProductionComplete()
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
  dead=bool(select.select([v['fd']],[],[],0)[0]);absent=not Path(f'/proc/{pid}').exists();r['cleanup'].append({'role':v['role'],'pid':pid,'owned_pidfd_death_observed':dead,'proc_absent':absent,'reaped':dead and absent,'evidence_kind':'pidfd death and proc absence after service cleanup; not independent syscall-traced parent wait. Per-boundary app removal/SDK occupied=false inference is recorded separately while the service parent is live.'});os.close(v['fd'])
 if receiver is not None:receiver.close()
 if sourcefd is not None:os.close(sourcefd)
 if servicefd is not None:os.close(servicefd)
 trace_pump();r['diagnostic_records']=trace_records;r['diagnostic_trace_errors']=trace_errors
 if trace_socket is not None:trace_socket.close()
 summary=next((v for v in reversed(trace_records) if v['category']==0),None)
 r['diagnostic_trace_qualified']=not trace_errors and summary is not None and summary['dropped_before_record']==0 and summary['sent_before_record']==len(trace_records)-1 and summary['attempted']==len(trace_records)
 for f in logs:f.close()
 r['combined_messages_children_reaped']={role:any(v['role']==role and v['reaped'] for v in r['cleanup']) for role in ['messages_writer','messages_read_worker']}
 r['source_events']=source_events() if (O/'service.log').exists() else [];r['baseline_frames']=frames;r['all_tracked_children_finally_absent_or_directly_waited']=not r.get('cleanup_errors') and all(v['reaped'] for v in r['cleanup']);r['passed']=r['passed'] and r['all_tracked_children_finally_absent_or_directly_waited']
 for p in observe.glob('*'):
  if p.suffix in ['.json','.png']:shutil.copyfile(p,O/p.name)
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
