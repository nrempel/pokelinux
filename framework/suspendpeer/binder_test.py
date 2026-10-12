#!/usr/bin/env python3
"""Exact frozen V1 proxies and native Rust dispatcher over owned kernel Binder."""
from pathlib import Path
import json,os,selectors,subprocess,sys,time

PHASE=sys.argv[1]
OUT=Path('artifacts/suspend-boundary/runtime')/PHASE
OUT.mkdir(parents=True,exist_ok=True)
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
PACKAGE='org.pokelinux.suspendpeer'
REMOTE='/data/local/tmp/pokelinux-suspend-boundary'
def now():return time.clock_gettime(time.CLOCK_BOOTTIME)
def run(*args,timeout=45):
 result=subprocess.run(ADB+list(args),capture_output=True,text=True,timeout=timeout)
 if result.returncode:raise RuntimeError(f'adb {args}: {result.returncode}: {result.stdout} {result.stderr}')
 return result.stdout

def client(mode,branch='locked'):
 result=subprocess.run(ADB+['shell','timeout','15',REMOTE+'/client-'+branch,mode],capture_output=True,text=True,timeout=25)
 (OUT/(mode+'-'+branch+'-stdout.txt')).write_text(result.stdout)
 (OUT/(mode+'-'+branch+'-stderr.txt')).write_text(result.stderr)
 return result

def snapshot():
 result=client('snapshot');assert result.returncode==0,result.stderr
 values=[int(x) for x in result.stdout.strip().split()];assert len(values)==11
 return values

def count(expected):
 end=now()+3
 while snapshot()[0]!=expected:
  assert now()<end,'wake object did not reach expected count';time.sleep(.02)

class RemoteProcess:
 def __init__(self,label,argv,executable):
  self.label=label;self.executable=executable;self.pid=None;self.pending=b'';self.lines=[]
  self.err=(OUT/(label+'-stderr.txt')).open('wb')
  self.process=subprocess.Popen(ADB+['shell','timeout','55',*argv],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.err)
  self.selector=selectors.DefaultSelector();self.selector.register(self.process.stdout,selectors.EVENT_READ)
 def line(self,prefix,timeout=12):
  end=now()+timeout
  while now()<end:
   if b'\n' in self.pending:
    raw,self.pending=self.pending.split(b'\n',1);line=raw.decode().rstrip('\r');self.lines.append(line)
    if line.startswith(prefix):return line
    continue
   events=self.selector.select(min(.1,max(0,end-now())))
   if events:
    data=os.read(self.process.stdout.fileno(),4096)
    if not data:raise RuntimeError(f'{self.label} EOF before {prefix}: {self.lines}')
    self.pending+=data
  raise RuntimeError(f'{self.label} timeout before {prefix}: {self.lines}')
 def resume(self):self.process.stdin.write(b'continue\n');self.process.stdin.flush()
 def wait(self,expected=0):
  result=self.process.wait(timeout=10);assert result==expected,(self.label,result,self.lines)
 def kill_reap(self):
  if self.pid is not None:
   result=subprocess.run(ADB+['shell','cat',f'/proc/{self.pid}/cmdline'],capture_output=True,timeout=5)
   if result.returncode==0 and result.stdout.split(b'\x00')[0]==self.executable.encode():
    run('shell','kill','-9',str(self.pid),timeout=5)
  try:self.process.wait(timeout=5)
  except subprocess.TimeoutExpired:self.process.kill();self.process.wait(timeout=5)
 def close(self):
  if self.process.poll() is None:self.kill_reap()
  self.selector.close();self.process.stdout.close();self.process.stdin.close();self.err.close()
  (OUT/(self.label+'-stdout.txt')).write_text('\n'.join(self.lines)+'\n')

owned=[]
epoch=0
def server():
 global epoch
 epoch+=1
 process=RemoteProcess('server-'+str(epoch),[REMOTE+'/server',REMOTE+'/epoch-'+str(epoch),'fresh' if epoch==1 else 'attach'],REMOTE+'/server');owned.append(process)
 line=process.line('suspend-rust-server\t');process.pid=int(line.split('\t')[1]);assert line.endswith('READY')
 return process

def holder(mode):
 process=RemoteProcess(mode,[REMOTE+'/client-locked',mode],REMOTE+'/client-locked');owned.append(process)
 line=process.line(mode+'\t');process.pid=int(line.split('\t')[1]);assert line.endswith('READY')
 return process

(OUT/'result.json').write_text(json.dumps({'passed':False,'phase':PHASE,'phone_accessed':False})+'\n')
run('wait-for-device');end=now()+180
while run('shell','getprop','sys.boot_completed').strip()!='1':assert now()<end;time.sleep(2)
assert run('shell','getenforce').strip()=='Enforcing'
(OUT/'image.txt').write_text(run('shell','getprop','ro.build.fingerprint'))
(OUT/'identity.txt').write_text(run('shell','id')+run('shell','cat','/proc/self/attr/current'))
run('shell','mkdir','-p',REMOTE)
run('push','artifacts/ims-native-audit/oracle/oracle.jar',REMOTE+'/oracle.jar')
bootstrap=run('shell','env','CLASSPATH='+REMOTE+'/oracle.jar','app_process','/system/bin','org.omarchy.audit.FrameworkBootstrapOracle')
assert bootstrap.splitlines()==['startInstrumentation\t51\tPASS','instrumentationStatus\t1\tPASS'],bootstrap
(OUT/'framework-bootstrap.txt').write_text(bootstrap)
run('install','--no-incremental','-r','artifacts/suspend-boundary/peer.apk')
run('push','artifacts/suspend-boundary/peer.apk',REMOTE+'/bootstrap.apk')
flags=run('shell','env','CLASSPATH='+REMOTE+'/bootstrap.apk','app_process','/system/bin','org.pokelinux.suspendpeer.FlagOracle')
assert flags.strip()=='no-restart\t8',flags
(OUT/'framework-flags.txt').write_text(flags)
for name,source in [('server','artifacts/suspend-boundary/rust/x86_64-unknown-linux-musl/release/suspend-binder-server'),('client-locked','artifacts/suspend-boundary/suspend-ndk-client-locked'),('client-unlocked','artifacts/suspend-boundary/suspend-ndk-client-unlocked'),('rust-client','artifacts/suspend-boundary/rust/x86_64-unknown-linux-musl/release/suspend-binder-client')]:
 run('push',source,REMOTE+'/'+name);run('shell','chmod','755',REMOTE+'/'+name)
try:
 current=server()
 basic=client('basic')
 print(basic.stdout,flush=True)
 if basic.returncode!=0:
  observed=client('snapshot');(OUT/'startup-observation.txt').write_text(observed.stdout+observed.stderr)
 if PHASE=='red':
  assert basic.returncode!=0 and basic.stdout.splitlines()==['frozen-v1-native-identity\tPASS','oneway-release-receipt\tPASS'] and 'wake count mismatch' in basic.stderr,(basic.returncode,basic.stdout,basic.stderr)
  (OUT/'result.json').write_text(json.dumps({'passed':False,'expected_red_observed':True,'scope':'actual generated NDK V1 proxy -> Rust dispatcher; one-way receipt occurs but release has no coordinator effect','phone_accessed':False})+'\n')
  raise SystemExit(1)
 assert basic.returncode==0,(basic.stdout,basic.stderr)
 reverse=subprocess.run(ADB+['shell','timeout','15',REMOTE+'/rust-client'],capture_output=True,text=True,timeout=25)
 (OUT/'rust-client-stdout.txt').write_text(reverse.stdout);(OUT/'rust-client-stderr.txt').write_text(reverse.stderr)
 assert reverse.returncode==0 and reverse.stdout.splitlines()==['rust-proxy-ndk-v1-dispatch-identity\tPASS','rust-proxy-ndk-release-once\tPASS','rust-proxy-ndk-last-reference\tPASS'],(reverse.returncode,reverse.stdout,reverse.stderr)
 print(reverse.stdout,flush=True)
 default=client('default-gate');assert default.returncode==0;print(default.stdout,flush=True)
 owner=client('transfer-owner');assert owner.returncode==0;count(1)
 second=holder('holder');count(1);second.kill_reap();count(0);second.close()
 print('binder-reference-transfer-and-client-death\tPASS',flush=True)
 for branch in ['locked','unlocked']:
  null=client('null-cache',branch);assert null.returncode==0,(null.stdout,null.stderr)
  power=RemoteProcess('power-cache-'+branch,[REMOTE+'/client-'+branch,'power-cache'],REMOTE+'/client-'+branch);owned.append(power)
  assert power.line('libpower-name-dedup\t').endswith('PASS');power.resume()
  assert power.line('libpower-dead-cache\t').endswith('PASS');current.wait(expected=77)
  current=server();power.resume();assert power.line('libpower-restart-stays-dead\t').endswith('PASS');power.wait();power.close()
  assert snapshot()[0]==0 and snapshot()[2]==0,'cached client rebound to new native service'
  print('original-libpower-'+branch+'-cache-death-restart\tPASS',flush=True)
 authority=holder('authority');authority.kill_reap();authority.close()
 end=now()+3
 while snapshot()[1]!=3:assert now()<end;time.sleep(.02)
 fenced=client('fenced');assert fenced.returncode==0,(fenced.stdout,fenced.stderr);print(fenced.stdout,flush=True)
 assert run('shell','getenforce').strip()=='Enforcing'
 result={'passed':True,'phone_accessed':False,'selinux':'Enforcing','scope':'exact frozenV1 generatedNDK proxies and dispatcher -> nativeRust generated frontend, actual kernel Binder objects','reverse_rust_proxy_ndk_v1_dispatcher':True,'frozen_version':1,'frozen_hash':'082a58c2505b703702f34ab8abcb1d1b5e890be4','native_caller_uid_pid_sid':True,'oneway_receipt_separate_from_release':True,'explicit_release_once':True,'last_reference_and_cross_process_transfer':True,'client_and_authority_death':True,'no_implicit_rearm':True,'public_libpower_unmodified':True,'public_libpower_branches':['locked','unlocked'],'discovery_adapter_used':True,'default_registration_qualified':False,'kernel_suspend_qualified':False,'vendor_code_executed':False}
 (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
finally:
 for process in reversed(owned):
  try:process.close()
  except Exception as error:print(f'owned process cleanup: {error}',file=sys.stderr)
 run('shell','am','force-stop',PACKAGE)
