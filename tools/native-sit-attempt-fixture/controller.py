#!/usr/bin/env python3
"""Independent owned-process fixture controller. Run only after Root build approval.
CLI: controller.py BUILD OUTPUT red|green. No original/vendor/device execution.
"""
import csv, ctypes, errno, hashlib, json, os, platform, re, selectors, signal
import socket, stat, struct, subprocess, sys, tempfile, time
from pathlib import Path
HERE=Path(__file__).resolve().parent
MONO=time.monotonic_ns
BOOT=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
NS=1_000_000_000
CASE_NS=20*NS
CLEANUP_NS=2*NS
PIPE_MAX=65536

def require(ok,why):
    if not ok: raise RuntimeError(why)
def digest_bytes(b): return hashlib.sha256(b).hexdigest()
def pin(p):
    p=Path(p); a=p.stat(); b=p.read_bytes(); z=p.stat()
    require((a.st_dev,a.st_ino,a.st_size)==(z.st_dev,z.st_ino,z.st_size),'input changed during read')
    return {'path':str(p),'sha256':digest_bytes(b),'bytes':len(b),'device':a.st_dev,'inode':a.st_ino}
def write_json(p,x): Path(p).write_text(json.dumps(x,indent=2)+'\n')
def remaining(until):
    left=(until-MONO())/NS
    require(left>0,'fixed MONOTONIC cutoff expired')
    return left
def status_map(text): return {k:v.strip() for k,v in (s.split(':',1) for s in text.splitlines() if ':' in s)}
def stat_map(text):
    head,tail=text.rsplit(') ',1); v=tail.split()
    return {'pid':int(head.split(' (',1)[0]),'state':v[0],'ppid':int(v[1]),'pgrp':int(v[2]),'session':int(v[3]),'threads':int(v[17]),'start_ticks':int(v[19])}
def proc_capture(pid,with_maps=False):
    p=Path('/proc')/str(pid); status=(p/'status').read_text(); st=(p/'stat').read_text(); cmd=(p/'cmdline').read_bytes()
    require(cmd.endswith(b'\0'),'proc argv lacks final NUL')
    tasks=[]
    for t in sorted((p/'task').iterdir(),key=lambda v:int(v.name)):
        tasks.append({'tid':int(t.name),'status':(t/'status').read_text(),'stat':(t/'stat').read_text(),'wchan':(t/'wchan').read_text().strip()})
    try: syscall=(p/'syscall').read_text().strip()
    except OSError as e: syscall={'read_errno':e.errno}
    captured={'pid':pid,'status':status,'stat':st,'stat_fields':stat_map(st),'argv_hex':cmd.hex(),'argv':[s.decode() for s in cmd[:-1].split(b'\0')],'exe':os.readlink(p/'exe'),'exe_pin':pin(p/'exe'),'wchan':(p/'wchan').read_text().strip(),'syscall':syscall,'tasks':tasks,'captured_monotonic_ns':MONO(),'captured_boottime_ns':BOOT()}
    if with_maps:
        maps=(p/'maps').read_text(); files=[];seen=set()
        for line in maps.splitlines():
            f=line.split(maxsplit=5)
            if len(f)!=6 or not f[5].startswith('/') or f[5] in seen:continue
            q=Path(f[5]);st=q.stat();require(stat.S_ISREG(st.st_mode) and st.st_size<=64*1024*1024,'bounded owned mapped regular file');seen.add(f[5])
            v=pin(q);major,minor=(int(x,16) for x in f[3].split(':'));require(v['device']==os.makedev(major,minor) and v['inode']==int(f[4]),'live map device/inode binding')
            with q.open('rb') as fd:header=fd.read(20)
            v['ELF_machine']=struct.unpack_from('<H',header,18)[0] if header[:4]==b'\x7fELF' else None;files.append(v)
            require(len(files)<=32,'bounded owned process mapping file count')
        captured['maps']=maps;captured['mapped_regular_files']=files
    return captured
def require_proc(x,argv,ppid,pgrp,uid,gid):
    v=status_map(x['status']); s=x['stat_fields']; pid=x['pid']
    require(x['argv']==argv and int(v['Pid'])==pid and int(v['Tgid'])==pid and int(v['PPid'])==ppid,'actual process identity/argv')
    require(s['pid']==pid and s['ppid']==ppid and s['pgrp']==s['session']==pgrp,'actual PPid/processgroup/session')
    require(v['Uid'].split()==[str(uid)]*4 and v['Gid'].split()==[str(gid)]*4,'actual equal final UID/GID')
    require(int(v['Threads'])==s['threads']==len(x['tasks'])==1 and x['tasks'][0]['tid']==pid,'single main-thread process required')
    t=status_map(x['tasks'][0]['status']); require(t['Uid']==v['Uid'] and t['Gid']==v['Gid'],'task final credentials')
def subreaper_enable():
    libc=ctypes.CDLL(None,use_errno=True); get=ctypes.c_int()
    if libc.prctl(37,ctypes.byref(get),0,0,0): raise OSError(ctypes.get_errno(),'PR_GET_CHILD_SUBREAPER')
    before=get.value
    if libc.prctl(36,1,0,0,0): raise OSError(ctypes.get_errno(),'PR_SET_CHILD_SUBREAPER')
    if libc.prctl(37,ctypes.byref(get),0,0,0): raise OSError(ctypes.get_errno(),'PR_GET_CHILD_SUBREAPER')
    require(get.value==1,'childsubreaper not enabled')
    return {'before':before,'actual_after':get.value}
def child_probe():
    """Call only before launch or after Popen actual wait, never steal its wait."""
    try:
        pid,raw=os.waitpid(-1,os.WNOHANG)
        return {'ECHILD':False,'pid':pid,'actual_wait_status':raw if pid else None}
    except ChildProcessError: return {'ECHILD':True}
def native_inputs(build,contract):
    exe=(build/'target/debug/pokelinux-sit-attempt-tracer').resolve(strict=True)
    machine=os.uname().machine;expected_machine={'x86_64':62,'aarch64':183}.get(machine)
    require(expected_machine is not None,'unsupported native kernel machine')
    b=exe.read_bytes();require(b[:6]==b'\x7fELF\x02\x01','native64-bit LE ELF required');actual_machine=struct.unpack_from('<H',b,18)[0]
    require(actual_machine==expected_machine and os.access(exe,os.X_OK),'ELF machine must match actual native kernel')
    review_path=build/'build-review-root.json';review=json.loads(review_path.read_text());require(review['passed'],'strict build review must pass')
    candidates=[x for x in review['compiled_executables'] if x['target']=='pokelinux-sit-attempt-tracer' and x['ELF_machine']==expected_machine]
    require(len(candidates)==1 and candidates[0]['sha256']==digest_bytes(b),'selected compiled artifact digest/machine')
    source_root=build/'source';proto=source_root/'tools/native-sit-attempt-fixture/src/main.rs';require(pin(proto)['sha256']==contract['frozen_protocol_source_sha256'],'compiled fixture protocol changed')
    listed=review['source_files'];require(len({x['path'] for x in listed})==len(listed),'unique source review paths')
    actual=sorted(p for p in source_root.rglob('*') if p.is_file());require({str(p.relative_to(source_root)) for p in actual}=={x['path'] for x in listed},'complete strict source inventory')
    for x in listed:require(pin(source_root/x['path'])['sha256']==x['sha256'],'strict reviewed source changed')
    paths=[HERE/'controller.py',HERE/'contract.json',HERE/'references/v3-ip.hex',HERE/'references/v22.tsv',Path(sys.executable).resolve(),review_path]+actual
    if (build/'build.sh').is_file():paths.append(build/'build.sh')
    paths.append(exe)
    binding={'actual_kernel_machine':machine,'actual_kernel_release':os.uname().release,'expected_ELF_machine':expected_machine,'actual_ELF_machine':actual_machine,'selected_original_build_artifact_record':candidates[0],'actual_execution_path':str(exe),'actual_execution_sha256':digest_bytes(b),'artifact_alias_same_hash_verified':True}
    return exe,[pin(p) for p in paths],binding

def reference(contract):
    request_path=HERE/'references/v3-ip.hex'; table=HERE/'references/v22.tsv'
    require(pin(request_path)['sha256']==contract['public_request_file_sha256'] and pin(table)['sha256']==contract['public_report_TSV_sha256'],'frozen public reference pin changed')
    request=bytes.fromhex(request_path.read_text()); rows=list(csv.DictReader(table.open(),delimiter='\t')); match=[r for r in rows if r['case_id']=='v4'];require(len(match)==1,'unique fixed original TSV row')
    row=match[0]; response=bytes.fromhex(row['input144_hex']);require(len(request)==983 and len(response)==144,'complete fixed record sizes')
    scalars=['cause','opaque_i64_at_8','cid','active','protocol_code','mtu_v4','mtu_v6']; expected={k:int(row[k]) for k in scalars}
    expected.update({k:(None if row[k]=='<null>' else row[k]) for k in ['ifname','address_text']})
    expected.update({k:([] if not row[k] else row[k].split('|')) for k in ['dnses','gateways','pcscf']})
    expected['structured_address_vector_empty']=row['structured_address_vector_empty']=='true';expected['QoS_TD_vectors_empty']=row['qos_td_vectors_empty']=='true'
    require(expected==contract['expected_semantic_report'],'frozen expected semantic report changed')
    return request,response,expected

class Pipes:
    def __init__(self,p,listener):
        self.p=p;self.sel=selectors.DefaultSelector();self.raw={'stdout':bytearray(),'stderr':bytearray()};self.eof={k:False for k in self.raw};self.pending=bytearray();self.rows=[];self.conn=None;self.cred=None;self.request=None;self.listener=listener
        for name,fd in [('stdout',p.stdout),('stderr',p.stderr)]:os.set_blocking(fd.fileno(),False);self.sel.register(fd,selectors.EVENT_READ,name)
        self.sel.register(listener,selectors.EVENT_READ,'listener')
    def pump(self,until):
        for key,_ in self.sel.select(min(.05,remaining(until))):
            name=key.data
            if name=='listener':
                require(self.conn is None,'second peer connection')
                self.conn,_=self.listener.accept();self.conn.setblocking(False)
                self.cred=dict(zip(['pid','uid','gid'],struct.unpack('3i',self.conn.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))))
                self.sel.unregister(self.listener);self.sel.register(self.conn,selectors.EVENT_READ,'request');continue
            if name=='request':
                try:data,anc,flags,_=self.conn.recvmsg(984,0,socket.MSG_DONTWAIT)
                except BlockingIOError:continue
                require(len(data)==983 and not anc and not flags & (socket.MSG_TRUNC|socket.MSG_CTRUNC),'exact one983-byte request record')
                self.request=bytes(data);self.sel.unregister(self.conn);continue
            try:data=os.read(key.fileobj.fileno(),8192)
            except BlockingIOError:continue
            if not data:self.eof[name]=True;self.sel.unregister(key.fileobj);continue
            self.raw[name].extend(data);require(len(self.raw[name])<=PIPE_MAX,'bounded parent pipe capture')
            if name=='stdout':
                self.pending.extend(data)
                while b'\n' in self.pending:
                    line,rest=self.pending.split(b'\n',1);self.pending=bytearray(rest);require(0<len(line)<4096,'bounded atomic JSON record');j=json.loads(line);require(isinstance(j,dict) and isinstance(j.get('event'),str),'event JSON object');self.rows.append(j);require(len(self.rows)<=16,'bounded event count')
    def until(self,predicate,until):
        while not predicate():
            require(not all(self.eof.values()),'parent pipes closed before expected stage');self.pump(until)
    def finish(self,until):
        while not all(self.eof.values()):self.pump(until)
        require(not self.pending,'partial final stdout JSON record')
    def close(self):
        self.sel.close()
        for fd in [self.p.stdout,self.p.stderr]:
            if fd:fd.close()
        if self.conn:self.conn.close()
        self.listener.close()

def emergency_cleanup(p,pipes,until,worker_identity):
    """Kill only the new owned process group; preserve actual waits even on failure."""
    began=MONO();cutoff=min(until,began+CLEANUP_NS);record={'started_ns':began,'original_cleanup_until_ns':cutoff,'owned_pgid':p.pid,'group_kill_sent':False,'adopted_actual_waits':[],'within_original_cleanup_bound':False}
    # An unreaped Popen PID cannot be reused. After its actual wait, signal only
    # a still-matching previously captured worker in the same owned group.
    safe=p.returncode is None
    if not safe and worker_identity:
        try:
            current=stat_map((Path('/proc')/str(worker_identity['pid'])/'stat').read_text())
            safe=current['start_ticks']==worker_identity['start_ticks'] and current['pgrp']==p.pid
        except FileNotFoundError:pass
    if safe:
        try:os.killpg(p.pid,signal.SIGKILL);record['group_kill_sent']=True
        except ProcessLookupError:pass
    if p.stdin:
        p.stdin.close();p.stdin=None
    # Keep peer open while owned parent/worker are being killed and waited.
    if pipes:
        try:pipes.finish(cutoff)
        except BaseException as e:record['drain_failure']=type(e).__name__+': '+str(e)
    if p.returncode is None:
        try:p.wait(timeout=max(.001,(cutoff-MONO())/NS))
        except subprocess.TimeoutExpired:
            record['parent_wait_exceeded_cleanup_bound']=True
            # No successful qualification can follow this fallback. Never leave
            # a killed owned parent or adopted worker detached or unreaped.
            p.wait()
    record['actual_parent_wait_exit']=p.returncode
    while True:
        probe=child_probe()
        if probe['ECHILD']:record['final_ECHILD']=True;break
        if probe['pid']:record['adopted_actual_waits'].append(probe);continue
        if MONO()>=cutoff:
            record['cleanup_bound_exceeded']=True
            # All descendants belong to the sole group launched in this case.
            # Group SIGKILL was sent before waiting; final blocking actual reap
            # is failure containment, never counted as bounded success.
            try:
                pid,raw=os.waitpid(-1,0);record['adopted_actual_waits'].append({'pid':pid,'actual_wait_status':raw})
            except ChildProcessError:record['final_ECHILD']=True;break
        else:time.sleep(.001)
    record['completed_ns']=MONO();record['within_original_cleanup_bound']=record['completed_ns']<cutoff
    return record

def run_case(build,out,phase,mode,contract,request,response,expected):
    start=MONO();until=start+CASE_NS;work_until=until-CLEANUP_NS;case=out/mode;case.mkdir(mode=0o700)
    runtime=Path(tempfile.mkdtemp(prefix='sit-att-'+mode+'-',dir='/tmp'));os.chmod(runtime,0o700);peer=runtime/'peer.sock';require(len(os.fsencode(peer))<108,'bounded AF_UNIX pathname')
    exe,before,artifact_binding=native_inputs(build,contract);uid=os.getuid();gid=os.getgid();controller=os.getpid();require(uid==os.geteuid() and gid==os.getegid(),'equal real/effective controller credentials required')
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);listener.setblocking(False);listener.bind(str(peer));listener.listen(1)
    argv=[str(exe),'--runtime',str(runtime),'--peer',str(peer),'--case',mode,'--peer-pid',str(controller)]
    rec={'schema':1,'case':mode,'phase':phase,'original_started_monotonic_ns':start,'original_until_monotonic_ns':until,'work_cutoff_ns':work_until,'runtime':str(runtime),'runtime_mode':stat.S_IMODE(runtime.stat().st_mode),'controller_pid':controller,'controller_UID':uid,'controller_GID':gid,'argv':argv,'inputs_before':before,'native_artifact_binding':artifact_binding,'passed':False,'failure':None,'classification':'CONTROLLER_SETUP_FAILURE'}
    p=None;pipes=None;worker_identity=None
    try:
        require(child_probe()=={'ECHILD':True},'fresh case must start with ECHILD')
        p=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,env={'PATH':'/usr/bin:/bin','LANG':'C'})
        rec['owned_parent_pid']=p.pid;pipes=Pipes(p,listener)
        pipes.until(lambda:pipes.request is not None,work_until)
        require(pipes.request==request,'complete983-byte request differs from public golden')
        (case/'request.bin').write_bytes(pipes.request);rec['request_pin']=pin(case/'request.bin');rec['peer_credentials']=pipes.cred
        require(pipes.cred['uid']==uid and pipes.cred['gid']==gid and pipes.cred['pid']>0 and pipes.cred['pid']!=p.pid,'actual worker peer PID/UID/GID')
        firstworker=proc_capture(pipes.cred['pid']);worker_identity=firstworker['stat_fields'];require(worker_identity['ppid']==p.pid and worker_identity['pgrp']==worker_identity['session']==p.pid,'peer worker actual child of owned Rust parent')
        while True:
            try:n=pipes.conn.send(response);require(n==144,'exact144-byte response send');break
            except BlockingIOError:pipes.pump(work_until)
        (case/'response.bin').write_bytes(response);rec['response_pin']=pin(case/'response.bin')
        pipes.until(lambda:any(x['event']=='snapshots' for x in pipes.rows),work_until)
        require([x['event'] for x in pipes.rows]==['dispatch-unknown','prepared','snapshots'],'precommit event order/hold')
        dispatch,prepared,snapshots=pipes.rows;identity=dispatch['incarnation'];require(re.fullmatch('[0-9a-f]{32}',identity) is not None,'source incarnation format')
        deadline=dispatch['until_ns'];require(dispatch['worker_pid']==pipes.cred['pid'] and dispatch['request_bytes']==983 and dispatch['sampled_ns']<deadline,'dispatch fixed worker/deadline binding')
        require(prepared['actual_source']==identity and bytes.fromhex(prepared['record_hex'])==response and prepared['peer_pid_observed']==controller and prepared['report']==expected,'full prepared source/144/report/peer identity comparison')
        require(prepared['original_until_ns']==deadline and prepared['prepared_at_ns']<deadline,'prepared before same original BOOTTIME cutoff')
        require(snapshots=={'event':'snapshots','count':100,'elapsed_ns':snapshots['elapsed_ns'],'worker_still_owned':True} and 0<=snapshots['elapsed_ns']<200_000_000,'100 snapshots below200ms with worker still owned')
        # Wait briefly, within existing cutoffs, for the deliberate fixture recv.
        block_until=min(work_until,MONO()+200_000_000)
        while True:
            worker=proc_capture(pipes.cred['pid'])
            if status_map(worker['status'])['State'].startswith('S') and re.search('skb|unix|sock',worker['wchan']):break
            require(MONO()<block_until,'worker not observed blocked in fixture Unix receive');time.sleep(.001)
        worker=proc_capture(pipes.cred['pid'],with_maps=True);parent=proc_capture(p.pid,with_maps=True);controller_snapshot=proc_capture(controller)
        workerargv=[str(exe),'--pokelinux-process-worker','--peer',str(peer),'--packet',request.hex(),'--peer-pid',str(controller),'--until-ns',str(deadline)]
        require_proc(parent,argv,controller,p.pid,uid,gid);require_proc(worker,workerargv,p.pid,p.pid,uid,gid)
        require(parent['exe']==worker['exe']==str(exe) and parent['exe_pin']['sha256']==worker['exe_pin']['sha256']==pin(exe)['sha256'],'actual executable binding')
        require(worker['stat_fields']['start_ticks']==worker_identity['start_ticks'],'worker start identity stable')
        require(BOOT()<deadline,'hold capture consumed original worker deadline')
        rec.update({'dispatch':dispatch,'prepared':prepared,'snapshots':snapshots,'incarnation':identity,'fixed_worker_BOOTTIME_until_ns':deadline,'parent_proc':parent,'worker_proc':worker,'controller_proc':controller_snapshot,'both_direction_peer_PID_UID_GID_bound':True,'worker_blocked_fixture_recv_observed':True,'hold_captured_before_release':True})
        rec['released_monotonic_ns']=MONO();rec['released_boottime_ns']=BOOT();require(rec['released_boottime_ns']<deadline,'G release after original BOOTTIME cutoff')
        p.stdin.write(b'G');p.stdin.flush();p.stdin.close();p.stdin=None
        # Crucial: do not close or read EOF on the fixture peer before parent
        # actual wait and both parent pipe EOFs. Stop must kill the blocked worker.
        pipes.finish(work_until);code=p.wait(timeout=remaining(work_until));rec['actual_parent_wait_exit']=code;rec['actual_parent_waited_ns']=MONO();rec['stdout_EOF']=pipes.eof['stdout'];rec['stderr_EOF']=pipes.eof['stderr']
        pipes.conn.settimeout(remaining(work_until));require(pipes.conn.recv(1)==b'','peer EOF/no extra requests only after parent wait');rec['peer_EOF_after_parent_wait']=True;rec['peer_EOF_checked_ns']=MONO()
        events=[x['event'] for x in pipes.rows];want=['dispatch-unknown','prepared','snapshots','commit','worker-wait','successor-refused']+(['MISSING_OWNED_SIT_COMPLETION'] if phase=='red' and mode=='completion' else [])
        require(events==want,'complete event order and post-cleanup missing marker')
        commit,wait,refused=pipes.rows[3:6];present=phase=='green' and mode=='completion';want_exit=101 if phase=='red' and mode=='completion' else 0
        require(code==want_exit and not pipes.raw['stderr'],'expected exact parent exit and empty stderr')
        require(commit['event']=='commit' and commit['case']==mode and commit['completion_present']==present and commit['history_present']==present and commit['capability_present']==(present or mode!='completion'),'phase completion/current/history contract')
        require(commit['disposition']=='OutcomeUnknown' and commit['withdrawal']==({'completion':'None','expiry':'Some(Deadline)','cancel':'Some(Cancelled)'}[mode]),'dispatch uncertainty/withdrawal observation')
        require(commit['original_until_ns']==deadline and (commit['sampled_ns']>=deadline if mode=='expiry' else commit['sampled_ns']<deadline),'actual original BOOTTIME commit/expiry boundary')
        require(wait['actual_wait'] is True and wait['worker_signal']==9 and wait['worker_code'] is None and wait['occupied'] is False and wait['admitted'] is False,'actual SIGKILL wait and closed admission')
        require(wait['original_cleanup_until_ns']-wait['stop_started_ns']==CLEANUP_NS and wait['stop_started_ns']<=wait['waited_at_ns']<wait['original_cleanup_until_ns'],'original two-second Stop cleanup bound')
        require(refused=={'event':'successor-refused','reason':'AdmissionUncertain','occupied':False},'durable successor refusal')
        if want_exit==101:require(pipes.rows[-1]=={'event':'MISSING_OWNED_SIT_COMPLETION','actual_exit':101},'specific missing capability only after cleanup')
        probe=child_probe();rec['final_children']=probe;require(probe=={'ECHILD':True},'no adopted child or remaining child after actual parent wait')
        try:other,_=listener.accept()
        except BlockingIOError:pass
        else:other.close();raise RuntimeError('unexpected successor fixture connection')
        rec.update({'commit':commit,'worker_wait':wait,'successor_refused':refused,'postcancel_invariant_assertions_in_frozen_parent_passed':True,'passed':True,'classification':'QUALIFIED_INTEGRATION_RED_MISSING_COMPLETION' if want_exit==101 else 'QUALIFIED_CONTROL' if phase=='red' else 'QUALIFIED_GREEN_CASE'})
    except BaseException as e:
        rec['failure']=type(e).__name__+': '+str(e)
        if p:rec['failure_cleanup']=emergency_cleanup(p,pipes,until,worker_identity)
    finally:
        if pipes:
            for name,b in pipes.raw.items():(case/(name+'.raw')).write_bytes(bytes(b))
            rec['events']=pipes.rows;rec['pipe_EOF_actual']=pipes.eof;rec['stdout_pin']=pin(case/'stdout.raw');rec['stderr_pin']=pin(case/'stderr.raw');pipes.close()
        else:listener.close()
        if p and p.stdin:p.stdin.close();p.stdin=None
        rec['inputs_after']=[pin(x['path']) for x in before];rec['inputs_unchanged']=rec['inputs_after']==before
        if not rec['inputs_unchanged']:rec['passed']=False;rec['failure']=rec['failure'] or 'input mutation'
        rec['completed_monotonic_ns']=MONO();rec['before_original_outer_cutoff']=rec['completed_monotonic_ns']<until
        if not rec['before_original_outer_cutoff']:rec['passed']=False;rec['failure']=rec['failure'] or 'original outer cutoff exceeded'
        rec['runtime_regular_file_pins']=[pin(v) for v in sorted(runtime.rglob('*')) if stat.S_ISREG(v.lstat().st_mode)]
        write_json(case/'result.json',rec)
    return rec

def main():
    require(len(sys.argv)==4,'usage: controller.py BUILD OUTPUT red|green')
    build=Path(sys.argv[1]).resolve(strict=True);out=Path(sys.argv[2]).absolute();phase=sys.argv[3];require(phase in {'red','green'} and out.is_absolute(),'absolute OUTPUT and red|green required')
    out.mkdir(mode=0o700,parents=False,exist_ok=False);os.chmod(out,0o700)
    contract=json.loads((HERE/'contract.json').read_text());request,response,expected=reference(contract)
    sub=subreaper_enable();require(child_probe()=={'ECHILD':True},'controller must begin with no children')
    results=[]
    for mode in ['completion','expiry','cancel']:
        r=run_case(build,out,phase,mode,contract,request,response,expected);results.append(r)
        if not r['passed']:break
    final=child_probe();ids=[r['incarnation'] for r in results if 'incarnation' in r];passed=len(results)==3 and all(r['passed'] for r in results) and final=={'ECHILD':True} and len(set(ids))==3
    record={'schema':1,'scope':'Explicit ordinary owned native-host process/Unix-record fixture; no CPIF/device/provider compatibility or APN/transport authority','phase':phase,'qualified':passed,'integration_outcome':'RED_MISSING_COMPLETION' if passed and phase=='red' else 'GREEN' if passed else 'SETUP_OR_FIXTURE_FAILURE','controller_pin':pin(__file__),'contract_pin':pin(HERE/'contract.json'),'build':str(build),'actual_controller_pid':os.getpid(),'actual_controller_uid':os.getuid(),'actual_controller_gid':os.getgid(),'child_subreaper':sub,'cases':[{'case':r['case'],'passed':r['passed'],'actual_parent_wait_exit':r.get('actual_parent_wait_exit'),'result_pin':pin(out/r['case']/'result.json')} for r in results],'final_children':final,'all_source_incarnations_distinct':len(set(ids))==len(ids),'physical_phone_accessed':False,'vendor_or_original_code_executed':False}
    write_json(out/'result.json',record);print(json.dumps({'qualified':passed,'phase':phase,'integration_outcome':record['integration_outcome'],'output':str(out)}),flush=True)
    return 0 if passed else 1
if __name__=='__main__':
    try:sys.exit(main())
    except BaseException as e:
        if isinstance(e,SystemExit):raise
        print('controller setup: '+type(e).__name__+': '+str(e),file=sys.stderr,flush=True);sys.exit(1)
