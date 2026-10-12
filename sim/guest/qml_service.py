#!/usr/bin/env python3
"""QML through installed CLI/Unix daemon, with an explicitly synthetic Android adapter."""
import hashlib,json,os,platform,shutil,subprocess,time,traceback
from pathlib import Path
out=Path('/results');fixture=None;log=None
result={'passed':False,'architecture':platform.machine(),'hardware_verified':False,'android_adapter':'synthetic','graphics':'Qt offscreen/software'}
try:
    package=Path('/workspace/artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar')
    result['package_sha256']=hashlib.sha256(package.read_bytes()).hexdigest()
    (out/'package-install.txt').write_text(subprocess.check_output(['pacman','-U','--noconfirm',str(package)],text=True,stderr=subprocess.STDOUT,timeout=30))
    base=Path('/tmp/phone-shell-fixture');base.mkdir(mode=0o700);os.chown(base,1000,1000)
    log=(out/'fixture.log').open('w')
    fixture=subprocess.Popen(['runuser','-u','alarm','--','python3','/workspace/omarchy-plugin/tests/service-fixture.py',str(base/'runtime'),'--recovery'],stdout=log,stderr=subprocess.STDOUT)
    end=time.monotonic()+10
    while not (base/'ready').exists():
        assert fixture.poll() is None,'Fixture stopped'
        assert time.monotonic()<end,'Fixture startup timeout'
        time.sleep(.1)
    direct=subprocess.check_output(['runuser','-u','alarm','--','/usr/bin/omarchy-phone-local','--socket',str(base/'runtime/phone.sock'),'snapshot'],text=True,timeout=5)
    (out/'direct-snapshot.json').write_text(direct)
    runtime=Path('/tmp/phone-shell-runtime');runtime.mkdir(mode=0o700);os.chown(runtime,1000,1000)
    stage=Path('/tmp/phone-service-config');shutil.copytree('/workspace/omarchy-plugin',stage)
    (stage/'shell.qml').write_text((stage/'tests/recovery-check.qml').read_text().replace('import "../qml"','import "qml"'))
    command=['runuser','-u','alarm','--','env','XDG_RUNTIME_DIR='+str(runtime),'QT_QPA_PLATFORM=offscreen','QT_QUICK_BACKEND=software','dbus-run-session','--','quickshell','-p',str(stage)]
    process=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=40)
    (out/'service-tests.txt').write_text(process.stdout)
    assert process.returncode==0 and 'PHONE_SERVICE_CHECK_PASSED' in process.stdout,process.stdout[-4000:]
    transmissions=json.loads((base/'transmissions.json').read_text())
    assert [v['action'] for v in transmissions]==['answer','disconnect'],transmissions
    assert len({v['operation_id'] for v in transmissions})==2
    result['transmissions']=transmissions
    result['sources']={str(p.relative_to('/workspace')):hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('/workspace/omarchy-plugin').rglob('*') if p.suffix in ('.qml','.py')}
    result['passed']=True
except Exception:result['error']=traceback.format_exc()
finally:
    if fixture is not None and fixture.poll() is None:
        fixture.terminate()
        try:fixture.wait(timeout=5)
        except subprocess.TimeoutExpired:fixture.kill();fixture.wait()
    if log:log.close()
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
