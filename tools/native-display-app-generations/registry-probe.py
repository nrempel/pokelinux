#!/usr/bin/env python3
"""Independent bounded actual Wayland probe, including hostile bind requests."""
import argparse,json,os,select,socket,struct,sys,time
MAX_BYTES=131072
BUDGET=3_000_000_000

def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def string(value):
    raw=value.encode()+b'\0'
    return struct.pack('=I',len(raw))+raw+b'\0'*((-len(raw))%4)
def request(object_id,opcode,body=b''):
    return struct.pack('=II',object_id,((len(body)+8)<<16)|opcode)+body

def text(body,start,maximum):
    if len(body)<start+4:raise ValueError('missing string length')
    length=struct.unpack_from('=I',body,start)[0]
    end=start+4+((length+3)//4)*4
    if not 1<=length<=maximum or len(body)<end:raise ValueError('invalid string bounds')
    raw=body[start+4:start+4+length]
    if raw[-1]!=0 or b'\0' in raw[:-1]:raise ValueError('invalid string termination')
    return raw[:-1].decode(),end

class Probe:
    def __init__(self,path):
        self.connection=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
        self.connection.settimeout(.05)
        self.connection.connect(path)
        self.data=b'';self.next_id=2;self.registries={};self.callbacks=set();self.bound={};self.error=None;self.eof=False
        self.peer_credentials=list(struct.unpack('3i',self.connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12)))
    def close(self):self.connection.close()
    def allocate(self):
        value=self.next_id;self.next_id+=1
        if self.next_id>128:raise ValueError('object ID budget exhausted')
        return value
    def send(self,data):
        end=now()+BUDGET;sent=0
        while sent<len(data):
            if now()>=end:raise TimeoutError('send deadline')
            try:sent+=self.connection.send(data[sent:])
            except socket.timeout:pass
    def messages(self):
        while len(self.data)>=8:
            obj,header=struct.unpack_from('=II',self.data);size=header>>16;opcode=header&65535
            if size<8 or size%4 or size>MAX_BYTES:raise ValueError('invalid message size')
            if len(self.data)<size:return
            body=self.data[8:size];self.data=self.data[size:]
            if obj in self.registries and opcode==0:
                if len(body)<12:raise ValueError('short global')
                name=struct.unpack_from('=I',body)[0];interface,end=text(body,4,256)
                if len(body)!=end+4:raise ValueError('invalid global size')
                version=struct.unpack_from('=I',body,end)[0]
                values=self.registries[obj];values.append({'name':name,'interface':interface,'version':version})
                if len(values)>256:raise ValueError('global count budget')
            elif obj in self.registries and opcode==1:
                if len(body)!=4:raise ValueError('invalid global removal')
                name=struct.unpack('=I',body)[0];self.registries[obj][:]=[v for v in self.registries[obj] if v['name']!=name]
            elif obj in self.callbacks and opcode==0:
                if len(body)!=4:raise ValueError('invalid sync completion')
                self.callbacks.remove(obj)
            elif obj==1 and opcode==1:
                if len(body)!=4:raise ValueError('invalid delete_id')
            elif obj==1 and opcode==0:
                if len(body)<16:raise ValueError('invalid display error')
                target,code=struct.unpack_from('=II',body);message,end=text(body,8,4096)
                if len(body)!=end:raise ValueError('invalid display error size')
                self.error={'object':target,'code':code,'message':message}
            elif self.bound.get(obj)=='xdg_wm_base' and opcode==0:
                if len(body)!=4:raise ValueError('invalid xdg ping')
                self.send(request(obj,3,body))
            else:raise ValueError('unexpected event '+str((obj,opcode)))
    def receive(self):
        try:part=self.connection.recv(65536)
        except socket.timeout:return
        if not part:self.eof=True;return
        self.data+=part
        if len(self.data)>MAX_BYTES:raise ValueError('receive budget')
        self.messages()
    def until(self,callback):
        end=now()+BUDGET
        while callback in self.callbacks and self.error is None and not self.eof:
            if now()>=end:raise TimeoutError('original protocol deadline')
            self.receive()
        return callback not in self.callbacks
    def sync(self):
        callback=self.allocate();self.callbacks.add(callback);self.send(request(1,0,struct.pack('=I',callback)))
        return self.until(callback)
    def registry(self):
        registry=self.allocate();self.registries[registry]=[];self.send(request(1,1,struct.pack('=I',registry)))
        if not self.sync() or self.error is not None or self.eof:raise RuntimeError('registry did not synchronize')
        return registry,list(self.registries[registry])
    def bind(self,registry,name,interface,version=1):
        object_id=self.allocate();self.bound[object_id]=interface
        # Send the sync in the same bounded write. The original server may reject
        # the bind and close before a second write, but its error must be read.
        callback=self.allocate();self.callbacks.add(callback)
        self.send(request(registry,0,struct.pack('=I',name)+string(interface)+struct.pack('=II',version,object_id))+request(1,0,struct.pack('=I',callback)))
        completed=self.until(callback)
        return {'forged_bind_error':self.error,'forged_bind_completed':completed,'eof':self.eof}
    def allowed(self,registry,globals):
        results=[]
        for interface in ['wl_compositor','xdg_wm_base']:
            global_=next(g for g in globals if g['interface']==interface)
            reply=self.bind(registry,global_['name'],interface,min(1,global_['version']))
            if not reply['forged_bind_completed'] or reply['forged_bind_error'] is not None:raise RuntimeError('allowed global bind failed')
            results.append(interface)
        return results

def inspect(path,forged=None,bind_allowed=False):
    probe=Probe(path)
    try:
        registry,globals=probe.registry();result={'globals':globals,'forged_bind_error':None,'forged_bind_completed':None,'pid':os.getpid(),'uid':os.getuid(),'gid':os.getgid(),'peer_credentials':probe.peer_credentials}
        if bind_allowed:result['allowed_bound']=probe.allowed(registry,globals)
        if forged is not None:result.update(probe.bind(registry,*forged))
        return result
    finally:probe.close()

def interactive(path):
    probe=Probe(path)
    try:
        registry,globals=probe.registry();bound=probe.allowed(registry,globals)
        print(json.dumps({'event':'ready','pid':os.getpid(),'uid':os.getuid(),'gid':os.getgid(),'globals':globals,'allowed_bound':bound}),flush=True)
        deadline=now()+30_000_000_000
        while now()<deadline:
            ready,_,_=select.select([sys.stdin,probe.connection],[],[],.05)
            if probe.connection in ready:
                probe.receive()
                if probe.error is not None or probe.eof:raise RuntimeError('idle connection failed')
            if sys.stdin not in ready:continue
            line=sys.stdin.readline(4097)
            if not line:return
            if len(line)>4096:raise ValueError('control line budget')
            command=json.loads(line);operation=command['op']
            if operation=='registry':
                registry,globals=probe.registry();print(json.dumps({'event':'registry','pid':os.getpid(),'globals':globals}),flush=True)
            elif operation=='forge':
                reply=probe.bind(registry,command['name'],command['interface']);print(json.dumps({'event':'forged',**reply}),flush=True);return
            elif operation=='stop':print(json.dumps({'event':'stopped'}),flush=True);return
            else:raise ValueError('unknown independent probe command')
        raise TimeoutError('interactive lifetime budget')
    finally:probe.close()

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('socket');parser.add_argument('--forged-name',type=int);parser.add_argument('--forged-interface');parser.add_argument('--bind-allowed',action='store_true');parser.add_argument('--interactive',action='store_true');args=parser.parse_args()
    if args.interactive:interactive(args.socket)
    else:
        forged=(args.forged_name,args.forged_interface) if args.forged_name is not None else None
        if (args.forged_name is None)!=(args.forged_interface is None):raise ValueError('incomplete forged selector')
        print(json.dumps(inspect(args.socket,forged,args.bind_allowed)),flush=True)
