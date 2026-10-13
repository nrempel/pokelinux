#!/usr/bin/env python3
"""Actual original ARM SDK-child supervisor controls and ownership boundary."""
import ctypes,hashlib,json,os,select,shutil,signal,socket,subprocess,time,traceback
from pathlib import Path
R=Path('/results');S=json.loads((R/'selection.json').read_text());O=Path('/tmp/display-supervisor-evidence');O.mkdir()
children=[];logs=[];owned_fds=[];kill_signals={};supervisor=None;source_receive=None;worker_pid=None;worker_pidfd=None;worker_waited=False;source_held=False
result={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'messages_actor_qualified':False,'messages_body_qualified':False,'PAM_qualified':False,'production_supervisor_qualified':False,'response_budget_ns':250000000,'original_initialization_budget_ns':5000000000}
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def wait(fn,seconds=10):
    until=now()+int(seconds*1e9)
    while now()<until:
        try:
            if value:=fn():return value
        except (FileNotFoundError,ConnectionRefusedError):pass
        time.sleep(.002)
    raise TimeoutError('bounded actual supervisor condition')
def start(args,label,uid=0,env=None,pass_fds=(),stdin=None):
    log=(O/label).open('w');logs.append(log)
    p=subprocess.Popen(args,stdout=log,stderr=subprocess.STDOUT,user=uid,group=uid,extra_groups=os.getgrouplist('alarm',1000) if uid==1000 else [],env=env,start_new_session=True,pass_fds=pass_fds,stdin=stdin);children.append((p,label));return p
def command(args,label,uid=0,env=None):
    p=start(args,label,uid,env);wait(lambda:p.poll() is not None,60);assert p.returncode==0,(label,(O/label).read_text()[-1600:]);return (O/label).read_text()
def lines(label):
    values=[]
    for line in (O/label).read_text().splitlines():
        if line.startswith('{'):
            try:values.append(json.loads(line))
            except json.JSONDecodeError:pass
    return values
def event(label,name,after=0,seconds=7):return wait(lambda:next((v for v in lines(label)[after:] if v.get('event')==name),None),seconds)
def state(pid):return next(v for v in Path(f'/proc/{pid}/status').read_text().splitlines() if v.startswith('State:'))
def kill(pid,sig):
    if pid==worker_pid and worker_pidfd is not None:signal.pidfd_send_signal(worker_pidfd,sig)
    else:os.kill(pid,sig)
    kill_signals.setdefault(pid,[]).append(signal.Signals(sig).name)
def observe_response(process,label,command,expected):
    before=len(lines(label));began=now();process.stdin.write((command+'\n').encode());process.stdin.flush();until=began+S['response_budget_ns'];response=None
    while now()<until:
        response=next((v for v in lines(label)[before:] if v.get('event')==expected),None)
        if response is not None:break
        time.sleep(.002)
    observed=now();return {'command':command,'response':response,'began_at_ns':began,'observed_at_ns':observed,'elapsed_ns':observed-began,'within_budget':response is not None and observed<=until}
def record(name,passed=True,**facts):result['groups'].append({'id':name,'passed':passed,**facts});print(('PASS ' if passed else 'DESIRED FAIL ')+name,flush=True)
def controls(command,expected):
    value=observe_response(supervisor,'owner.log',command,expected);assert value['within_budget'],value;return value
def same_owner(value,spawned):
    assert value['worker_pid']==spawned['worker_pid'] and value['until_ns']==spawned['until_ns'],value
    assert isinstance(value['admission'],bool) and isinstance(value['occupied'],bool),value
    return value
def worker_absent_and_dead():
    return not Path(f'/proc/{worker_pid}').exists() and bool(select.select([worker_pidfd],[],[],0)[0])
def terminal_denial(spawned):
    until=now()+2_000_000_000;observations=[]
    while now()<until:
        value=controls('status','status');response=same_owner(value['response'],spawned);observations.append(value)
        assert response['admission'] is False,response
        if response['state'] in ['failed','stopped'] and response['occupied'] is False and worker_absent_and_dead():return {'responses':observations,'worker_dead_and_reaped':True}
        time.sleep(.01)
    raise TimeoutError('native Root owner did not deny and reap within two seconds after terminal cause')
def worker_limits(pid):
    lines_=Path(f'/proc/{pid}/limits').read_text().splitlines();values={}
    for name,expected in [('Max address space',S['worker_memory_bytes']),('Max open files',S['worker_fd_limit']),('Max core file size',0)]:
        line=next(v for v in lines_ if v.startswith(name));parts=line[len(name):].split();assert int(parts[0])==int(parts[1])==expected,line;values[name]={'soft':int(parts[0]),'hard':int(parts[1])}
    status_=Path(f'/proc/{pid}/status').read_text();ppid=int(next(v.split()[1] for v in status_.splitlines() if v.startswith('PPid:')));nnp=int(next(v.split()[1] for v in status_.splitlines() if v.startswith('NoNewPrivs:')))
    assert ppid==supervisor.pid and nnp==1,(ppid,nnp)
    return {'limits':values,'actual_parent_pid':ppid,'NoNewPrivs':nnp}
def subreaper_wait(pid):
    try:
        got,status_=os.waitpid(pid,os.WNOHANG)
    except ChildProcessError:return None
    return {'pid':got,'wait_status':status_,'signal':os.WTERMSIG(status_) if os.WIFSIGNALED(status_) else None,'exitcode':os.waitstatus_to_exitcode(status_)} if got==pid else None
try:
    assert os.getuid()==0 and os.uname().release=='7.1.6-1-aarch64-ARCH';assert [p.name for p in Path('/sys/class/net').iterdir()]==['lo']
    for name,digest in S['files'].items():assert sha(R/name)==digest
    assert sha('/usr/bin/Hyprland')==S['hyprland_sha256']
    assert ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)==0
    result['actual_guest_child_subreaper']=True
    result['kernel']=os.uname().release;result['namespaces']={n:os.readlink('/proc/self/ns/'+n) for n in ['pid','user','mnt']}
    command(['modprobe','virtio_gpu'],'modprobe.log');command(['udevadm','settle'],'udev.log');command(['usermod','-a','-G','video,input','alarm'],'groups.log');command(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'],'runtime.log')
    start(['seatd','-g','video'],'seatd.log',env=dict(os.environ,SEATD_VTBOUND='0'))
    config=O/'hypr.lua';config.write_text('hl.monitor({ output = "", mode = "320x240@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
    env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','PATH':'/usr/local/bin:/usr/bin'}
    source_receive,source_send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET);source_receive.setblocking(False)
    env.update(POKELINUX_SESSION_FD=str(source_send.fileno()),POKELINUX_SESSION_SOURCE=os.urandom(16).hex())
    compositor=start(['/usr/bin/Hyprland','--config',str(config)],'compositor.log',1000,env,pass_fds=(source_send.fileno(),));source_send.close()
    source_alive=os.pidfd_open(compositor.pid,os.O_NONBLOCK);owned_fds.append(source_alive)
    wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),90)
    parent=Path('/run/pokelinux-display');parent.mkdir();parent.chmod(0o750);os.chown(parent,0,11010);listener=parent/'messages-wayland.sock'
    if S['phase']=='green' and S['case'] in ['held-control','initialization-deadline','parent-death','queued-late-ready']:
        kill(compositor.pid,signal.SIGSTOP);wait(lambda:'T' in state(compositor.pid),1);source_held=True
        record('actual_original_source_held_before_native_owner_spawn',pid=compositor.pid,kernel_state=state(compositor.pid))
    executable=R/'bins'/S['binary']
    if S['phase']=='green':
        install=Path('/usr/libexec/pokelinux-display-supervisor-fixture');install.mkdir(parents=True);install.chmod(0o755)
        protected_executable=install/'display-supervisor-fixture';shutil.copyfile(executable,protected_executable);protected_executable.chmod(0o755)
        assert sha(protected_executable)==sha(executable)
        for path in [protected_executable,*protected_executable.parents]:
            facts=path.stat();assert facts.st_uid==0 and not facts.st_mode&0o022,(str(path),facts.st_uid,oct(facts.st_mode))
        result['installed_root_protected_executable']={'path':str(protected_executable),'sha256':sha(protected_executable),'all_ancestors_root_owned_and_not_group_or_other_writable':True}
        executable=protected_executable
    transferred_watch=os.dup(source_alive)
    try:supervisor=start([str(executable),str(wayland),str(listener),'org.omarchy.Messages',os.urandom(16).hex(),'11010',str(compositor.pid),'1000',str(transferred_watch)],'owner.log',env=dict(os.environ,NATIVE_DISPLAY_ISOLATED='1'),pass_fds=(transferred_watch,),stdin=subprocess.PIPE)
    finally:os.close(transferred_watch)
    result['source']={'pid':compositor.pid,'uid':1000,'gid':1000,'launcher_retained_pidfd':True}
    if S['phase']=='red':
        assert S['supervisor_binary_source_existed'] is False
        ready=event('owner.log','ready');assert ready['state']=='ready' and ready['protocol_creator_disconnected'];assert supervisor.poll() is None
        kill(supervisor.pid,signal.SIGSTOP);held=wait(lambda:(v if 'T' in (v:=state(supervisor.pid)) else None),1)
        result['setup_complete']=True;record('actual_current_protocol_child_ready_then_kernel_SIGSTOP',pid=supervisor.pid,kernel_state=held,ready=ready)
        status=observe_response(supervisor,'owner.log','status','status');record('Root_owner_status_responds_with_SDK_child_held_within_250ms',passed=status['within_budget'],observed=status)
        stop=observe_response(supervisor,'owner.log','stop','stop_requested');record('Root_owner_stop_fences_with_SDK_child_held_within_250ms',passed=stop['within_budget'],observed=stop)
        assert not status['within_budget'] and not stop['within_budget'];assert 'T' in state(supervisor.pid)
        kill(supervisor.pid,signal.SIGKILL);supervisor.wait(timeout=3);assert supervisor.returncode==-signal.SIGKILL
        record('held_baseline_owned_child_actually_killed_and_reaped',pid=supervisor.pid,returncode=supervisor.returncode,signals=kill_signals[supervisor.pid]);result['red_detected']=True
    else:
        spawned=event('owner.log','spawned');assert spawned['state']=='initializing' and spawned['occupied'] is True and spawned['admission'] is False,spawned
        assert spawned['memory_bytes']==S['worker_memory_bytes'] and spawned['fd_limit']==S['worker_fd_limit'],spawned
        assert now()<spawned['until_ns'] and 0<spawned['until_ns']-now()<=S['original_initialization_budget_ns'],spawned
        worker_pid=spawned['worker_pid'];assert worker_pid!=supervisor.pid and worker_pid not in [p.pid for p,_ in children]
        worker_pidfd=os.pidfd_open(worker_pid,os.O_NONBLOCK);owned_fds.append(worker_pidfd)
        result['spawned']=spawned;result['setup_complete']=True
        if S['case']=='queued-late-ready':
            initialized=controls('status','status');status_=same_owner(initialized['response'],spawned)
            assert status_['state']=='initializing' and not status_['admission'] and status_['occupied'],status_
            assert 'T' in state(compositor.pid)
            record('native_parent_first_poll_transfers_bootstrap_before_parent_hold',observed=initialized)
            kill(supervisor.pid,signal.SIGSTOP);wait(lambda:'T' in state(supervisor.pid),1)
            assert not any(v.get('event')=='ready' for v in lines('owner.log'))
            limits=worker_limits(worker_pid);record('actual_SDK_worker_resource_bounds_before_protocol_setup',observed=limits)
            kill(compositor.pid,signal.SIGCONT);source_held=False
            wait(lambda:listener.exists() and listener.is_socket(),2)
            filtered=json.loads(command(['python3',str(R/'source/registry-probe.py'),str(listener),'--bind-allowed'],'queued-original-context-registry.log',11010))
            privileged={'ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1','zwp_virtual_keyboard_manager_v1','zwlr_layer_shell_v1'}
            assert not privileged&{g['interface'] for g in filtered['globals']};assert filtered['allowed_bound']==['wl_compositor','xdg_wm_base']
            assert 'T' in state(supervisor.pid);record('actual_original_context_accepts_filtered_client_while_native_parent_held',pid=supervisor.pid,probe=filtered,worker_pid=worker_pid)
            wait(lambda:now()>spawned['until_ns']+20_000_000,6);held_until=now();assert 'T' in state(supervisor.pid)
            kill(supervisor.pid,signal.SIGCONT);denied=terminal_denial(spawned)
            assert not any(v.get('event')=='ready' for v in lines('owner.log'))
            record('queued_original_context_ready_cannot_admit_after_unchanged_absolute_deadline',held_until_ns=held_until,original_until_ns=spawned['until_ns'],observed=denied)
        elif S['case'] in ['held-control','initialization-deadline','parent-death']:
            kill(worker_pid,signal.SIGSTOP);held=wait(lambda:(v if 'T' in (v:=state(worker_pid)) else None),1)
            assert not any(v.get('event')=='ready' for v in lines('owner.log'))
            limits=worker_limits(worker_pid);record('actual_SDK_worker_initializing_held_with_enforced_resource_bounds',pid=worker_pid,kernel_state=held,observed=limits)
            kill(compositor.pid,signal.SIGCONT);source_held=False
            if S['case']=='held-control':
                value=controls('status','status');status_=same_owner(value['response'],spawned);assert status_['state']=='initializing' and status_['occupied'] and not status_['admission'];assert 'T' in state(worker_pid)
                record('native_parent_status_responds_with_actual_SDK_worker_held_within_250ms',observed=value)
                stopped=controls('stop','stop_requested');response=same_owner(stopped['response'],spawned);assert response['state'] in ['stopping','stopped'] and not response['admission'];assert response['occupied']==(response['state']=='stopping'),response
                record('native_parent_stop_fences_held_worker_within_250ms',observed=stopped)
                final=event('owner.log','stopped');assert final['occupied'] is False and final['admission'] is False and worker_absent_and_dead(),final
                assert not any(v.get('event')=='ready' for v in lines('owner.log'))
                record('occupied_clears_only_after_actual_held_SDK_worker_reap',final=final,worker_pidfd_dead=True,worker_proc_absent=True)
            elif S['case']=='initialization-deadline':
                statuses=[]
                while now()<spawned['until_ns']:
                    value=controls('status','status');response=same_owner(value['response'],spawned);assert not response['admission'];statuses.append(value)
                    time.sleep(.04)
                denied=terminal_denial(spawned);assert not any(v.get('event')=='ready' for v in lines('owner.log'))
                record('held_initialization_rejects_readiness_at_original_five_second_deadline_and_reaps',original_until_ns=spawned['until_ns'],last_status_before_until=statuses[-1] if statuses else None,observed=denied)
            else:
                parent_limits=limits;kill(supervisor.pid,signal.SIGKILL);supervisor.wait(timeout=3);assert supervisor.returncode==-signal.SIGKILL
                wait(lambda:bool(select.select([worker_pidfd],[],[],0)[0]),2);adopted=wait(lambda:subreaper_wait(worker_pid),2);worker_waited=True
                assert adopted['signal']==signal.SIGKILL and worker_absent_and_dead(),adopted
                record('native_parent_death_kills_actual_held_SDK_worker_and_guest_subreaper_actually_reaps',parent_pid=supervisor.pid,parent_returncode=supervisor.returncode,worker_pid=worker_pid,worker_wait=adopted,worker_pidfd_dead=True)
                result['adopted_worker_reap']=adopted
        else:
            ready=event('owner.log','ready');same_owner(ready,spawned);assert ready['state']=='ready' and ready['admission'] is True and ready['occupied'] is True,ready
            assert now()<spawned['until_ns'];limits=worker_limits(worker_pid);record('native_owner_authentic_current_original_context_ready_with_resource_bounds',ready=ready,observed=limits)
            value=controls('status','status');response=same_owner(value['response'],spawned);assert response['state']=='ready' and response['admission'] is True and response['occupied'] is True
            record('native_parent_ready_status_observed_within_250ms',observed=value)
            if S['case']=='source-death':
                kill(compositor.pid,signal.SIGKILL);compositor.wait(timeout=3);assert compositor.returncode==-signal.SIGKILL
                denied=terminal_denial(spawned);record('actual_original_source_death_fences_admission_and_reaps_SDK_worker',source_pid=compositor.pid,observed=denied)
            else:
                requested=controls('stop','stop_requested');response=same_owner(requested['response'],spawned);assert not response['admission'] and response['state'] in ['stopping','stopped']
                final=event('owner.log','stopped');assert not final['occupied'] and not final['admission'] and worker_absent_and_dead()
                record('normal_native_owner_stop_fences_then_actually_reaps_SDK_worker',requested=requested,final=final)
        if S['case']!='parent-death':
            assert worker_absent_and_dead();result['native_parent_owned_worker_reaped']={'pid':worker_pid,'proc_absent':True,'pidfd_dead':True}
        result['isolated_native_parent_case_qualified']=True;result['passed']=True
except BaseException as error:result['error']=str(error);result['traceback']=traceback.format_exc();print(result['traceback'],flush=True)
finally:
    for p,label in reversed(children):
        try:
            if p.poll() is None:
                os.killpg(p.pid,signal.SIGTERM);kill_signals.setdefault(p.pid,[]).append('SIGTERM')
                try:p.wait(timeout=3)
                except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);kill_signals.setdefault(p.pid,[]).append('SIGKILL');p.wait(timeout=3)
            p.wait(timeout=1);result['cleanup'].append({'pid':p.pid,'role':label,'signals':kill_signals.get(p.pid,[]),'returncode':p.returncode,'reaped':True})
        except BaseException as error:result.setdefault('cleanup_errors',[]).append(str(error))
    if worker_pid is not None and not worker_waited:
        try:
            adopted=subreaper_wait(worker_pid)
            if adopted:result['cleanup_adopted_worker_reap']=adopted;worker_waited=True
            elif Path(f'/proc/{worker_pid}').exists():
                kill(worker_pid,signal.SIGKILL);adopted=wait(lambda:subreaper_wait(worker_pid),3);result['cleanup_adopted_worker_reap']=adopted;worker_waited=True
            if not worker_absent_and_dead():raise RuntimeError('SDK worker not actually dead and reaped after owned cleanup')
        except BaseException as error:result.setdefault('cleanup_errors',[]).append(str(error))
    if source_receive is not None:source_receive.close()
    for fd in owned_fds:os.close(fd)
    for log in logs:log.close()
    result['all_children_reaped']=not result.get('cleanup_errors') and len(result['cleanup'])==len(children);result['passed']=result['passed'] and result['all_children_reaped']
    (O/'result.json').write_text(json.dumps(result,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
