#!/usr/bin/env python3
"""One frozen actual PID1/private-PipeWire backend scenario; never grants audio.
The original installed353 must fail observed.available=True after a completed
Rust backend observation. Later identical code checks the private real graph and
PID1 Stop propagation. No Source, lease, route selection, playback or PCM request.
"""
import argparse,hashlib,json,os,re,select,socket,stat,struct,time,traceback,sys
from pathlib import Path
sys.dont_write_bytecode=True
from owned import Scope,now
RUNTIME=Path('/run/pokelinux-audio-backend');REMOTE='pokelinux-audio-test-manager'
TARGET='pokelinux-audio-backend.target'
AUDIO='pokelinux-audio.service'
UNITS=[(AUDIO,'/usr/libexec/pokelinux/pokelinux-audio'),('pokelinux-audio-bus.service','/usr/bin/dbus-daemon'),('pokelinux-audio-pipewire.service','/usr/bin/pipewire'),('pokelinux-audio-wireplumber.service','/usr/bin/wireplumber')]
FIELDS='LoadState,UnitFileState,ActiveState,SubState,MainPID,ControlGroup,ExecMainCode,ExecMainStatus,Result,InvocationID'
FAIL='Native Rust audio observed.available must be true after completed backend observation'
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--output',required=True,type=Path);p.add_argument('--until-ns',required=True,type=int);a=p.parse_args()
 until=min(a.until_ns,now()+90_000_000_000);logs=a.output.with_suffix('.logs');logs.mkdir(mode=0o700);scope=Scope(logs);serial=0;fds=[];identities={}
 result={'schema':1,'passed':False,'scenario':'installed-private-virtual-backend-availability-and-PID1-Stop','original_until_ns':until,'Source_or_lease_or_playback_or_PCM_claimed':False,'upstream_allowed_Main_outcomes':[[1,0],[2,15]],'Rust_Main_required_outcome':[1,0]}
 def command(argv,name,limit=65536,env=None,identity=None):
  nonlocal serial
  serial+=1;code,body=scope.run(argv,f'{serial:03d}-{name}.log',until,limit=limit,env=env,identity=identity);assert code==0,(name,code,body[-2000:]);return body.decode()
 def props(unit):
  return dict(line.split('=',1) for line in command(['/usr/bin/systemctl','show',unit,'--property='+FIELDS],'show-'+unit).splitlines())
 def identity(unit,exe):
  row=props(unit);assert row['LoadState']=='loaded' and row['ActiveState']=='active' and row['SubState']=='running' and row['UnitFileState']=='enabled','actual cold-enabled PID1 process required: '+unit
  pid=int(row['MainPID']);assert pid>1 and re.fullmatch('[0-9a-f]{32}',row['InvocationID']);fd=os.pidfd_open(pid,os.O_NONBLOCK);fds.append((pid,fd));assert not select.select([fd],[],[],0)[0]
  text=Path(f'/proc/{pid}/status').read_text();assert int(re.search(r'^PPid:\s+(\d+)$',text,re.M)[1])==1 and re.search(r'^Uid:\s+(.+)$',text,re.M)[1].split()==['11040']*4 and re.search(r'^Gid:\s+(.+)$',text,re.M)[1].split()==['11040']*4
  assert os.readlink(f'/proc/{pid}/exe')==exe
  assert row['ControlGroup'].startswith('/system.slice/') and Path(f'/proc/{pid}/cgroup').read_text().strip()=='0::'+row['ControlGroup']
  for raw in (Path('/sys/fs/cgroup'+row['ControlGroup'])/'cgroup.procs').read_text().split():
   child=int(raw)
   if child!=pid:
    try:fds.append((child,os.pidfd_open(child,os.O_NONBLOCK)))
    except ProcessLookupError:pass
  identities[unit]=row;return row
 def status():
  assert now()<until
  with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
   s.settimeout(min(1,(until-now())/1e9));s.connect('/run/pokelinux-audio/audio.sock')
   assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(int(identities[AUDIO]['MainPID']),11040,11040)
   s.sendall(b'{"op":"status"}\n');body=bytearray()
   while not body.endswith(b'\n'):
    block=s.recv(65537-len(body));assert block;body.extend(block);assert len(body)<=65536
   assert now()<until;return json.loads(body)
 try:
  assert os.getresuid()==os.getresgid()==(0,0,0) and Path('/proc/1/comm').read_text().strip()=='systemd'
  identity(AUDIO,UNITS[0][1]);observation_until=min(until,now()+5_000_000_000);last=None
  # The missing-endpoint baseline is a real Rust observation failure, never a
  # missing test executable/socket setup error. Fixed readiness budget, no reset.
  while now()<observation_until:
   try:last=status()
   except (ConnectionError,FileNotFoundError,socket.timeout):time.sleep(.01);continue
   result['Rust_audio_status']=last
   assert last['ok'] is True and last['phase']=='running'
   if last['observed']['revision']>=1 and last['observed']['available'] is True:break
   time.sleep(.02)
  assert last is not None and last['observed']['revision']>=1,'completed original Native Rust backend observation required'
  assert last['observed']['available'] is True,FAIL
  assert last['desired']==[] and last['lease'] is None and last['observed']['links']==[],'availability-only owner must have no authority or desired routes'
  result['backend_availability_assertion_passed']=True
  target=props(TARGET);assert target['LoadState']=='loaded' and target['ActiveState']=='active' and target['UnitFileState']=='enabled'
  for unit,exe in UNITS[1:]:identity(unit,exe)
  tools=json.loads((Path(__file__).parent/'BACKEND-TOOLS.json').read_text())
  for name,digest in tools.items():assert sha('/usr/bin/'+name)==digest
  m=RUNTIME.stat();assert stat.S_ISDIR(m.st_mode) and m.st_uid==m.st_gid==11040 and stat.S_IMODE(m.st_mode)==0o700
  endpoint=RUNTIME/REMOTE;sm=endpoint.stat();assert stat.S_ISSOCK(sm.st_mode) and sm.st_uid==sm.st_gid==11040
  with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
   s.settimeout(min(1,(until-now())/1e9));s.connect(str(endpoint));peer=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer==(int(identities['pokelinux-audio-pipewire.service']['MainPID']),11040,11040)
  env=dict(PATH='/usr/bin',LANG='C.UTF-8',HOME='/var/lib/pokelinux-audio-backend',XDG_RUNTIME_DIR=str(RUNTIME),PIPEWIRE_RUNTIME_DIR=str(RUNTIME),PIPEWIRE_REMOTE=REMOTE,PIPEWIRE_DEBUG='0',DBUS_SESSION_BUS_ADDRESS='unix:path='+str(RUNTIME/'bus'))
  raw=command(['/usr/bin/pw-dump'],'independent-private-graph',limit=1048576,env=env,identity={'user':11040,'group':11040,'extra_groups':[]});graph=json.loads(raw)
  core=[x for x in graph if x['type'].endswith(':Core')];assert len(core)==1 and isinstance(core[0]['info']['cookie'],int)
  nodes={x['id']:x['info']['props']['node.name'] for x in graph if x['type'].endswith(':Node')}
  required={'source.media','source.microphone','route.speaker','route.earpiece','route.capture'};assert required<=set(nodes.values()) and all(list(nodes.values()).count(n)==1 for n in required)
  assert 'source.call' not in nodes.values(),'no playback-generated call source in availability-only graph'
  links=[]
  for x in graph:
   if x['type'].endswith(':Link'):
    info=x['info'];props=info.get('props',{});node=int(info.get('output-node-id',props.get('link.output.node',-1)))
    if nodes.get(node) in ['source.media','source.microphone','source.call']:links.append(x['id'])
  assert not links,'no service-source route application in availability-only scenario'
  wp_pid=int(identities['pokelinux-audio-wireplumber.service']['MainPID']);clients=[]
  for x in graph:
   z=x.get('info',{}).get('props',{})
   if x['type'].endswith(':Client') and z.get('application.process.binary')=='wireplumber' and int(z.get('application.process.id',-1))==wp_pid:
    for k,v in [('pipewire.sec.pid',wp_pid),('pipewire.sec.uid',11040),('pipewire.sec.gid',11040)]:
     if k in z:assert int(z[k])==v
    clients.append(x['id'])
  assert len(clients)==1,'actual PID1 WirePlumber must be connected to private graph'
  result['independent_private_graph']={'cookie':core[0]['info']['cookie'],'node_names':sorted(nodes.values()),'service_source_links':links,'WirePlumber_MainPID':wp_pid,'connected_WirePlumber_client_id':clients[0],'endpoint_peer':list(peer),'raw_graph_sha256':hashlib.sha256(raw.encode()).hexdigest()}
  # One actual target Stop must propagate to the selected stack. After ordering
  # drains Rust before backend teardown; no manual daemon signaling is used here.
  command(['/usr/bin/systemctl','stop',TARGET],'actual-whole-stack-Stop')
  result['final_units']={}
  for unit,_ in UNITS:
   f=props(unit);result['final_units'][unit]=f
   assert f['ActiveState']=='inactive' and f['SubState']=='dead' and f['MainPID']=='0' and f['Result']=='success'
   outcome=(int(f['ExecMainCode']),int(f['ExecMainStatus']));assert outcome in ([(1,0)] if unit==AUDIO else [(1,0),(2,15)]),'explicit original normal/SIGTERM outcome: '+unit
   cg=Path('/sys/fs/cgroup'+identities[unit]['ControlGroup']);assert not cg.exists() or (cg/'cgroup.events').read_text().split('populated ')[1].splitlines()[0]=='0'
  for pid,fd in fds:assert select.select([fd],[],[],0)[0] and not Path(f'/proc/{pid}').exists(),'actual retained backend/worker death and proc absence'
  result['PID1_native_Rust_Main_exit0']=True;result['all_unit_cgroups_empty_and_retained_processes_dead']=True;result['guest_waitpid_for_PID1_Main_claimed']=False;result['passed']=True
 except AssertionError as e:result['failure']='AssertionError: '+str(e);result['failure_kind']='assertion'
 except BaseException as e:result['failure']=type(e).__name__+': '+str(e);result['failure_kind']='unexpected';result['traceback']=traceback.format_exc()
 finally:
  result['active_identities']=identities
  if not result['passed']:
   result['failure_cleanup']={}
   for unit,_ in UNITS:
    try:
     row=props(unit)
     if row['LoadState']=='loaded':command(['/usr/bin/systemctl','stop',unit],'failure-stop-'+unit);result['failure_cleanup'][unit]=props(unit)
    except BaseException as e:result['failure_cleanup'][unit]={'error':repr(e)}
  for _,fd in fds:os.close(fd)
  result['cleanup_errors']=scope.close(a.until_ns);result['owned_process_receipts']=scope.receipts;result['completed_ns']=now();result['passed']=bool(result['passed'] and not result['cleanup_errors'] and now()<until)
  a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(('PASS ' if result['passed'] else 'FAIL ')+result.get('failure','actual private backend and PID1 Stop'),flush=True)
 return 0 if result['passed'] else 1
if __name__=='__main__':raise SystemExit(main())
