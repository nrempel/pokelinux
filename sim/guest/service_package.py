#!/usr/bin/env python3
"""Install and exercise the actual package and user unit in disposable Arch."""
import hashlib,json,os,platform,subprocess,time,traceback
from pathlib import Path
OUT=Path('/results')
result={'passed':False,'architecture':platform.machine(),'phone_accessed':False,'hardware_verified':False}
env={'XDG_RUNTIME_DIR':'/run/user/1000','DBUS_SESSION_BUS_ADDRESS':'unix:path=/run/user/1000/bus','HOME':'/home/alarm'}
def run(command):return subprocess.check_output(command,text=True,stderr=subprocess.STDOUT,timeout=30)
def user(command):return ['runuser','-u','alarm','--','env',*[f'{k}={v}' for k,v in env.items()],*command]
def ctl(*arguments):return run(user(['systemctl','--user',*arguments]))
def wait(predicate):
    end=time.monotonic()+20
    while time.monotonic()<end:
        value=predicate()
        if value:return value
        time.sleep(.2)
    raise AssertionError('User service condition not reached')
def snapshot():
    try:
        value=json.loads(run(user(['/usr/bin/omarchy-phone-local'])))
        return value['snapshot'] if value['available'] else None
    except (subprocess.CalledProcessError,ValueError):return None
try:
    package=Path('/workspace/artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar')
    result['package_sha256']=hashlib.sha256(package.read_bytes()).hexdigest()
    (OUT/'pacman.txt').write_text(run(['pacman','-U','--noconfirm',str(package)]))
    result['installed_sources']={}
    for source in Path('/workspace/transport').glob('*.py'):
        installed=Path('/usr/lib/omarchy-phone')/source.name
        if installed.exists():
            assert installed.read_bytes()==source.read_bytes(),source.name
            result['installed_sources'][source.name]=hashlib.sha256(source.read_bytes()).hexdigest()
    assert len(result['installed_sources'])==11
    assert Path('/usr/lib/systemd/user/omarchy-phone.service').read_bytes()==Path('/workspace/transport/omarchy-phone.service').read_bytes()
    result['unit_sha256']=hashlib.sha256(Path('/usr/lib/systemd/user/omarchy-phone.service').read_bytes()).hexdigest()
    run(['systemctl','start','user@1000.service'])
    wait(lambda:Path('/run/user/1000/bus').exists())
    (OUT/'unit-verify.txt').write_text(run(user(['systemd-analyze','--user','verify','/usr/lib/systemd/user/omarchy-phone.service'])))
    assert ctl('show','omarchy-phone.service','--property=UnitFileState','--value').strip()=='disabled'
    assert not Path('/run/user/1000/omarchy-phone/phone.sock').exists()
    seed_code="import sys;sys.path.insert(0,'/usr/lib/omarchy-phone');from phone_ledger import Ledger;l=Ledger('/home/alarm/.local/state/omarchy-phone');l.reserve(l.epoch,'a'*32,{'fixture':'interrupted'});print(l.epoch);l.close()"
    old_epoch=run(user(['python3','-c',seed_code])).strip()
    ctl('start','omarchy-phone.service')
    first=wait(snapshot)
    recovered=json.loads(run(user(['omarchy-phone-local','recovery'])))
    assert recovered=={'available':True,'recovery':{'epoch':first['epoch'],'more':False,
        'items':[{'epoch':old_epoch,'id':'a'*32,'state':'unknown','result':None}]}},recovered
    result['installed_recovery']=recovered
    assert not first['controls']['fresh'] and first['controls']['value'] is None
    info=Path('/run/user/1000/omarchy-phone/phone.sock').stat()
    assert info.st_uid==1000 and info.st_mode & 0o777==0o600
    assert ctl('show','omarchy-phone.service','--property=NoNewPrivileges','--value').strip()=='yes'
    pid=ctl('show','omarchy-phone.service','--property=MainPID','--value').strip()
    ctl('start','omarchy-phone.service')
    assert ctl('show','omarchy-phone.service','--property=MainPID','--value').strip()==pid
    ctl('kill','--signal=KILL','--kill-whom=main','omarchy-phone.service')
    def restarted():
        value=snapshot()
        return value if value and value['epoch']!=first['epoch'] else None
    second=wait(restarted)
    assert int(ctl('show','omarchy-phone.service','--property=NRestarts','--value'))>=1
    ctl('stop','omarchy-phone.service')
    assert not Path('/run/user/1000/omarchy-phone/phone.sock').exists()
    database=Path('/home/alarm/.local/state/omarchy-phone/operations.sqlite')
    assert database.stat().st_uid==1000 and database.stat().st_mode & 0o777==0o600
    ctl('start','omarchy-phone.service')
    third=wait(snapshot)
    assert third['epoch']!=second['epoch']
    ctl('stop','omarchy-phone.service')
    assert ctl('show','omarchy-phone.service','--property=ActiveState','--value').strip()=='inactive'
    result.update(passed=True,uid=1000,install_did_not_enable=True,crash_recovered=True,
                  explicit_stop_respected=True,restart_epoch_changed=True,
                  missing_pairing_disables_authority=True)
except Exception:
    result['error']=traceback.format_exc();print(result['error'],flush=True)
finally:
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    try:(OUT/'service-journal.txt').write_text(run(['journalctl','--no-pager','-u','user@1000.service','-n','100']))
    except Exception:pass
