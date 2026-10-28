"""Capability-free nonce/ACK or fresh loss observation; one send, no retry."""
from pathlib import Path
import errno,json,os,socket,stat,sys,time
sys.dont_write_bytecode=True
now=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def caps():return {line.split(':')[0]:line.split(':')[1].strip()for line in Path('/proc/self/status').read_text().splitlines()if line.startswith(('CapInh:','CapPrm:','CapEff:','CapBnd:','CapAmb:','NoNewPrivs:'))}
def inventory(udp):
 values={}
 for fd in os.listdir('/proc/self/fd'):
  try:values[fd]=os.readlink('/proc/self/fd/'+fd)
  except FileNotFoundError:pass
 assert set(values)=={'0','1','2',str(udp.fileno())}
 assert values['0']=='/dev/null'and values['1'].startswith('pipe:[')and values['2']==values['1']
 assert values[str(udp.fileno())]=='socket:['+str(os.fstat(udp.fileno()).st_ino)+']'
 return values
if len(sys.argv)==3 and sys.argv[1]=='path-probe':
 directory=Path(sys.argv[2]);assert directory.is_absolute()and directory.resolve(strict=True)==directory
 cap=caps();assert os.getresuid()==os.getresgid()==(0,0,0)
 assert all(int(v,16)==0 for k,v in cap.items()if k.startswith('Cap'))and cap['NoNewPrivs']=='1'
 ancestors=[]
 for path in [directory,*directory.parents]:
  st=path.lstat();assert stat.S_ISDIR(st.st_mode)and st.st_uid==0 and stat.S_IMODE(st.st_mode)&0o100
  assert not st.st_mode&0o022
  ancestors.append({'path':str(path),'uid':st.st_uid,'gid':st.st_gid,'mode_octal':format(stat.S_IMODE(st.st_mode),'04o')})
 assert ancestors[0]['mode_octal']=='0700'
 path=directory/('.cap0-guest-gate-probe-'+str(os.getpid()));fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_CLOEXEC,0o600)
 try:
  payload=b'actual cap0 guest gate probe\n';assert os.write(fd,payload)==len(payload);os.fsync(fd)
 finally:os.close(fd);path.unlink()
 assert not path.exists()
 print(json.dumps({'passed':True,'mode':'path-probe','uid':os.getuid(),'gid':os.getgid(),'caps':cap,'ancestors':ancestors,'actual_create_fsync_close_unlink':True,'probe_absent':True}),flush=True)
 raise SystemExit(0)
mode,payload_hex,raw_until=sys.argv[1:];until=int(raw_until);started=now();assert started<until
payload=bytes.fromhex(payload_hex);assert len(payload)==37 and payload[:4]==b'PKL1'and payload[4]in[1,2,3]
cap=caps();assert all(int(v,16)==0 for k,v in cap.items()if k.startswith('Cap'))and cap['NoNewPrivs']=='1'
st=Path('/proc/self/ns/net').stat();result={'passed':False,'mode':mode,'payload_hex':payload_hex,'original_until_ns':until,'started_at_ns':started,'caps':cap,'namespace':{'dev':st.st_dev,'ino':st.st_ino}}
with socket.socket(socket.AF_INET,socket.SOCK_DGRAM|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC)as udp:
 udp.bind(('192.0.2.1'if mode=='roundtrip'else'0.0.0.0',0));result['source']=list(udp.getsockname());result['actual_fd_inventory']=inventory(udp)
 assert now()<until
 try:sent=udp.sendto(payload,('192.0.2.2',36042));assert sent==len(payload);result['send_result']={'bytes':sent}
 except OSError as error:
  assert mode=='loss'and error.errno in[errno.EADDRNOTAVAIL,errno.ENETUNREACH,errno.EHOSTUNREACH];result['send_result']={'errno':error.errno}
 result['sent_at_ns']=now();assert result['sent_at_ns']<until
 while now()<until:
  try:body,peer=udp.recvfrom(128)
  except (BlockingIOError,InterruptedError):time.sleep(.001);continue
  assert mode=='roundtrip'and peer==('192.0.2.2',36042)and body==b'ACK1'+payload
  assert now()<until
  result.update(reply_hex=body.hex(),peer=list(peer),completed_at_ns=now(),passed=True);break
 else:
  assert mode=='loss';result.update(completed_at_ns=now(),received_datagrams=0,passed=True)
 assert result['passed']
print(json.dumps(result),flush=True)
