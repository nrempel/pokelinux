"""Private fixed peer roles only; no physical interface or arbitrary configuration."""
import array,ctypes,json,os,select,socket,struct,sys,time
from pathlib import Path
sys.dont_write_bytecode=True
L=ctypes.CDLL(None,use_errno=True)
NET_ADMIN=12
now=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def call(name,*args):
 if getattr(L,name)(*args)!=0:raise OSError(ctypes.get_errno(),name)
def caps():
 return {v.split(':')[0]:v.split(':')[1].strip()for v in Path('/proc/self/status').read_text().splitlines()if v.startswith(('CapInh:','CapPrm:','CapEff:','CapBnd:','CapAmb:','NoNewPrivs:'))}
def reduce_caps(keep=0):
 call('prctl',38,1,0,0,0)
 for number in range(int(Path('/proc/sys/kernel/cap_last_cap').read_text())+1):
  if not keep&(1<<number):call('prctl',24,number,0,0,0)
 class Head(ctypes.Structure):_fields_=[('version',ctypes.c_uint32),('pid',ctypes.c_int)]
 class Data(ctypes.Structure):_fields_=[('effective',ctypes.c_uint32),('permitted',ctypes.c_uint32),('inheritable',ctypes.c_uint32)]
 header=Head(0x20080522,0);data=(Data*2)()
 for i in range(2):data[i].effective=data[i].permitted=(keep>>(32*i))&0xffffffff
 call('capset',ctypes.byref(header),ctypes.byref(data));call('prctl',4,1,0,0,0)
 value=caps();assert all(int(value[k],16)==keep for k in ['CapEff','CapPrm','CapBnd'])and int(value['CapInh'],16)==int(value['CapAmb'],16)==0 and value['NoNewPrivs']=='1';return value
