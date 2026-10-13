#!/usr/bin/env python3
"""Own isolated native app regression with actual Rust test fixtures; no Android."""
from pathlib import Path
import json,os,signal,subprocess,time
out=Path('/results/qt-final');out.mkdir()
root=Path('/workspace');rust=Path('/results/rust-fixtures/debug')
env=dict(os.environ,HOME=str(out/'home'),XDG_CACHE_HOME=str(out/'cache'),LANG='C.UTF-8')
Path(env['HOME']).mkdir();Path(env['XDG_CACHE_HOME']).mkdir()
with (out/'setup.log').open('w') as log:
    peer=subprocess.Popen([str(rust/'examples/qt_android_peer'),str(out/'enrollment')],stdout=log,stderr=subprocess.STDOUT,start_new_session=True,env=env)
    try:
        deadline=time.monotonic()+5
        while not (out/'enrollment/endpoint.json').exists():
            if peer.poll() is not None or time.monotonic()>deadline:raise RuntimeError('Synthetic enrollment not ready')
            time.sleep(.02)
    finally:
        os.killpg(peer.pid,signal.SIGTERM);peer.wait(timeout=2)
    subprocess.run(['cmake','-S',str(root/'native-app'),'-B',str(out/'build'),'-G','Ninja','-DCMAKE_INSTALL_PREFIX='+str(out/'stage')],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=60)
    subprocess.run(['cmake','--build',str(out/'build'),'-j3'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
    subprocess.run(['cmake','--install',str(out/'build')],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=30)
    import shutil
    shutil.copy2(rust/'omarchy-phone',out/'stage/bin/omarchy-phone')
env.update(PHONE_TEST_BINARIES=str(rust),PHONE_TEST_ENROLLMENT=str(out/'enrollment'),
    PHONE_TEST_SEED=str(rust/'examples/seed_pending'),PHONE_TEST_PEER=str(rust/'examples/qt_android_peer'),
    PHONE_TEST_ACTIVATOR=str(rust/'examples/activate_phone'),PHONE_TEST_APP=str(out/'stage/bin/omarchy-phone-app'),
    PHONE_TEST_SERVICES=str(out/'stage/share/dbus-1/services'))
with (out/'tests.log').open('w') as log:
    test=subprocess.run(['ctest','--test-dir',str(out/'build'),'--output-on-failure'],stdout=log,stderr=subprocess.STDOUT,env=env,timeout=180)
(out/'result.json').write_text(json.dumps({'passed':test.returncode==0,'exit_code':test.returncode,
    'scope':'Native Qt/QML full isolated regression with actual Rust daemon/CLI and synthetic authenticated peer; no Omarchy/Hyprland/Android/hardware proof'},indent=2)+'\n')
raise SystemExit(test.returncode)
