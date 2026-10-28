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
def namespace(fd):
 value=os.fstat(fd);return {'dev':value.st_dev,'ino':value.st_ino}
def enter(path):
 fd=os.open(path,os.O_RDONLY|os.O_CLOEXEC)
 try:call('setns',fd,0x40000000)
 finally:os.close(fd)
def publish(root,name,value):
 path=root/(name+'.tmp');path.write_text(json.dumps(value)+'\n');os.replace(path,root/name)
def attr(kind,value):
 data=struct.pack('=HH',4+len(value),kind)+value;return data+b'\0'*((-len(data))%4)
def header(kind,flags,sequence,body):return struct.pack('=IHHII',16+len(body),kind,flags,sequence,0)+body
def provision(path,end,name,expected_index,expected_namespace):
 enter(path);index=socket.if_nametoindex(name);assert index==expected_index and socket.if_nameindex()==[(1,'lo'),(index,name)]
 current=os.open('/proc/self/ns/net',os.O_RDONLY|os.O_CLOEXEC)
 try:assert namespace(current)==expected_namespace
 finally:os.close(current)
 receipts=[]
 with socket.socket(socket.AF_NETLINK,socket.SOCK_RAW|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC,0)as route:
  route.bind((0,0));port=route.getsockname()[0]
  for sequence,(kind,flags,body)in enumerate([
   (16,5,struct.pack('=BBHiII',0,0,0,index,1,1)),
   (20,0x605,struct.pack('=BBBBI',2,30,0,0,index)+attr(2,socket.inet_aton('192.0.2.2'))+attr(1,socket.inet_aton('192.0.2.2'))),
  ],1):
   request=header(kind,flags,sequence,body);before=now();assert before<end;assert route.sendto(request,(0,0))==len(request)
   while True:
    assert now()<end
    try:data,anc,msgflags,sender=route.recvmsg(4096,0)
    except BlockingIOError:time.sleep(.001);continue
    assert not anc and not msgflags and sender==(0,0)and len(data)>=36
    length,rkind,_,rseq,rport=struct.unpack_from('=IHHII',data);assert rkind==2 and rseq==sequence and rport in [0,port]and 36<=length<=len(data)and(length+3)&~3==len(data)and data[20:36]==request[:16]
    error=struct.unpack_from('=i',data,16)[0];assert error==0,('peer provision ACK',error)
    receipts.append({'kind':kind,'sequence':sequence,'kernel_sender':sender,'ack_error':error,'before_ns':before,'ack_at_ns':now()});break
  # Independent GETADDR dump on this provisioner socket verifies actual prefix.
  request=header(22,0x301,3,struct.pack('=BBBBI',2,0,0,0,0));assert route.sendto(request,(0,0))==len(request);addresses=[];done=False
  while not done:
   assert now()<end
   try:data,anc,msgflags,sender=route.recvmsg(65536,0)
   except BlockingIOError:time.sleep(.001);continue
   assert not anc and not msgflags and sender==(0,0);at=0
   while at<len(data):
    length,kind,flags,sequence,rport=struct.unpack_from('=IHHII',data,at);assert length>=16 and at+length<=len(data)and sequence==3 and rport in [0,port]and not flags&0x10
    body=data[at+16:at+length]
    if kind==3:assert len(body)==4 and struct.unpack('=i',body)[0]==0;done=True
    elif kind==20:
     family,prefix,_,_,ifindex=struct.unpack_from('=BBBBI',body);attrs={};offset=8
     while offset<len(body):
      size,tag=struct.unpack_from('=HH',body,offset);assert size>=4 and offset+size<=len(body);attrs[tag]=body[offset+4:offset+size];offset+=(size+3)&~3
     addresses.append({'family':family,'prefix':prefix,'ifindex':ifindex,'local':socket.inet_ntoa(attrs.get(2,attrs.get(1)))})
    else:raise AssertionError(('unexpected provision dump',kind))
    at+=(length+3)&~3
 assert addresses==[{'family':2,'prefix':30,'ifindex':index,'local':'192.0.2.2'}]
 assert not any(line.split()[1]=='00000000'for line in Path('/proc/net/route').read_text().splitlines()[1:])
 assert now()<end
 return {'passed':True,'pid':os.getpid(),'uid':os.getuid(),'actual_caps':caps(),'peer_index':index,'addresses':addresses,'default_route_absent':True,'admin_socket_closed':True,'namespace_fd_closed':True,'mutations':receipts,'completed_at_ns':now()}
def fd_inventory(udp):
 observed={}
 for raw in os.listdir('/proc/self/fd'):
  try:observed[raw]=os.readlink('/proc/self/fd/'+raw)
  except FileNotFoundError:pass # The enumeration's own descriptor is already closed.
 assert set(observed)=={'0','1','2',str(udp.fileno())}
 assert observed['0']=='/dev/null'and observed['1'].startswith('pipe:[')and observed['2']==observed['1']
 assert observed[str(udp.fileno())]=='socket:['+str(os.fstat(udp.fileno()).st_ino)+']'
 assert not any(value.startswith('net:[')for value in observed.values())
 return observed
def serve(path,root,end):
 enter(path);cap=reduce_caps();assert os.getuid()==0
 with socket.socket(socket.AF_INET,socket.SOCK_DGRAM|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC)as udp:
  udp.bind(('192.0.2.2',36042));packets=[];quiet=None;armed=None;quiet_count=0
  publish(root,'peer-ready.json',{'pid':os.getpid(),'uid':0,'caps':cap,'ready_at_ns':now(),'only_packet_socket_no_admin_descriptors':True,'actual_fd_inventory':fd_inventory(udp),'bound':list(udp.getsockname())})
  while now()<end and not(root/'stop-peer').exists():
   if quiet is None and(root/'quiet-request.json').exists():
    quiet=json.loads((root/'quiet-request.json').read_text());assert len(packets)==2 and now()<quiet['original_until_ns'];armed=now();publish(root,'peer-quiet-armed.json',{'armed_at_ns':armed,'original_until_ns':quiet['original_until_ns']})
   if quiet is not None and now()>=quiet['original_until_ns']and not(root/'peer-quiet.json').exists():
    drained=0;drain_started=now()
    while drained<16:
     try:data,source=udp.recvfrom(128)
     except BlockingIOError:eagain=now();break
     drained+=1;quiet_count+=1;publish(root,'peer-unexpected-datagram.json',{'payload_hex':data.hex(),'source':list(source),'observed_at_ns':now()})
    else:raise AssertionError('bounded quiet-end UDP drain overflow without EAGAIN')
    publish(root,'peer-quiet.json',{'received_datagrams':quiet_count,'armed_at_ns':armed,'original_until_ns':quiet['original_until_ns'],'completed_at_ns':now(),'quiet_end_drain_started_at_ns':drain_started,'quiet_end_drained_datagrams':drained,'quiet_end_actual_EAGAIN':True,'quiet_end_EAGAIN_at_ns':eagain,'caps':caps(),'actual_fd_inventory':fd_inventory(udp)})
   if not select.select([udp],[],[],.002)[0]:continue
   data,source=udp.recvfrom(128)
   if quiet is not None:
    quiet_count+=1;assert quiet_count<=16,'bounded quiet datagram budget';publish(root,'peer-unexpected-datagram.json',{'payload_hex':data.hex(),'source':list(source),'observed_at_ns':now()});continue
   assert len(data)==37 and data[:4]==b'PKL1'and data[4]==len(packets)+1 and source[0]=='192.0.2.1'
   assert all(row['payload_hex']!=data.hex()for row in packets)
   packets.append({'ordinal':len(packets)+1,'payload_hex':data.hex(),'source':list(source),'received_at_ns':now()})
   assert udp.sendto(b'ACK1'+data,source)==len(data)+4
   publish(root,'peer-packets.json',{'packets':packets,'caps':caps(),'uid':os.getuid()})
  assert(root/'stop-peer').exists(),'original packet peer work cutoff'
 return {'peer_stopped':True,'packets':len(packets),'caps':caps(),'observed_at_ns':now()}
def keeper(raw_fd,end):
 control=socket.socket(fileno=int(raw_fd));call('unshare',0x40000000);fd=os.open('/proc/thread-self/ns/net',os.O_RDONLY|os.O_CLOEXEC)
 assert socket.if_nameindex()==[(1,'lo')];value={'namespace':namespace(fd),'caps':reduce_caps(),'uid':0}
 control.sendmsg([json.dumps(value).encode()],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[fd]))]);os.close(fd)
 while not select.select([control],[],[],.002)[0]:assert now()<end
 assert control.recv(16)==b'STOP';control.close();return {'keeper_stopped':True}
if __name__=='__main__':
 mode=sys.argv[1]
 if mode=='keeper':value=keeper(sys.argv[2],int(sys.argv[3]))
 elif mode=='provision':value=provision(sys.argv[2],int(sys.argv[3]),sys.argv[4],int(sys.argv[5]),{'dev':int(sys.argv[6]),'ino':int(sys.argv[7])})
 elif mode=='serve':value=serve(sys.argv[2],Path(sys.argv[3]),int(sys.argv[4]))
 else:raise AssertionError('fixed private role required')
 print(json.dumps(value),flush=True)

# Independent Python/UAPI readback; does not reuse the production Rust parser.
# Controller calls this cap0 in the retained current namespace between Rust gates.
def ipv4_snapshot(name,end):
 index=socket.if_nametoindex(name);addresses=[]
 with socket.socket(socket.AF_NETLINK,socket.SOCK_RAW|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC,0)as route:
  route.bind((0,0));port=route.getsockname()[0];request=header(22,0x301,77,struct.pack('=BBBBI',2,0,0,0,0));assert now()<end;assert route.sendto(request,(0,0))==len(request);done=False
  while not done:
   assert now()<end
   try:data,anc,flags,sender=route.recvmsg(65536,0)
   except BlockingIOError:time.sleep(.001);continue
   assert sender==(0,0)and not anc and not flags;at=0
   while at<len(data):
    length,kind,msgflags,sequence,rport=struct.unpack_from('=IHHII',data,at);assert 16<=length<=len(data)-at and sequence==77 and rport in [0,port]and not msgflags&0x10;body=data[at+16:at+length]
    if kind==3:assert len(body)==4 and struct.unpack('=i',body)[0]==0;done=True
    elif kind==20:
     family,prefix,_,_,found=struct.unpack_from('=BBBBI',body);offset=8;attrs={}
     while offset<len(body):
      size,tag=struct.unpack_from('=HH',body,offset);assert size>=4 and offset+size<=len(body);attrs[tag]=body[offset+4:offset+size];offset+=(size+3)&~3
     if found==index:assert family==2;addresses.append({'ifindex':index,'prefix':prefix,'local':socket.inet_ntoa(attrs.get(2,attrs.get(1)))})
    else:raise AssertionError(('independent GETADDR kind',kind))
    at+=(length+3)&~3
 routes=Path('/proc/net/route').read_text();assert not any(v.split()[1]=='00000000'for v in routes.splitlines()[1:]);assert now()<end
 return {'ipv4':addresses,'ifindex':index,'default_route_absent':True,'raw_ipv4_routes':routes,'observed_at_ns':now(),'caps':caps()}
