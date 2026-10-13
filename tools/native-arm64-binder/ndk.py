"""Synthetic NDK peer over original retained Android libraries; isolated ARM guest only."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import selectors
import struct
import subprocess
import time


def ready(process, marker, log):
    prefix = b''
    limit = time.monotonic() + 20
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout, selectors.EVENT_READ)
        while not any(line.startswith(marker) for line in prefix.splitlines()):
            assert time.monotonic() < limit, 'original Android peer readiness timeout'
            if selector.select(0.5):
                chunk = os.read(process.stdout.fileno(), 4096)
                assert chunk, 'peer exited before readiness'
                prefix += chunk
                log.write_bytes(prefix)
                assert len(prefix) < 16384
    return prefix


def execute(account, out, client):
    base = Path('/workspace/artifacts/native-arm64-binder')
    manifest = json.loads((base/'android-runtime-closure.json').read_text())
    for entry in manifest['files'].values():
        path = Path('/workspace')/entry['path']
        assert hashlib.sha256(path.read_bytes()).hexdigest() == entry['sha256']
    loader = '/workspace/artifacts/runtime/apex/com.android.runtime/bin/linker64'
    paths = '/workspace/artifacts/runtime/system/lib64:/workspace/artifacts/runtime/apex/com.android.runtime/lib64/bionic'
    environment = dict(os.environ, LD_LIBRARY_PATH=paths)
    if not Path('/dev/binder').exists():
        os.symlink('/dev/binderfs/radio', '/dev/binder')
    result = {'peer_class': 'synthetic NDK peer over genuine retained ARM64 Android Binder/Bionic; synthetic Rust service manager',
              'runtime_closure': manifest, 'runtime_substitution': False, 'checks': {}}
    for name in ['ndk-peer','property-info','property-init']:
        result[name+'_sha256'] = hashlib.sha256((base/name).read_bytes()).hexdigest()
    for scenario in ['callbacks', 'death-register', 'death-active']:
        os.unlink('/dev/binderfs/radio')
        with open('/dev/binderfs/binder-control','rb',buffering=0) as control:
            device = bytearray(struct.pack('256sII',b'radio',0,0))
            fcntl.ioctl(control,0xc1086201,device,True)
        os.chown('/dev/binderfs/radio',account.pw_uid,account.pw_gid)
        os.chmod('/dev/binderfs/radio',0o600)
        manager = None
        ndk = None
        manager_prefix = b''
        ndk_prefix = b''
        try:
            manager = subprocess.Popen([str(client.with_name('manager'))],stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,bufsize=0,user=account.pw_uid,group=account.pw_gid)
            manager_prefix = ready(manager,b'READY-MANAGER ',out/(scenario+'-manager-prefix.log'))
            if scenario == 'callbacks':
                Path('/dev/__properties__').mkdir(mode=0o711)
                table = '/dev/__properties__/property_info'
                generate = subprocess.run([loader,str(base/'property-info'),table],capture_output=True,env=environment,timeout=10)
                (out/'property-serializer.log').write_bytes(generate.stdout+generate.stderr)
                assert generate.returncode == 0, 'original public serializer failed'
                os.chmod(table,0o644)
                initialize = subprocess.run([loader,str(base/'property-init')],capture_output=True,env=environment,timeout=10)
                (out/'property-init.log').write_bytes(initialize.stdout+initialize.stderr)
                assert initialize.returncode == 0, 'genuine Bionic property initialization failed'
                result['fixture_property_info_sha256'] = hashlib.sha256(Path(table).read_bytes()).hexdigest()
            ndk = subprocess.Popen([loader,str(base/'ndk-peer')],stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                bufsize=0,env=environment,user=account.pw_uid,group=account.pw_gid)
            ndk_prefix = ready(ndk,b'READY-NDK ',out/(scenario+'-ndk-peer-prefix.log'))
            maps = Path(f'/proc/{ndk.pid}/maps').read_text()
            (out/(scenario+'-ndk-maps.txt')).write_text(maps)
            assert '/artifacts/runtime/system/lib64/libbinder_ndk.so' in maps
            assert '/android-ndk-r30/' not in maps, 'NDK stub linked as runtime'
            run = subprocess.run([str(client),'ndk-'+scenario],capture_output=True,text=True,timeout=40,
                user=account.pw_uid,group=account.pw_gid)
            (out/(scenario+'-client.log')).write_text(run.stdout+run.stderr)
            result['checks'][scenario] = {'passed':run.returncode==0,'exit':run.returncode,
                'ndk_pid':ndk.pid,'manager_pid':manager.pid,'client_markers':run.stdout.splitlines(),
                'genuine_android_library_mapping':True,'ndk_stub_runtime':False}
            assert run.returncode == 0, 'actual ARM NDK interop scenario failed: '+scenario
        finally:
            for process,prefix,label in [(ndk,ndk_prefix,'ndk-peer'),(manager,manager_prefix,'manager')]:
                if process is not None:
                    if process.poll() is None: process.kill()
                    stdout,stderr = process.communicate(timeout=10)
                    saved = out/(scenario+'-'+label+'-prefix.log')
                    if saved.exists(): prefix=saved.read_bytes()
                    (out/(scenario+'-'+label+'.log')).write_bytes(prefix+stdout+stderr)
                    if scenario in result['checks']:
                        result['checks'][scenario][label+'_reaped'] = process.returncode is not None
            for name in ['stats','state','transactions','transaction_log','failed_transaction_log']:
                path = Path('/dev/binderfs/binder_logs')/name
                if path.is_file(): (out/(scenario+'-binder-'+name+'.txt')).write_text(path.read_text())
    return result
