#!/usr/bin/env python3
"""Scaffold qualification on actual ARM kernel Binder; no storage/authority claim."""
import gzip,hashlib,json,platform,pwd,subprocess,traceback
from pathlib import Path
from peer_session import BASE,PeerSession
out=Path('/results')
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),'kernel_version':platform.version(),
        'qemu_mode':'system','network':'none','phone_accessed':False,'vendor_execution':False,
        'scope':'frozen messaging V3 generated NDK/Rust callback/death fixture only',
        'storage_qualified':False,'installed_authority_qualified':False,'checks':{}}
try:
    assert platform.machine()=='aarch64'
    config=gzip.decompress(Path('/proc/config.gz').read_bytes()).decode();(out/'kernel.config').write_text(config)
    result['binder_config']=[line for line in config.splitlines() if 'BINDER' in line]
    assert 'CONFIG_ANDROID_BINDER_IPC=y' in config and 'CONFIG_ANDROID_BINDERFS=y' in config
    inputs=json.loads((BASE/'verified-inputs.json').read_text())
    for row in inputs['runtime']['files'].values():assert hashlib.sha256((Path('/workspace')/row['path']).read_bytes()).hexdigest()==row['sha256']
    result['runtime_inputs']=inputs
    for name in ['incoming-peer','property-info','property-init']:
        result[name+'_sha256']=hashlib.sha256((BASE/name).read_bytes()).hexdigest()
    binary=BASE/'target/aarch64-unknown-linux-gnu/debug/incoming-smoke'
    result['smoke_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
    Path('/dev/binderfs').mkdir();subprocess.run(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],check=True)
    account=pwd.getpwnam('alarm')
    phase=next(x.split('=',1)[1] for x in Path('/proc/cmdline').read_text().split() if x.startswith('pixel.messages-incoming-phase='))
    if phase!='smoke':
        import integration
        result['scope']='actual frozen messaging V3 Binder to supervised raw/inbox storage and current-authority read'
        if phase=='incoming-systemd':integration.systemd_restart(account,out,result)
        elif phase.startswith('incoming-systemd-stop-'):integration.systemd_stop_pending(account,out,result,phase)
        elif phase.startswith('incoming-stop-'):integration.stop_completion(account,out,result,phase)
        else:integration.run(account,out,result,phase)
    else:
        session=PeerSession(account,out,'smoke')
        try:
            session.start()
            run=subprocess.run([str(binary),session.generation],capture_output=True,text=True,timeout=25,user=account.pw_uid,group=account.pw_gid)
            (out/'callbacks-smoke.log').write_text(run.stdout+run.stderr)
            result['checks']['callbacks']={'exit':run.returncode,'passed':run.returncode==0,'fixture':session.evidence}
            assert run.returncode==0,'actual generated incoming Binder scaffold failed'
            result['checks']['callbacks']['observed']=json.loads(run.stdout)
        finally:session.close()
        assert result['checks']['callbacks']['fixture']['peer_reaped'] and result['checks']['callbacks']['fixture']['manager_reaped']
    if phase=='incoming-green':
        for row in json.loads((BASE/'kernel-test-artifacts.json').read_text()):
            path=BASE/row['path'];assert hashlib.sha256(path.read_bytes()).hexdigest()==row['sha256']
            run=subprocess.run([str(path)],capture_output=True,text=True,timeout=8)
            (out/('kernel-identity-'+row['name']+'.log')).write_text(run.stdout+run.stderr)
            assert run.returncode==0,'actual ARM kernel-origin boundary test failed'
        result['kernel_identity_boundary']={'passed':True,'cases':3,'actual_kernel_cases':2,'injected_shape_actual_descriptor_cleanup_cases':1,'test_elves':json.loads((BASE/'kernel-test-artifacts.json').read_text())}
        result['storage_qualified']=result['checks']['closed_window_raw_and_current_authority']['passed']
        result['installed_authority_qualified']=result['checks']['closed_window_raw_and_current_authority']['actual_current_authority_read']
    result['passed']=True
except Exception:result['error']=traceback.format_exc()
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2),flush=True)
    subprocess.run(['systemctl','poweroff'],check=False)
