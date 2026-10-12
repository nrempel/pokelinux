"""Wait for certificate validity in the test emulator; never change its clock."""
import json
from pathlib import Path
import ssl
import subprocess
import time

def wait_for_time(read_time,target,timeout=30,clock=time.monotonic,sleep=time.sleep):
    deadline=clock()+timeout
    samples=[]
    while clock()<deadline:
        samples.append(int(read_time()))
        if samples[-1]>=target:return samples
        sleep(.5)
    raise TimeoutError(f'Emulator clock did not reach certificate notBefore {target}; samples={samples}')

def await_certificate_clock(adb,issued,evidence):
    starts=[]
    for name in ('ca','server','client'):
        output=subprocess.check_output(['openssl','x509','-in',str(Path(issued)/(name+'.pem')),
                                        '-noout','-startdate'],text=True,timeout=10).strip()
        starts.append(ssl.cert_time_to_seconds(output.removeprefix('notBefore=')))
    target=max(starts)
    samples=wait_for_time(lambda:int(adb('shell','date','+%s')),target)
    Path(evidence).write_text(json.dumps({'not_before':target,'android_samples':samples,
        'host_time':time.time(),'clock_changed':False,'validation_disabled':False},indent=2)+'\n')
