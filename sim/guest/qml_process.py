#!/usr/bin/env python3
"""Exercise subprocess lifecycle in actual Quickshell, not a QtTest substitute."""
import hashlib,json,os,platform,shutil,subprocess,traceback
from pathlib import Path
out=Path('/results');result={'passed':False,'architecture':platform.machine(),'hardware_verified':False,'graphics':'Qt offscreen/software'}
try:
    runtime=Path('/tmp/phone-process-runtime');runtime.mkdir(mode=0o700);os.chown(runtime,1000,1000)
    stage=Path('/tmp/phone-process-config');shutil.copytree('/workspace/omarchy-plugin',stage)
    (stage/'shell.qml').write_text((stage/'tests/process-check.qml').read_text().replace('import "../qml"','import "qml"'))
    command=['runuser','-u','alarm','--','env','XDG_RUNTIME_DIR='+str(runtime),'QT_QPA_PLATFORM=offscreen','QT_QUICK_BACKEND=software','dbus-run-session','--','quickshell','-p',str(stage)]
    process=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=40)
    (out/'process-tests.txt').write_text(process.stdout)
    assert process.returncode==0 and 'PHONE_PROCESS_CHECK_PASSED' in process.stdout,process.stdout[-4000:]
    result['sources']={str(p.relative_to('/workspace')):hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('/workspace/omarchy-plugin').rglob('*.qml')}
    result['passed']=True
except Exception:result['error']=traceback.format_exc()
finally:(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
