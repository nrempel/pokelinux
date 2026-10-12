#!/usr/bin/env python3
"""Render and exercise the real phone QML component with explicit test data."""
import hashlib,json,os,platform,shutil,subprocess,traceback
from pathlib import Path
out=Path('/results')
result={'passed':False,'architecture':platform.machine(),'hardware_verified':False,'graphics':'Qt offscreen/software','data':'synthetic QML component states'}
try:
    runtime=Path('/tmp/phone-qml-runtime');runtime.mkdir(mode=0o700);os.chown(runtime,1000,1000)
    command=['runuser','-u','alarm','--','env','XDG_RUNTIME_DIR='+str(runtime),'QT_QPA_PLATFORM=offscreen','QT_QUICK_BACKEND=software','dbus-run-session','--','/usr/lib/qt6/bin/qmltestrunner','-input','/workspace/omarchy-plugin/tests','-o','-,txt']
    run=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=90)
    (out/'qml-card-tests.txt').write_text(run.stdout)
    result['returncode']=run.returncode
    assert run.returncode==0,run.stdout[-5000:]
    for name in ('incoming','answering','active','unknown','recovery','controller-answering','controller-unknown'):
        source=Path('/tmp/phone-card-'+name+'.png')
        assert source.is_file() and source.stat().st_size>1000,name
        shutil.copyfile(source,out/(name+'.png'))
    result['sources']={str(p.relative_to('/workspace')):hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('/workspace/omarchy-plugin').rglob('*.qml')}
    result['passed']=True
except Exception:
    result['error']=traceback.format_exc()
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
