"""Actual isolated-kernel baseline and controlled Reader lifecycle cases."""
import errno,json,os,select,signal,socket,struct,time
from pathlib import Path
from owned import now
KEYS={'schema','revision','until_ns','available','local_interfaces','address_records'}
def wait(scope,until,predicate,label):
    while now()<until:
        scope.pump();scope.check()
        value=predicate()
        if value:return value
        time.sleep(.002)
    raise AssertionError('fixture prerequisite: '+label)
def snapshot(path,pid):
    cutoff=now()+150_000_000
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as peer:
        peer.settimeout(max(.001,(cutoff-now())/1e9));peer.connect(str(path))
        assert struct.unpack('3i',peer.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(pid,0,0)
        body=b''
        while True:
            assert now()<cutoff,'snapshot original request deadline'
            peer.settimeout(max(.001,(cutoff-now())/1e9));part=peer.recv(513-len(body))
            if not part:break
            body+=part
            assert len(body)<=512,'public snapshot byte bound'
        assert now()<cutoff and body.endswith(b'\n') and body.count(b'\n')==1
    value=json.loads(body)
    assert set(value)==KEYS and value['schema']==1 and type(value['revision'])is int
    assert type(value['available'])is bool and type(value['until_ns'])is str
    until=int(value['until_ns']);assert str(until)==value['until_ns']
    if value['available']:
        assert value['revision']>0 and now()<until<=now()+2_000_000_000
        links=value['local_interfaces'];addresses=value['address_records']
        assert type(links)is int and type(addresses)is int and 0<=links<=64 and 0<=addresses<=16*links
    else:
        assert until==0 and value['local_interfaces']is None and value['address_records']is None
    return value
def release(scope,path,until):
    def opened():
        try:return os.open(path,os.O_WRONLY|os.O_NONBLOCK|os.O_CLOEXEC)
        except OSError as e:
            if e.errno==errno.ENXIO:return None
            raise
    fd=wait(scope,until,opened,'actual FIFO reader waiting')
    try:assert os.write(fd,b'x')==1
    finally:os.close(fd)
def proc_state(pid):
    try:return Path('/proc/%d/stat'%pid).read_text().rsplit(') ',1)[1].split()
    except FileNotFoundError:return None
def children(pid):
    return [int(p)for p in Path('/proc/%d/task/%d/children'%(pid,pid)).read_text().split()]
def stop(scope,item,until):
    if scope.live(item):signal.pidfd_send_signal(item['pidfd'],signal.SIGCONT)
    if scope.live(item):signal.pidfd_send_signal(item['pidfd'],signal.SIGTERM)
    code=scope.finish(item,until)
    assert code==0,'normal Main0 required after own reader closure'
def one_case(scope,normal,fixture,root,name):
    root=root/name;root.mkdir(mode=0o700);path=root/'snapshot.sock';cutoff=now()+6_000_000_000
    private=name!='normal_kernel_empty'
    if private:
        for n in ['release-result','release-exit']:os.mkfifo(root/n,0o600)
        argv=[fixture,'--fixture-serve',path,root]
    else:argv=[normal,'--serve',path]
    item=scope.start(argv,name+'.log',65536,cutoff=cutoff,wait_cutoff=cutoff)
    row={'case':name,'passed':False,'controlled_collector':private,'normal_production_binary':not private,'Main_pid':item['child'].pid}
    worker=None;worker_pidfd=None;behavior=False
    try:
        wait(scope,min(cutoff,now()+1_000_000_000),path.exists,'real listener')
        initial=snapshot(path,item['child'].pid);row['initial']=initial
        if not private:
            values=[];until=min(cutoff,now()+1_600_000_000)
            while now()<until:
                value=snapshot(path,item['child'].pid);values.append(value)
                if value['available']:break
                scope.pump();time.sleep(.01)
            row['last']=values[-1];row['snapshots']=len(values)
            behavior=values[-1]['available']and values[-1]['local_interfaces']==0 and values[-1]['address_records']==0
        else:
            ready=wait(scope,min(cutoff,now()+500_000_000),lambda:next(iter(root.glob('ready-*.json')),None),'actual fixture collector ready')
            marker=json.loads(ready.read_text());worker=int(marker['pid']);worker_pidfd=os.pidfd_open(worker,os.O_NONBLOCK)
            assert marker['parent']==item['child'].pid and children(item['child'].pid)==[worker]
            original_until=int(marker['until_ns']);row['collector']=marker
            if name=='held_collector_Stop':
                samples=[snapshot(path,item['child'].pid)for _ in range(3)]
                behavior=all(not v['available']for v in samples)
                row['held_snapshots']=len(samples)
            elif name=='wait_and_EOF_before_publication':
                release(scope,root/'release-result',min(original_until,now()+200_000_000))
                wait(scope,min(original_until,now()+200_000_000),lambda:(root/('written-%d'%worker)).exists(),'result buffered while worker retained')
                before=snapshot(path,item['child'].pid)
                row['before_actual_wait_and_EOF']=before
                assert proc_state(worker)is not None and scope.live(item),'actual worker/Main retained'
                release(scope,root/'release-exit',min(original_until,now()+200_000_000))
                value=before;until=min(cutoff,original_until+200_000_000)
                while now()<until:
                    value=snapshot(path,item['child'].pid)
                    if value['available']:break
                    scope.pump();time.sleep(.005)
                row['after_actual_worker_exit']=value
                behavior=(not before['available']and value['available']and value['local_interfaces']==3
                    and value['address_records']==7 and proc_state(worker)is None and scope.live(item))
            elif name=='queued_success_after_original_deadline':
                signal.pidfd_send_signal(item['pidfd'],signal.SIGSTOP)
                wait(scope,min(original_until,now()+200_000_000),lambda:proc_state(item['child'].pid)[0]=='T','actual Main stopped before releasing collector')
                release(scope,root/'release-result',min(original_until,now()+200_000_000))
                wait(scope,min(original_until,now()+200_000_000),lambda:(root/('written-%d'%worker)).exists(),'complete valid result written before deadline')
                release(scope,root/'release-exit',min(original_until,now()+200_000_000))
                fields=wait(scope,original_until,lambda:(lambda f:f if f and f[0]=='Z'else None)(proc_state(worker)),'actual successful collector zombie while Main stopped')
                assert int(fields[49])==0,'actual raw worker exit0 required'
                assert now()<original_until and proc_state(item['child'].pid)[0]=='T'
                row['successful_zombie_before_original_until']=True
                while now()<original_until+20_000_000:time.sleep(.005)
                signal.pidfd_send_signal(item['pidfd'],signal.SIGCONT)
                value=snapshot(path,item['child'].pid);row['after_expiry']=value
                wait(scope,min(cutoff,now()+300_000_000),lambda:proc_state(worker)is None,'same live Main actually reaped old collector')
                behavior=not value['available']and scope.live(item)
            else:raise AssertionError('unknown frozen case')
        stop(scope,item,min(cutoff,now()+500_000_000));row['Main_exit']=0
        if worker is not None:
            row['collector_absent_after_Main_wait']=proc_state(worker)is None
            row['collector_pidfd_terminal']=bool(select.select([worker_pidfd],[],[],0)[0])
            behavior=behavior and row['collector_absent_after_Main_wait']and row['collector_pidfd_terminal']
        row['socket_removed_after_Main_wait']=not path.exists()
        row['passed']=bool(behavior and row['socket_removed_after_Main_wait'])
        row['behavioral_assertion_failed']=not row['passed']
    except BaseException as error:
        row['setup_or_custody_failure']=type(error).__name__+': '+str(error)
    finally:
        if scope.live(item):
            signal.pidfd_send_signal(item['pidfd'],signal.SIGCONT)
            signal.pidfd_send_signal(item['pidfd'],signal.SIGTERM)
        if item['wait']is None:
            try:scope.finish(item,cutoff)
            except BaseException as error:row['closure_failure']=str(error)
        if worker_pidfd is not None:os.close(worker_pidfd)
    return row
