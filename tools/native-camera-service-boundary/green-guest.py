#!/usr/bin/env python3
"""Actual same-actor capture/owned-child Binder integration; headless only."""
import ctypes,fcntl,gzip,hashlib,json,os,platform,pwd,selectors,shutil,signal,socket,struct,subprocess,time,traceback
from pathlib import Path
ROOT=Path('/workspace');BASE=ROOT/'artifacts/native-camera-service-boundary-green/freeze';BINS=BASE/'bins';OUT=Path('/results')
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),'qemu_mode':'system','network':'none','ui':False,'Source_claim':False,'original_ndk_claim':False,'vendor_execution':False,'physical_phone_accessed':False,'checks':{},'owned_reaps':[]}
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def wait(fn,seconds=4):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        try:
            value=fn()
            if value:return value
        except (FileNotFoundError,ConnectionRefusedError,ProcessLookupError):pass
        time.sleep(.005)
    raise AssertionError('bounded actual camera integration wait expired')
def call(path,value):
    with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET) as sock:
        sock.settimeout(2);sock.connect(str(path));sock.send(json.dumps(value,separators=(',',':')).encode());return json.loads(sock.recv(4096))
def ready(child,marker,label):
    saved=b'';end=time.monotonic()+15
    with selectors.DefaultSelector() as selector:
        selector.register(child.stdout,selectors.EVENT_READ)
        while not any(line.startswith(marker) for line in saved.splitlines()):
            assert time.monotonic()<end
            if selector.select(.2):
                chunk=os.read(child.stdout.fileno(),4096);assert chunk;saved+=chunk;assert len(saved)<=16384
    (OUT/(label+'-prefix.log')).write_bytes(saved);return saved
class Realm:
    def __init__(self,label):self.label=label;self.children=[];self.prefix={}
    def start(self):
        assert not Path('/dev/binder').exists() and not Path('/dev/binderfs/camera').exists()
        with open('/dev/binderfs/binder-control','rb',buffering=0) as control:
            device=bytearray(struct.pack('256sII',b'camera',0,0));fcntl.ioctl(control,0xc1086201,device,True)
        os.chown('/dev/binderfs/camera',11030,11030);Path('/dev/binderfs/camera').chmod(0o600);os.symlink('/dev/binderfs/camera','/dev/binder')
        for name,marker in [('camera-red-manager',b'READY-MANAGER '),('camera-red-peer',b'READY-CAMERA ')]:
            child=subprocess.Popen([str(BINS/name)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0,user=11030,group=11030)
            self.children.append((name,child));self.prefix[name]=ready(child,marker,self.label+'-'+name)
        return self
    def probe(self,mode='info',okay=True):
        run=subprocess.run([str(BINS/'camera-red-probe'),mode],capture_output=True,text=True,timeout=8,user=11030,group=11030)
        (OUT/(self.label+'-probe-'+mode+'-'+str(time.time_ns())+'.log')).write_text(run.stdout+run.stderr)
        if okay:assert run.returncode==0,'actual Binder control failed: '+run.stderr
        return json.loads(run.stdout) if mode in ['info','smoke'] and run.returncode==0 else run
    def close(self):
        for name,child in reversed(self.children):
            if child.poll() is None:child.kill()
            stdout,stderr=child.communicate(timeout=4)
            (OUT/(self.label+'-'+name+'.log')).write_bytes(self.prefix.get(name,b'')+stdout+stderr)
            result['owned_reaps'].append({'realm':self.label,'name':name,'pid':child.pid,'exit':child.returncode,'reaped':child.returncode is not None})
        self.children=[]
        Path('/dev/binder').unlink(missing_ok=True);Path('/dev/binderfs/camera').unlink(missing_ok=True)
def children(pid):return [int(x) for x in Path('/proc',str(pid),'task',str(pid),'children').read_text().split()]
class Service:
    seq=0
    def __init__(self,realm,fault='none',store=None):
        Service.seq+=1;self.realm=realm;self.fault=fault;self.child=None;self.known=[]
        self.install=Path('/opt/camera-green');self.runtime=Path('/run/camera-green-'+str(Service.seq));self.store=store or Path('/var/lib/camera-green-'+str(Service.seq))
        for directory,mode in [(self.install,0o755),(self.runtime,0o755),(self.runtime/'app',0o700),(self.store,0o700),(self.store/'policy',0o700),(self.store/'camera',0o700)]:directory.mkdir(parents=True,exist_ok=True);directory.chmod(mode)
        os.chown(self.runtime/'app',11020,11020)
        for name in ['camera-composed-service','lifecycle-launcher','lifecycle-client']:
            shutil.copyfile(BINS/name,self.install/name);(self.install/name).chmod(0o755)
        self.profile=self.runtime/'profile.json';self.profile.write_text(json.dumps({'apps':[{'app':'org.omarchy.Camera','uid':11020,'executable':str(self.install/'lifecycle-client'),'args':[str(self.runtime/'app.sock'),str(self.runtime/'app/driver.sock')]}]}))
    def admin(self,value):return call(self.runtime/'admin.sock',value)
    def status(self):return self.admin({'op':'camera_status'})
    def driver(self,value):return call(self.runtime/'app/driver.sock',value)
    def start(self,available=True):
        self.child=subprocess.Popen([str(self.install/'camera-composed-service'),str(self.runtime),str(self.store/'policy'),str(self.profile),str(self.install/'lifecycle-launcher'),str(self.store/'camera'),self.fault],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        wait(lambda:self.admin({'op':'snapshot'}).get('ready'))
        if available:wait(lambda:self.status()['available'])
        else:wait(lambda:not self.status()['occupied'])
        self.known=children(self.child.pid);return self
    def worker(self):
        found=[pid for pid in children(self.child.pid) if b'--camera-worker' in Path('/proc',str(pid),'cmdline').read_bytes().split(b'\0')]
        assert len(found)==1;return found[0]
    def launch(self):
        assert self.admin({'op':'launch','app':'org.omarchy.Camera'})=={'result':'ok'}
        wait(lambda:(self.runtime/'app/driver.sock').exists());assert self.driver({'op':'query'})=={'result':'ok'}
        self.known=children(self.child.pid)
    def authorise(self):
        for value in [{'op':'locked','locked':False},{'op':'grant','app':'org.omarchy.Camera','permission':'capture','allowed':True},{'op':'background','app':'org.omarchy.Camera','background':False}]:assert self.admin(value)=={'result':'ok'}
    def send(self,ttl=1000):
        assert self.driver({'driver':'send','channel':0,'request':{'op':'camera_capture','request':{'ttl_ms':ttl}}})=={'driver_result':'sent'}
    def receive(self):return wait(lambda:(value if (value:=self.driver({'driver':'receive','channel':0})).get('driver_result')!='pending' else False))
    def capture(self,ttl=1000):self.send(ttl);return self.receive()
    def retired(self):wait(lambda:not self.status()['occupied']);assert not self.status()['available']
    def stop(self):
        if self.child is None:return
        if self.child.poll() is None:assert self.admin({'op':'stop'})=={'result':'ok'}
        stdout,stderr=self.child.communicate(timeout=5);(OUT/(self.realm.label+'-service-'+str(Service.seq)+'.log')).write_bytes(stdout+stderr)
        assert self.child.returncode==0,'actor did not complete owned cleanup: '+stderr.decode()
        for pid in self.known:assert not Path('/proc',str(pid)).exists(),'owned app/backend/storage not reaped'
        result['owned_reaps'].append({'realm':self.realm.label,'name':'camera-composed-service','pid':self.child.pid,'exit':self.child.returncode,'reaped':True,'children_absent':self.known});self.child=None
    def force_cleanup(self):
        if self.child is not None:
            if self.child.poll() is None:self.child.kill()
            self.child.communicate(timeout=5)
            for pid in self.known:
                try:os.kill(pid,signal.SIGKILL)
                except ProcessLookupError:pass
                try:wait(lambda:(value if (value:=os.waitpid(pid,os.WNOHANG))[0]==pid else False),seconds=4)
                except ChildProcessError:pass
            self.child=None
def marker_check(service):
    marker=service.store/'camera/camera-backend-attempt';stat=marker.stat();assert stat.st_uid==0 and stat.st_mode&0o777==0o600
    worker=service.worker();text=Path('/proc',str(worker),'status').read_text();(OUT/'backend-status.txt').write_text(text)
    fields={line.split(':',1)[0]:line.split(':',1)[1].split() for line in text.splitlines() if ':' in line}
    assert fields['Uid']==['11030']*4 and fields['Gid']==['11030']*4 and int(fields['CapEff'][0],16)==0 and fields['NoNewPrivs']==['1']
    opened=[os.readlink(path) for path in Path('/proc',str(worker),'fd').iterdir()];assert not any('camera-backend-attempt' in x for x in opened)
    pid=os.fork()
    if pid==0:
        try:
            os.setgroups([]);os.setgid(11030);os.setuid(11030)
            try:marker.unlink();os._exit(5)
            except PermissionError:os._exit(0)
        except BaseException:os._exit(6)
    assert os.waitpid(pid,0)[1]==0 and marker.is_file()
    return {'root_marker_mode':'0600','backend_uid':11030,'caps':0,'no_new_privs':True,'marker_writable_fd_absent':True,'actual_unprivileged_unlink_denied':True}
realm=None;service=None
try:
    assert platform.machine()=='aarch64' and os.getuid()==os.geteuid()==0
    assert ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)==0,'isolated fixture subreaper setup failed'
    selection=json.loads((BASE/'selection.json').read_text());result['selection_sha256']=digest(BASE/'selection.json')
    for name,value in selection['files'].items():assert digest(BASE/name)==value,'frozen code changed: '+name
    config=gzip.decompress(Path('/proc/config.gz').read_bytes()).decode();(OUT/'kernel.config').write_text(config)
    assert 'CONFIG_ANDROID_BINDER_IPC=y' in config and 'CONFIG_ANDROID_BINDERFS=y' in config
    Path('/dev/binderfs').mkdir();subprocess.run(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],check=True)
    realm=Realm('public-smoke').start();smoke=realm.probe('smoke');assert smoke['submitted']==1;realm.close();realm=None
    result['checks']['separate_public_frozen_fd_owner_smoke']={'passed':True,'observed':smoke}
    realm=Realm('current-capture').start();service=Service(realm).start();guard=marker_check(service)
    assert realm.probe()['registered']==realm.probe()['submitted']==0
    service.launch();assert service.capture()=={'result':'denied'}
    assert service.admin({'op':'locked','locked':False})=={'result':'ok'}
    assert service.admin({'op':'grant','app':'org.omarchy.Camera','permission':'messages_read','allowed':True})=={'result':'ok'}
    assert service.capture()=={'result':'denied'}
    assert realm.probe()['registered']==realm.probe()['submitted']==0
    service.authorise();state=service.capture();assert state=={'result':'camera_state','state':{'receipt':'accepted','returned':False,'capture_complete':False,'fence_ready':False,'quarantined':False}}
    counts=realm.probe();assert counts['submitted']==counts['registered']==1
    assert service.capture()['result'] in ['denied','unavailable'];assert realm.probe()['submitted']==1
    result['checks']['current_capture_route_one_effect_and_separate_state']={'passed':True,'request':{'op':'camera_capture','request':{'ttl_ms':1000}},'real_submitted':1,'real_registered':1,'independent_capture_grant':True,'bootstrap_zero_effect':True,'second_attempt_refused':True,'state':state,'marker':guard}
    assert service.admin({'op':'grant','app':'org.omarchy.Camera','permission':'capture','allowed':False})=={'result':'ok'};service.retired()
    assert realm.probe('callbacks',okay=False).returncode!=0;assert realm.probe()['submitted']==1
    old_store=service.store;service.stop();service=Service(realm,store=old_store).start(available=False)
    service.launch();service.authorise();assert service.capture()['result'] in ['denied','unavailable'];assert realm.probe()['submitted']==realm.probe()['registered']==1
    result['checks']['revoke_late_callback_and_healthy_whole_restart_quarantine']={'passed':True,'late_callback_refused':True,'persistent_marker':True,'no_implicit_register_or_submit':True}
    service.stop();service=None;realm.close();realm=None
    for mode in ['grant','lock','background','suspend','eof','death','expiry','stop']:
        realm=Realm('held-'+mode).start();service=Service(realm,fault='held').start();service.launch();service.authorise();worker=service.worker();service.send(ttl=250 if mode=='expiry' else 1000)
        wait(lambda:realm.probe()['held']);assert realm.probe()['submitted']==1
        before=time.monotonic();assert service.admin({'op':'snapshot'})['ready'];snapshot_seconds=time.monotonic()-before
        if mode=='grant':assert service.admin({'op':'grant','app':'org.omarchy.Camera','permission':'capture','allowed':False})=={'result':'ok'}
        elif mode=='lock':assert service.admin({'op':'locked','locked':True})=={'result':'ok'}
        elif mode=='background':assert service.admin({'op':'background','app':'org.omarchy.Camera','background':True})=={'result':'ok'}
        elif mode=='suspend':assert service.admin({'op':'suspended','suspended':True})=={'result':'ok'}
        elif mode=='eof':assert service.driver({'driver':'reconnect'})=={'result':'ok'}
        elif mode=='death':
            try:service.driver({'driver':'exit'})
            except (OSError,ValueError):pass
        elif mode=='stop':service.stop()
        if mode!='stop':service.retired()
        assert not Path('/proc',str(worker)).exists() and realm.probe()['submitted']==1
        result['checks']['held_'+mode]={'passed':True,'real_binder_held':True,'actual_backend_reaped':True,'retained_remote_submission':1,'snapshot_observed_seconds':snapshot_seconds,'hard_latency_claim':False,'marker_retained':(service.store/'camera/camera-backend-attempt').is_file()}
        if service.child is not None:service.stop()
        service=None;realm.close();realm=None
    for mode in ['grant','eof','expiry']:
        realm=Realm('eagain-'+mode).start();service=Service(realm,fault='eagain').start();service.launch();service.authorise();worker=service.worker();service.send(ttl=500 if mode=='expiry' else 1000)
        wait(lambda:service.admin({'op':'snapshot'})['receipts']==1 and not service.admin({'op':'snapshot'})['storage_busy'])
        assert service.status()=={'result':'camera_status','occupied':True,'busy':False,'available':True,'pending':1}
        assert 'T (stopped)' in Path('/proc',str(worker),'status').read_text()
        if mode=='grant':assert service.admin({'op':'grant','app':'org.omarchy.Camera','permission':'capture','allowed':False})=={'result':'ok'}
        elif mode=='eof':assert service.driver({'driver':'reconnect'})=={'result':'ok'}
        service.retired();assert not Path('/proc',str(worker)).exists();assert realm.probe()['registered']==realm.probe()['submitted']==0
        service.stop();text=(OUT/(realm.label+'-service-'+str(Service.seq)+'.log')).read_text();assert 'ACTUAL-CAMERA-CHANNEL-FULL' in text and 'errno=11' in text
        result['checks']['actual_supervisor_eagain_'+mode]={'passed':True,'actual_channel_full':True,'worker_stopped_before_revoke':True,'backend_reaped':True,'real_registered':0,'real_submitted':0,'original_deadline_extended':False,'queued_dispatch':False}
        service=None;realm.close();realm=None
    realm=Realm('late-before-lookup').start();service=Service(realm,fault='late-before-lookup').start();service.launch();service.authorise();service.send(ttl=180);service.retired()
    assert realm.probe()['registered']==realm.probe()['submitted']==0
    result['checks']['original_deadline_before_first_binder_effect']={'passed':True,'real_registered':0,'real_submitted':0,'backend_reaped':True,'deadline_extended':False}
    service.stop();service=None;realm.close();realm=None
    realm=Realm('peer-death').start();service=Service(realm).start();service.launch();service.authorise();assert service.capture()['result']=='camera_state';worker=service.worker();realm.probe('die',okay=False);service.retired();assert not Path('/proc',str(worker)).exists()
    result['checks']['retained_peer_death_fences_owned_child']={'passed':True,'backend_reaped':True,'reset_inferred':False}
    service.stop();service=None;realm.close();realm=None
    realm=Realm('parent-death').start();service=Service(realm,fault='held').start();service.launch();service.authorise();service.send();wait(lambda:realm.probe()['held']);orphans=children(service.child.pid)
    service.child.kill();service.child.communicate(timeout=4)
    orphan_waits=[]
    for pid in orphans:
        waited,status=wait(lambda:(value if (value:=os.waitpid(pid,os.WNOHANG))[0]==pid else False),seconds=4)
        assert waited==pid;orphan_waits.append({'pid':pid,'status':status,'actual_wait':True})
    result['checks']['actual_parent_death_after_uid_drop']={'passed':True,'orphan_waits':orphan_waits,'protected_marker':(service.store/'camera/camera-backend-attempt').is_file()}
    service.child=None;service=None;realm.close();realm=None
    result['passed']=True
except Exception:result['error']=traceback.format_exc()
finally:
    try:
        if service is not None:service.force_cleanup()
        if realm is not None:realm.close()
        result['fixture_cleanup_passed']=all(row['reaped'] for row in result['owned_reaps'])
    except Exception:result['cleanup_error']=traceback.format_exc();result['passed']=False
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2),flush=True)
    subprocess.run(['systemctl','poweroff'],check=False)
