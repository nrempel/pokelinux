"""Authored RFB FramebufferUpdate bytes only; same actual final-frame codec caller."""
import argparse,hashlib,importlib.util,json,struct,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--module',required=True);a=p.parse_args();spec=importlib.util.spec_from_file_location('codec',a.module);codec=importlib.util.module_from_spec(spec);spec.loader.exec_module(codec)
now=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
class Wire:
 def __init__(self,data,expired=False):self.data=data;self.offset=0;self.writes=[];self.until=now()+(-1 if expired else 1_000_000_000)
 def guard(self):
  if now()>=self.until:raise TimeoutError('authored original cutoff')
 def read(self,n):
  self.guard();assert 0<=n<=12*1024*1024;v=self.data[self.offset:self.offset+n];self.offset+=len(v)
  if len(v)!=n:raise EOFError('authored truncated data')
  self.guard();return v
 def write(self,data):self.guard();self.writes.append(data)
def header(padding,count):return struct.pack('>BBH',0,padding,count)
def rect(x,y,w,h,encoding=0):return struct.pack('>HHHHi',x,y,w,h,encoding)
pixels=bytes([0x10,0x20,0x30,0])*4
cases=[('valid_zero_padding',header(0,1)+rect(0,0,2,2)+pixels,2,2,None,False),('valid_nonzero_padding',header(0xa5,1)+rect(0,0,2,2)+pixels,2,2,None,False),('zero_count_rejected',header(0,0),2,2,'bounded rectangle count',False),('excess_count_rejected',header(0,257),2,2,'bounded rectangle count',False),('nonraw_rejected',header(0,1)+rect(0,0,2,2,1),2,2,'expected raw rectangle',False),('out_of_canvas_rejected',header(0,1)+rect(1,0,2,2),2,2,'rectangle exceeds canvas',False),('incomplete_coverage_rejected',header(0,1)+rect(0,0,1,2)+pixels[:8],2,2,'nonincremental update did not cover canvas',False),('raw_bytes_limit_rejected',header(0,2)+rect(0,0,1080,2424)+bytes(1080*2424*4)+rect(0,0,1080,2424),1080,2424,'bounded raw pixels',False),('expired_original_cutoff_rejected',header(0,1)+rect(0,0,2,2)+pixels,2,2,'authored original cutoff',True)]
rows=[]
for name,data,w,h,expected_error,expired in cases:
 wire=Wire(data,expired);row={'name':name,'expected':'accept'if expected_error is None else'reject','passed':False}
 try:
  frame,meta=codec.request_raw_frame(wire,w,h,'a'*64)
  assert expected_error is None and frame==pixels and meta['frame_bytes']==16 and meta['frame_sha256']==hashlib.sha256(pixels).hexdigest()and meta['rectangles']==[{'x':0,'y':0,'width':2,'height':2}]
  assert wire.writes==[struct.pack('>BBHHHH',3,0,0,0,w,h)]and wire.offset==len(data)
  row.update(passed=True,accepted_frame_sha256=meta['frame_sha256'],metadata=meta)
 except BaseException as error:
  message=type(error).__name__+': '+str(error);row.update(error=message,passed=expected_error is not None and expected_error in message)
 row['authored_header_padding']=data[1];row['authored_header_rectangles']=struct.unpack('>H',data[2:4])[0];rows.append(row)
result={'schema':1,'scope':'Frozen authored-byte parser consumer, no socket/VM/input/actual failed-header claim','cases':rows,'passed':all(r['passed']for r in rows)};print(json.dumps(result,sort_keys=True));raise SystemExit(0 if result['passed']else 1)
