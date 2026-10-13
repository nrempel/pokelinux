"""Explicit synthetic peer generations, independent of local Messages recovery."""
import fcntl,hashlib,json,os,selectors,struct,subprocess,time,uuid
from pathlib import Path
BASE=Path('/workspace/artifacts/native-messages-incoming-binder')
LOADER='/workspace/artifacts/runtime/apex/com.android.runtime/bin/linker64'
LIBRARIES='/workspace/artifacts/runtime/system/lib64:/workspace/artifacts/runtime/apex/com.android.runtime/lib64/bionic'
def ready(process,marker,path):
    saved=b'';until=time.monotonic()+20
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout,selectors.EVENT_READ)
        while not any(line.startswith(marker) for line in saved.splitlines()):
            assert time.monotonic()<until,'fixture readiness deadline'
            if selector.select(.5):
                chunk=os.read(process.stdout.fileno(),4096);assert chunk,'fixture exited before readiness'
                saved+=chunk;path.write_bytes(saved);assert len(saved)<=16384
    return saved
def storage_children():
    found=[]
    for path in Path('/proc').iterdir():
        if not path.name.isdecimal():continue
        try:parts=(path/'cmdline').read_bytes().split(b'\0')
        except OSError:continue
        if b'--messages-worker' in parts or b'--messages-incoming-worker' in parts:found.append(int(path.name))
    return found
class PeerSession:
    active=False
    properties_ready=False
    def __init__(self,account,out,label,hold=False):
        assert not PeerSession.active and not storage_children(),'fresh peer reset requires actual local storage-child reap'
        PeerSession.active=True;self.account=account;self.out=out;self.label=label;self.hold=hold
        self.generation=uuid.uuid4().hex;self.owned=[];self.prefix={};self.evidence={'generation':self.generation,'held_registration':hold}
    def start(self):
        device=Path('/dev/binderfs/incoming')
        assert not device.exists(),'previous fixture Binder device must be removed only after owned reaping'
        with open('/dev/binderfs/binder-control','rb',buffering=0) as control:
            value=bytearray(struct.pack('256sII',b'incoming',0,0));fcntl.ioctl(control,0xc1086201,value,True)
        os.chown(device,self.account.pw_uid,self.account.pw_gid);device.chmod(0o600)
        self.evidence['device']=dict(zip(['major','minor'],struct.unpack('256sII',value)[1:]))
        assert not Path('/dev/binder').exists();os.symlink(device,'/dev/binder')
        manager=subprocess.Popen([str(BASE/'target/aarch64-unknown-linux-gnu/debug/incoming-manager')],stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0,user=self.account.pw_uid,group=self.account.pw_gid)
        self.owned.append(('manager',manager));self.prefix['manager']=ready(manager,b'READY-MANAGER ',self.out/(self.label+'-manager-prefix.log'))
        env=dict(os.environ,LD_LIBRARY_PATH=LIBRARIES)
        if not PeerSession.properties_ready:
            Path('/dev/__properties__').mkdir(mode=0o711)
            table='/dev/__properties__/property_info'
            for name,args in [('property-info',[table]),('property-init',[])]:
                run=subprocess.run([LOADER,str(BASE/name),*args],capture_output=True,env=env,timeout=10)
                (self.out/(self.label+'-'+name+'.log')).write_bytes(run.stdout+run.stderr);assert run.returncode==0
                if name=='property-info':os.chmod(table,0o644)
            PeerSession.properties_ready=True
        args=[LOADER,str(BASE/'incoming-peer'),self.generation]
        if self.hold:args.append('hold-register')
        peer=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0,env=env,user=self.account.pw_uid,group=self.account.pw_gid)
        self.owned.append(('peer',peer));self.prefix['peer']=ready(peer,b'READY-INCOMING ',self.out/(self.label+'-peer-prefix.log'))
        maps=Path(f'/proc/{peer.pid}/maps').read_text();(self.out/(self.label+'-peer-maps.txt')).write_text(maps)
        assert '/artifacts/runtime/system/lib64/libbinder_ndk.so' in maps
        assert '/android-ndk-r30/' not in maps,'link stub cannot be runtime'
        self.evidence.update({'peer_pid':peer.pid,'manager_pid':manager.pid,'genuine_android_library_mapping':True,'ndk_stub_runtime':False,'fresh_test_device':True})
        return self
    def close(self):
        assert not storage_children(),'reap the Messages storage owner before explicit synthetic source teardown'
        for label,process in reversed(self.owned):
            if process.poll() is None:process.kill()
            stdout,stderr=process.communicate(timeout=10)
            prefix=self.prefix.get(label,b'');saved=self.out/(self.label+'-'+label+'-prefix.log')
            if saved.exists():prefix=saved.read_bytes()
            (self.out/(self.label+'-'+label+'.log')).write_bytes(prefix+stdout+stderr)
            self.evidence[label+'_reaped']=process.returncode is not None
            self.evidence[label+'_exit']=process.returncode
        for name in ['stats','state','transactions','transaction_log','failed_transaction_log']:
            path=Path('/dev/binderfs/binder_logs')/name
            if path.is_file():(self.out/(self.label+'-binder-'+name+'.txt')).write_text(path.read_text())
        Path('/dev/binder').unlink(missing_ok=True);Path('/dev/binderfs/incoming').unlink(missing_ok=True)
        self.evidence['explicit_fixture_reset_after_actual_reap']=True
        PeerSession.active=False
