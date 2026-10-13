#!/usr/bin/env python3
"""New desired enumeration RED through actual frozen live ARM Binder33rd arrival."""
import json,platform,pwd,subprocess,sys,traceback
from pathlib import Path
ROOT=Path('/workspace');BASE=ROOT/'artifacts/native-messages-enumeration';out=Path('/results')
frozen=ROOT/'artifacts/native-messages-incoming-binder/selections/incoming-green-final-v2/sources/tools/native-messages-incoming-binder'
sys.path.insert(0,str(frozen))
import peer_session
peer_session.BASE=BASE
from peer_session import PeerSession
import integration
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),'qemu_mode':'system','network':'none','phone_accessed':False,'vendor_execution':False,'source':'frozen f733d3b incoming milestone before enumeration implementation','checks':{}}
session=None;service=None
try:
    assert platform.machine()=='aarch64'
    Path('/dev/binderfs').mkdir();subprocess.run(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],check=True)
    phase=next(x.split('=',1)[1] for x in Path('/proc/cmdline').read_text().split() if x.startswith('pixel.messages-enumeration-phase='))
    if phase=='green':
        import green
        green.run(pwd.getpwnam('alarm'),out,result)
        for manifest in ['receipt-arm-tests.jsonl','projection-arm-tests.jsonl']:
            for line in (BASE/'target'/manifest).read_text().splitlines():
                row=json.loads(line)
                if row.get('reason')=='compiler-artifact' and row.get('executable') and row['profile']['test']:
                    path=BASE/'target'/Path(row['executable']).relative_to('/target')
                    run=subprocess.run([str(path)],capture_output=True,text=True,timeout=35)
                    (out/(manifest+'.log')).write_text(run.stdout+run.stderr);assert run.returncode==0
        result['passed']=True
        result['source']='current receipt/store/worker with immutable f733 lifecycle actor and generated receive-only peer'
    else:
        session=PeerSession(pwd.getpwnam('alarm'),out,'enumeration-red');session.start();service=integration.Service(out,session).start()
        for count in range(1,34):
            assert service.status()['receiving'],'actual source must remain live before33rd callback'
            assert service.arrival().returncode==0
            if count<33:integration.wait(lambda:service.status()['incoming_completed']==count)
        integration.wait(lambda:service.status()['incoming_raw_committed']==33 or not service.status()['occupied'])
        service.unavailable_reaped();raw=service.raw();records=service.records();assert len(raw)==33 and len(records)==32
        ids=[row[0] for row in raw];assert len(set(ids))==33
        counters=[json.loads(integration.probe('counters',slot).stdout) for slot in range(2)];assert all(row[3:]==[0,0,0,0] for row in counters)
        result['checks']['live_binder33_desired_complete_background_projection']={'expected_records':33,'actual_records':len(records),'raw_occurrences':len(raw),'distinct_equal_receipts':True,'actual_live33rd_generated_binder_callback':True,'genuine_runtime':session.evidence,'zero_ack_send_response_ack_other':True,'status':service.status()}
        result['expected_red']=True;result['signature']='live_binder33_complete_projection_unavailable'
        assert service.status()['available'] and len(records)==33,'new requirement: all33 committed IDs project while closed and source remains available'
        result['passed']=True
except Exception:result['error']=traceback.format_exc()
finally:
    if service:service.stop()
    if session:session.close()
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2),flush=True);subprocess.run(['systemctl','poweroff'],check=False)
