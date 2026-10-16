#!/usr/bin/env python3
"""One genuine public Hyprland focus request; owns only its control socket, never Source."""
import argparse,errno,json,os,select,socket,struct,time
from pathlib import Path

def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def emit(value):print(json.dumps(value,separators=(',',':')),flush=True)
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--control',required=True,type=Path);p.add_argument('--compositor-pid',required=True,type=int);p.add_argument('--target',required=True,choices=['render-stall-A','render-stall-B']);p.add_argument('--until-ns',required=True,type=int);a=p.parse_args()
assert os.getresuid()==(0,0,0) and a.compositor_pid>1 and now()<a.until_ns
with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM|socket.SOCK_NONBLOCK) as s:
 code=s.connect_ex(str(a.control));assert code in [0,errno.EINPROGRESS,errno.EAGAIN,errno.EWOULDBLOCK]
 while code:
  assert now()<a.until_ns;select.select([],[s],[],min(.01,max(0,(a.until_ns-now())/1e9)));code=s.getsockopt(socket.SOL_SOCKET,socket.SO_ERROR);assert code in [0,errno.EINPROGRESS]
 assert struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(a.compositor_pid,1000,1000)
 data=('dispatch hl.dsp.focus({window="class:^'+a.target+'$"})').encode();offset=0;before=now()
 while offset<len(data):
  assert now()<a.until_ns
  try:count=s.send(data[offset:]);assert count>0;offset+=count
  except BlockingIOError:select.select([],[s],[],.001)
 emit({'event':'genuine_focus_request_sent','before_ns':before,'after_ns':now(),'compositor_PID':a.compositor_pid,'target_app_id':a.target,'bytes_sent':offset})
 body=bytearray()
 while now()<a.until_ns:
  try:part=s.recv(1024)
  except BlockingIOError:select.select([s],[],[],.001);continue
  if not part:break
  body.extend(part);assert len(body)<=4096
 else:raise TimeoutError('original actual control response cutoff')
 emit({'event':'genuine_focus_response','at_ns':now(),'actual_public_dispatch_reply':body.decode(),'target_app_id':a.target})
