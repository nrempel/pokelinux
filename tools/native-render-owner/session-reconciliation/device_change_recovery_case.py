"""Frozen real NORMAL recoverable DEVICE_CHANGE case. No owner/native stand-in.
The caller retains actual Scope/seatd ownership and original35/45s cutoffs.
"""
import errno,json,os,select,signal,socket,stat,struct
from pathlib import Path
from owned import now
from process_custody import Descendants
from first_stop_oracle import TargetFailure,DiagnosticUnavailable
from device_change_recovery_oracle import snapshot as validate_snapshot,clean_pending,clean_bootstrap,device_change_terminal,public_ready,ordered_recovery,RecoveryUnmet

class GateUnmet(RuntimeError):
 def __init__(self,gate,detail):self.gate=gate;super().__init__(detail)

def unique(pairs):
 d={}
 for k,v in pairs:
  if k in d:raise ValueError('duplicate JSON key')
  d[k]=v
 return d

def run_case(scope,base,main_path,guest_work,guest_close,cohort=None):
 t=now();work=t+35_000_000_000;close=t+45_000_000_000
 assert work<guest_work and close<guest_close
 runtime=Path(base)/'normal';runtime.mkdir(mode=0o700);os.chown(runtime,1000,1000)
 home=runtime/'home';home.mkdir(mode=0o700);os.chown(home,1000,1000)
 config=runtime/'normal.lua'
 config.write_text('hl.monitor({output="",mode="1080x2424@60",position="auto",scale=2.625})\n'+'hl.config({debug={disable_scale_checks=true},animations={enabled=false},decoration={blur={enabled=false},shadow={enabled=false}},misc={disable_hyprland_logo=true,disable_splash_rendering=true}})\n')
 config.chmod(0o644)
 c={'schema':1,'phase':'native-x86-normal-device-change-recovery','started_at_ns':t,'work_until_ns':work,'close_until_ns':close,'classification':'SETUP_UNQUALIFIED','native_frame_path_qualified':False,'capture_prefix_qualified':False,'client_records':[],'public_requests':[],'observed_stages':[],'phone_accessed':False,'normal_test_macro_selected':False,'forced_failure_case_selected':False,'outer_closure_pending':True}
 descendants=Descendants(scope);main=client=None;control=None;buffer=bytearray();used=0;loaded=False
 def pump(until):
  assert now()<until<=work
  scope.pump();scope.check()
  if main is not None:
   descendants.watch(main['child'].pid)
   if not scope.live(main):raise GateUnmet('NORMAL_MAIN_STARTUP_OR_RUNTIME_EXIT','actual normal main pidfd became terminal')
  select.select([],[],[],min(.01,max(0,(until-now())/1e9)))
 def path(pattern):
  while True:
   values=[p for p in runtime.glob(pattern)if p.is_socket()];assert len(values)<=1
   if values:
    assert len(os.fsencode(values[0]))<=107;return values[0]
   pump(work)
 def request(text):
  nonlocal control
  body=text.encode('utf-8');assert 0<len(body)<=64
  enter=now();until=enter+2_000_000_000;assert until<work
  record={'request_utf8_hex':body.hex(),'enter_ns':enter,'original_until_ns':until,'client_nonblocking':True,'client_shutdown_write_EOF':False,'response_EOF':False}
  c['public_requests'].append(record)
  sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC)
  try:
   code=sock.connect_ex(str(control));assert code in (0,errno.EINPROGRESS,errno.EAGAIN)
   while not select.select([],[sock],[],0)[1]:pump(until)
   assert sock.getsockopt(socket.SOL_SOCKET,socket.SO_ERROR)==0
   peer=list(struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12)));assert peer==[main['child'].pid,1000,1000];record['actual_server_SO_PEERCRED']=peer
   sent=0
   while sent<len(body):
    assert now()<until
    try:
     count=sock.send(body[sent:]);assert count>0;sent+=count
    except BlockingIOError:pump(until)
   sock.shutdown(socket.SHUT_WR);record['client_shutdown_write_EOF']=True
   reply=bytearray()
   while True:
    assert now()<until
    try:part=sock.recv(2049-len(reply))
    except BlockingIOError:pump(until);continue
    if not part:break
    reply.extend(part);assert len(reply)<=2048
   record.update(return_ns=now(),response_EOF=True,response_bytes=len(reply));assert record['return_ns']<until
   assert sum(v.get('response_bytes',0)for v in c['public_requests'])<=65536
   text=reply.decode('utf-8');record['response_utf8']=text
   # These are actual client wire observations, not fabricated server-mode/timing facts.
   try:record['reply']=json.loads(text,object_pairs_hook=unique)
   except json.JSONDecodeError:record['reply_text']=text
   return record.get('reply',text)
  finally:sock.close()
 try:
  env={'HOME':str(home),'USER':'alarm','LOGNAME':'alarm','LANG':'C.UTF-8','LC_ALL':'C','PATH':'/usr/bin','XDG_RUNTIME_DIR':str(runtime),'XDG_SESSION_TYPE':'wayland','LIBSEAT_BACKEND':'seatd','SEATD_SOCK':'/run/seatd.sock','AQ_NO_MODIFIERS':'1','HYPRLAND_NO_SD_VARS':'1','HYPRLAND_NO_SD_NOTIFY':'1'}
  assert not any(k.startswith(('LD_','POKELINUX_STARTUP_'))for k in env)
  main=scope.start([str(main_path),'--config',str(config)],'normal-main.log',2*1024**2,env=env,identity={'user':1000,'group':1000,'extra_groups':[]},cutoff=work,wait_cutoff=close)
  c['main_PID']=main['child'].pid
  control=path('hypr/*/.socket.sock');c['control_path']=str(control)
  from admission import mapped_main
  assert cohort is not None;c['actual_loaded_main_AQ_maps']=mapped_main(main['child'].pid,cohort)
  c['startup_snapshots']=[];generation=None;saw_bootstrap=False
  c['public_Stop_sent_before_target_observation']=False
  c['full_ordered_Rust_recovery_witness_mapping_bound']=False
  while True:
   if sum(v.get('response_bytes',0)for v in c['public_requests'])+8192>65536:
    raise GateUnmet('RECOVERY_PREREQUISITE_NOT_OBSERVED_WITHIN_ORIGINAL_BYTE_BUDGET','original8KiB response reserve cannot fit the original64KiB public budget')
   if now()+2_000_000_000>=work:
    raise GateUnmet('RECOVERY_PREREQUISITE_NOT_OBSERVED_WITHIN_ORIGINAL_WORK','no new2s request can fit the original35s work cutoff')
   value=request('owner-startup snapshot');c['startup_snapshots'].append(value)
   generation=validate_snapshot(value,main['child'].pid,c['public_requests'][-1],generation)
   c['startup_not_after_ns']=value['startup_not_after_ns'];c['observed_stages'].append(value['stage'])
   if value['observed_boottime_ns']>=value['startup_not_after_ns']:
    raise GateUnmet('NORMAL_BACKEND_NOT_READY_BEFORE_ORIGINAL_STARTUP_DEADLINE','same original20s cutoff; no recovery renewal')
   if value['phase']=='STARTING':
    clean_pending(value)
    if value['stage']==8:
     clean_bootstrap(value);saw_bootstrap=True
   elif value['phase']=='STOPPING':
    if not saw_bootstrap:raise GateUnmet('ACTUAL_BOOTSTRAP_STAGE8_NOT_OBSERVED','terminalStop alone cannot qualify the bootstrap prerequisite')
    first,count=device_change_terminal(value,generation)
    c['actual_native_public_prerequisites_qualified']=True
    c['actual_recoverable_DEVICE_CHANGE_prerequisites_qualified']=True
    c['first_spontaneous_stopping_snapshot']=value;c['first_record_observed']=first;c['actual_stop_request_count']=count
    c['classification']='RECOVERABLE_DEVICE_CHANGE_TARGET_RED'
    c['target_failure']='DEVICE_CHANGE_TERMINATED_NORMAL_STARTUP_BEFORE_READY'
    c['target_failure_at_ns']=now()
    break
   elif value['phase']=='BACKEND_READY':
    public_ready(value);c['public_backend_ready_snapshot']=value
    c['actual_ordered_Rust_recovery_witness']=ordered_recovery(value,generation)
    if not saw_bootstrap:raise GateUnmet('ACTUAL_BOOTSTRAP_STAGE8_NOT_OBSERVED','Ready alone cannot substitute for observed native/bootstrap path')
    c['actual_native_public_prerequisites_qualified']=True
    c['full_ordered_Rust_recovery_witness_mapping_bound']=True
    c['classification']='RECOVERABLE_DEVICE_CHANGE_TARGET_PASS'
    c['recoverable_DEVICE_CHANGE_target_passed']=True
    break
   else:
    raise GateUnmet('ACTUAL_NORMAL_RECOVERY_PREREQUISITE_UNMET','unexpected actual native/bootstrap/Rust state')
   next_query=min(work,now()+250_000_000)
   while now()<next_query:pump(work)
 except BaseException as error:
  c['classification']='SETUP_UNQUALIFIED'
  c['earliest_unmet_gate']=error.gate if isinstance(error,(GateUnmet,RecoveryUnmet))else str(error)if isinstance(error,DiagnosticUnavailable)else'FIXTURE_OR_NATIVE_PUBLIC_PREREQUISITE'
  c['failure']={'type':type(error).__name__,'detail':str(error)[:1024],'observed_at_ns':now()}
 finally:
  c['witness_loaded_before_process_disposal']=False
  c['no_client_plugin_or_proc_diagnostic_started']=True
  if main is not None:
   try:c['main_process_disposal']=descendants.dispose(main,close)
   except BaseException as error:c['process_disposal_error']=str(error)[:512]
  c['completed_at_ns']=now();c['completed_before_original_case_close']=c['completed_at_ns']<close
  c['all_owned_descendants_closed']=main is not None and c.get('main_process_disposal',{}).get('descendants_closed',False)
  descendants.close()
 return c
