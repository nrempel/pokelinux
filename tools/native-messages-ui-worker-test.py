#!/usr/bin/env python3
"""Actual Qt client/thread + credentialled Unix socket; no GUI or modem."""
import hashlib,json,resource,signal,subprocess,sys
from pathlib import Path
if len(sys.argv) not in (2,4) or sys.argv[1] not in ('red','green'):
 raise SystemExit('Usage: native-messages-ui-worker-test.py red|green [EXACT_ARCHIVE SHA256]')
root=Path('/workspace');out=Path('/results');phase=sys.argv[1]
consumer=root/'artifacts/native-messages-ui-worker/baseline' if phase=='red' else root/'native-messages-ui'
source=out/'source';source.mkdir()
explicit_archive=None
if phase=='red':
 if len(sys.argv)!=2:raise SystemExit('Historical RED cannot link a current Rust engine')
 manifest=json.loads((consumer/'manifest.json').read_text())
 for name,digest in manifest['files'].items():
  relative=Path(name)
  if relative.is_absolute() or '..' in relative.parts:raise SystemExit('Historical manifest contains a nonlocal path')
  if hashlib.sha256((consumer/relative).read_bytes()).hexdigest()!=digest:raise SystemExit('Historical frozen source digest mismatch: '+name)
 # Historical execution is a separate immutable build/source set. Never compile
 # old QObject headers through the current full-engine CMake selection.
 names=['MessagesClient.cpp','MessagesClient.h','CMakeLists.txt','process.cpp']
 if any(name not in manifest['files'] for name in names):
  raise SystemExit('Historical RED prerequisite: manifest must pin its own original CMakeLists.txt and process.cpp alongside MessagesClient.cpp/.h')
 for name in names:(source/name).write_bytes((consumer/name).read_bytes())
 build_source=source
else:
 names=['MessagesClient.cpp','MessagesClient.h','client-core/include/pokelinux_messages_client.h']
 for name in names:
  destination=source/name;destination.parent.mkdir(parents=True,exist_ok=True);destination.write_bytes((consumer/name).read_bytes())
 build_source=root/'native-messages-ui/worker-tests'
 if len(sys.argv)==4:
  explicit_archive=Path(sys.argv[2]).resolve();expected=sys.argv[3]
  if len(expected)!=64 or any(c not in '0123456789abcdef' for c in expected):raise SystemExit('Exact lowercase archive SHA256 required')
  if hashlib.sha256(explicit_archive.read_bytes()).hexdigest()!=expected:raise SystemExit('Explicit Messages archive digest mismatch')
result={'phase':phase,'passed':False,'scope':'Actual Qt 6.8 client/thread + actual root Unix seqpacket peer; caller clock/wait-return injections and real-recv completion delay only','suspend_executed':False,'kernel_uninterruptible_thread_produced':False,'cases':[],'consumer_source_sha256':{name:hashlib.sha256((source/name).read_bytes()).hexdigest() for name in names},'build_cmake_sha256':hashlib.sha256((build_source/'CMakeLists.txt').read_bytes()).hexdigest(),'historical_source_selection':phase=='red'}
configure=['cmake','-S',str(build_source),'-B',str(out/'build'),'-G','Ninja','-DMESSAGES_CONSUMER_DIR='+str(source),'-DCMAKE_RUNTIME_OUTPUT_DIRECTORY='+str(out)]
if explicit_archive:
 configure+=['-DPOKELINUX_MESSAGES_CLIENT_ARCHIVE='+str(explicit_archive),'-DPOKELINUX_MESSAGES_CLIENT_ARCHIVE_SHA256='+sys.argv[3]]
result['configure_argv']=configure
with (out/'build.log').open('w') as log:
 subprocess.run(configure,check=True,stdout=log,stderr=subprocess.STDOUT,timeout=30)
 subprocess.run(['cmake','--build',str(out/'build'),'-j2'],check=True,stdout=log,stderr=subprocess.STDOUT,timeout=90)
if phase=='green':
 actual_archive=explicit_archive or out/'build/rust-target/release/libpokelinux_messages_client.a'
 result['engine_archive']={'path':str(actual_archive),'sha256':hashlib.sha256(actual_archive.read_bytes()).hexdigest(),'selection':'explicit-pinned' if explicit_archive else 'native-locked-offline-Cargo'}
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
