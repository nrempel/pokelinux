#!/usr/bin/env python3
"""Actual Qt client/thread + credentialled Unix socket; no GUI or modem."""
import hashlib,json,resource,signal,subprocess,sys
from pathlib import Path
root=Path('/workspace');out=Path('/results');phase=sys.argv[1]
consumer=root/'artifacts/native-messages-ui-worker/baseline' if phase=='red' else root/'native-messages-ui'
if phase=='red':
 manifest=json.loads((consumer/'manifest.json').read_text())
 for name,digest in manifest['files'].items():assert hashlib.sha256((consumer/name).read_bytes()).hexdigest()==digest,name
source=out/'source';source.mkdir()
for name in ['MessagesClient.cpp','MessagesClient.h']:(source/name).write_bytes((consumer/name).read_bytes())
result={'phase':phase,'passed':False,'scope':'Actual Qt 6.8 client/thread + actual root Unix seqpacket peer; caller clock/wait-return injections and real-recv completion delay only','suspend_executed':False,'kernel_uninterruptible_thread_produced':False,'cases':[],'consumer_source_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in source.iterdir()}}
with (out/'build.log').open('w') as log:
 subprocess.run(['cmake','-S',str(root/'native-messages-ui/worker-tests'),'-B',str(out/'build'),'-G','Ninja','-DMESSAGES_CONSUMER_DIR='+str(source),'-DCMAKE_RUNTIME_OUTPUT_DIRECTORY='+str(out)],check=True,stdout=log,stderr=subprocess.STDOUT,timeout=30)
 subprocess.run(['cmake','--build',str(out/'build'),'-j2'],check=True,stdout=log,stderr=subprocess.STDOUT,timeout=90)
result['binary_sha256']=hashlib.sha256((out/'messages-worker-probe').read_bytes()).hexdigest();result['qt_version']=(out/'build/qt-version.txt').read_text().strip()
for case in ['held-close','clock-admission','clock-completion','held-clock-close','wait-report-failure','within-budget','late-completion']:
 def no_core():resource.setrlimit(resource.RLIMIT_CORE,(0,0))
 try:
  p=subprocess.run([str(out/'messages-worker-probe'),case],capture_output=True,text=True,timeout=3,preexec_fn=no_core)
  stdout,stderr,code=p.stdout,p.stderr,p.returncode;timeout=False
 except subprocess.TimeoutExpired as e:
  stdout=(e.stdout or b'').decode() if isinstance(e.stdout,bytes) else (e.stdout or '')
  stderr=(e.stderr or b'').decode() if isinstance(e.stderr,bytes) else (e.stderr or '')
  code=None;timeout=True
 (out/(case+'.stdout')).write_text(stdout);(out/(case+'.stderr')).write_text(stderr)
 passed=code==0
 if case=='wait-report-failure':passed=code==-signal.SIGABRT and 'actual_wait=1 actual_finished=1' in stderr and 'owner_destroyed' not in stdout
 if case=='held-clock-close':passed=(code==0 or (code==-signal.SIGABRT and 'owner_destroyed' not in stdout)) and 'request_seen=1' in stderr
 result['cases'].append({'case':case,'exit_code':code,'timeout':timeout,'passed':passed,'stdout':stdout,'stderr':stderr})
result['passed']=all(c['passed'] for c in result['cases'])
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'phase':phase,'passed':result['passed'],'cases':[{k:v for k,v in c.items() if k not in ['stdout','stderr']} for c in result['cases']]},indent=2))
raise SystemExit(0 if result['passed'] else 101)
