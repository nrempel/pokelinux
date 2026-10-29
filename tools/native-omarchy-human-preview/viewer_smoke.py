"""One external automated RFB client; normal-app process observation, not human qualification."""
import hashlib,json,os,select,socket,struct,sys,time
from pathlib import Path
r=Path(sys.argv[1]);sys.path.insert(0,str(r/'source'))
from rfb_frames import receive_once,request_raw_frame
from rfb_input_packets import click_plan
now=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
until=now()+700_000_000_000
record={'schema':1,'passed':False,'human_client_qualified':False,'fresh_QWindow_profile_qualified':False,'actions':[],'original_until_ns':until}
sock=None
try:
 sha=hashlib.sha256((r/'selection.json').read_bytes()).hexdigest()
 while not (r/'viewer-input/human-ready.json').exists():
  assert now()<until
  assert not (r/'execution.json').exists(),'VM ended before human-ready'
  select.select([],[],[],.1)
 ready=json.loads((r/'viewer-input/human-ready.json').read_text());assert ready['selection_sha256']==sha
 until=min(until,ready['host_dwell_until_ns'],now()+40_000_000_000);record['client_original_until_ns']=until
 sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);sock.settimeout(max(.001,(until-now())/1e9));sock.connect(str(r/'viewer.sock'))
 peer=struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer[1:]==(1000,1000);record['actual_socket_peer_PID_UID_GID']=list(peer)
 class Wire:
  received=0;sent=0
  def read(self,n):
   assert 0<=n<=12*1024*1024
   out=bytearray()
   while len(out)<n:
    assert now()<until;sock.settimeout(max(.001,(until-now())/1e9));chunk=sock.recv(min(65536,n-len(out)));assert chunk
    out.extend(chunk);self.received+=len(chunk);assert self.received<=24*1024*1024
   assert now()<until;return bytes(out)
  def write(self,data):
   assert now()<until;sock.settimeout(max(.001,(until-now())/1e9));sock.sendall(data);self.sent+=len(data);assert self.sent<65536 and now()<until
 wire=Wire();frame,info=receive_once(wire);record['first_frame']=info;del frame
 viewport={'x':0,'y':0,'width':1080,'height':2424}
 for action,point in [('bar-1',(54,34)),('apps-1',(540,644)),('sounds-1',(540,1196))]:
  plan=click_plan(action,*point,viewport);entry={'action':action,'point':point,'attempted_ns':now(),'sent_once':False};record['actions'].append(entry)
  for i,packet in enumerate(plan['packets']):
   wire.write(packet)
   if i==plan['hold_after_packet_index']:
    pause=now()+plan['hold_ns'];assert pause<until
    while now()<pause:select.select([],[],[],min(.01,max(0,(pause-now())/1e9)))
  entry.update(sent_once=True,completed_ns=now())
  pause=now()+700_000_000;assert pause<until
  while now()<pause:select.select([],[],[],min(.01,max(0,(pause-now())/1e9)))
 while True:
  assert now()<until
  apps=list((r/'viewer-input').glob('app-*.json'))
  if apps:break
  select.select([],[],[],.025)
 assert len(apps)==1
 observed=json.loads(apps[0].read_text());assert observed['selection_sha256']==sha
 app=observed['native_app'];assert app['actual_arguments'][0]=='/usr/libexec/pokelinux/pokelinux-sounds'
 record['actual_normal_app_observation']=observed
 pause=now()+1_000_000_000;assert pause<until
 while now()<pause:select.select([],[],[],min(.01,max(0,(pause-now())/1e9)))
 frame,info=request_raw_frame(wire,1080,2424,info['server_name_sha256']);record['final_frame']=info
 (r/'human-smoke-frame.bgrx').write_bytes(frame)
 sock.shutdown(socket.SHUT_WR)
 while True:
  assert now()<until;sock.settimeout(max(.001,(until-now())/1e9));data=sock.recv(65536)
  if not data:record['actual_RFB_EOF_ns']=now();break
  wire.received+=len(data);assert wire.received<=24*1024*1024
 record['passed']=True
except BaseException as error:
 record['error']=type(error).__name__+': '+str(error)
finally:
 if sock is not None:sock.close()
 record['completed_ns']=now();(r/'human-smoke-result.json').write_text(json.dumps(record,indent=2)+'\n')
 print(json.dumps({'passed':record['passed'],'error':record.get('error')}),flush=True)
if not record['passed']:raise SystemExit(1)
