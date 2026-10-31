"""Fixed loader preflight and one owned case; no compilation or normal-mode substitution."""
from pathlib import Path
import hashlib,json,os,sys,traceback
sys.dont_write_bytecode=True;sys.path.insert(0,'/runner')
from owned import Scope,now
from closure import close_owned
S=json.loads(Path('/selection.json').read_bytes());case=sys.argv[1];assert case in S['cases'];O=Path('/results');O.mkdir(exist_ok=True);logs=O/'inside';logs.mkdir();scope=Scope(logs);T=now();W=min(T+30_000_000_000,int(os.environ['ORIGINAL_HOST_WORK_UNTIL_NS']));C=min(T+40_000_000_000,int(os.environ['ORIGINAL_HOST_CLOSE_UNTIL_NS']));r={'schema':1,'case':case,'passed':False,'original_work_until_ns':W,'original_close_until_ns':C,'no_build':True}
def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
try:
 probe=O/'.owned-output-probe';fd=os.open(probe,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600);os.write(fd,b'owned');os.fsync(fd);os.close(fd);probe.unlink();directory=os.open(O,os.O_RDONLY|os.O_DIRECTORY);os.fsync(directory);os.close(directory);r['actual_results_create_fsync_close_unlink']=True
 for name,pin in S['runner_files'].items():
  p=Path('/runner')/name;assert p.stat().st_size==pin['bytes']and sha(p)==pin['sha256'],name
 for name,pin in S['providers'].items():
  p=Path('/providers')/name;assert p.stat().st_size==pin['bytes']and sha(p)==pin['sha256'],name
  code,data=scope.run(['/usr/bin/ldd','-r',str(p)],'loader-'+name,W,limit=65536);assert code==0 and b'not found'not in data.lower()and b'undefined symbol'not in data.lower();r.setdefault('loader',{})[name]={'exit':code,'stdout_sha256':hashlib.sha256(data).hexdigest()}
 Path('/run/fixture').mkdir(mode=0o700,exist_ok=True);Path('/run/fixture').chmod(0o700)
 argv=['/usr/bin/python3','-B','/runner/controller.py','--case',case,'--main','/providers/pokelinux-connectivity-main','--worker','/providers/pokelinux-connectivity-service','--output','/results/case','--work-until-ns',str(W),'--close-until-ns',str(C)]
 code,data=scope.run(argv,'actual-normal-Main-worker-case',C,limit=1024*1024);r['actual_controller_exit']=code;r['actual_controller_argv']=argv;assert code==0
 d=json.loads((O/'case/case-result.json').read_bytes());assert d['custody_qualified']and not d.get('error');r['case_record_sha256']=sha(O/'case/case-result.json');r['case_runtime_closed']=True
except BaseException as e:r.update(error=repr(e),traceback=traceback.format_exc())
finally:
 r['cleanup']=close_owned(scope,C,reap_adopted=True);r['receipts']=scope.receipts
 r['passed']=bool(r.get('case_runtime_closed')and r['cleanup']['closed']and r['cleanup']['wait_all_ECHILD']and not r['cleanup']['adopted_actual_waits']and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']for v in scope.receipts)and now()<C)
 (O/'inside-result.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({'passed':r['passed'],'case':case,'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['passed']else 2)
