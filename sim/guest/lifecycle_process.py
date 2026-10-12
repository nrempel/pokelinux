#!/usr/bin/env python3
"""Exact native ARM64 lifecycle binaries in a disposable root guest, never host power."""
import hashlib,json,os,platform,resource,subprocess,traceback
from pathlib import Path
out=Path('/results')
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),'qemu_mode':'system','hardware_verified':False,'phone_accessed':False,'network':'none','binary_sha256':{}}
try:
    assert os.getuid()==0 and platform.machine()=='aarch64'
    assert 'pixel.mode=lifecycle-process' in Path('/proc/cmdline').read_text()
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    manifest_path=Path('/workspace/artifacts/native-lifecycle/arm64-artifacts.json')
    manifest=json.loads(manifest_path.read_text())
    result['artifact_manifest_sha256']=hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    for item in manifest['artifacts']:
        f=Path(item['guest_path'])
        assert hashlib.sha256(f.read_bytes()).hexdigest()==item['sha256'],item['name']
        result['binary_sha256'][item['name']]=item['sha256']
        m=Path('/workspace')/item['cargo_manifest']
        assert hashlib.sha256(m.read_bytes()).hexdigest()==item['cargo_manifest_sha256']
    fixture=Path('/workspace')/manifest['fixture']
    assert hashlib.sha256(fixture.read_bytes()).hexdigest()==manifest['fixture_sha256']
    test=[a for a in manifest['artifacts'] if a['kind']=='library_test']
    assert len(test)==1
    env=dict(os.environ,**manifest['component_environment'])
    listing=subprocess.check_output([test[0]['guest_path'],'--list'],text=True,timeout=30,env=env)
    (out/'components-list.txt').write_text(listing)
    run=subprocess.run([test[0]['guest_path'],'--nocapture'],text=True,capture_output=True,timeout=45,env=env)
    (out/'components.log').write_text(run.stdout+run.stderr)
    assert run.returncode==0 and '4 passed' in run.stdout,run.stdout+run.stderr
    result['actual_pidfd_durable_file_components']=4
    env=dict(os.environ,**manifest['fixture_environment'])
    installed=out/'installed';installed.mkdir()
    run=subprocess.run(['python3',str(fixture),'--binary-root',manifest['fixture_binary_root'],'--results-dir',str(installed),'--install-root','/opt/pokelinux-lifecycle-arm64','--runtime-root','/run/native-lifecycle-arm64','--policy-root','/var/lib/native-lifecycle-arm64'],text=True,capture_output=True,timeout=90,env=env)
    (out/'installed.log').write_text(run.stdout+run.stderr)
    observed=json.loads((installed/'result.json').read_text())
    assert run.returncode==0 and observed['passed'] and len(observed['groups'])==18,run.stdout+run.stderr
    result['installed_boundary']=observed
    result['passed']=True
except BaseException as e:
    result['error']=str(e);(out/'failure.txt').write_text(traceback.format_exc())
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    subprocess.run(['systemctl','poweroff'],check=False)
