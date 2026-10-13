#!/usr/bin/env python3
"""Isolated ARM kernel Binder fixture; no network or Android substitute."""
import fcntl, gzip, hashlib, json, os, platform, pwd, selectors, struct, subprocess, traceback
from pathlib import Path
out = Path('/results')
result = {'passed': False, 'architecture': platform.machine(), 'kernel': platform.release(), 'kernel_version': platform.version(),
          'qemu_mode': 'system', 'hardware_verified': False, 'phone_accessed': False,
          'peer_class': 'independent synthetic native Rust peer; no original interop', 'network': 'none'}
try:
    assert platform.machine() == 'aarch64'
    config = gzip.decompress(Path('/proc/config.gz').read_bytes()).decode()
    (out/'kernel.config').write_text(config)
    result['binder_config'] = [x for x in config.splitlines() if 'BINDER' in x]
    result['filesystems'] = Path('/proc/filesystems').read_text()
    phase = next(x.split('=',1)[1] for x in Path('/proc/cmdline').read_text().split() if x.startswith('pixel.binder-phase='))
    if phase != 'driver-red':
        Path('/dev/binderfs').mkdir()
        subprocess.run(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],check=True)
        with open('/dev/binderfs/binder-control','rb',buffering=0) as control:
            device=bytearray(struct.pack('256sII',b'radio',0,0))
            fcntl.ioctl(control,0xc1086201,device,True)  # BINDER_CTL_ADD, upstream 264-byte UAPI
            _,major,minor=struct.unpack('256sII',device)
        account=pwd.getpwnam('alarm')
        os.chown('/dev/binderfs/radio',account.pw_uid,account.pw_gid)
        os.chmod('/dev/binderfs/radio',0o600)
        with open('/dev/binderfs/radio','rb',buffering=0) as driver:
            version=bytearray(4); fcntl.ioctl(driver,0xc0046209,version,True)
        result['driver']={'major':major,'minor':minor,'binder_protocol':struct.unpack('i',version)[0],
                          'uid':account.pw_uid,'mode':'0600','created_in':'guest only'}
        assert result['driver']['binder_protocol']==8
    binary = Path('/workspace/artifacts/native-arm64-binder/target/aarch64-unknown-linux-gnu/debug/client')
    if phase=='held-red':
        binary=Path('/workspace/artifacts/native-arm64-binder/held-red/client')
        assert binary.is_file(), 'historical held-control red requires its preserved exact binaries'
    result['client_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    if phase.startswith('driver-'):
        run = subprocess.run(['runuser','-u','alarm','--',str(binary),'driver'], capture_output=True, text=True, timeout=45)
        (out/'client.log').write_text(run.stdout + run.stderr)
        result['client_exit'] = run.returncode
        assert run.returncode == 0, 'actual Rust client could not initialize kernel Binder'
    elif phase=='ndk-green':
        import ndk
        result.update(ndk.execute(account,out,binary))
    elif phase=='ndk-red':
        manifest=json.loads(Path('/workspace/artifacts/native-arm64-binder/android-runtime-closure.json').read_text())
        for entry in manifest['files'].values():
            path=Path('/workspace')/entry['path']
            assert hashlib.sha256(path.read_bytes()).hexdigest()==entry['sha256']
        os.symlink('/dev/binderfs/radio','/dev/binder')
        peer=subprocess.Popen([str(binary.with_name('peer'))],stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                              text=True,user=account.pw_uid,group=account.pw_gid)
        first=''
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(peer.stdout,selectors.EVENT_READ)
                assert selector.select(20)
                first=peer.stdout.readline(); assert first.startswith('READY pid=')
            loader='/workspace/artifacts/runtime/apex/com.android.runtime/bin/linker64'
            paths='/workspace/artifacts/runtime/system/lib64:/workspace/artifacts/runtime/apex/com.android.runtime/lib64/bionic'
            cmd=[loader,'/workspace/artifacts/native-arm64-binder/ndk-peer']
            ndk_env=dict(os.environ,LD_LIBRARY_PATH=paths)
            result['ndk_command']=cmd
            try:
                ndk=subprocess.run(cmd,capture_output=True,text=True,timeout=12,user=account.pw_uid,group=account.pw_gid,env=ndk_env)
                result['ndk_exit']=ndk.returncode
                (out/'ndk-peer.log').write_text(ndk.stdout+ndk.stderr)
                assert ndk.returncode==0, 'genuine NDK peer lacks matching service-manager fixture capability'
            except subprocess.TimeoutExpired as error:
                (out/'ndk-peer.log').write_bytes((error.stdout or b'')+(error.stderr or b''))
                result['ndk_timeout']=True
                raise
        finally:
            if peer.poll() is None:peer.kill()
            stdout,stderr=peer.communicate(timeout=10)
            (out/'baseline-manager.log').write_text(first+stdout+stderr)
    else:
        peer_binary=binary.with_name('peer')
        result['peer_sha256']=hashlib.sha256(peer_binary.read_bytes()).hexdigest()
        result['checks']={}
        scenarios=['callbacks','hold-cancel'] if phase=='held-red' else [
            'callbacks','hold-cancel','hold-expire','death-register','death-active',
            'expired-request','segments','late-request','supervised-permanent']
        for scenario in scenarios:
            # No successor on the old connection: allocate a fresh private test device
            # after both prior test processes have actually been waited/reaped.
            os.unlink('/dev/binderfs/radio')
            with open('/dev/binderfs/binder-control','rb',buffering=0) as control:
                device=bytearray(struct.pack('256sII',b'radio',0,0))
                fcntl.ioctl(control,0xc1086201,device,True)
            os.chown('/dev/binderfs/radio',account.pw_uid,account.pw_gid)
            os.chmod('/dev/binderfs/radio',0o600)
            peer=subprocess.Popen([str(peer_binary)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                                  text=True,user=account.pw_uid,group=account.pw_gid)
            first=''; client=None
            try:
                with selectors.DefaultSelector() as selector:
                    selector.register(peer.stdout,selectors.EVENT_READ)
                    assert selector.select(20), 'independent peer readiness timeout'
                    first=peer.stdout.readline()
                assert first.startswith('READY pid='), 'peer not ready: '+first
                client=subprocess.run([str(binary),scenario],capture_output=True,text=True,timeout=45,
                                      user=account.pw_uid,group=account.pw_gid)
                (out/(scenario+'-client.log')).write_text(client.stdout+client.stderr)
                result['checks'][scenario]={'exit':client.returncode,'passed':client.returncode==0,
                                            'peer_pid':peer.pid,'client_markers':client.stdout.splitlines()}
            finally:
                if peer.poll() is None: peer.kill()
                stdout,stderr=peer.communicate(timeout=10) # Retain Popen ownership until actual wait.
                (out/(scenario+'-peer.log')).write_text(first+stdout+stderr)
                if scenario in result['checks']: result['checks'][scenario]['peer_reaped']=peer.returncode is not None
                logs=Path('/dev/binderfs/binder_logs')
                if logs.exists():
                    for name in ['stats','state','transactions','transaction_log','failed_transaction_log']:
                        path=logs/name
                        if path.is_file():
                            (out/(scenario+'-binder-'+name+'.txt')).write_text(path.read_text())
            assert client is not None and client.returncode==0, 'actual ARM kernel Binder scenario failed: '+scenario
    result['passed'] = True
except Exception:
    result['error'] = traceback.format_exc()
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2),flush=True)
    subprocess.run(['systemctl','poweroff'], check=False)
