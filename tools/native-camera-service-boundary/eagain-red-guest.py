#!/usr/bin/env python3
"""Actual Supervisor-channel EAGAIN/cancellation RED, before actor stop fix."""
from pathlib import Path
source=Path('/workspace/artifacts/native-camera-service-boundary-eagain/freeze/sources/tools/native-camera-service-boundary/green-guest.py').read_text()
prefix=source.split('realm=None;service=None\ntry:',1)[0]
assert prefix!=source
exec(compile(prefix.replace('native-camera-service-boundary-green/freeze','native-camera-service-boundary-eagain/freeze'),'frozen-fixture-definitions','exec'))
result['scope']='actual ARM camera actor + Supervisor channel pressure, root synthetic lifecycle controls'
result['expected_red']=False
realm=None;service=None
try:
    assert platform.machine()=='aarch64' and os.getuid()==os.geteuid()==0
    assert ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)==0
    selection=json.loads((BASE/'selection.json').read_text());result['selection_sha256']=digest(BASE/'selection.json')
    for name,value in selection['files'].items():assert digest(BASE/name)==value,'frozen code changed: '+name
    config=gzip.decompress(Path('/proc/config.gz').read_bytes()).decode();assert 'CONFIG_ANDROID_BINDER_IPC=y' in config and 'CONFIG_ANDROID_BINDERFS=y' in config
    Path('/dev/binderfs').mkdir();subprocess.run(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],check=True)
    realm=Realm('eagain-red').start();service=Service(realm,fault='eagain').start();worker=service.worker();guard=marker_check(service)
    assert '\nState:\tT (stopped)' in Path('/proc',str(worker),'status').read_text()
    service.launch();service.authorise();service.send(ttl=1000)
    wait(lambda:service.admin({'op':'snapshot'})['receipts']==1 and not service.admin({'op':'snapshot'})['storage_busy'])
    before=service.status();assert before=={'result':'camera_status','occupied':True,'busy':False,'available':True,'pending':1}
    assert service.driver({'driver':'receive','channel':0})=={'driver_result':'pending'}
    assert realm.probe()['registered']==realm.probe()['submitted']==0
    assert service.admin({'op':'grant','app':'org.omarchy.Camera','permission':'capture','allowed':False})=={'result':'ok'}
    wait(lambda:service.status()['pending']==0)
    after=service.status();actual_alive=Path('/proc',str(worker)).exists()
    assert actual_alive and after['occupied'] and after['available'] and not after['busy']
    assert 'T (stopped)' in Path('/proc',str(worker),'status').read_text()
    counts=realm.probe();assert counts['registered']==counts['submitted']==0
    result['checks']['actual_channel_eagain_cancellation']={'actual_before':before,'actual_after':after,'actual_worker_stopped_alive':actual_alive,'expected_after_occupied':False,'real_registered':0,'real_submitted':0,'durable_receipt_is_effect_claim':False,'marker':guard}
    service.stop()
    text=(OUT/('eagain-red-service-'+str(Service.seq)+'.log')).read_text();assert 'ACTUAL-CAMERA-CHANNEL-FULL' in text and 'errno=11' in text
    result['checks']['actual_channel_eagain_cancellation']['actual_channel_full_witness']=next(line for line in text.splitlines() if line.startswith('ACTUAL-CAMERA-CHANNEL-FULL'))
    result['red_signature']='actual_supervisor_eagain_revoke_retains_ready_worker_expected_reap'
    result['expected_red']=True;service=None;realm.close();realm=None
except Exception:result['error']=traceback.format_exc()
finally:
    try:
        if service is not None:service.force_cleanup()
        if realm is not None:realm.close()
        result['fixture_cleanup_passed']=all(row['reaped'] for row in result['owned_reaps'])
    except Exception:result['cleanup_error']=traceback.format_exc();result['expected_red']=False
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2),flush=True)
    subprocess.run(['systemctl','poweroff'],check=False)
