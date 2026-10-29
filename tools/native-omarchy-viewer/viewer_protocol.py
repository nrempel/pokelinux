"""Fixed seven-action test handoff over the already-selected private 9P tree."""
import hashlib,json,os,stat
from pathlib import Path
ACTIONS=('bar-1','apps-1','sounds-1','close-1','bar-2','apps-2','sounds-2')

def read(path):
 p=Path(path)
 try:fd=os.open(p,os.O_RDONLY|os.O_NONBLOCK|os.O_CLOEXEC|os.O_NOFOLLOW)
 except FileNotFoundError:return None
 try:
  st=os.fstat(fd);assert stat.S_ISREG(st.st_mode)and 0<st.st_size<=8192
  raw=os.read(fd,8193);assert len(raw)==st.st_size and len(raw)<=8192
 finally:os.close(fd)
 value=json.loads(raw);assert isinstance(value,dict);return value

def publish(path,value):
 p=Path(path);assert not p.exists();raw=(json.dumps(value,sort_keys=True,separators=(',',':'))+'\n').encode();assert len(raw)<=8192
 tmp=p.with_name(p.name+'.tmp');fd=os.open(tmp,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_CLOEXEC|os.O_NOFOLLOW,0o600)
 try:
  sent=0
  while sent<len(raw):sent+=os.write(fd,raw[sent:])
  os.fsync(fd)
 finally:os.close(fd)
 os.replace(tmp,p)

def digest(value):return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()

def validate_action(value,seq,selection_sha):
 assert value['schema']==1 and value['selection_sha256']==selection_sha and value['seq']==seq
 assert value['action']==ACTIONS[seq-1]and 1<=seq<=7
 if value['action']=='close-1':assert value['point']is None
 else:
  assert isinstance(value['point'],list)and len(value['point'])==2
  x,y=value['point'];assert type(x)is type(y)is int and 0<x<1080 and 0<y<2424
 assert type(value['guest_until_ns'])is int
 return value
