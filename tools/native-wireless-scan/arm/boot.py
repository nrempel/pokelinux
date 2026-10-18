"""Direct-QEMU original wireless case; readonly393 chain; disposable overlay never exported."""
from pathlib import Path
import hashlib,json,os,sys,traceback
sys.dont_write_bytecode=True
from owned import Scope,now
R=Path('/results');S=json.loads((R/'selection.json').read_text());T=now();W=min(T+780_000_000_000,int(os.environ['ORIGINAL_HOST_WORK_UNTIL_NS']));C=min(T+800_000_000_000,int(os.environ['ORIGINAL_HOST_CLOSE_UNTIL_NS']))
scope=Scope(R);r={'schema':1,'passed':False,'original_QEMU_until_ns':W,'original_close_until_ns':C,'normal_app_or_window_executed':False}
def sha(p):
 with Path(p).open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def run(argv,label,limit=1048576):
 assert all(v['wait']is not None and v['eof']for v in scope.children)
 try:code,data=scope.run(argv,label,W,limit=limit)
 except BaseException:scope.close(C);raise
 assert all(v['wait']is not None and v['eof']for v in scope.children)
 assert code==0 and now()<W,(label,code,data[-1200:]);return data.decode()
try:
 overlay=Path('/tmp/arm-hwsim-original-overlay.qcow2');assert not overlay.exists()
 run(['/usr/bin/qemu-img','create','-f','qcow2','-F','qcow2','-b','/vm/prepared-joined-layer.qcow2',str(overlay)],'qemu-img.log',65536)
 chain=json.loads(run(['/usr/bin/qemu-img','info','--backing-chain','--output=json',str(overlay)],'image-chain.log',65536))
 paths=['/vm/prepared-joined-layer.qcow2','/vm/prepared-gain-layer.qcow2','/vm/base-preview-layer.qcow2','/vm/prepared-backend-layer.qcow2','/vm/prepared-services-layer.qcow2','/vm/root.img']
 assert len(chain)==len(paths)+1 and all(chain[i]['full-backing-filename']==p for i,p in enumerate(paths))and chain[-1]['format']=='raw'
 r['actual_readonly_chain']=chain;argv=S['QEMU_argv']
 assert argv[0]=='qemu-system-aarch64'and argv[argv.index('-accel')+1]=='tcg,thread=multi'and argv[argv.index('-smp')+1]=='4'and argv[argv.index('-m')+1]=='3072'
 assert argv[argv.index('-nic')+1]=='none'and argv[argv.index('-monitor')+1]=='none'and argv[argv.index('-display')+1]=='none'
 assert all(x not in argv for x in ['-qmp','-S','-dtb','-enable-kvm'])
 run(argv,'console.log',8388608);r['actual_QEMU_wait_exit']=0;r['actual_QEMU_argv']=argv
 g=json.loads((R/'result.json').read_text());assert g['fixture_custody_passed']and g['all_commands_actual_wait_EOF']and g['all_command_handles_closed']and g['normal_poweroff_command_closed']and g['original393_versions_unchanged_after_exact3']
 assert g['desired_passed']and not g['behavior_RED_qualified']and g['desired_assertion_calls']==1
 assert g['consumer_review']['passed']and not g['consumer_review']['behavior_RED_qualified']and g['consumer_review']['prerequisites_and_cleanup_qualified']
 worker=[v for v in g['receipts']if v['role']==g['native_worker_role']]
 assert len(worker)==1 and worker[0]['returncode']==0 and worker[0]['argv']==g['native_worker_exact_argv']
 r['desired_passed']=True;r['guest_behavior_RED_qualified']=False;r['guest_desired_GREEN_qualified']=True
 r['guest_result_sha256']=sha(R/'result.json');r['fixture_contract_passed']=True
except BaseException as error:r.update(error=repr(error),traceback=traceback.format_exc())
finally:
 r['cleanup_errors']=scope.close(C);r['receipts']=scope.receipts
 r['actual_same_parent_QEMU_custody']=len(scope.children)==len(scope.receipts)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and v['returncode']==0 for v in scope.receipts)
 r['all_QEMU_command_handles_closed']=all(v['log'].closed and v['child'].stdout.closed and v['pidfd']is None for v in scope.children)
r['disposable_overlay_not_exported']=True
r['completed_ns']=now()
r['passed']=bool(r.get('fixture_contract_passed') and r['actual_same_parent_QEMU_custody'] and r['all_QEMU_command_handles_closed'] and not r['cleanup_errors'] and now()<C)
(R/'boot-result.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({'passed':r['passed'],'error':r.get('error')}),flush=True)
raise SystemExit(0 if r['passed'] and r.get('guest_desired_GREEN_qualified') else 2)
