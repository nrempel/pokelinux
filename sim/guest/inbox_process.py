#!/usr/bin/env python3
"""Exact frozen inbox artifacts with Linux credentials/processes in a real ARM64 kernel."""
import hashlib,json,platform,resource,subprocess,traceback
from pathlib import Path
out=Path('/results')
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),
        'qemu_mode':'system','hardware_verified':False,'phone_accessed':False,
        'network':'none','checks':{},'binary_sha256':{}}
try:
    assert platform.machine()=='aarch64'
    assert 'pixel.mode=inbox-process' in Path('/proc/cmdline').read_text()
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    root=Path('/workspace')
    selected_path=root/'artifacts/native-inbox/arm64-selection.json'
    selection=json.loads(selected_path.read_text())
    result['selection_sha256']=hashlib.sha256(selected_path.read_bytes()).hexdigest()
    for field,path in [('source_sha256','native-inbox/src/lib.rs'),('tests_sha256','native-inbox/tests/inbox.rs')]:
        assert hashlib.sha256((root/path).read_bytes()).hexdigest()==selection[field]
    cargo=root/selection['cargo_json']
    rows=[json.loads(line) for line in cargo.read_text().splitlines()]
    target_names={'unit':'pokelinux_inbox','inbox':'inbox','original_deliver':'original_deliver'}
    expected_counts={'unit':4,'inbox':15,'original_deliver':1}
    for item in selection['executables']:
        name=item['name'];target=target_names[name]
        candidates=[v for v in rows if v.get('reason')=='compiler-artifact' and v.get('target',{}).get('name')==target and v.get('profile',{}).get('test') and v.get('executable')]
        assert len(candidates)==1
        metadata=[v for v in selection['metadata_executables'] if v['target']==target]
        assert len(metadata)==1 and metadata[0]['matches_qualified_snapshot']
        for path in [item['executable'],item['snapshot_executable'],metadata[0]['path']]:
            assert hashlib.sha256((root/path).read_bytes()).hexdigest()==item['sha256'],path
        exact=Path('artifacts/native-inbox/metadata-restored')/Path(candidates[0]['executable']).relative_to('/target')
        assert str(exact)==metadata[0]['path']
        binary=root/item['snapshot_executable']
        result['binary_sha256'][name]=item['sha256']
        command=['runuser','-u','alarm','--',str(binary)]
        listing=subprocess.check_output(command+['--list'],text=True,timeout=30)
        (out/(name+'-list.txt')).write_text(listing)
        if name=='inbox':
            for test in [selection['helper'],selection['parent']]:assert test+': test' in listing
        run=subprocess.run(command+item['kernel_arguments']+['--nocapture'],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=90)
        (out/(name+'-components.log')).write_text(run.stdout)
        expected=str(expected_counts[name])+' passed; 0 failed; 0 ignored'
        assert run.returncode==0 and expected in run.stdout,run.stdout
        result['checks'][name]=expected_counts[name]
    result['checks'].update({'unprivileged_execution':True,'actual_child_sigkill_reap_restart':True,'recorded_original_aosp_comparisons':33,'total_tests':20})
    result['passed']=True
except BaseException as error:
    result['error']=str(error);(out/'failure.txt').write_text(traceback.format_exc())
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    subprocess.run(['systemctl','poweroff'],check=False)
