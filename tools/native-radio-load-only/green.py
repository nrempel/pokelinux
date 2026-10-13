#!/usr/bin/env python3
"""Actual public Linux boundaries, independent driver retains every Popen/pidfd."""
import hashlib,json,os,selectors,shutil,signal,socket,subprocess,time
from pathlib import Path
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def boottime():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def wait(fn,seconds=2):
    until=boottime()+int(seconds*1e9)
    while boottime()<until:
        value=fn()
        if value:return value
        time.sleep(.002)
    raise AssertionError('public driver bounded wait expired')
def call(path,command):
    start=boottime()
    with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as s:
        s.settimeout(.25);s.connect(str(path));s.send(command.encode());answer=json.loads(s.recv(8192))
    elapsed=boottime()-start
    assert elapsed<250_000_000,'observed control budget exceeded'
    return answer,elapsed
def readiness(child):
    data=b'';until=boottime()+2_000_000_000
    with selectors.DefaultSelector() as sel:
        sel.register(child.stdout,selectors.EVENT_READ)
        while b'\n' not in data:
            assert boottime()<until
            if sel.select(.05):
                chunk=os.read(child.stdout.fileno(),4096);assert chunk,'owner exited before readiness';data+=chunk;assert len(data)<4096
    assert data.startswith(b'READY ')
    return data,json.loads(data.splitlines()[0][6:])
def execute(freeze,out):
    root=Path('/run/radio-public');root.mkdir()
    Path('/run/public-outside-marker').write_text('public-outside-root-marker\n')
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as endpoint:
        endpoint.bind('/run/public-outside.sock');endpoint.listen(1);endpoint.setblocking(False)
        cg=Path('/sys/fs/cgroup')
        if not (cg/'cgroup.controllers').exists():
            cg.mkdir(parents=True,exist_ok=True);subprocess.run(['mount','-t','cgroup2','none',str(cg)],check=True,timeout=5)
        controllers=(cg/'cgroup.controllers').read_text().split();assert all(x in controllers for x in ['memory','pids'])
        if (cg/'cgroup.procs').read_text().strip():
            pid1cg=cg/'radio-public-pid1';pid1cg.mkdir();(pid1cg/'cgroup.procs').write_text(str(os.getpid()))
        (cg/'cgroup.subtree_control').write_text('+memory +pids')
        checks={};reaps=[];max_control=0
        for mode in ['held','probes','second-exec','task-limit','queued-late','parent-death']:
            actual_mode='held' if mode=='parent-death' else mode
            case=root/mode;case.mkdir();minimal=case/'root';minimal.mkdir()
            for name in ['bin','proc','dev','tmp','run']:(minimal/name).mkdir()
            shutil.copyfile(freeze/'bin/radio-harmless-child',minimal/'bin/radio-harmless-child');(minimal/'bin/radio-harmless-child').chmod(0o755)
            assert digest(minimal/'bin/radio-harmless-child')==digest(freeze/'bin/radio-harmless-child')
            (minimal/'fixture').write_text('public-readonly-fixture\n')
            cgroup=cg/('radio-public-'+mode);control=case/'control.sock'
            child=subprocess.Popen([str(freeze/'bin/radio-load-owner'),str(minimal),str(cgroup),actual_mode,str(control)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0)
            prefix=b'';owned_worker=None;owner_pidfd=os.pidfd_open(child.pid);worker_pidfd=None;normal=False
            try:
                prefix,ready=readiness(child);owned_worker=ready['child_pid'];worker_pidfd=os.pidfd_open(owned_worker)
                stat=lambda:call(control,'status')[0]
                current=wait(lambda:(v if (v:=stat())['setup_verified'] else None))
                status=Path('/proc',str(owned_worker),'status').read_text();assert 'Uid:\t12020\t12020\t12020\t12020' in status
                (out/(mode+'-final-credentials.txt')).write_text(status)
                pidfds=[]
                for p in Path('/proc',str(child.pid),'fdinfo').iterdir():
                    text=p.read_text()
                    if any(line.startswith('Pid:') and line.split()[1]==str(owned_worker) for line in text.splitlines()):pidfds.append({'fd':p.name,'info':text})
                assert pidfds,'native owner must retain actual child pidfd';(out/(mode+'-retained-pidfd.json')).write_text(json.dumps(pidfds,indent=2)+'\n')
                samples=[]
                for _ in range(12):
                    value,elapsed=call(control,'status');samples.append(elapsed);assert value['registration_admission'] is False
                max_control=max(max_control,max(samples))
                if mode=='parent-death':
                    assert current['occupied'] and current['setup_verified']
                    signal.pidfd_send_signal(owner_pidfd,signal.SIGKILL)
                    stdout,stderr=child.communicate(timeout=2);reaps.append({'mode':mode,'owner_pid':child.pid,'owner_actual_wait':True,'owner_exit':child.returncode})
                    until=boottime()+2_000_000_000
                    waited=None
                    while boottime()<until:
                        pid,status=os.waitpid(owned_worker,os.WNOHANG)
                        if pid:waited={'pid':pid,'status':status,'actual_direct_wait':True};break
                        time.sleep(.002)
                    assert waited and os.waitstatus_to_exitcode(waited['status'])==-signal.SIGKILL
                    assert (cgroup/'pids.current').read_text().strip()=='0';cgroup.rmdir()
                    checks[mode]={'passed':True,'final_credentials_before_parent_death':True,'parent_killed_by_retained_pidfd':True,'orphan_actual_wait':waited,'scope':'public held child; kernel PDEATHSIG plus PTRACE_EXITKILL; no vendor cleanup','control_ns':samples}
                    (out/(mode+'-owner.log')).write_bytes(prefix+stdout+stderr);normal=True;continue
                if mode=='held':
                    current=wait(lambda:(v if (v:=stat())['checkpoints'] else None));assert current['occupied']
                elif mode=='probes':
                    current=wait(lambda:(v if len((v:=stat())['checkpoints'])==12 else None))
                    packets=current['checkpoints'];by={p['event']:p for p in packets if p['event']!='forbidden-path'}
                    assert by['outside-root']['denied'] and by['outside-root']['errno']==2
                    for event in ['ip-socket','unix-connect','filesystem-write']:assert by[event]['denied'] and by[event]['errno']==1
                    assert by['namespace']['mount_denied'] and by['namespace']['mount_errno']==1 and by['namespace']['unshare_denied'] and by['namespace']['unshare_errno']==1
                    assert all(p['denied'] and p['errno']==2 for p in packets if p['event']=='forbidden-path')
                    assert by['memory-limit']['denied'] and by['memory-limit']['errno']==12
                    assert by['fd-limit']['denied'] and by['fd-limit']['errno']==24 and by['fd-limit']['opened']==61
                    try:endpoint.accept();raise AssertionError('contained child reached public Unix endpoint')
                    except BlockingIOError:pass
                elif mode=='second-exec':
                    current=wait(lambda:(v if (v:=stat())['fenced']=='second-exec' and not v['occupied'] else None))
                    assert current['execs']==2
                elif mode=='task-limit':
                    current=wait(lambda:(v if (v:=stat())['checkpoints'] else None))
                    assert current['checkpoints'][0]['event']=='task-limit' and current['checkpoints'][0]['denied'] and current['checkpoints'][0]['threads']==15
                    assert (cgroup/'pids.current').read_text().strip()=='16'
                elif mode=='queued-late':
                    original_until=current['until_boottime_ns'];assert not current['checkpoints']
                    current=wait(lambda:(v if (v:=stat())['fenced']=='absolute-deadline' and not v['occupied'] and v['now_boottime_ns']>original_until+20_000_000 else None),6)
                    assert current['until_boottime_ns']==original_until and not current['checkpoints']
                before_stop=boottime();ack,stop_ns=call(control,'stop');assert ack['ack']=='fenced' and ack['registration_admission'] is False
                max_control=max(max_control,stop_ns)
                stopped=wait(lambda:(v if not(v:=stat())['occupied'] else None))
                assert boottime()-before_stop<2_000_000_000
                final,_=call(control,'finish');assert not final['occupied']
                stdout,stderr=child.communicate(timeout=2);assert child.returncode==0,(stdout,stderr)
                (out/(mode+'-owner.log')).write_bytes(prefix+stdout+stderr)
                lines=stdout.splitlines();report=json.loads(next(line[7:] for line in lines if line.startswith(b'RESULT ')))
                assert report['actual_root_wait'] and report['all_traced_tasks_waited'] and report['cgroup_pids_current']=='0'
                if mode=='queued-late':
                    late=[e for e in report['events'] if e['event']=='authenticated-checkpoint'];assert late and all(not e['admitted'] and e['boottime_ns']>=original_until for e in late)
                if mode=='second-exec':assert any(e['event']=='second-exec-denied' for e in report['events'])
                checks[mode]={'passed':True,'observed_before_stop':current,'stop_ns':stop_ns,'control_ns':samples,'report':report}
                reaps.append({'mode':mode,'owner_pid':child.pid,'owner_actual_wait':True,'owner_exit':child.returncode,'worker_actual_wait':report['actual_root_wait'],'all_thread_waits':report['all_traced_tasks_waited']});normal=True
            finally:
                if not normal:
                    if child.poll() is None:
                        try:call(control,'stop');wait(lambda:not call(control,'status')[0]['occupied']);call(control,'finish')
                        except Exception:signal.pidfd_send_signal(owner_pidfd,signal.SIGKILL)
                    stdout,stderr=child.communicate(timeout=2);(out/(mode+'-failed-owner.log')).write_bytes(prefix+stdout+stderr)
                    # If exact owner was killed, PID1 retains and waits its orphan. Never infer wait from VM death.
                    if owned_worker is not None:
                        try:
                            until=boottime()+2_000_000_000
                            while boottime()<until:
                                pid,status=os.waitpid(owned_worker,os.WNOHANG)
                                if pid:(out/(mode+'-failure-orphan-wait.json')).write_text(json.dumps({'pid':pid,'status':status,'actual_direct_wait':True}));break
                                time.sleep(.002)
                        except ChildProcessError:pass
                if worker_pidfd is not None:os.close(worker_pidfd)
                os.close(owner_pidfd)
        return {'passed':True,'cleanup_passed':True,'checks':checks,'owned_reaps':reaps,'maximum_observed_control_ns':max_control,'containment_scope':'public static Rust child, actual ARM64 kernel; original loader/Bionic/vendor unavailable','deadline_ns':5_000_000_000,'cleanup_deadline_ns':2_000_000_000,'registration_admission':False,'vendor_cleanup':False}
