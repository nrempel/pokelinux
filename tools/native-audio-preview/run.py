#!/usr/bin/env python3
"""One isolated real QML/actor/PCM case; no container/VM/host-service launcher."""
import sys
sys.dont_write_bytecode=True
from pathlib import Path
import argparse,ctypes,hashlib,json,os,re,resource,shutil,stat,struct,traceback
TOOLS=Path(__file__).resolve().parent
sys.path.insert(0,str(TOOLS.parent/'native-installed-services'))
from owned import Scope,now
from closure import close_owned
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--isolated-fixture',action='store_true',required=True,help='Run only inside a dedicated owned container or disposable guest')
p.add_argument('--providers',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--work-until-ns',type=int,required=True)
p.add_argument('--close-until-ns',type=int,required=True)
a=p.parse_args();T=now();W=min(a.work_until_ns,T+240_000_000_000);C=min(a.close_until_ns,T+270_000_000_000)
assert os.getresuid()==os.getresgid()==(0,0,0),'isolated fixture requires root; never run on the host'
assert T<W<C,'original work and close cutoffs must be in the future'
assert a.output.is_absolute()and a.providers.is_absolute(),'explicit absolute provider/output paths required'
O=a.output;assert not O.exists()or O.is_dir()and not any(O.iterdir()),'fresh empty output directory required'
O.mkdir(mode=0o755,parents=False,exist_ok=True)
resource.setrlimit(resource.RLIMIT_CORE,(0,0));assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
D=O/'commands';D.mkdir(mode=0o700);scopes=[];serial=0;admission=True
r={'schema':1,'passed':False,'classification':'SETUP_FAILURE','started_boottime_ns':T,'original_work_until_ns':W,'original_close_until_ns':C,'normal_app_Main_visible_window_Source_hardware_executed':False,'commands':[]}
def pin(p):
 p=Path(p)
 with p.open('rb')as f:h=hashlib.file_digest(f,'sha256').hexdigest()
 return {'bytes':p.stat().st_size,'sha256':h}
def complete(scope):return all(x['wait']is not None and x['eof']and x['log'].closed and x['child'].stdout.closed and(x['stderr_file']is None or x['stderr_file'].closed)and x['pidfd']is None for x in scope.children)
def command(argv,label,seconds=10,expected=(0,),env=None,reap_adopted=False):
 global serial,admission
 assert admission and all(complete(s)for s in scopes),'prior command custody unresolved'
 serial+=1;scope=Scope(D);scopes.append(scope);entry={'role':label};r['commands'].append(entry)
 try:
  code,data=scope.run(argv,f'{serial:03d}-'+label+'.log',min(W,now()+seconds*1_000_000_000),limit=4*1024**2,env=env);entry['returncode']=code
  assert code in expected,(label,code,data[-1000:]);return code,data
 finally:
  closure=close_owned(scope,C,reap_adopted=reap_adopted);entry['closure']=closure;entry['receipts']=scope.receipts;entry['handles_closed']=complete(scope)
  if not closure['closed']or not complete(scope):admission=False
  assert closure['closed']and complete(scope),(label,closure)
def provider(row):
 assert set(row)=={'path','bytes','sha256'}and type(row['path'])is str and type(row['bytes'])is int and row['bytes']>0 and re.fullmatch('[0-9a-f]{64}',row['sha256'])
 path=Path(row['path']);assert path.is_absolute()and stat.S_ISREG(path.stat().st_mode)
 assert pin(path)=={k:row[k]for k in ['bytes','sha256']};return path
inputs={};manifest_pin=None
try:
 m=json.loads(a.providers.read_text());manifest_pin=pin(a.providers)
 assert set(m)=={'schema','ELF_machine','roles','appearance'}and m['schema']==1 and type(m['ELF_machine'])is int and m['ELF_machine']in[62,183]
 assert set(m['roles'])=={'preview-actor','sounds-qml-input-client','pokelinux-audio','lifecycle-launcher','preview-qml-assert'}
 inputs={str(path):pin(path)for path in [*TOOLS.glob('*.py'),TOOLS/'ORACLES.json',TOOLS/'qml-input-client.cpp',TOOLS/'fixture/qml-positive-assert.rs',TOOLS.parent/'native-installed-services/owned.py']}
 r['test_source_inputs']=inputs;r['provider_manifest']=manifest_pin;r['ELF_machine']=m['ELF_machine'];r['selected_prebuilt_roles']={}
 appearance=m['appearance'];assert set(appearance)=={'directory','bytes','sha256'}and type(appearance['directory'])is str
 directory=Path(appearance['directory']);assert directory.is_absolute();qml=provider({'path':str(directory/'PhoneAppearance.qml'),'bytes':appearance['bytes'],'sha256':appearance['sha256']});assert stat.S_IMODE(qml.stat().st_mode)==0o644
 bins=O/'bins';bins.mkdir(mode=0o755)
 for name,row in m['roles'].items():
  source=provider(row)
  with source.open('rb')as f:header=f.read(20)
  assert header[:4]==b'\x7fELF'and struct.unpack('<H',header[18:20])[0]==m['ELF_machine'],name
  dest=bins/name;shutil.copyfile(source,dest);dest.chmod(0o755);os.chown(dest,0,0);r['selected_prebuilt_roles'][name]=row
  _,body=command(['/usr/bin/ldd','-r',str(dest)],name+'-ldd-r');assert b'not found'not in body and b'undefined symbol'not in body,name
 r['selected_role_loaders_passed']=True
 env=dict(os.environ,PHASE_WORK_UNTIL_NS=str(W),PHASE_CLOSE_UNTIL_NS=str(C),POKELINUX_PREVIEW_OUTPUT=str(O),POKELINUX_PREVIEW_RUNTIME=str(O/'runtime'),POKELINUX_PREVIEW_APPEARANCE=str(directory))
 code,_=command(['/usr/bin/python3','-B',str(TOOLS/'controller.py')],'one-real-unshown-QML-actor-PCM',150,(0,1),env,reap_adopted=True)
 inside=json.loads((O/'runtime/result.json').read_text());r['runtime_result']=pin(O/'runtime/result.json');r['runtime_exit']=code;r['runtime_cleanup_qualified']=inside.get('cleanup_qualified')
 assert code==0 and inside['cleanup_qualified']and inside['ready_for_compiled_assertion'],'runtime setup/custody must close before native assertion'
 assert r['commands'][-1]['closure']['wait_all_ECHILD']is True and all(complete(s)for s in scopes)
 r['native_assertion_admitted_only_after_runtime_cleanup']=True
 code,_=command([str(bins/'preview-qml-assert'),str(O/'runtime/result.json')],'compiled-positive-assertion-after-cleanup',10,(0,101))
 r['compiled_positive_assertion_exit']=code;r['classification']='ACTUAL_UNSHOWN_QML_PCM_PASS'if code==0 and inside['passed']else'ACTUAL_UNSHOWN_QML_PCM_BEHAVIOR_FAILURE';r['functional_passed']=bool(code==0 and inside['passed'])
except BaseException as e:r['error']=repr(e);r['traceback']=traceback.format_exc()
finally:
 r['cleanup_errors']=[]
 for scope in scopes:r['cleanup_errors'].extend(scope.close(C))
 r['owned_receipts']=[v for s in scopes for v in s.receipts];receipts=r['owned_receipts'];r['all_actual_wait_EOF_timely']=bool(receipts)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in receipts)
 r['all_command_handles_closed']=all(complete(s)for s in scopes);r['source_unchanged']=bool(inputs)and all(pin(path)==value for path,value in inputs.items());r['provider_manifest_unchanged']=manifest_pin is not None and pin(a.providers)==manifest_pin;r['completed_ns']=now();r['custody_qualified']=bool(not r['cleanup_errors']and r['all_actual_wait_EOF_timely']and r['all_command_handles_closed']and r.get('runtime_cleanup_qualified')and now()<C)
 r['passed']=bool(r.get('functional_passed')and r['custody_qualified']and r['source_unchanged']and r['provider_manifest_unchanged']);(O/'result.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({'passed':r['passed'],'classification':r['classification'],'custody_qualified':r['custody_qualified']}),flush=True)
raise SystemExit(0 if r['passed']else 1)
