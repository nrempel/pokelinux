#!/usr/bin/env python3
"""Final native storage supervision under a real isolated ARM64 Linux kernel."""
import hashlib,json,os,platform,resource,subprocess,traceback
from pathlib import Path
out=Path('/results')
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),
        'qemu_mode':'system','hardware_verified':False,'phone_accessed':False,
        'network':'none','binary_sha256':{},'groups':[]}
try:
    assert os.getuid()==0 and platform.machine()=='aarch64'
    assert 'pixel.mode=lifecycle-storage-process' in Path('/proc/cmdline').read_text()
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    root=Path('/workspace')
    selected_path=root/'artifacts/native-lifecycle-storage/arm64-artifacts.json'
    selection=json.loads(selected_path.read_text())
    result['selection_sha256']=hashlib.sha256(selected_path.read_bytes()).hexdigest()
    for item in selection['artifacts']:
        binary=Path(item['guest_path'])
        assert hashlib.sha256(binary.read_bytes()).hexdigest()==item['sha256']
        cargo=root/item['cargo_manifest']
        assert hashlib.sha256(cargo.read_bytes()).hexdigest()==item['cargo_manifest_sha256']
        rows=[json.loads(line) for line in cargo.read_text().splitlines()]
        matching=[row for row in rows if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')==item['name'] and row.get('executable')==item['cargo_path']]
        assert len(matching)==1
        assert str(Path('/workspace/artifacts/native-lifecycle-storage/target')/Path(item['cargo_path']).relative_to('/target'))==item['guest_path']
        result['binary_sha256'][item['name']]=item['sha256']
    for item in selection['fixtures']:
        assert hashlib.sha256((root/item['path']).read_bytes()).hexdigest()==item['sha256']
    tests=[a for a in selection['artifacts'] if a['kind']=='library_test'];assert len(tests)==1
    env=dict(os.environ,**selection['component_environment'])
    listing=subprocess.check_output([tests[0]['guest_path'],'--list'],text=True,timeout=30,env=env)
    (out/'component-list.txt').write_text(listing)
    for name in selection['expected_component_tests']:assert name+': test' in listing,name
    run=subprocess.run([tests[0]['guest_path'],'--test-threads=1','--nocapture'],text=True,capture_output=True,timeout=60,env=env)
    (out/'components.log').write_text(run.stdout+run.stderr)
    assert run.returncode==0 and '7 passed; 0 failed; 0 ignored' in run.stdout,run.stdout+run.stderr
    result['component_tests']=7
    env=dict(os.environ,**selection['fixture_environment'])
    installed=out/'installed'
    run=subprocess.run(['python3',str(root/'tools/native-lifecycle-storage-fixture.py'),'--binary-root',selection['fixture_binary_root'],'--results-dir',str(installed),'--baseline-fixture',str(root/'tools/native-lifecycle-fixture.py')],text=True,capture_output=True,timeout=120,env=env)
    (out/'installed.log').write_text(run.stdout+run.stderr)
    observed=json.loads((installed/'result.json').read_text())
    assert run.returncode==0 and observed['passed'] and len(observed['groups'])==8,run.stdout+run.stderr
    assert observed['groups'][0]=='preserved18_installed_cases_with_loading_bootstrap_adapter'
    baseline=json.loads((installed/'baseline/result.json').read_text())
    assert baseline['passed'] and len(baseline['groups'])==18
    result['groups'].extend(baseline['groups']);result['groups'].extend(observed['groups'][1:])
    result['installed_boundary']=observed;result['baseline_boundary']=baseline
    for mode in selection['cleanup_modes']:
        folder=out/('cleanup-'+mode)
        run=subprocess.run(['python3',str(root/'tools/native-lifecycle-storage-cleanup-fixture.py'),'--mode',mode,'--binary-root',selection['fixture_binary_root'],'--results-dir',str(folder)],text=True,capture_output=True,timeout=30,env=env)
        (out/('cleanup-'+mode+'.log')).write_text(run.stdout+run.stderr)
        observed=json.loads((folder/'result.json').read_text())
        assert run.returncode==0 and observed['passed'] and len(observed['groups'])==1,run.stdout+run.stderr
        result['groups'].extend(observed['groups']);result['cleanup_'+mode]=observed
    assert result['groups']==selection['expected_installed_process_groups']
    assert len(result['groups'])==selection['expected_installed_process_count']==27
    result['mechanism_limits']=selection['mechanism_limits']
    result['passed']=True
except BaseException as error:
    result['error']=str(error);(out/'failure.txt').write_text(traceback.format_exc())
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    subprocess.run(['systemctl','poweroff'],check=False)
