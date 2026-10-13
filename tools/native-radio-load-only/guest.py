#!/usr/bin/env python3
"""Public harmless-child actual ARM64-kernel selection; no original radio runtime."""
import hashlib,json,os,platform,subprocess,sys,traceback
from pathlib import Path
BASE=Path('/workspace/artifacts/native-radio-load-only')
OUT=Path('/results')
FREEZE=BASE/(sys.argv[2] if len(sys.argv)>2 else 'freeze')
selection=json.loads((FREEZE/'selection.json').read_text())
result={'passed':False,'expected_red':False,'architecture':platform.machine(),'kernel':platform.release(),'qemu_mode':'system','vendor_execution':False,'physical_phone_accessed':False,'hardware_verified':False,'registration_admission':False}
try:
    assert platform.machine()=='aarch64' and os.geteuid()==0 and os.getpid()==1
    result['pid1_exe']=os.readlink('/proc/1/exe')
    result['pid1_cmdline']=Path('/proc/1/cmdline').read_bytes().split(b'\0')[:-1]
    result['pid1_cmdline']=[part.decode() for part in result['pid1_cmdline']]
    processes={}
    for path in Path('/proc').iterdir():
        if path.name.isdecimal():
            try:
                args=(path/'cmdline').read_bytes()
                if args:processes[path.name]=args.decode().replace('\0',' ')
            except FileNotFoundError:pass
    assert list(processes)==['1'],f'userspace service-free boot failed: {processes}'
    result['service_free_userspace']=processes
    for name,digest in selection['files'].items():
        assert hashlib.sha256((FREEZE/name).read_bytes()).hexdigest()==digest,name
    case=sys.argv[1]
    if case=='red':
        run=subprocess.run([str(BASE/'freeze/bin/radio-containment-red'),'/run/radio-containment-red',str(BASE/'freeze/bin/radio-harmless-child')],capture_output=True,text=True,timeout=15)
        (OUT/'public-control.log').write_text(run.stdout+run.stderr)
        observation=json.loads(run.stdout)
        result.update(observation)
        result['actual_exit']=run.returncode
        assert run.returncode==1 and observation['expected_red'] and observation['cleanup_passed'],'bootstrap failure does not count as RED'
    elif case=='green':
        sys.path.insert(0,str(FREEZE/'source/tools/native-radio-load-only'))
        import green
        result.update(green.execute(FREEZE,OUT))
    else:raise AssertionError('not an authorized selection')
except Exception:
    result['bootstrap_or_cleanup_error']=traceback.format_exc()
    result['expected_red']=False
finally:
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    subprocess.run(['sync'],check=True,timeout=5)
    subprocess.run(['poweroff','-f'],check=True,timeout=5)
