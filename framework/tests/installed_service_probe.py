"""Real packaged user-service call path, inside the disposable Arch guest only."""
import hashlib,json,os,subprocess,time,uuid
from pathlib import Path
ENV={'XDG_RUNTIME_DIR':'/run/user/1000','DBUS_SESSION_BUS_ADDRESS':'unix:path=/run/user/1000/bus','HOME':'/home/alarm'}
def run(command):return subprocess.check_output(command,text=True,stderr=subprocess.STDOUT,timeout=30)
def user(command):return ['runuser','-u','alarm','--','env',*[f'{k}={v}' for k,v in ENV.items()],*command]
def ctl(*args):return run(user(['systemctl','--user',*args]))
def cli(*args):return json.loads(run(user(['/usr/bin/omarchy-phone-local',*args])))
def wait(predicate):
    end=time.monotonic()+15
    while time.monotonic()<end:
        value=predicate()
        if value:return value
        time.sleep(.1)
    raise AssertionError('Packaged service observation timed out')
def prepare(config):
    package=Path('/workspace/artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar')
    run(['pacman','-U','--noconfirm',str(package)])
    directory=Path('/home/alarm/.config/omarchy-phone')
    directory.mkdir(parents=True,mode=0o700,exist_ok=True)
    os.chown(directory,1000,1000)
    path=directory/'client.json'
    path.write_text(json.dumps(config));path.chmod(0o600);os.chown(path,1000,1000)
    run(['systemctl','start','user@1000.service'])
    ctl('start','omarchy-phone.service')
    def ready():
        try:return cli().get('available')
        except subprocess.CalledProcessError:return False
    wait(ready)
    return {'ready':True,'package_sha256':hashlib.sha256(package.read_bytes()).hexdigest()}
def probe():
    authority=cli('session')['session']
    def observed(state):
        value=cli()
        if not value['available']:return None
        snapshot=value['snapshot'];lane=snapshot['controls']
        if not lane['fresh'] or not lane['value']:return None
        calls=lane['value']['calls']
        return snapshot if (not calls if state is None else len(calls)==1 and calls[0]['state']==state) else None
    def command(snapshot,action,identity):
        value=snapshot['controls']['value'];call=value['calls'][0]
        return ['control','--epoch',snapshot['epoch'],'--id',identity,'--session',authority['session'],
                '--generation',value['generation'],'--call-id',str(call['id']),
                '--revision',str(value['revision']),'--state',str(call['state']),'--action',action]
    expected={'state':'finished','result':{'outcome':'submitted','reason':'accepted'}}
    try:
        ringing=wait(lambda:observed(2))
        identity=uuid.uuid4().hex
        args=command(ringing,'answer',identity)
        assert cli(*args)['operation']==expected
        assert cli('result','--epoch',ringing['epoch'],'--id',identity)['operation']==expected
        active=wait(lambda:observed(4))
        assert cli(*args)['operation']==expected  # Original identity, not a second action.
        assert cli(*command(active,'disconnect',uuid.uuid4().hex))['operation']==expected
        wait(lambda:observed(None))
        result={'passed':True,'uid':1000,'installed_entrypoints':True,'answer_active':True,
                'duplicate_receipt':True,'end_idle':True,'phone_accessed':False,
                'package_sha256':hashlib.sha256(Path('/workspace/artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar').read_bytes()).hexdigest(),
                'sources':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('/usr/lib/omarchy-phone').glob('*.py')}}
        return result
    finally:
        ctl('stop','omarchy-phone.service')

def probe_qml():
    """Programmatic QML gestures through the real installed Android bridge."""
    import shutil
    stage=Path('/tmp/phone-android-qml')
    shutil.copytree('/workspace/omarchy-plugin',stage)
    source=(stage/'tests/service-check.qml').read_text()
    source=source.replace('import "../qml"','import "qml"').replace('/tmp/phone-shell-fixture/runtime/phone.sock','/run/user/1000/omarchy-phone/phone.sock')
    (stage/'shell.qml').write_text(source)
    ctl('start','omarchy-phone.service')
    try:
        command=user(['env','QT_QPA_PLATFORM=offscreen','QT_QUICK_BACKEND=software','quickshell','-p',str(stage)])
        process=subprocess.run(command,capture_output=True,text=True,timeout=40)
        output=process.stdout+process.stderr
        Path('/results/qml-service.log').write_text(output)
        assert process.returncode==0 and 'PHONE_SERVICE_CHECK_PASSED' in output,output[-4000:]
        return {'passed':True,'uid':1000,'input':'programmatic QML method calls, not touch',
                'android_adapter':'real enrolled Android emulator','phone_accessed':False,
                'package_sha256':hashlib.sha256(Path('/workspace/artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar').read_bytes()).hexdigest(),
                'qml_sources':{str(p.relative_to('/workspace/omarchy-plugin')):hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('/workspace/omarchy-plugin').rglob('*.qml')}}
    finally:ctl('stop','omarchy-phone.service')


def probe_outgoing():
    """Installed CLI -> user daemon -> enrolled Android, before QML claims a session."""
    authority=cli('session')['session']
    def dial_ready():
        s=cli()['snapshot'];lane=s['dial_status']
        return s if lane['fresh'] and lane['value'] and lane['value']['can_dial'] else None
    snapshot=wait(dial_ready);offer=snapshot['dial_status']['value'];identity=uuid.uuid4().hex
    args=['dial','--epoch',snapshot['epoch'],'--session',authority['session'],'--id',identity,
          '--generation',offer['generation'],'--offer',offer['offer'],'--number','5550100']
    receipt=cli(*args)['operation']
    assert receipt=={'state':'finished','result':{'outcome':'invocation_attempted','reason':'returned'}},receipt
    assert cli(*args)['operation']==receipt
    def live_call():
        s=cli()['snapshot'];lane=s['controls'];value=lane['value']
        return s if lane['fresh'] and value and len(value['calls'])==1 and 'disconnect' in value['calls'][0]['actions'] else None
    call_snapshot=wait(live_call)
    Path('/results/outgoing.json').write_text(json.dumps({'receipt':receipt,'snapshot':call_snapshot,'duplicate_receipt':True}))
    wait(lambda:Path('/results/outgoing-verified.json').exists())
    call_snapshot=wait(live_call);value=call_snapshot['controls']['value'];call=value['calls'][0]
    ended=cli('control','--epoch',call_snapshot['epoch'],'--session',authority['session'],'--id',uuid.uuid4().hex,
        '--generation',value['generation'],'--call-id',str(call['id']),'--revision',str(value['revision']),
        '--state',str(call['state']),'--action','disconnect')['operation']
    assert ended=={'state':'finished','result':{'outcome':'submitted','reason':'accepted'}},ended
    def idle():
        lane=cli()['snapshot']['controls']
        return lane['fresh'] and lane['value'] and not lane['value']['calls']
    wait(idle)
    return {'dial':receipt,'end':ended,'duplicate_receipt':True,'idle':True,'input':'installed CLI, not native dial pad'}
