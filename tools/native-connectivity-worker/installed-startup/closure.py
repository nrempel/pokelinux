"""Failure closure for retained owned Scope children; never touches a VM/daemon."""
import os,signal,time
from pathlib import Path
from owned import now

def close_owned(scope,cutoff,reap_adopted=False):
    errors=[];adopted=[];signaled=set();echild=False
    # Every Popen was created by Scope with start_new_session=True. Signal only
    # its still-live pidfd and verified original session/process group.
    for item in reversed(scope.children):
        try:
            if item["child"].poll() is None:
                if item["pidfd"] is not None and scope.live(item):
                    if os.getpgid(item["child"].pid)==item["child"].pid:
                        os.killpg(item["child"].pid,signal.SIGKILL)
                    else:signal.pidfd_send_signal(item["pidfd"],signal.SIGKILL)
        except ProcessLookupError:pass
        except BaseException as e:errors.append(type(e).__name__+": "+str(e))
    while now()<cutoff:
        scope.pump()
        all_waited=True
        for item in scope.children:
            if item["wait"] is None:
                if item["child"].poll() is None:all_waited=False;continue
                code=item["child"].wait(timeout=0);observed=now();original=item["original_wait_cutoff_ns"]
                receipt={"role":item["name"],"pid":item["child"].pid,"argv":item["argv"],"returncode":code,"actual_direct_parent_wait":True,"waited_at_boottime_ns":observed,"original_cutoff_ns":original,"wait_before_original_cutoff":original is not None and observed<original,"cleanup_receipt":True}
                item["wait"]=receipt;scope.receipts.append(receipt)
        if reap_adopted and all_waited and not echild:
            try:
                while True:
                    pid,status=os.waitpid(-1,os.WNOHANG)
                    if pid==0:break
                    adopted.append({"pid":pid,"returncode":os.waitstatus_to_exitcode(status),"actual_adopted_parent_wait":True,"observed_ns":now()})
            except ChildProcessError:echild=True
            if not echild:
                # PID1/subreaper owns these remaining direct adopted children.
                # Open their pidfd before signaling; never signal a guessed PID.
                children=Path("/proc/self/task/"+str(os.getpid())+"/children")
                for raw in children.read_text().split():
                    pid=int(raw)
                    if pid in signaled:continue
                    try:
                        fd=os.pidfd_open(pid,os.O_NONBLOCK)
                        try:signal.pidfd_send_signal(fd,signal.SIGKILL)
                        finally:os.close(fd)
                        signaled.add(pid)
                    except ProcessLookupError:pass
        if all_waited and all(i["eof"] for i in scope.children) and (not reap_adopted or echild):break
        time.sleep(.002)
    for item in scope.children:
        if item["wait"] is not None:
            original=item["original_wait_cutoff_ns"]
            item["wait"].update({"stdout_EOF":item["eof"],"stdout_EOF_at_ns":item.get("eof_at_ns"),"EOF_before_original_cutoff":bool(item["eof"] and original is not None and item["eof_at_ns"]<original),"stdout_bytes":item["bytes"],"stdout_limit":item["limit"],"output_overflow":item["overflow"]})
    errors.extend(scope.close(cutoff))
    closed=all(i["wait"] is not None and i["eof"] and not i["overflow"] for i in scope.children) and not errors and (not reap_adopted or echild)
    return {"closed":closed,"errors":errors,"adopted_actual_waits":adopted,"adopted_pidfds_signaled":sorted(signaled),"wait_all_ECHILD":echild if reap_adopted else None}
