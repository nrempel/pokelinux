#!/usr/bin/env python3
"""No GUI, host font settings, suspend or vendor process. Own Qt process worker only."""
import hashlib,json,os,resource,signal,subprocess,sys
from pathlib import Path
out=Path('/results');root=Path('/workspace')
phase=sys.argv[1]
consumer=root/'artifacts/native-theme-worker/baseline' if phase=='red' else root/'native-app'
if phase=='red':
 manifest=json.loads((consumer/'manifest.json').read_text())
 for name,digest in manifest['files'].items():assert hashlib.sha256((consumer/name).read_bytes()).hexdigest()==digest,name
result={'passed':False,'scope':'Actual Qt/QProcess + actual fc-match on private FIFO; wait-report and clock-failure linker injection only',
        'suspend_executed':False,'d_state_child_produced':False,'physical_hardware_verified':False,'cases':[]}
# Private access changes no fields/methods/production code. Only own test compile sees it.
source=out/'source';source.mkdir()
for name in ['PhoneThemeFiles.cpp','PhoneThemeFiles.h']:(source/name).write_bytes((consumer/name).read_bytes())
result['consumer_sources_sha256']={n:hashlib.sha256((source/n).read_bytes()).hexdigest() for n in ['PhoneThemeFiles.cpp','PhoneThemeFiles.h']}
header=source/'PhoneThemeFiles.h'
header.write_text(header.read_text().replace('private:\n','private:\n    friend class NativeThemeWorkerProbe;\n',1))
result['test_only_header_sha256']=hashlib.sha256(header.read_bytes()).hexdigest()
# ELF wrapper changes only the application's wait return after the real Qt reaps.
with (out/'build.log').open('w') as log:
 subprocess.run(['cmake','-S',str(root/'native-theme/worker-tests'),'-B',str(out/'build'),'-G','Ninja','-DTHEME_CONSUMER_DIR='+str(source),'-DCMAKE_RUNTIME_OUTPUT_DIRECTORY='+str(out)],check=True,stdout=log,stderr=subprocess.STDOUT,timeout=30)
 subprocess.run(['cmake','--build',str(out/'build'),'-j2'],check=True,stdout=log,stderr=subprocess.STDOUT,timeout=90)
result['binary_sha256']=hashlib.sha256((out/'worker-probe').read_bytes()).hexdigest()
result['qt_version']=(out/'build/qt-version.txt').read_text().strip()
for case in ['normal-destruction','reap-failure','within-budget','late-completion','clock-start-failure','clock-completion-failure']:
 def no_core():resource.setrlimit(resource.RLIMIT_CORE,(0,0))
 process=subprocess.run([str(out/'worker-probe'),case],capture_output=True,text=True,timeout=8,preexec_fn=no_core)
 (out/(case+'.stdout')).write_text(process.stdout);(out/(case+'.stderr')).write_text(process.stderr)
 expected=-signal.SIGABRT if case=='reap-failure' else 0
 record={'case':case,'exit_code':process.returncode,'expected_exit_code':expected,'passed':process.returncode==expected,'stdout':process.stdout,'stderr':process.stderr}
 if case=='reap-failure':record['passed'] &= 'actual_wait=1 actual_state=0' in process.stderr and 'owner_destroyed' not in process.stdout
 result['cases'].append(record)
result['passed']=all(row['passed'] for row in result['cases'])
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'passed':result['passed'],'cases':[{k:v for k,v in r.items() if k not in ['stdout','stderr']} for r in result['cases']]},indent=2))
raise SystemExit(0 if result['passed'] else 101)
