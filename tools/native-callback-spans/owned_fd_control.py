#!/usr/bin/env python3
"""Real sequential Scope.run FD pressure at fixed RLIMIT64; no VM or GUI."""
import argparse,errno,fcntl,hashlib,importlib.util,json,os,resource,subprocess,sys,traceback
from pathlib import Path
sys.dont_write_bytecode=True

def fd_count():
 count=0
 for fd in range(64):
  try:fcntl.fcntl(fd,fcntl.F_GETFD);count+=1
  except OSError as error:
   if error.errno!=errno.EBADF:raise
 return count

def handles_open(item):
 if item['log'].closed or item['child'].stdout.closed or item['pidfd']is None:return False
 os.fstat(item['pidfd']);return True

def handles_retired(item):
 stream=item['child'].stdin
 return item['log'].closed and item['child'].stdout.closed and item['pidfd']is None and(stream is None or stream.closed)

def custody(receipts):
 return all(r['actual_direct_parent_wait']and r['stdout_EOF']and r['wait_before_original_cutoff']and r['EOF_before_original_cutoff']for r in receipts)

def pressure(module,folder):
 folder.mkdir();scope=module.Scope(folder);work=module.now()+30_000_000_000;close=work+5_000_000_000
 result={'passed':False,'requested_commands':48,'completed_commands':0,'samples':[]};peer=None
 try:
  peer=scope.start([sys.executable,'-c','import sys;print("peer-ready",flush=True);sys.stdin.buffer.read(1)'],'async-peer.log',128,stdin=subprocess.PIPE,cutoff=work)
  initial=fd_count();retired=True;expected={}
  for index in range(48):
   wanted=7 if index==5 else 0
   code,data=scope.run([sys.executable,'-c','import sys;print(sys.argv[1]);raise SystemExit(int(sys.argv[2]))',str(index),str(wanted)],f'command-{index:02d}.log',min(work,module.now()+3_000_000_000),stdin=subprocess.PIPE if index==0 else None)
   assert code==wanted and data==(str(index)+'\n').encode()
   item=scope.children[-1];expected[item['child'].pid]=wanted
   retired &= handles_retired(item)
   assert scope.live(peer)and handles_open(peer)and not peer['child'].stdin.closed
   result['completed_commands']+=1;result['samples'].append(fd_count())
  result.update(initial_with_async_peer=initial,max_after_run=max(result['samples']),all_sync_handles_retired=retired,
                async_peer_kept_live_during_runs=True,returned_nonzero_code_preserved=True)
  result['behavior_passed']=retired and max(result['samples'])<=initial and all(r['returncode']==expected[r['pid']]for r in scope.receipts)
 except OSError as error:result['actual_os_error']={'type':type(error).__name__,'errno':error.errno};result['exception_traceback']=traceback.format_exc()
 except BaseException as error:result['control_error']=repr(error);result['exception_traceback']=traceback.format_exc()
 finally:
  try:
   if peer is not None:
    peer['child'].stdin.close();assert scope.finish(peer,work)==0
    result['async_finish_handles_retained']=handles_open(peer)
  except BaseException as error:result['async_finish_error']=repr(error)
  result['cleanup_errors']=scope.close(close);result['receipts']=scope.receipts
  result['exact_wait_EOF_before_cutoffs']=custody(scope.receipts)
  result['fd_count_after_close']=fd_count()
  result['passed']=bool(result.get('behavior_passed')and result.get('async_finish_handles_retained')and result['exact_wait_EOF_before_cutoffs']and not result['cleanup_errors']and not result.get('control_error')and not result.get('async_finish_error'))
 return result

def failed_run(module,folder,mode):
 folder.mkdir();scope=module.Scope(folder);work=module.now()+5_000_000_000;close=work+3_000_000_000
 result={'passed':False,'mode':mode}
 try:
  if mode=='live-overflow':
   argv=[sys.executable,'-c','import os,time;os.write(1,b"overflow");time.sleep(20)'];limit=1
  else:
   argv=[sys.executable,'-c','import os,sys;os.unlink(sys.argv[1]);print("data")',str(folder/'command.log')];limit=128
  caught=None
  try:scope.run(argv,'command.log',work,limit=limit)
  except BaseException as error:caught=error
  item=scope.children[-1];retained=handles_open(item)
  if mode=='live-overflow':
   result['behavior_passed']=isinstance(caught,AssertionError)and scope.live(item)and item['wait']is None and retained
   result['live_child_not_detached']=scope.live(item)
  else:
   result['behavior_passed']=isinstance(caught,FileNotFoundError)and item['wait']is not None and item['eof']and retained
  result.update(observed_error=type(caught).__name__ if caught is not None else None,handles_retained_for_close=retained)
 except BaseException as error:result['control_error']=repr(error);result['traceback']=traceback.format_exc()
 finally:
  result['cleanup_errors']=scope.close(close);result['receipts']=scope.receipts
  result['exact_wait_EOF_before_cutoffs']=custody(scope.receipts)
  result['passed']=bool(result.get('behavior_passed')and result['exact_wait_EOF_before_cutoffs']and not result['cleanup_errors']and not result.get('control_error'))
 return result

def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--owned',required=True,type=Path);parser.add_argument('--output',required=True,type=Path)
 args=parser.parse_args();args.output.mkdir(mode=0o700)
 spec=importlib.util.spec_from_file_location('scope_under_test',args.owned);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 before=resource.getrlimit(resource.RLIMIT_NOFILE);assert before[0]>=64 and before[1]>=64
 assert all(int(fd)<64 for fd in os.listdir('/proc/self/fd'))
 resource.setrlimit(resource.RLIMIT_NOFILE,(64,64))
 start=fd_count();report={'schema':1,'scope':__doc__,'passed':False,'owned_source_sha256':hashlib.sha256(args.owned.read_bytes()).hexdigest(),'original_RLIMIT_NOFILE':list(before),'test_RLIMIT_NOFILE':list(resource.getrlimit(resource.RLIMIT_NOFILE)),'limits_widened':False,'initial_fd_count':start,'cases':{}}
 report['cases']['sequential-with-live-async-peer']=pressure(module,args.output/'pressure')
 for mode in ['live-overflow','data-read-failure']:report['cases'][mode]=failed_run(module,args.output/mode,mode)
 report['final_fd_count']=fd_count();report['limit_unchanged']=resource.getrlimit(resource.RLIMIT_NOFILE)==(64,64)
 report['passed']=all(r['passed']for r in report['cases'].values())and report['final_fd_count']==start and report['limit_unchanged']
 (args.output/'result.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':report['passed'],'cases':{n:{k:r.get(k)for k in ['passed','completed_commands','actual_os_error','control_error','cleanup_errors']}for n,r in report['cases'].items()},'fd_before':start,'fd_after':report['final_fd_count']},indent=2))
 return 0 if report['passed']else 1

if __name__=='__main__':raise SystemExit(main())
