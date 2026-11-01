"""One external acceptance RFB peer; finite actions, no reconnect or uncertain retry."""
import hashlib,json,os,select,socket,struct,sys,time
from pathlib import Path
R=Path(sys.argv[1]);sys.path.insert(0,str(R/'source'))
from viewer_protocol import read,publish,digest
from rfb_frames import receive_once,request_raw_frame
from rfb_input_packets import click_plan,close_plan
from rfb_navigation_packets import navigation_plan
now=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
D=R/'viewer-input';sha=hashlib.sha256((R/'selection.json').read_bytes()).hexdigest()
until=now()+700_000_000_000;sock=None;peerfd=None;held=[];held_point=None;framing=True
result={'schema':1,'selection_sha256':sha,'passed':False,'actions':[],'human_client_qualified':False,'fresh_QWindow_DPR_qualified':False,'remote_event_queues_drained_claim':False}

def check():
 assert now()<until
 if peerfd is not None:assert not select.select([peerfd],[],[],0)[0],'owned viewer server ended before stream close'
 assert read(D/'acceptance-abort.json')is None,'guest acceptance aborted'

def wait(path):
 while True:
  check();value=read(path)
  if value is not None:return value
  assert not(R/'execution.json').exists(),'VM ended before acceptance handoff'
  select.select([],[],[],.005)

class Wire:
 received=0;sent=0
 def read(self,count):
  assert 0<=count<=12*1024*1024;data=bytearray()
  while len(data)<count:
   check();sock.settimeout(max(.001,(until-now())/1e9));part=sock.recv(min(65536,count-len(data)));assert part
   data.extend(part);self.received+=len(part);assert self.received<=24*1024*1024
  check();return bytes(data)
 def write(self,data):
  global framing
  view=memoryview(data);written=0
  try:
   while view:
    check();sock.settimeout(max(.001,(until-now())/1e9));count=sock.send(view);assert count>0
    written+=count;self.sent+=count;assert self.sent<=65536;view=view[count:]
  except BaseException:
   if written and view:framing=False
   raise
  check()

try:
 ready=wait(D/'human-ready.json');assert ready['selection_sha256']==sha
 until=min(until,ready['host_dwell_until_ns']);session_until=until
 sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);sock.settimeout(max(.001,(until-now())/1e9));sock.connect(str(R/'viewer.sock'))
 peer=struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer[1:]==(1000,1000);peerfd=os.pidfd_open(peer[0],os.O_NONBLOCK)
 wire=Wire();frame,initial=receive_once(wire);del frame;result['initial_frame']=initial
 publish(D/'acceptance-connected.json',{'schema':1,'selection_sha256':sha,'connected':True,'actual_peer_PID_UID_GID':list(peer),'first_frame_sha256':initial['frame_sha256']})
 seq=1
 while True:
  check();finished=read(D/'acceptance-finished.json')
  if finished is not None:
   assert finished=={'schema':1,'selection_sha256':sha,'actions':seq-1};break
  request=read(D/f'accept-{seq:02d}.request.json')
  if request is None:select.select([],[],[],.005);continue
  assert seq<=24 and request['schema']==1 and request['selection_sha256']==sha and request['seq']==seq
  action=request['action'];assert action in('click','alt-tab','close','page-down','down','return')
  point=request['point'];assert(point is None)==(action!='click')
  if point is not None:assert len(point)==2 and all(type(v)is int for v in point)and 0<point[0]<1080 and 0<point[1]<2424
  prefix=f'accept-{seq:02d}';h0=now();request_hash=digest(request)
  publish(D/(prefix+'.ready.json'),{'schema':1,'selection_sha256':sha,'seq':seq,'request_sha256':request_hash,'host_ready_anchor_ns':h0})
  commit=wait(D/(prefix+'.commit.json'))
  assert commit['schema']==1 and commit['selection_sha256']==sha and commit['seq']==seq and commit['request_sha256']==request_hash and commit['host_ready_anchor_ns']==h0
  assert commit['guest_sample_ns']+commit['remaining_ns']==commit['guest_until_ns']==request['guest_until_ns']and type(commit['remaining_ns'])is int and commit['remaining_ns']>0
  until=min(session_until,h0+commit['remaining_ns']);check()
  if action=='click':plan=click_plan('bar-1',*point,{'x':0,'y':0,'width':1080,'height':2424})
  elif action=='close':plan=close_plan('close-1')
  elif action in('page-down','down','return'):
   plan=navigation_plan(action)
  else:plan={'packets':tuple(struct.pack('>BBHI',4,down,0,key)for down,key in[(1,0xffe9),(1,0xff09),(0,0xff09),(0,0xffe9)]),'hold_after_packet_index':1,'hold_ns':100_000_000}
  assert now()+plan['hold_ns']<until
  entry={'seq':seq,'action':action,'point':point,'request_sha256':request_hash,'attempted_host_ns':now(),'outcome':'uncertain','guest_clock_fields_not_compared_to_host_absolute':True};result['actions'].append(entry)
  for index,packet in enumerate(plan['packets']):
   if action=='click':
    if index==1:held_point=point
   else:
    kind,down,padding,key=struct.unpack('>BBHI',packet);assert kind==4 and down in(0,1)and padding==0
    if down:
     assert key not in held;held.append(key)
    else:assert key in held
   wire.write(packet)
   if action=='click'and index==2:held_point=None
   elif action!='click'and not down:held.remove(key)
   if index==plan['hold_after_packet_index']:
    end=now()+plan['hold_ns'];assert end<until
    while now()<end:check();select.select([],[],[],min(.005,max(0,(end-now())/1e9)))
  entry.update(outcome='sent_once',completed_host_ns=now(),packets_sha256=hashlib.sha256(b''.join(plan['packets'])).hexdigest());check()
  publish(D/(prefix+'.done.json'),{'schema':1,'selection_sha256':sha,**entry});seq+=1;until=session_until
 frame,final=request_raw_frame(wire,1080,2424,initial['server_name_sha256']);del frame;result['final_frame']=final
 sock.shutdown(socket.SHUT_WR)
 # The actual server may exit normally while its stream reaches EOF.
 os.close(peerfd);peerfd=None
 while True:
  assert now()<until;sock.settimeout(max(.001,(until-now())/1e9));data=sock.recv(65536)
  if not data:result['actual_RFB_EOF_host_ns']=now();break
  wire.received+=len(data);assert wire.received<=24*1024*1024
 result['passed']=True
except BaseException as error:
 result['error']=type(error).__name__+': '+str(error)
finally:
 if sock is not None:
  if not result['passed']and framing:
   try:
    sock.settimeout(.2)
    for key in reversed(held):sock.sendall(struct.pack('>BBHI',4,0,0,key))
    if held_point is not None:sock.sendall(struct.pack('>BBHH',5,0,*held_point))
   except OSError:pass
  sock.close()
 if peerfd is not None:os.close(peerfd)
 result['completed_host_ns']=now();(R/'permissions-viewer-result.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'passed':result['passed'],'error':result.get('error')}),flush=True)
if not result['passed']:raise SystemExit(1)
