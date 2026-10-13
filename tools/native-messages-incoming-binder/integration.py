"""Actual Binder -> supervised sole storage child -> current-authority fixture."""
import hashlib,json,os,select,shutil,signal,socket,sqlite3,struct,subprocess,time
from pathlib import Path
from peer_session import BASE,PeerSession,storage_children
PDU='000008915155100000006201213141510005e8329bfd06'
Q={'query':'conversations','after':None,'limit':8}
def wait(fn,seconds=4):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        try:
            value=fn()
            if value:return value
        except (FileNotFoundError,ConnectionRefusedError,ProcessLookupError,sqlite3.OperationalError):pass
        time.sleep(.005)
    raise AssertionError('bounded actual integration wait')
def call(path,value):
    with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as sock:
        sock.settimeout(2);sock.connect(str(path));sock.send(json.dumps(value,separators=(',',':')).encode());return json.loads(sock.recv(4096))
def probe(*args,okay=True):
    run=subprocess.run([str(BASE/'target/aarch64-unknown-linux-gnu/debug/incoming-probe'),*map(str,args)],capture_output=True,text=True,timeout=4)
    if okay:assert run.returncode==0,'synthetic probe control failed: '+run.stderr
    return run
class Service:
    next=0
    def __init__(self,out,session,store=None,fault='none'):
        Service.next+=1;self.out=out;self.session=session;self.fault=fault;self.child=None
        self.i=Path('/opt/incoming-messages');self.r=Path('/run/incoming-messages-'+str(Service.next))
        self.s=store or Path('/var/lib/incoming-messages-'+str(Service.next));self.i.mkdir(exist_ok=True)
        for d,m in [(self.r,0o755),(self.r/'app',0o700),(self.s,0o700),(self.s/'policy',0o700)]:d.mkdir(parents=True,exist_ok=True);d.chmod(m)
        os.chown(self.r/'app',11010,11010)
        bins=BASE/'target/aarch64-unknown-linux-gnu/debug'
        for name in ['messages-incoming-fixture-service','messages-service','lifecycle-launcher','lifecycle-client','messages-stop']:
            shutil.copyfile(bins/name,self.i/name);(self.i/name).chmod(0o755)
        self.profile=self.r/'profile.json';self.profile.write_text(json.dumps({'apps':[{'app':'org.omarchy.Messages','uid':11010,
            'executable':str(self.i/'lifecycle-client'),'args':[str(self.r/'app.sock'),str(self.r/'app/driver.sock')]}]}))
    def admin(self,value):return call(self.r/'admin.sock',value)
    def status(self):return self.admin({'op':'messages_status'})
    def start(self,available=True):
        args=[str(self.i/'messages-incoming-fixture-service'),str(self.r),str(self.s/'policy'),str(self.profile),str(self.i/'lifecycle-launcher'),str(self.s/'raw'),str(self.s/'inbox'),self.session.generation,self.fault]
        self.child=subprocess.Popen(args,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        wait(lambda:(self.r/'admin.sock').exists());wait(lambda:self.admin({'op':'snapshot'}).get('ready'))
        if available:wait(lambda:self.status().get('available'))
        return self
    def stop(self):
        if self.child is None:return
        if self.child.poll() is None:
            try:assert self.admin({'op':'stop'})=={'result':'ok'}
            except (OSError,AssertionError,ValueError):self.child.kill()
        self.child.wait(timeout=5);self.child=None
        assert not storage_children(),'owned storage child must actually be reaped'
    def worker(self):
        children=storage_children();assert len(children)==1,'exactly one Messages storage owner required';return children[0]
    def raw(self):
        with sqlite3.connect('file:'+str(self.s/'raw/receipts.sqlite3')+'?mode=ro',uri=True,timeout=0) as db:
            return db.execute('SELECT id,slot,source_incarnation,scope,ack,raw FROM receipts ORDER BY seq').fetchall()
    def records(self):
        with sqlite3.connect('file:'+str(self.s/'inbox/inbox.sqlite3')+'?mode=ro',uri=True,timeout=0) as db:
            return db.execute('SELECT id,slot,source_incarnation,state,body FROM records ORDER BY seq').fetchall()
    def driver(self,value):return call(self.r/'app/driver.sock',value)
    def launch(self):
        (self.r/'app/driver.sock').unlink(missing_ok=True)
        pid=os.fork()
        if pid==0:
            try:
                os.setgroups([]);os.setgid(1000);os.setuid(1000)
                assert call(self.r/'launch-messages.sock',{'op':'launch_messages'})=={'result':'ok'};os._exit(0)
            except BaseException:os._exit(9)
        assert os.waitpid(pid,0)[1]==0,'actual desktop UID launch ingress failed'
        wait(lambda:(self.r/'app/driver.sock').exists())
    def close_app(self):
        try:self.driver({'driver':'exit'})
        except (OSError,ValueError):pass
        wait(lambda:self.admin({'op':'snapshot'}).get('principals')==0)
    def authorise(self):
        for request in [{'op':'locked','locked':False},{'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':True},{'op':'background','app':'org.omarchy.Messages','background':False}]:
            assert self.admin(request)=={'result':'ok'}
    def send(self,q=Q,ttl=1000):
        assert self.driver({'driver':'send','channel':0,'request':{'op':'messages_read','request':{'query':q,'ttl_ms':ttl}}})=={'driver_result':'sent'}
    def receive(self):
        return wait(lambda:(value if (value:=self.driver({'driver':'receive','channel':0})).get('driver_result')!='pending' else False))
    def query(self,q=Q):self.send(q);return self.receive()
    def unavailable_reaped(self):
        wait(lambda:not self.status()['occupied']);assert not self.status()['available'] and not self.status()['receiving']
    def history_restart(self):
        assert self.admin({'op':'messages_restart'})=={'result':'ok'};wait(lambda:self.status()['available']);assert not self.status()['receiving']
    def arrival(self,slot=0,kind=0,hex=PDU,typ=0):return probe('incoming',slot,kind,hex,typ,okay=False)
def run(account,out,result,phase):
    groups=result['checks'];session=PeerSession(account,out,'basic');service=None
    try:
        session.start();service=Service(out,session).start();worker=service.worker()
        assert service.status()['receiving'] and service.admin({'op':'snapshot'})['principals']==0
        opened=[os.readlink(path) for path in (Path('/proc')/str(worker)/'fd').iterdir() if path.exists()]
        assert any('receipts.sqlite3' in path for path in opened) and any('inbox.sqlite3' in path for path in opened)
        parent_fds=[os.readlink(path) for path in (Path('/proc')/str(service.child.pid)/'fd').iterdir() if path.exists()]
        assert not any('receipts.sqlite3' in path or 'inbox.sqlite3' in path for path in parent_fds)
        for _ in range(2):assert service.arrival().returncode==0
        wait(lambda:service.status()['incoming_completed']==2);wait(lambda:not service.status()['busy']);assert len(service.raw())==2
        raw=service.raw();ids=[row[0] for row in raw]
        assert ids[0]!=ids[1] and all(row[3:5]==('test_backend','stored') and row[5]==bytes.fromhex(PDU) for row in raw)
        before=len(service.records())
        service.launch();assert service.query()=={'result':'denied'};service.authorise()
        conversations=service.query();assert len(conversations['items'])==1
        mq={'query':'messages','conversation':conversations['items'][0]['id'],'after_ordinal':0,'limit':2}
        messages=service.query(mq);assert [row['receipt'] for row in messages['items']]==ids and all(row['text']=='hello' for row in messages['items'])
        groups['closed_window_raw_and_current_authority']={'passed':before==2,'projected_before_any_read':before,
            'real_binder_callbacks':2,'raw_occurrences':2,'distinct_equal_receipts':True,'sole_storage_child':True,
            'default_denied':True,'actual_current_authority_read':True,'fixture':session.evidence}
        if before!=2:
            result['expected_red']=phase=='incoming-red';result['red_signature']='raw_committed_background_projection_missing'
            raise AssertionError('real Binder raw committed twice; projection absent before window/read')
        assert phase=='incoming-green'
        service.close_app();assert service.worker()==worker and service.status()['receiving']
        assert service.arrival(slot=1).returncode==0;assert service.arrival(kind=1,hex='00').returncode==0;assert service.arrival(hex='01').returncode==0
        wait(lambda:service.status()['incoming_completed']==5);assert len(service.records())==5;assert len([r for r in service.records() if r[3]=='text'])==3
        assert {r[3] for r in service.records()}=={'text','status_report','malformed'}
        service.launch();assert service.query()=={'result':'denied'};service.authorise()
        for operation in [{'op':'locked','locked':True},{'op':'suspended','suspended':True},{'op':'background','app':'org.omarchy.Messages','background':True}]:
            assert service.admin(operation)=={'result':'ok'};count=len(service.raw());assert service.arrival().returncode==0
            wait(lambda:service.status()['incoming_completed']==count+1);wait(lambda:not service.status()['busy']);assert len(service.records())==count+1;assert service.query()=={'result':'denied'}
            key=next(k for k in operation if k in ['locked','suspended','background']);assert service.admin(dict(operation,**{key:False}))=={'result':'ok'}
        groups['clean_close_background_lock_and_exclusions']={'passed':True,'same_storage_child':service.worker()==worker,'slots':2,'closed_app_arrivals':3,'policy_denied_arrivals':3,'fixture':session.evidence}
        # Existing held-read close is an explicit source failure, not continuous receiving.
        os.kill(worker,signal.SIGSTOP);service.send(mq);wait(lambda:service.status()['busy'])
        start=time.monotonic();assert service.admin({'op':'snapshot'})['ready'];assert time.monotonic()-start<.5
        assert service.admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':False})=={'result':'ok'}
        os.kill(worker,signal.SIGCONT);assert service.receive()=={'result':'denied'};service.authorise()
        os.kill(worker,signal.SIGSTOP);service.send(mq);wait(lambda:service.status()['busy']);service.close_app();service.unavailable_reaped()
        old_ids=[r[0] for r in service.raw()];service.history_restart();assert [r[0] for r in service.raw()]==old_ids
        service.arrival();assert service.status()['incoming_started']==0 and [r[0] for r in service.raw()]==old_ids
        counts=json.loads(probe('counters',0).stdout);assert counts[:2]==[1,1]
        service.launch();assert service.query()=={'result':'denied'};service.authorise();assert service.query(mq)['page']=='messages'
        service.close_app();store=service.s;service.stop();service=Service(out,session,store=store).start()
        assert not service.status()['receiving'] and json.loads(probe('counters',0).stdout)[:2]==[1,1]
        assert [r[0] for r in service.raw()]==old_ids
        groups['held_read_failure_and_history_only_child_whole_restart']={'passed':True,'raw_ids_unchanged':True,'backend_registration_attempts':1,'stale_callback_cannot_admit':True,'fixture':session.evidence}
        service.stop();service=None;old_generation=session.generation;old_source=raw[0][2];session.close()
        session=PeerSession(account,out,'fresh-successor');session.start();assert session.generation!=old_generation
        service=Service(out,session,store=store).start();assert service.status()['receiving'];assert service.arrival().returncode==0
        wait(lambda:service.status()['incoming_completed']==1);assert len(service.raw())==len(old_ids)+1 and len(service.records())==len(old_ids)+1
        assert [r[0] for r in service.raw()][:-1]==old_ids and service.raw()[-1][2]!=old_source
        groups['explicit_independently_fresh_synthetic_successor']={'passed':True,'new_remote_generation':True,'preserved_old_ids':True,'new_source_incarnation':True,'fixture':session.evidence}
    except Exception:
        if service:
            result['diagnostic']={'status':service.status(),'raw_count':len(service.raw()),'record_count':len(service.records())}
        raise
    finally:
        if service:service.stop()
        session.close()
    if phase=='incoming-green':
        failures(account,out,result)
        systemd_restart(account,out,result)
        stop_completion(account,out,result,'incoming-stop-green')
        systemd_stop_pending(account,out,result,'incoming-systemd-stop-green')

def failures(account,out,result):
    scenarios=[('held-registration','none',True),('death-active','none',False),('oversized','none',False),
        ('ack-exp','none',False),('raw-sqlite-busy','none',False),('projection-sqlite-busy','none',False),
        ('queue-overflow','hold-after-raw',False),('ingest-deadline-SIGSTOP','hold-after-raw',False),
        ('completion-loss','exit-before-complete',False),('start-ipc-EAGAIN','small-ipc',False),('snapshot33','none',False)]
    for name,fault,hold in scenarios:
        session=PeerSession(account,out,name,hold=hold);service=None;writer=None;parent_stopped=False
        detail={'passed':False,'fixture':session.evidence};result['checks'][name]=detail
        try:
            session.start();service=Service(out,session,fault=fault).start(available=not hold)
            if hold:
                wait(lambda:not service.status()['occupied']);assert not service.status()['available']
                counters=json.loads(probe('counters',0).stdout);assert counters[:2]==[1,0]
                assert len(list((service.s/'raw').glob('incoming-attempt-*')))==1
                service.history_restart();assert json.loads(probe('counters',0).stdout)[:2]==[1,0]
                store=service.s;service.stop();service=Service(out,session,store=store).start()
                assert not service.status()['receiving'] and json.loads(probe('counters',0).stdout)[:2]==[1,0]
                detail.update({'durable_uncertainty_across_child_and_whole_restart':True,'remote_registration_not_retried':True})
            elif name=='death-active':
                assert service.arrival().returncode==0;wait(lambda:service.status()['incoming_completed']==1);assert len(service.records())==1
                assert all(json.loads(probe('counters',slot).stdout)[3:]==[0,0,0,0] for slot in range(2));detail['zero_ack_send_response_ack_other_before_death']=True
                probe('die');service.unavailable_reaped();ids=[r[0] for r in service.raw()];service.history_restart()
                assert [r[0] for r in service.records()]==ids;detail['history_ids_exact']=True
            elif name in ['oversized','ack-exp']:
                service.arrival(hex='00'*513 if name=='oversized' else PDU,typ=1 if name=='ack-exp' else 0)
                service.unavailable_reaped();assert len(service.raw())==0;detail['no_raw_admission_or_ack_promise']=True
            elif name in ['raw-sqlite-busy','projection-sqlite-busy']:
                target=service.s/('raw/receipts.sqlite3' if name=='raw-sqlite-busy' else 'inbox/inbox.sqlite3')
                writer=sqlite3.connect(target,timeout=0);writer.execute('BEGIN IMMEDIATE')
                assert service.arrival().returncode==0;service.unavailable_reaped()
                raw=service.raw();assert len(raw)==(0 if name=='raw-sqlite-busy' else 1)
                assert len(service.records())==0;writer.rollback();writer.close();writer=None
                ids=[r[0] for r in raw];service.history_restart();assert [r[0] for r in service.records()]==ids
                detail.update({'real_external_sqlite_lock':True,'raw_commit_preserved':bool(ids),'history_recovered_exact_ids':True})
            elif name in ['queue-overflow','ingest-deadline-SIGSTOP']:
                assert service.arrival().returncode==0;wait(lambda:service.status()['incoming_raw_committed']==1);assert len(service.raw())==1;worker=service.worker()
                if name=='queue-overflow':
                    burst=probe('burst',0,0,PDU,0,16);detail['callbacks_submitted']=json.loads(burst.stdout)['submitted']
                else:
                    os.kill(worker,signal.SIGSTOP);start=time.monotonic();assert service.admin({'op':'snapshot'})['ready'];assert time.monotonic()-start<.5
                service.unavailable_reaped();assert not Path('/proc/'+str(worker)).exists();assert len(service.raw())==1 and len(service.records())==0
                ids=[r[0] for r in service.raw()];failure=service.status().get('incoming_failure');service.history_restart()
                assert [r[0] for r in service.records()]==ids and json.loads(probe('counters',0).stdout)[:2]==[1,1]
                detail.update({'failure':failure,'original_ingress_deadline':True,'actual_child_reaped':True,'raw_projection_gap_recovered_exact_ids':True})
            elif name=='completion-loss':
                assert service.arrival().returncode==0;service.unavailable_reaped();ids=[r[0] for r in service.raw()]
                assert len(ids)==1 and [r[0] for r in service.records()]==ids;service.history_restart();assert [r[0] for r in service.raw()]==ids
                detail.update({'committed_projection_survives_lost_completion':True,'no_arrival_recreation':True})
            elif name=='start-ipc-EAGAIN':
                os.kill(service.child.pid,signal.SIGSTOP);parent_stopped=True;probe('burst',0,0,PDU,0,16)
                wait(lambda:not storage_children() or all((Path('/proc')/str(pid)/'stat').read_text().split()[2]=='Z' for pid in storage_children()))
                os.kill(service.child.pid,signal.SIGCONT);parent_stopped=False;service.unavailable_reaped()
                assert service.status()['incoming_failure']=='child_exit_74','actual start-send EAGAIN marker missing'
                detail.update({'actual_start_send_EAGAIN':True,'failure_notification_independent_of_full_socket':True});service.history_restart()
            elif name=='snapshot33':
                for count in range(1,34):
                    assert service.arrival().returncode==0
                    if count<33:wait(lambda:service.status()['incoming_completed']==count);wait(lambda:not service.status()['busy']);assert len(service.raw())==count
                service.unavailable_reaped();assert len(service.raw())==33 and len(service.records())==32
                assert service.admin({'op':'messages_restart'})=={'result':'ok'};wait(lambda:not service.status()['occupied']);assert not service.status()['available']
                detail.update({'raw33_committed':True,'partial_latest32_projection_refused':True,'continuous_production_ingestion_qualified':False})
            if name!='death-active':
                counters=[json.loads(probe('counters',slot).stdout) for slot in range(2)]
                assert all(row[3:]==[0,0,0,0] for row in counters);detail['zero_ack_send_response_ack_other']=True
            detail['passed']=True
        finally:
            if parent_stopped and service and service.child and service.child.poll() is None:os.kill(service.child.pid,signal.SIGCONT)
            if writer:writer.rollback();writer.close()
            if service:service.stop()
            session.close()

def systemd_restart(account,out,result):
    session=PeerSession(account,out,'systemd-restart');service=None;unit='pokelinux-incoming-fixture.service';started=False
    detail={'passed':False,'fixture':session.evidence};result['checks']['actual-systemd-whole-service-source-fence']=detail
    try:
        session.start();device=Path('/dev/binderfs/incoming');os.chown(device,account.pw_uid,0);device.chmod(0o660)
        service=Service(out,session)
        stable_profile=service.i/'systemd-profile.json';shutil.copyfile(service.profile,stable_profile)
        args=[str(service.i/'messages-incoming-fixture-service'),str(service.r),str(service.s/'policy'),str(stable_profile),str(service.i/'lifecycle-launcher'),str(service.s/'raw'),str(service.s/'inbox'),session.generation,'none']
        path=Path('/etc/systemd/system')/unit
        path.write_text('[Unit]\nDescription=Isolated incoming Messages source-fence fixture\n[Service]\nType=simple\nExecStart='+ ' '.join(args)+'\nExecStop='+str(service.i/'messages-stop')+' '+str(service.r/'admin.sock')+'\nRuntimeDirectory='+service.r.name+'\nRuntimeDirectoryMode=0755\nUMask=0077\nNoNewPrivileges=yes\nCapabilityBoundingSet=CAP_SETUID CAP_SETGID CAP_KILL\nTasksMax=96\nMemoryMax=512M\nKillMode=control-group\nTimeoutStopSec=5\nRestart=no\n')
        subprocess.run(['systemctl','daemon-reload'],check=True,timeout=5);subprocess.run(['systemctl','start',unit],check=True,timeout=5);started=True
        wait(lambda:service.status().get('receiving'));old_worker=service.worker();assert service.arrival().returncode==0;wait(lambda:service.status()['incoming_completed']==1);assert len(service.records())==1
        ids=[r[0] for r in service.raw()]
        subprocess.run(['systemctl','restart',unit],check=True,timeout=8);wait(lambda:service.status()['available'])
        assert not service.status()['receiving'] and service.worker()!=old_worker and not Path('/proc/'+str(old_worker)).exists()
        assert [r[0] for r in service.raw()]==ids and [r[0] for r in service.records()]==ids
        assert json.loads(probe('counters',0).stdout)[:2]==[1,1]
        detail.update({'passed':True,'actual_systemd_start_restart':True,'source_attempt_durable':True,'no_backend_reregistration':True,'exact_history_ids':True})
    except Exception:
        show=subprocess.run(['systemctl','show',unit,'-p','Result','-p','ExecMainStatus','-p','ActiveState','-p','MainPID'],capture_output=True,text=True,timeout=5)
        (out/'systemd-restart-status.txt').write_text(show.stdout+show.stderr)
        journal=subprocess.run(['journalctl','-u',unit,'-n','40','--no-pager'],capture_output=True,text=True,timeout=5)
        (out/'systemd-restart-journal.txt').write_text(journal.stdout+journal.stderr)
        detail['policy_files']=[p.name for p in (service.s/'policy').iterdir()]
        try:detail['last_status']=service.status()
        except OSError:detail['last_status']='socket unavailable'
        raise
    finally:
        if started:subprocess.run(['systemctl','stop',unit],check=True,timeout=8)
        assert not storage_children();session.close()


def stop_completion(account,out,result,phase):
    session=PeerSession(account,out,'stop-completion');service=None;held=None;peerfd=None
    detail={'passed':False,'fixture':session.evidence};result['checks']['actual-stop-pending-completion']=detail
    try:
        session.start();service=Service(out,session).start();main=service.child.pid
        # Pin the actual root socket origin in the kernel; never open a numeric PID.
        with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as sock:
            sock.connect(str(service.r/'admin.sock'));pid,uid,gid=struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
            peerfd=sock.getsockopt(socket.SOL_SOCKET,77) # Linux SO_PEERPIDFD
            fdinfo=Path('/proc/self/fdinfo/'+str(peerfd)).read_text()
            assert pid==main and uid==0 and 'Pid:\t'+str(main)+'\n' in fdinfo
            poll=select.poll();poll.register(peerfd,select.POLLIN);assert not poll.poll(0)
            for path in Path('/proc').iterdir():
                if not path.name.isdecimal():continue
                try:parts=(path/'cmdline').read_bytes().split(b'\0');stat=(path/'stat').read_text().split()
                except OSError:continue
                if b'--storage-worker' in parts and int(stat[3])==main:held=int(path.name)
            assert held,'actual policy storage child required';os.kill(held,signal.SIGSTOP)
            helper=subprocess.Popen([str(service.i/'messages-stop'),str(service.r/'admin.sock')],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            returned_early=False
            try:
                helper.wait(timeout=.25);returned_early=helper.returncode==0 and not poll.poll(0)
                detail.update({'helper_exit':helper.returncode,'actual_service_still_alive':not poll.poll(0)})
            except subprocess.TimeoutExpired:
                assert not poll.poll(0),'service must still be completing actual held policy write'
                assert service.admin({'op':'snapshot'})['ready'] is False,'stop invalidates authority and leaves snapshot actor responsive'
            finally:
                os.kill(held,signal.SIGCONT);held=None
            helper.wait(timeout=4);assert helper.returncode==0
            wait(lambda:bool(poll.poll(0)));service.child.wait(timeout=4)
            assert service.child.returncode==0 and not storage_children()
            detail.update({'socket_origin_pidfd':True,'same_peer_kernel_pid':True,'root_peer_credentials':True,
                'actual_policy_worker_SIGSTOP':True,'actual_service_exit':True,'service_exit_status':service.child.returncode,
                'helper_waits_until_service_exit':not returned_early})
            service.child=None
            if phase=='incoming-stop-red':
                assert returned_early,'strict RED must reproduce old helper returning before actual service exit'
                result['expected_red']=True;result['red_signature']='stop_ack_returned_before_actual_service_exit'
                raise AssertionError('Stop ACK returned while actual held policy completion and service exit remain pending')
            assert not returned_early,'Stop helper returned before kernel pidfd exit'
            detail['passed']=True
    finally:
        if held:os.kill(held,signal.SIGCONT)
        if peerfd is not None:os.close(peerfd)
        if service:service.stop()
        session.close()


def systemd_stop_pending(account,out,result,phase):
    session=PeerSession(account,out,'systemd-stop-pending');service=None;held=None;unit='pokelinux-incoming-stop-fixture.service';started=False;peerfd=None
    detail={'passed':False,'fixture':session.evidence};result['checks']['actual-systemd-pending-completion']=detail
    try:
        session.start();device=Path('/dev/binderfs/incoming');os.chown(device,account.pw_uid,0);device.chmod(0o660)
        service=Service(out,session);profile=service.i/'systemd-stop-profile.json';shutil.copyfile(service.profile,profile)
        args=[str(service.i/'messages-incoming-fixture-service'),str(service.r),str(service.s/'policy'),str(profile),str(service.i/'lifecycle-launcher'),str(service.s/'raw'),str(service.s/'inbox'),session.generation,'none']
        (Path('/etc/systemd/system')/unit).write_text('[Unit]\nDescription=Isolated Messages actual pending stop fixture\n[Service]\nType=simple\nExecStart='+ ' '.join(args)+'\nExecStop='+str(service.i/'messages-stop')+' '+str(service.r/'admin.sock')+'\nExecStopPost=/usr/bin/python3 /workspace/tools/native-messages-incoming-binder/systemd-exit-observer.py '+str(service.s/'systemd-main-exit.json')+'\nRuntimeDirectory='+service.r.name+'\nRuntimeDirectoryMode=0755\nUMask=0077\nNoNewPrivileges=yes\nCapabilityBoundingSet=CAP_SETUID CAP_SETGID CAP_KILL\nTasksMax=96\nMemoryMax=512M\nKillMode=control-group\nTimeoutStopSec=5\nRestart=no\n')
        subprocess.run(['systemctl','daemon-reload'],check=True,timeout=5);subprocess.run(['systemctl','start',unit],check=True,timeout=5);started=True
        wait(lambda:service.status().get('receiving'))
        with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as sock:
            sock.connect(str(service.r/'admin.sock'));pid,uid,gid=struct.unpack('3i',sock.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));peerfd=sock.getsockopt(socket.SOL_SOCKET,77)
            assert uid==0 and 'Pid:\t'+str(pid)+'\n' in Path('/proc/self/fdinfo/'+str(peerfd)).read_text()
        for path in Path('/proc').iterdir():
            if not path.name.isdecimal():continue
            try:parts=(path/'cmdline').read_bytes().split(b'\0');stat=(path/'stat').read_text().split()
            except OSError:continue
            if b'--storage-worker' in parts and int(stat[3])==pid:held=int(path.name)
        assert held;os.kill(held,signal.SIGSTOP)
        stop=subprocess.Popen(['systemctl','stop',unit],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        # Real scheduling hold below the actor's one-second shutdown-write deadline.
        time.sleep(.25)
        try:os.kill(held,signal.SIGCONT)
        except ProcessLookupError:pass
        held=None;stdout,stderr=stop.communicate(timeout=8)
        (out/'systemd-pending-stop.log').write_bytes(stdout+stderr)
        state=subprocess.run(['systemctl','show',unit,'-p','Result','-p','ExecMainCode','-p','ExecMainStatus','-p','ActiveState','-p','MainPID'],capture_output=True,text=True,check=True,timeout=5)
        (out/'systemd-pending-status.txt').write_text(state.stdout)
        journal=subprocess.run(['journalctl','-u',unit,'-n','30','--no-pager'],capture_output=True,text=True,timeout=5);(out/'systemd-pending-journal.txt').write_text(journal.stdout+ journal.stderr)
        values=dict(line.split('=',1) for line in state.stdout.splitlines() if '=' in line)
        poll=select.poll();poll.register(peerfd,select.POLLIN);assert poll.poll(0) and not storage_children()
        shutil.copyfile(service.s/'systemd-main-exit.json',out/'systemd-main-exit.json')
        outcome=json.loads((out/'systemd-main-exit.json').read_text())[-1]
        clean=outcome['EXIT_CODE']=='exited' and outcome['EXIT_STATUS']=='0'
        detail.update({'socket_origin_pidfd':True,'root_peer_credentials':True,'actual_policy_child_SIGSTOP':True,'actual_systemd_stop':True,'main_exit':outcome,'post_stop_reset_status':values,'actual_service_exit':True,'owned_storage_children_reaped':True,'main_clean_exit':clean})
        if phase.endswith('-red'):
            assert not clean,'strict RED requires actual pre-completion systemd signal'
            result['expected_red']=True;result['red_signature']='systemd_signalled_before_actual_service_completion'
            raise AssertionError('systemd signalled Main while actual policy completion remained pending after old helper ACK')
        assert clean,'local pidfd exit must separately have normal Main exit status'
        policy=service.s/'policy';live=json.loads((policy/'owner.live').read_text());seal=json.loads((policy/'owner.clean').read_text())
        policy_bytes=(policy/'policy.json').read_bytes() if (policy/'policy.json').exists() else b''
        assert live['epoch']==seal['epoch'] and seal['hash']==hashlib.sha256(policy_bytes).hexdigest()
        detail.update({'healthy_policy_seal_exact_epoch_and_bytes':True,'seal_regular_private_root_owned':all(p.is_file() and p.stat().st_uid==0 and p.stat().st_mode&0o077==0 for p in [policy/'owner.live',policy/'owner.clean'])})
        assert detail['seal_regular_private_root_owned']
        subprocess.run(['systemctl','start',unit],check=True,timeout=5);wait(lambda:service.admin({'op':'snapshot'})['ready']);wait(lambda:service.status()['available'])
        assert not service.status()['receiving'] and json.loads(probe('counters',0).stdout)[:2]==[1,1]
        detail.update({'passed':True,'policy_validation_and_history_restart_ready':True,'no_backend_reregistration':True})
    finally:
        if held:
            try:os.kill(held,signal.SIGCONT)
            except ProcessLookupError:pass
        if peerfd is not None:os.close(peerfd)
        if started:subprocess.run(['systemctl','stop',unit],check=False,timeout=8)
        assert not storage_children();session.close()
