#!/usr/bin/env python3
"""Real ARM ptrace observes owned service signals at actual preflight failure.
Linux UAPI: https://github.com/torvalds/linux/blob/v6.6/include/uapi/linux/ptrace.h
AArch64 generic numbers: https://github.com/torvalds/linux/blob/v6.6/include/uapi/asm-generic/unistd.h
No child-follow tracing, altered syscall result, SDK reply or artificial drain.
"""
import ctypes as C,errno,hashlib,json,os,pwd,select,shutil,signal,socket,struct,subprocess,time,traceback
from pathlib import Path
R=Path('/results');O=Path('/run/preflight-evidence');O.mkdir();s=json.loads((R/'selection.json').read_text());result={'passed':False,'expected_red':False,'actual_preflight_EADDRINUSE':False,'scope':s['scope'],'signals':[],'binds':[],'cleanup':[],'SDK_Ready_claimed':False,'exhausted_cleanup_budget_claimed':False,'source_restarted':False,'phone_accessed':False}
children=[];owned={};logs=[];service=None;trace_active=False;recv=None;sourcefd=None
libc=C.CDLL(None,use_errno=True);libc.ptrace.argtypes=[C.c_uint,C.c_uint,C.c_void_p,C.c_void_p];libc.ptrace.restype=C.c_long
TRACEME=0;SYSCALL=24;SETOPTIONS=0x4200;GET_SYSCALL_INFO=0x420e;TRACESYSGOOD=1;EXITKILL=0x100000
# UAPI op ENTRY/EXIT; AArch64 audit architecture and syscall numbers.
ARCH=0xc00000b7;NR_WRITE=64;NR_BIND=200;NR_KILL=129;NR_TKILL=130;NR_TGKILL=131;NR_PIDFD_SIGNAL=424
runtime=Path('/run/preflight-runtime');state=Path('/var/lib/preflight-state');install=Path('/opt/pokelinux-preflight')
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def pc(request,pid,addr=0,data=0):
 C.set_errno(0);v=libc.ptrace(request,pid,C.c_void_p(addr),C.c_void_p(data))
 if v==-1 and C.get_errno():raise OSError(C.get_errno(),os.strerror(C.get_errno()))
 return v
def traceme():pc(TRACEME,0)
def wait(fn,seconds=20):
 until=now()+int(seconds*1e9)
 while now()<until:
  try:
   if v:=fn():return v
  except FileNotFoundError:pass
  time.sleep(.002)
 raise TimeoutError('bounded owned preflight condition')
def start(args,name,env=None,user=False,fds=(),traced=False):
 f=(O/name).open('w');logs.append(f);opts={'user':1000,'group':1000,'extra_groups':os.getgrouplist('alarm',1000)} if user else {}
 p=subprocess.Popen(args,stdout=f,stderr=subprocess.STDOUT,env=env,pass_fds=fds,start_new_session=True,preexec_fn=traceme if traced else None,**opts);children.append((p,name));return p
def done(p,name):
 if any(v['pid']==p.pid and v['reaped'] for v in result['cleanup']):return
 if p.poll() is None:
  try:os.killpg(p.pid,signal.SIGTERM)
  except ProcessLookupError:pass
  try:p.wait(timeout=3)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=3)
 else:p.wait(timeout=1)
 result['cleanup'].append({'role':name,'pid':p.pid,'reaped':True,'returncode':p.returncode})
def cmd(args,name):
 p=start(args,name);p.wait(timeout=15);done(p,name);assert p.returncode==0,(args,(O/name).read_text());return (O/name).read_text()
def scan():
 for f in Path('/proc').iterdir():
  if not f.name.isdecimal():continue
  try:
   if int((f/'stat').read_text().split()[3])!=service.pid:continue
   pid=int(f.name);args=(f/'cmdline').read_bytes().split(b'\0')
   role='display_context_worker' if b'--display-context-worker' in args else 'messages_writer' if b'--messages-worker' in args else None
   if role and pid not in owned:owned[pid]={'role':role,'pidfd':os.pidfd_open(pid,os.O_NONBLOCK),'observed_at_ns':now()}
  except (FileNotFoundError,ProcessLookupError):pass

def target_fd(fd):
 text=Path(f'/proc/{service.pid}/fdinfo/{fd}').read_text();pid=next(int(line.split(':',1)[1]) for line in text.splitlines() if line.startswith('Pid:'));assert pid>0;return pid

def remote_bind_path(pointer,length):
 assert 2<=length<=110
 with open(f'/proc/{service.pid}/mem','rb',buffering=0) as mem:mem.seek(pointer);data=mem.read(length)
 if struct.unpack_from('H',data)[0]!=socket.AF_UNIX:return None
 return data[2:].split(b'\0')[0].decode()
def trace():
 global trace_active
 pid,status=os.waitpid(service.pid,0);assert pid==service.pid and os.WIFSTOPPED(status) and os.WSTOPSIG(status)==signal.SIGTRAP
 trace_active=True;pc(SETOPTIONS,pid,0,TRACESYSGOOD|EXITKILL);pc(SYSCALL,pid);pending=None;until=now()+20_000_000_000;steps=0
 while now()<until:
  found,status=os.waitpid(pid,os.WNOHANG)
  if not found:time.sleep(.0001);continue
  if os.WIFEXITED(status) or os.WIFSIGNALED(status):service.returncode=os.waitstatus_to_exitcode(status);trace_active=False;result['service_exit']=service.returncode;result['traced_syscall_stops']=steps;return
  assert os.WIFSTOPPED(status);sig=os.WSTOPSIG(status)
  if sig==signal.SIGTRAP|0x80:
   buf=C.create_string_buffer(128);size=pc(GET_SYSCALL_INFO,pid,128,C.addressof(buf));raw=buf.raw;op=raw[0];assert struct.unpack_from('I',raw,4)[0]==ARCH;steps+=1
   if op==1:
    assert size>=80;nr=struct.unpack_from('Q',raw,24)[0];args=struct.unpack_from('6Q',raw,32);pending=None
    if nr==NR_WRITE and args[0]==1:pending={'kind':'stdout'}
    elif nr==NR_BIND:
     path=remote_bind_path(args[1],args[2]);pending={'kind':'bind','path':path,'entry_ns':now()}
    elif nr in [NR_KILL,NR_TKILL,NR_TGKILL,NR_PIDFD_SIGNAL]:
     sigarg=args[1] if nr!=NR_TGKILL else args[2]
     if sigarg==signal.SIGKILL:
      scan();target=target_fd(args[0]) if nr==NR_PIDFD_SIGNAL else args[1] if nr==NR_TGKILL else args[0];assert target in owned,('signal target not owned',nr,target,list(owned))
      sdk=next((p for p,v in owned.items() if v['role']=='display_context_worker'),None);assert sdk is not None
      event={'syscall':nr,'target_pid':target,'target_role':owned[target]['role'],'signal':sigarg,'entry_ns':now(),'sdk_pid':sdk,'sdk_pidfd_live_before_signal':not bool(select.select([owned[sdk]['pidfd']],[],[],0)[0]),'sdk_proc_present_before_signal':Path(f'/proc/{sdk}').exists(),'target_owned_direct_child':True};result['signals'].append(event);pending={'kind':'signal','event':event}
   elif op==2:
    assert size>=33;value=struct.unpack_from('q',raw,24)[0]
    if pending:
     pending['exit_ns']=now();pending['syscall_return']=value
     if pending['kind']=='stdout':scan()
     elif pending['kind']=='bind':result['binds'].append(pending);result['actual_preflight_EADDRINUSE'] |= pending['path']==str(runtime/'admin.sock') and value==-errno.EADDRINUSE
     else:pending['event'].update({'exit_ns':pending['exit_ns'],'syscall_return':value})
    pending=None
   else:raise AssertionError(('unexpected syscall stop operation',op))
   pc(SYSCALL,pid)
  else:pc(SYSCALL,pid,0,0 if sig==signal.SIGTRAP else sig)
 raise TimeoutError('bounded20s ptrace Root fixture')
try:
 assert os.getuid()==0 and os.uname().machine=='aarch64';assert sorted(p.name for p in Path('/sys/class/net').iterdir())==['lo'];result['kernel']=os.uname().release
 for name,value in s['files'].items():assert sha(R/name)==value
 result['actual_service_elf_sha256']=sha(R/'bins/messages-session-display-fixture');assert result['actual_service_elf_sha256']==s['service_elf_sha256']
 cmd(['modprobe','virtio_gpu'],'modprobe.log');cmd(['udevadm','settle'],'udev.log');cmd(['usermod','-a','-G','video,input','alarm'],'groups.log');cmd(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'],'source-runtime.log');start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
 for p,mode in [(install,0o755),(runtime,0o755),(state,0o700),(state/'policy',0o700),(Path('/run/preflight-display'),0o700)]:p.mkdir(parents=True,exist_ok=True);p.chmod(mode)
 for name in ['messages-session-display-fixture','lifecycle-launcher']:shutil.copyfile(R/'bins'/name,install/name);(install/name).chmod(0o755)
 config=runtime/'hypr.lua';config.write_text('hl.monitor({ output = "", mode = "320x240@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })\n')
 recv,send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET);recv.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);source=os.urandom(16).hex()
 env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','PATH':'/usr/local/bin:/usr/bin','POKELINUX_SESSION_FD':str(send.fileno()),'POKELINUX_SESSION_SOURCE':source}
 compositor=start(['/usr/bin/Hyprland','--config',str(config)],'hyprland.log',env,True,(send.fileno(),));sourcefd=os.pidfd_open(compositor.pid,os.O_NONBLOCK);send.close()
 wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),120);result['original_compositor']={'pid':compositor.pid,'uid':1000,'gid':1000,'sha256':sha('/usr/bin/Hyprland'),'retained_owned_pidfd':True}
 profile=runtime/'profile.json';profile.write_text(json.dumps({'apps':[{'app':'org.omarchy.Messages','uid':11010,'executable':'/usr/bin/true','args':[]}]}));profile.chmod(0o644)
 (runtime/'admin.sock').write_text('authored regular-file obstruction for genuine actor bind EADDRINUSE\n')
 argv=[str(install/'messages-session-display-fixture'),str(runtime),str(state/'policy'),str(profile),str(install/'lifecycle-launcher'),str(state/'raw'),str(state/'inbox'),str(recv.fileno()),str(sourcefd),str(compositor.pid),'1000',source,str(wayland),'/run/preflight-display/app.sock']
 service=start(argv,'service.log',dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),fds=(recv.fileno(),sourcefd),traced=True);recv.close();recv=None;trace()
 assert service.returncode==1 and result['actual_preflight_EADDRINUSE'];assert result['signals'] and all(v['syscall_return']==0 for v in result['signals'])
 first=result['signals'][0];result['sdk_fenced_before_first_writer_signal']=first['target_role']=='display_context_worker';result['all_signal_roles']=[v['target_role'] for v in result['signals']];assert 'display_context_worker' in result['all_signal_roles'] and 'messages_writer' in result['all_signal_roles']
 if s['phase']=='red':assert first['target_role']=='messages_writer' and first['sdk_pidfd_live_before_signal'] and first['sdk_proc_present_before_signal'];result['expected_red']=True;result['red_signature']='actual_preflight_failed_bind_writer_SIGKILL_precedes_live_initializing_SDK_fence'
 else:assert result['sdk_fenced_before_first_writer_signal'];result['passed']=True
except BaseException as e:result['error']=str(e);result['traceback']=traceback.format_exc();print(result['traceback'],flush=True)
finally:
 if trace_active and service is not None:
  try:
   os.kill(service.pid,signal.SIGKILL);pc(SYSCALL,service.pid)
   while True:
    pid,status=os.waitpid(service.pid,0)
    if os.WIFEXITED(status) or os.WIFSIGNALED(status):service.returncode=os.waitstatus_to_exitcode(status);trace_active=False;break
    pc(SYSCALL,service.pid,0,signal.SIGKILL)
  except BaseException as e:result.setdefault('cleanup_errors',[]).append(str(e))
 for p,name in reversed(children):
  try:done(p,name)
  except BaseException as e:result.setdefault('cleanup_errors',[]).append(str(e))
 for pid,v in owned.items():
  dead=bool(select.select([v['pidfd']],[],[],0)[0]);absent=not Path(f'/proc/{pid}').exists();result['cleanup'].append({'role':v['role'],'pid':pid,'pidfd_death':dead,'proc_absent':absent,'ownership_observed_at_ns':v['observed_at_ns'],'reaped':dead and absent});os.close(v['pidfd'])
 if recv is not None:recv.close()
 if sourcefd is not None:os.close(sourcefd)
 for f in logs:f.close()
 result['all_tracked_children_reaped']=not result.get('cleanup_errors') and all(v['reaped'] for v in result['cleanup']);result['passed'] &= result['all_tracked_children_reaped'];(O/'result.json').write_text(json.dumps(result,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
