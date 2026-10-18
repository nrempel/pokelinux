"""Actual unshown QML buttons → retained actor → original three full PCM windows; assertion after cleanup."""
from pathlib import Path
import base64,hashlib,json,os,re,select,shutil,signal,socket,struct,subprocess,time,traceback
import sys
sys.dont_write_bytecode=True
sys.path.insert(0,str(Path(__file__).resolve().parent.parent/'native-installed-services'))
from owned import Scope,now
import audio_config
R=Path(os.environ['POKELINUX_PREVIEW_OUTPUT'])/'runtime';R.mkdir(mode=0o755);B=R.parent/'bins';SRC=Path(__file__).resolve().parent;APPEARANCE=Path(os.environ['POKELINUX_PREVIEW_APPEARANCE']);T=now();W=min(T+120_000_000_000,int(os.environ['PHASE_WORK_UNTIL_NS']));C=min(T+150_000_000_000,int(os.environ['PHASE_CLOSE_UNTIL_NS']));scope=Scope(R);expected={};serial=0;actor=audio=appfd=None;app_pid=None;errors=[];setup=False;functional=False;observer=None;case_until=None;observer_wait_until=None;producerfds=[]
RUN=Path('/run/preview-case');INSTALL=Path('/opt/preview-case');POLICY=Path('/var/lib/preview-case');APP='org.omarchy.Sounds'
r={'schema':1,'passed':False,'classification':'SETUP_FAILURE','original_work_until_ns':W,'original_close_until_ns':C,'exactly_two_explicit_Plays_one_explicit_Stop_no_retries':True,'unshown_QML_input_selected':True,'normal_visible_window_Source_auth_hardware_selected':False}
def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def start(argv,label,uid=None,env=None,allowed=(0,),stdin=None):
 global serial
 serial+=1;item=scope.start(argv,f'{serial:03d}-'+label,4*1024**2,env=env,stdin=stdin,identity={}if uid is None else{'user':uid,'group':uid,'extra_groups':[]},cutoff=W,wait_cutoff=C);expected[item['child'].pid]=list(allowed);return item
def command(argv,label,seconds=10,uid=None,env=None,allowed=(0,)):
 item=start(argv,label,uid,env,allowed);code=scope.finish(item,min(W,now()+int(seconds*1e9)));assert code in allowed,(label,code);return(R/item['name']).read_text()
def wait(fn,seconds=5):
 until=min(W,now()+int(seconds*1e9))
 while now()<until:
  scope.pump();scope.check();v=fn()
  if v:return v
  time.sleep(.01)
 raise TimeoutError('original finite '+str(seconds)+'s wait')
def packet(path,body,pid,uid,cleanup=False,limit=4096,with_peer=False):
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)as p:
  deadline=C if cleanup else min(W,case_until or W);assert now()<deadline
  p.settimeout(min(2,max(.001,(deadline-now())/1e9)));p.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);p.connect(str(path))
  peer=struct.unpack('3i',p.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer[0]>0 and peer[1:]==(uid,uid)and(pid is None or peer[0]==pid)
  assert now()<deadline,'original control cutoff before send';assert p.send(body)==len(body)
  v,anc,flags,_=p.recvmsg(limit,socket.CMSG_SPACE(12))
  if path==RUN/'app/control.sock':
   # One bounded last receive survives even EOF/framing/credential/JSON failure.
   r['last_private_control_receive']={'path':str(path),'peer_pid':peer[0],'peer_uid':peer[1],'peer_gid':peer[2],'observed_boottime_ns':now(),'bytes_count':len(v),'bytes_base64':base64.b64encode(v).decode('ascii'),'flags':flags,'ancillary_count':len(anc),'ancillary':[{'level':level,'type':kind,'bytes':len(data),'data_base64':base64.b64encode(data).decode('ascii')}for level,kind,data in anc],'request_bytes_base64':base64.b64encode(body).decode('ascii')}
  assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC)and len(anc)==1 and anc[0][:2]==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS)and struct.unpack('3i',anc[0][2])==peer
  value=json.loads(v);assert now()<deadline
  if path==RUN/'app/control.sock'and isinstance(value,dict)and value.get('diagnostic')=='QML_INPUT_COMPONENT_SETUP_FAILURE':
   assert len(v)<=4096;r['private_control_diagnostic']=value
   raise RuntimeError('actual private QML component SETUP diagnostic: '+str(value.get('reason')))
  return(value,peer)if with_peer else value

def admin(v,cleanup=False):return packet(RUN/'admin.sock',json.dumps(v).encode(),actor['child'].pid,0,cleanup)
qml={'records':[],'actor_sends':[],'states':[],'clicks':[]};cursor=0;actor_fd=None
r['qml']=qml
state_keys={'routes','requested','applied','status','canChoose','canPlay','canStop','qml_status'}
def decimal(v,signed=False):
 assert type(v)is str and re.fullmatch(r'-?(0|[1-9][0-9]*)'if signed else r'0|[1-9][0-9]*',v);return int(v)
def page(value,after):
 global cursor,actor_fd
 assert set(value)in[{'operation','state','telemetry'},{'operation','state','telemetry','dispatch'}]
 model=value['state'];assert set(model)==state_keys and type(model['routes'])is list and all(type(model[k])is bool for k in ['canChoose','canPlay','canStop'])and all(type(model[k])is str for k in ['requested','applied','status','qml_status']);assert len(qml['states'])<4096;qml['states'].append({'observed_boottime_ns':now(),'state':model})
 t=value['telemetry'];assert set(t)=={'after','next','available_through','more','overflow','records','complete_play_sends','play_choices'}and t['after']==after and t['overflow']is False and len(t['records'])<=4 and after<=t['next']<=t['available_through']<=128 and t['more']==(t['next']<t['available_through'])
 for record in t['records']:
  assert record['marker']==cursor+1;cursor+=1;qml['records'].append(record)
  if not record['actor_peer']:continue
  raw=base64.b64decode(record['bytes_base64'],validate=True);length=decimal(record['length']);returned=decimal(record['result'],True);before=decimal(record['before_boottime_ns']);after_send=decimal(record['after_boottime_ns'])
  assert record['peer_valid']is True and(record['peer_pid'],record['peer_uid'],record['peer_gid'])==(actor['child'].pid,0,0)and type(record['fd'])is int and record['fd']>=0 and record['fd']!=record['controller_fd_at_send']and record['clocks_valid']is True and before<=after_send and len(raw)==record['copied_bytes']==length<=4096 and record['complete_send']==(returned==length and returned>=0)and record['flags']==socket.MSG_DONTWAIT|socket.MSG_NOSIGNAL
  if actor_fd is None:actor_fd=record['fd']
  assert record['fd']==actor_fd,'same actual observed actor FD, no reconnect'
  wire=json.loads(raw);assert type(wire)is dict and wire.get('op')in['media_status','media_route','media_preview_play','media_preview_stop']
  item={'record':record,'wire_utf8':raw.decode('utf8')};qml['actor_sends'].append(item)
 assert cursor==t['next'];return t
def check_choices(t):
 groups=[]
 for ordinal in [1,2]:
  attempts=[x for x in qml['actor_sends']if json.loads(x['wire_utf8'])['op']=='media_preview_play'and x['record']['play_choice']==ordinal];complete=[x for x in attempts if x['record']['complete_send']];assert len(complete)<=1
  until=None;ttl=1001
  for x in attempts:
   wire=json.loads(x['wire_utf8']);request=wire['request'];assert set(wire)=={'op','request'}and set(request)=={'route','ttl_ms','until_boottime_ns'}and request['route']=='media-a7'and type(request['ttl_ms'])is int and 1<=request['ttl_ms']<=ttl<=1001 and type(request['until_boottime_ns'])is int and request['until_boottime_ns']>0
   assert until is None or until==request['until_boottime_ns'];until=request['until_boottime_ns'];ttl=request['ttl_ms']
   if x['record']['complete_send']:assert decimal(x['record']['after_boottime_ns'])<until
  expected={'ordinal':ordinal,'attempts':len(attempts),'complete_sends':len(complete)}
  if complete:expected['original_until_boottime_ns']=str(until)
  assert t['play_choices'][ordinal-1]==expected;groups.append(expected)
 assert len(t['play_choices'])==2 and t['complete_play_sends']==sum(x['complete_sends']for x in groups);qml['play_choices']=groups
def component(op='state'):
 global app_pid
 requested=op;first_response=None
 while True:
  body=json.dumps({'op':op,'after':cursor},separators=(',',':')).encode();assert len(body)<=256;value,peer=packet(RUN/'app/control.sock',body,app_pid,11042,limit=65536,with_peer=True)
  if app_pid is None:app_pid=peer[0]
  assert value['operation']==op;t=page(value,cursor)
  if first_response is None:first_response=value
  if op!='state':
   dispatch=value['dispatch'];assert dispatch['clicked_signals']==1 and all(dispatch[k]is True for k in ['press_delivered','release_delivered','all_quick_windows_hidden']);assert decimal(dispatch['before_boottime_ns'])<=decimal(dispatch['after_boottime_ns']);qml['clicks'].append({'operation':op,'dispatch':dispatch})
  if not t['more']:check_choices(t);break
  op='state'
 return first_response
def first(item):
 lines=(R/item['name']).read_text().splitlines();return lines[0]if lines else None
try:
 assert os.getresuid()==os.getresgid()==(0,0,0)
 for path,mode in [(INSTALL,0o755),(RUN,0o755),(POLICY,0o700),(RUN/'app',0o700)]:path.mkdir(parents=True,exist_ok=True);path.chmod(mode)
 os.chown(RUN/'app',11042,11042)
 for n in ['preview-actor','sounds-qml-input-client','lifecycle-launcher']:
  shutil.copyfile(B/n,INSTALL/n);(INSTALL/n).chmod(0o755);os.chown(INSTALL/n,0,0)
 shared=INSTALL/'shared';shared.mkdir(mode=0o755);shutil.copyfile(APPEARANCE/'PhoneAppearance.qml',shared/'PhoneAppearance.qml');(shared/'PhoneAppearance.qml').chmod(0o644);os.chown(shared/'PhoneAppearance.qml',0,0);assert sha(shared/'PhoneAppearance.qml')==sha(APPEARANCE/'PhoneAppearance.qml');profile=INSTALL/'profile.json';profile.write_text(json.dumps({'apps':[{'app':APP,'uid':11042,'executable':str(INSTALL/'sounds-qml-input-client'),'args':[str(RUN/'app.sock'),str(RUN/'app/control.sock'),str(shared)]}]}));profile.chmod(0o644)
 (R/'pcm').mkdir(mode=0o755);os.chown(R/'pcm',11040,11040)
 _,private,control,audio_profile,env=audio_config.configure();asock=control/'audio.sock'
 bus=start(['/usr/bin/dbus-daemon','--session','--nofork','--print-address=1'],'private-bus.log',11040,env,(0,-15));env['DBUS_SESSION_BUS_ADDRESS']=wait(lambda:first(bus))
 for tool in ['pipewire','wireplumber','pw-cat','pw-record','pw-dump','pw-link','pw-cli']:r.setdefault('selected_tool_sha256',{})[tool]=sha('/usr/bin/'+tool)
 r['actual_pw_cat_version']=command(['/usr/bin/pw-cat','--version'],'pw-cat-version.log',uid=11040,env=env);helptext=command(['/usr/bin/pw-cat','--help'],'pw-cat-help.log',uid=11040,env=env);assert all(v in helptext for v in ['--playback','--raw','--format','--rate','--channels','--target','--properties']);r['actual_pw_cat_help_sha256']=hashlib.sha256(helptext.encode()).hexdigest()
 pw=start(['/usr/bin/pipewire','-c',str(private.parent/'config/pipewire.conf')],'private-PipeWire.log',11040,env,(0,-15));wait(lambda:(private/'pokelinux-audio-test').is_socket())
 wp=start(['/usr/bin/wireplumber','--profile','native-audio-test'],'private-WirePlumber.log',11040,env,(0,-15))
 audio=start([str(B/'pokelinux-audio'),str(asock),str(audio_profile),'11041','0'],'normal-nativeaudio.log',11040,env);wait(lambda:asock.is_socket())
 (R/'audio-main.json').write_text(json.dumps({'pid':audio['child'].pid,'uid':11040,'gid':11040,'sha256':sha(B/'pokelinux-audio')}))
 actor=start([str(INSTALL/'preview-actor'),str(RUN),str(POLICY),str(profile),str(INSTALL/'lifecycle-launcher'),str(asock)],'plain-real-owner-actor.log',env=dict(os.environ,NATIVE_LIFECYCLE_ISOLATED='1',QT_QPA_PLATFORM='offscreen',QT_QUICK_CONTROLS_STYLE='Basic',QSG_RHI_BACKEND='software'));wait(lambda:(RUN/'admin.sock').is_socket())
 assert admin({'op':'locked','locked':False})=={'result':'ok'};wait(lambda:admin({'op':'snapshot'})['ready']);command([str(INSTALL/'lifecycle-launcher'),str(RUN),APP],'actual-UID11042-launch.log');wait(lambda:(RUN/'app/control.sock').is_socket())
 (R/'actor-main.json').write_text(json.dumps({'pid':actor['child'].pid,'uid':0,'gid':0}));r['initial_actual_component']=component();appfd=os.pidfd_open(app_pid,os.O_NONBLOCK);assert int(Path(f'/proc/{app_pid}/stat').read_text().rsplit(') ',1)[1].split()[1])==actor['child'].pid
 assert admin({'op':'grant','app':APP,'permission':'media_route','allowed':True})=={'result':'ok'};assert admin({'op':'background','app':APP,'background':False})=={'result':'ok'}
 def model_until(predicate,seconds=8,after_ns=0):
  until=min(W,case_until or W,now()+int(seconds*1e9))
  while now()<until:
   scope.pump();scope.check()
   for sample in reversed(qml['states']):
    if sample['observed_boottime_ns']>=after_ns and predicate(sample['state']):return sample
   component();time.sleep(.005)
  raise TimeoutError('bounded actual public QML readiness')
 def complete_sends(op,choice=None):return[x for x in qml['actor_sends']if x['record']['complete_send']and json.loads(x['wire_utf8'])['op']==op and(choice is None or x['record']['play_choice']==choice)]
 def sent_until(op,choice=None):
  until=min(W,case_until or W)
  while now()<until:
   rows=complete_sends(op,choice)
   if rows:assert len(rows)==1;return rows[0]['record']
   component();scope.pump();scope.check();time.sleep(.002)
  raise TimeoutError('no complete real production '+op+' send')
 def clicked(op,button,choice):
  value=component(op);dispatch=value['dispatch'];assert dispatch['button']==button and dispatch['play_choice']==choice;return dispatch
 def publish(name,value):
  target=R/name;temporary=R/(name+'.tmp');temporary.write_text(json.dumps(value)+'\n');os.replace(temporary,target)
 def stage(name):
  while now()<case_until:
   scope.pump();scope.check();path=R/'pcm/qml'/name
   if path.exists():return json.loads(path.read_text())
   if(R/'pcm/qml/result.json').exists():raise RuntimeError('observer concluded before '+name)
   time.sleep(.002)
  raise TimeoutError('one original six-second case cutoff waiting for '+name)
 idle=lambda s:s['canPlay']and s['requested']=='media-a7'and s['applied']==''and s['status']==s['qml_status']=='Output selected; preview idle'
 playing=lambda s:s['canStop']and s['requested']==s['applied']=='media-a7'and s['status']==s['qml_status']=='Preview playing'
 retiring=lambda s:s['status']==s['qml_status']=='Preview retiring'
 r['actual_granted_catalog']=model_until(lambda s:s['canChoose']and any(x['id']=='media-a7'for x in s['routes']));assert qml['actor_sends']and json.loads(qml['actor_sends'][0]['wire_utf8'])['op']=='media_status'
 r['route_click']=clicked('click_route','route_media-a7',0);r['route_send']=sent_until('media_route');r['selected_idle']=model_until(idle,after_ns=decimal(r['route_click']['before_boottime_ns']))
 observer=start(['/usr/bin/python3',str(SRC/'pcm_observer.py'),str(R/'pcm/qml')],'actual-QML-and-full-paired-PCM.log',11040,env,(0,1));wait(lambda:(R/'pcm/qml/ready.json').exists(),5);r['monitor_ready']=json.loads((R/'pcm/qml/ready.json').read_text());assert r['monitor_ready']['no_fixture_media_source'];case_until=r['monitor_ready']['original_capture_until_boottime_ns'];observer_wait_until=min(W,case_until+4_000_000_000)
 setup=True;r['setup_qualified']=True;r['phase']='actual-unshown-QML-route-Play-Stop-new-Play'
 first_click=clicked('click_play','previewPlay',1);first_send=sent_until('media_preview_play',1);r['first_Play']={'dispatch':first_click,'send':first_send};publish('qml-play-sent.json',r['first_Play']);r['first_QML_Playing']=model_until(playing,after_ns=decimal(first_click['before_boottime_ns']));first_stage=stage('before-Stop.json');r['before_Stop_full_PCM']=first_stage
 r['QML_Playing_before_Stop']=model_until(playing,after_ns=first_stage['full_PCM']['captured_boottime_ns']);first_pid=first_stage['producer']['pid'];firstfd=os.pidfd_open(first_pid,os.O_NONBLOCK);producerfds.append((first_pid,firstfd));first_fields=Path(f'/proc/{first_pid}/stat').read_text().rsplit(') ',1)[1].split();r['producer_live_before_Stop']=not bool(select.select([firstfd],[],[],0)[0])and int(first_fields[1])==audio['child'].pid and first_fields[19]==first_stage['producer']['starttime']
 stop_click=clicked('click_stop','previewStop',1);stop_send=sent_until('media_preview_stop',1);r['QML_Retiring']=model_until(retiring,after_ns=decimal(stop_click['before_boottime_ns']));r['explicit_Stop']={'dispatch':stop_click,'send':stop_send,'observed_retiring':r['QML_Retiring']};publish('qml-stop-sent.json',r['explicit_Stop'])
 r['independent_quiet_stage']=stage('quiet.json');r['QML_idle_before_new_Play']=model_until(idle,after_ns=r['independent_quiet_stage']['captured_boottime_ns']);second_click=clicked('click_play','previewPlay',2);second_send=sent_until('media_preview_play',2);r['new_explicit_Play']={'dispatch':second_click,'send':second_send};publish('qml-new-play-sent.json',r['new_explicit_Play']);r['new_QML_Playing']=model_until(playing,after_ns=decimal(second_click['before_boottime_ns']));code=scope.finish(observer,observer_wait_until);assert code in[0,1]
 pcm=json.loads((R/'pcm/qml/result.json').read_text());r['PCM']=pcm;r['PCM_result_sha256']=sha(R/'pcm/qml/result.json');assert pcm['classification']in['ACTUAL_PCM_ORACLES_PASS','BEHAVIORAL_SAMPLE_FAILURE'];component()
 if pcm.get('passed'):
  second_pid=pcm['new_explicit_original_start_gate']['producer']['pid'];secondfd=os.pidfd_open(second_pid,os.O_NONBLOCK);producerfds.append((second_pid,secondfd));r['new_producer_live_before_cleanup']=not bool(select.select([secondfd],[],[],0)[0])
 r['same_Main_actor_Qt_app_live']=bool(scope.live(audio)and scope.live(actor)and not select.select([appfd],[],[],0)[0]);r['effect_counts']={'route':len(complete_sends('media_route')),'plays':len(complete_sends('media_preview_play')),'stops':len(complete_sends('media_preview_stop'))};functional=bool(r['producer_live_before_Stop']and r.get('new_producer_live_before_cleanup')and r['same_Main_actor_Qt_app_live']and r['effect_counts']=={'route':1,'plays':2,'stops':1}and pcm['passed']);r['classification']='ACTUAL_QML_PCM_PASS'if functional else'ACTUAL_QML_PCM_BEHAVIOR_FAILURE'
except BaseException as e:r['error']=repr(e);r['traceback']=traceback.format_exc()
finally:
 if observer is not None and observer['wait']is None:
  try:scope.finish(observer,observer_wait_until or min(C,now()+10_000_000_000))
  except BaseException as e:errors.append('observerFinish '+repr(e))
 if(R/'pcm/qml/result.json').exists():
  r['PCM']=json.loads((R/'pcm/qml/result.json').read_text());r['PCM_result_sha256']=sha(R/'pcm/qml/result.json')
 if actor is not None and scope.live(actor):
  try:assert admin({'op':'stop'},cleanup=True)=={'result':'ok'};assert scope.finish(actor,min(C,now()+5_000_000_000))==0
  except BaseException as e:errors.append('actorStop '+repr(e))
 if audio is not None and scope.live(audio):
  try:signal.pidfd_send_signal(audio['pidfd'],signal.SIGTERM);assert scope.finish(audio,min(C,now()+5_000_000_000))==0
  except BaseException as e:errors.append('audioSIGTERM '+repr(e))
 for item in reversed(scope.children):
  if item['wait']is not None:continue
  try:
   if scope.live(item):signal.pidfd_send_signal(item['pidfd'],signal.SIGTERM)
   scope.finish(item,min(C,now()+5_000_000_000))
  except BaseException as e:errors.append(item['name']+' '+repr(e))
 errors.extend(scope.close(C));r['cleanup_errors']=errors;r['owned_receipts']=scope.receipts;r['all_wait_EOF_timely']=len(scope.receipts)==len(scope.children)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and v['returncode']in expected[v['pid']]for v in scope.receipts)
 if appfd is not None:r['actor_app_pidfd_dead']=bool(select.select([appfd],[],[],0)[0]);r['actor_app_proc_absent']=not Path(f'/proc/{app_pid}').exists();os.close(appfd)
 r['producer_cleanup_facts']=[{'pid':pid,'retained_pidfd_dead':bool(select.select([fd],[],[],0)[0]),'proc_absent':not Path(f'/proc/{pid}').exists()}for pid,fd in producerfds];[os.close(fd)for _,fd in producerfds];r['cleanup_qualified']=bool(not errors and r['all_wait_EOF_timely']and r.get('actor_app_pidfd_dead')and r.get('actor_app_proc_absent')and all(v['retained_pidfd_dead']and v['proc_absent']for v in r['producer_cleanup_facts'])and now()<C);r['passed']=bool(functional and r['cleanup_qualified']);r['ready_for_compiled_assertion']=bool(setup and r['cleanup_qualified']and r.get('PCM',{}).get('classification')in['ACTUAL_PCM_ORACLES_PASS','BEHAVIORAL_SAMPLE_FAILURE']);(R/'result.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({'passed':r['passed'],'classification':r['classification'],'cleanup_qualified':r['cleanup_qualified']}),flush=True)
# The compiled consumer asserts behavior only in a subsequent owned process.
raise SystemExit(0 if r['ready_for_compiled_assertion'] else 1)
