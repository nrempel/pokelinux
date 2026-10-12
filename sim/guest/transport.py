#!/usr/bin/env python3
"""Native Arch ARM64 client controlled only by the disposable test harness."""
import hashlib,json,os,platform,ssl,sys,time,traceback
from pathlib import Path
sys.path.insert(0,'/workspace/transport')
from omarchy_bridge import Client
from guest_response import run_probe
out=Path('/results')
config=json.loads(Path('/workspace/android-framework/transport-fixture/client.json').read_text())
config['host']='10.0.2.2'
(out/'ready.json').write_text(json.dumps({'architecture':platform.machine(),'os':Path('/etc/os-release').read_text(),'python':sys.version,'openssl':ssl.OPENSSL_VERSION,'client_sha256':hashlib.sha256(Path('/workspace/transport/omarchy_bridge.py').read_bytes()).hexdigest()}))
last=None
end=time.monotonic()+900
while time.monotonic()<end:
    try:
        request=json.loads((out/'request.json').read_text())
    except (FileNotFoundError,json.JSONDecodeError):time.sleep(.1);continue
    if request['id']==last:time.sleep(.1);continue
    last=request['id']
    if request['op']=='stop':break
    selected=dict(config)
    if request['op'].startswith('enrolled'):
        selected=json.loads(Path('/workspace/android-framework/enrollment-client.json').read_text())
        selected['host']='10.0.2.2'
        if request['op']=='enrolled_wrong_pin':selected['server_sha256']='0'*64
    if request['op']=='wrong_hostname':selected['server_name']='wrong.invalid'
    if request['op']=='wrong_pin':selected['server_sha256']='0'*64
    if request['op'] in ('expired','wrongpurpose','untrusted'):
        selected['certificate']=selected['certificate'].replace('client.pem',request['op']+'.pem')
        selected['private_key']=selected['private_key'].replace('client.key',request['op']+'.key')
    if request['op']=='wrong_peer':
        selected['certificate']=selected['certificate'].replace('client.pem','other.pem')
        selected['private_key']=selected['private_key'].replace('client.key','other.key')
    def execute():
        client=Client(selected)
        if request['op'] in ('enrolled_prepare_service','enrolled_installed_service','enrolled_qml_service'):
            sys.path.insert(0,'/workspace/framework/tests')
            import installed_service_probe
            if request['op']=='enrolled_prepare_service':value=installed_service_probe.prepare(selected)
            elif request['op']=='enrolled_qml_service':value=installed_service_probe.probe_qml()
            else:value=installed_service_probe.probe()
        elif request['op']=='enrolled_service':
            sys.path.insert(0,'/workspace/framework/tests')
            from service_probe import probe
            value=probe(selected, action=request.get('action', 'answer'))
        elif request['op']=='enrolled_controls':value=client.controls()
        elif request['op']=='enrolled_control':value=client.control(**request['command'])
        else:value=client.status()
        return value
    response=run_probe(last, execute)
    tmp=out/'response.tmp';tmp.write_text(json.dumps(response));tmp.replace(out/'response.json')
