#!/usr/bin/env python3
"""Actual watch functions, real task-stat ESRCH after parent wait, live ENOENT.

The host's held children file returns EOF after exit, so the scheduling hook
substitutes the same task's held stat node to obtain a real kernel ESRCH. It
does not claim to reproduce the ARM children-file kernel implementation.
The live negative uses a deliberately absent proc filename, not a claim that
the host kernel omits CONFIG_PROC_CHILDREN. No GUI, VM or vendor cleanup proof.
"""
import argparse,ast,hashlib,json,os,select,sys,time,traceback
from pathlib import Path
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent.parent/'native-installed-services'))
from owned import Scope,now
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()

def actual_functions(path,namespace):
 source=path.read_text();tree=ast.parse(source);nodes=[]
 for name in ['live_fd','watch']:
  found=[node for node in ast.walk(tree)if isinstance(node,ast.FunctionDef)and node.name==name]
  assert len(found)==1,(path,name);nodes.extend(found)
 exec(compile(ast.Module(body=nodes,type_ignores=[]),str(path),'exec'),namespace)
 return {node.name:hashlib.sha256(ast.get_source_segment(source,node).encode()).hexdigest()for node in nodes}

def run_case(source,folder,mode):
 folder.mkdir(mode=0o700);scope=Scope(folder);started=now();work=started+15_000_000_000;close=started+20_000_000_000
 control=folder/'control';control.mkdir(mode=0o700);watched={};events=[];item=None;child_pid=None
 report={'scope':'real procfs/pidfds and owned parent/child; injected scheduling or absent proc filename only','mode':mode,'passed':False,'source_sha256':sha(source)}
 def wait_json(name):
  while now()<work:
   scope.pump();scope.check();path=control/name
   if path.exists():
    try:return json.loads(path.read_text())
    except json.JSONDecodeError:pass
   if item is not None:assert scope.live(item),'fixture parent ended before expected fact'
   select.select([],[],[],.002)
  raise TimeoutError('original procfs control cutoff')
 namespace={}
 class ScheduledProcPath(type(Path())):
  def read_text(self,*args,**kwargs):
   if str(self)!=f'/proc/{child_pid}/task/{child_pid}/children':return super().read_text(*args,**kwargs)
   assert not events,'exact descendant-read boundary must occur once'
   row=watched[child_pid];fd=row['fd'];assert namespace['live_fd'](fd)
   event={'bound_descendant_pidfd_live_before':True};events.append(event)
   if mode=='vanished':
    held=os.open(self.with_name('stat'),os.O_RDONLY|os.O_CLOEXEC)
    try:
     event['actual_same_task_stat_opened_while_live']=True
     (control/'exit-child').write_text('exit\n');wait=wait_json('child-wait.json')
     assert wait['actual_original_parent_wait']and wait['returncode']==0 and wait['child_pid']==child_pid
     assert not namespace['live_fd'](fd)
     event['actual_original_parent_wait']=wait;event['bound_descendant_pidfd_dead_before_read']=True
     event['operation']='read-held-same-task-stat-after-original-parent-wait'
     try:
      os.read(held,65536)
     except OSError as error:
      event['actual_procfs_errno']=error.errno;event['actual_procfs_error_type']=type(error).__name__
      assert isinstance(error,ProcessLookupError)and error.errno==3
      raise
     raise AssertionError('kernel did not expose ESRCH at held same-task stat read')
    finally:os.close(held)
   # This really asks procfs for an absent sibling file while the same task lives.
   missing=Path(str(self)+'.intentionally-absent-control')
   try:missing.read_text()
   except OSError as error:
    event['actual_procfs_errno']=error.errno;event['actual_procfs_error_type']=type(error).__name__
    event['bound_descendant_pidfd_still_live']=namespace['live_fd'](fd)
    assert isinstance(error,FileNotFoundError)and error.errno==2 and event['bound_descendant_pidfd_still_live']
    raise
   raise AssertionError('expected real procfs ENOENT')
 try:
  argv=[sys.executable,str(HERE/'proc-parent-fixture.py'),str(control),str(work)]
  item=scope.start(argv,'fixture-parent.log',65536,cutoff=work,wait_cutoff=close)
  ready=wait_json('ready.json');assert ready['original_parent_pid']==item['child'].pid;child_pid=ready['child_pid']
  namespace.update({'os':os,'select':select,'Path':ScheduledProcPath,'now':now,'hashlib':hashlib,
    'watched':watched,'actor':item,'service':None,'compositor_pid':None,'compositor_pidfd':None,
    'app_pid':None,'app_pidfd':None,'normal_argv_bound':False,'startup':None,
    'FOCUS_LOG':folder/'no-focus-log','result':{},'S':{},'sha':sha,
    'bind_app_arguments':lambda *a:(_ for _ in ()).throw(AssertionError('no native app fixture'))})
  report['actual_function_hashes']=actual_functions(source,namespace)
  caught=None
  try:namespace['watch']()
  except OSError as error:caught={'type':type(error).__name__,'errno':error.errno}
  row=watched[child_pid];os.fstat(row['fd'])
  report.update(observed_error=caught,events=events,retained_watched_pidfd=True,
                watched_marked_dead=bool(row.get('dead')),bound_pidfd_live_after=namespace['live_fd'](row['fd']))
  assert len(events)==1
  if mode=='vanished':
   report['passed']=caught is None and row.get('dead')is True and not report['bound_pidfd_live_after']and events[0]['actual_procfs_errno']==3
  else:
   report['passed']=caught=={'type':'FileNotFoundError','errno':2}and report['bound_pidfd_live_after']and not row.get('dead')
 except BaseException as error:report['setup_or_control_error']=repr(error);report['traceback']=traceback.format_exc()
 finally:
  try:
   (control/'exit-child').write_text('exit\n');(control/'stop').write_text('stop\n')
   if item is not None:
    report['actual_fixture_parent_wait']=scope.finish(item,close);assert report['actual_fixture_parent_wait']==0
    wait=json.loads((control/'child-wait.json').read_text());assert wait['actual_original_parent_wait']and wait['returncode']==0
    report['original_child_wait']=wait
  except BaseException as error:report['fixture_close_error']=repr(error)
  report['scope_cleanup_errors']=scope.close(close);report['actual_parent_receipts']=scope.receipts
  for row in watched.values():os.close(row['fd'])
  report['all_actual_parent_wait_EOF']=len(scope.children)==len(scope.receipts)and all(r['actual_direct_parent_wait']and r['stdout_EOF']and r['wait_before_original_cutoff']and r['EOF_before_original_cutoff']for r in scope.receipts)
  report['passed']=bool(report['passed']and report['all_actual_parent_wait_EOF']and not report.get('setup_or_control_error')and not report.get('fixture_close_error')and not report['scope_cleanup_errors'])
 return report

def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--private',type=Path);parser.add_argument('--public',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
 args=parser.parse_args();args.output.mkdir(mode=0o700)
 report={'schema':1,'passed':False,'scope':__doc__,'kernel':os.uname().release,'cases':{},'no_empty_tree_or_vendor_cleanup_claim':True}
 for role,source in [('private',args.private),('public',args.public)]:
  if source is None:continue
  for mode in ['vanished','live-missing']:
   name=role+'-'+mode;report['cases'][name]=run_case(source,args.output/name,mode)
 report['passed']=all(row['passed']for row in report['cases'].values())
 (args.output/'result.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':report['passed'],'cases':{n:{k:r.get(k)for k in ['passed','observed_error','setup_or_control_error','fixture_close_error']}for n,r in report['cases'].items()}},indent=2))
 return 0 if report['passed']else 1

if __name__=='__main__':raise SystemExit(main())
