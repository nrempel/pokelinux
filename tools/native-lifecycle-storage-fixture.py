#!/usr/bin/env python3
"""Actual SIGSTOP of owned storage process; never labelled a filesystem syscall stall."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import runpy
import select
import shutil
import signal
import socket
import subprocess
import sys
import time

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary-root',type=Path,default=Path('/bins'))
parser.add_argument('--results-dir',type=Path,default=Path('/evidence'))
parser.add_argument('--baseline-fixture',type=Path,default=Path('/baseline-fixture.py'))
options=parser.parse_args()
assert os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1' and os.geteuid() == 0
options.results_dir.mkdir(exist_ok=True,parents=True)
source=options.baseline_fixture.read_bytes()
body=ast.parse(source)
assert isinstance(body.body[-1],ast.Try)
body.body.pop()  # Preserve the18 case body; replace its program entry and cleanup only.
argv=sys.argv
sys.argv=[str(options.baseline_fixture),'--binary-root',str(options.binary_root),'--results-dir',str(options.results_dir/'baseline'),'--install-root','/opt/lifecycle-storage-qualification','--runtime-root','/run/lifecycle-storage-baseline','--policy-root','/var/lib/lifecycle-storage-baseline']
base={'__file__':str(options.baseline_fixture),'__name__':'lifecycle_baseline'}
exec(compile(body,str(options.baseline_fixture),'exec'),base)
sys.argv=argv
raw_daemon=base['daemon']
def daemon_ready():
    child=raw_daemon()
    base['until'](lambda:base['snapshot']().get('ready',True))
    return child
base['daemon']=daemon_ready
owned_clients=[]
stopped=[]
groups=[]

def record(name):
    groups.append(name);print('PASS '+name,flush=True)

def setup(case,mode=None,ready=True):
    install=Path('/opt/lifecycle-storage-qualification')
    install.mkdir(exist_ok=True,parents=True);install.chmod(0o755)
    for name in ['lifecycle-daemon','lifecycle-launcher','lifecycle-client']:
        if not (install/name).exists():
            shutil.copyfile(options.binary_root/name,install/name);(install/name).chmod(0o755)
    runtime=Path('/run/lifecycle-storage-'+case);policy=Path('/var/lib/lifecycle-storage-'+case)
    runtime.mkdir();runtime.chmod(0o755);policy.mkdir();policy.chmod(0o700)
    app=runtime/'one';app.mkdir();app.chmod(0o700);os.chown(app,10000,10000)
    profile=install/('profile-'+case+'.json')
    profile.write_text(json.dumps({'apps':[{'app':'one','uid':10000,'executable':str(install/'lifecycle-client'),'args':[str(runtime/'app.sock'),str(app/'driver.sock')]}]}))
    base['INSTALL']=install;base['RUNTIME']=runtime;base['POLICY']=policy
    command=[str(install/'lifecycle-daemon'),str(runtime),str(policy),str(profile),str(install/'lifecycle-launcher')]
    if mode:command.append('--storage-test='+mode)
    child=subprocess.Popen(command,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    base['owned'].append(child)
    base['until'](lambda:(runtime/'admin.sock').exists())
    if not ready:return child,runtime,policy
    base['until'](lambda:base['snapshot']().get('ready',True))
    assert base['admin']({'op':'locked','locked':False}) == {'result':'ok'}
    assert base['admin']({'op':'grant','app':'one','permission':'capture','allowed':True}) == {'result':'ok'}
    base['launch']('one')
    return child,runtime,policy

def worker_pid(child):
    children=Path(f'/proc/{child.pid}/task/{child.pid}/children').read_text().split()
    matches=[]
    for pid in children:
        cmd=Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
        if b'--storage-worker' in cmd:matches.append(int(pid))
    assert len(matches) == 1,matches
    return matches[0]

def pause(pid):
    handle=os.pidfd_open(pid)
    signal.pidfd_send_signal(handle,signal.SIGSTOP)
    stopped.append(handle)
    base['until'](lambda:'\nState:\tT' in Path(f'/proc/{pid}/status').read_text())
    return handle

def fast_admin(value):
    start=time.monotonic()
    with base['connect'](base['RUNTIME']/'admin.sock') as sock:
        sock.settimeout(.3)
        try: result=base['packet'](sock,value)
        except TimeoutError as exc:raise AssertionError('status/stop blocked behind SIGSTOP owned storage process') from exc
    assert time.monotonic()-start < .3
    return result

def caller(path,value):
    code='''import json,socket,sys
s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.settimeout(3);s.connect(sys.argv[1]);s.send(sys.argv[2].encode());print("sent",flush=True);print(s.recv(4096).decode(),flush=True)
'''
    p=subprocess.Popen([sys.executable,'-c',code,str(path),json.dumps(value)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    owned_clients.append(p)
    assert select.select([p.stdout],[],[],2)[0]
    assert p.stdout.readline().strip() == 'sent'
    return p

def response(p):
    out,err=p.communicate(timeout=3)
    assert p.returncode == 0,err
    return json.loads(out)

def wait_stopped(pid):
    base['until'](lambda:'\nState:\tT' in Path(f'/proc/{pid}/status').read_text())

def send_channel(channel,request):
    assert base['driver']('one',{'driver':'send','channel':channel,'request':request}) == {'driver_result':'sent'}

def receive_channel(channel):
    return base['driver']('one',{'driver':'receive','channel':channel})

def wait_reply(channel):
    return base['until'](lambda:(r if (r:=receive_channel(channel)).get('driver_result') != 'pending' else None))

try:
    if os.environ.get('LIFECYCLE_STORAGE_RUN_BASELINE','1') == '1':
        base['main']()
        assert len(base['observed']) == 18
        record('preserved18_installed_cases_with_loading_bootstrap_adapter')
    child,runtime,policy=setup('paused-stop')
    pid=worker_pid(child);handle=pause(pid)
    pending=caller(runtime/'admin.sock',{'op':'grant','app':'one','permission':'capture','allowed':False})
    time.sleep(.05)
    status=fast_admin({'op':'snapshot'})
    assert status['storage_occupied'] and status['storage_busy']
    assert fast_admin({'op':'stop'}) == {'result':'ok'}
    assert response(pending) == {'result':'unavailable'}
    child.wait(timeout=2);assert child.returncode == 0,child.stderr.read().decode()
    assert select.select([handle],[],[],1)[0]
    assert not Path(f'/proc/{pid}').exists()
    stopped.remove(handle);os.close(handle)
    record('SIGSTOP_storage_process_status_stop_responsive_and_reaped')
    replacement_runtime=Path('/run/lifecycle-storage-uncertain-restart');replacement_runtime.mkdir();replacement_runtime.chmod(0o755)
    base['RUNTIME']=replacement_runtime
    replacement=subprocess.Popen([str(base['INSTALL']/'lifecycle-daemon'),str(replacement_runtime),str(policy),str(base['INSTALL']/'profile-paused-stop.json'),str(base['INSTALL']/'lifecycle-launcher')],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    base['owned'].append(replacement)
    time.sleep(.15)
    assert replacement.poll() is not None and replacement.returncode != 0,'uncertain storage owner silently reopened after kill'
    record('uncertain_storage_owner_refuses_restart_after_reaping')
    child,runtime,policy=setup('busy-permit','pause:2')
    pid=worker_pid(child);handle=os.pidfd_open(pid);stopped.append(handle)
    first=base['prepare']();second=base['prepare']()
    send_channel(0,{'op':'admit','permit':first});wait_stopped(pid)
    assert len(json.loads((policy/'policy.json').read_text())['receipts']) == 1
    send_channel(1,{'op':'admit','permit':second})
    assert wait_reply(1) == {'result':'unavailable'}
    signal.pidfd_send_signal(handle,signal.SIGCONT);stopped.remove(handle);os.close(handle)
    assert wait_reply(0)['result'] == 'admitted_uncertain'
    send_channel(1,{'op':'admit','permit':second})
    assert wait_reply(1) == {'result':'denied'},'busy refusal left an owned permit reusable after another write completed'
    base['stop'](child)
    record('concurrent_same_principal_busy_refusal_consumes_permit')
    child,runtime,policy=setup('late-lifecycle','pause:2')
    pid=worker_pid(child);handle=os.pidfd_open(pid);stopped.append(handle)
    send_channel(0,{'op':'admit','permit':base['prepare']()});wait_stopped(pid)
    assert len(json.loads((policy/'policy.json').read_text())['receipts']) == 1
    assert fast_admin({'op':'locked','locked':True}) == {'result':'ok'}
    assert fast_admin({'op':'locked','locked':False}) == {'result':'ok'}
    signal.pidfd_send_signal(handle,signal.SIGCONT);stopped.remove(handle);os.close(handle)
    assert wait_reply(0) == {'result':'unavailable'}
    assert base['snapshot']()['degraded'] and base['snapshot']()['receipts'] == 1
    assert base['query']('one') == {'result':'unavailable'}
    base['stop'](child)
    record('synced_late_success_after_lifecycle_change_is_historical_only')
    child,runtime,policy=setup('late-deadline','pause:2')
    pid=worker_pid(child);handle=os.pidfd_open(pid);stopped.append(handle)
    send_channel(0,{'op':'admit','permit':base['prepare']()});wait_stopped(pid)
    assert fast_admin({'op':'snapshot'})['storage_busy']
    assert wait_reply(0) == {'result':'unavailable'}
    base['until'](lambda:base['snapshot']()['degraded'] and not base['snapshot']()['storage_occupied'])
    assert len(json.loads((policy/'policy.json').read_text())['receipts']) == 1
    assert select.select([handle],[],[],1)[0]
    assert not Path(f'/proc/{pid}').exists()
    assert base['query']('one') == {'result':'unavailable'}
    time.sleep(.1)
    assert not any(b'--storage-worker' in Path(f'/proc/{p}/cmdline').read_bytes() for p in Path(f'/proc/{child.pid}/task/{child.pid}/children').read_text().split())
    stopped.remove(handle);os.close(handle);base['stop'](child)
    record('absolute_deadline_revokes_before_kill_reap_and_no_successor')
    child,runtime,policy=setup('stale-source','stale:2')
    send_channel(0,{'op':'admit','permit':base['prepare']()})
    assert wait_reply(0) == {'result':'unavailable'}
    assert base['snapshot']()['degraded']
    assert len(json.loads((policy/'policy.json').read_text())['receipts']) == 1
    base['stop'](child)
    record('actual_worker_previous_token_reply_cannot_publish_current_admission')
    child,runtime,policy=setup('initial-read','pause:0',ready=False)
    pid=worker_pid(child);handle=os.pidfd_open(pid);stopped.append(handle);wait_stopped(pid)
    status=fast_admin({'op':'snapshot'})
    assert not status['ready'] and status['storage_busy'] and status['storage_occupied']
    assert fast_admin({'op':'stop'}) == {'result':'ok'}
    child.wait(timeout=2);assert child.returncode == 0,child.stderr.read().decode()
    assert select.select([handle],[],[],1)[0]
    assert not Path(f'/proc/{pid}').exists()
    stopped.remove(handle);os.close(handle)
    record('paused_initial_storage_load_keeps_status_stop_responsive')
    (options.results_dir/'result.json').write_text(json.dumps({'passed':True,'groups':groups,'baseline_fixture_sha256':hashlib.sha256(source).hexdigest(),'kernel':os.uname().release,'machine':os.uname().machine,'mechanism':'SIGSTOP process supervision; no held filesystem syscall'},indent=2)+'\n')
finally:
    for handle in stopped:
        try:signal.pidfd_send_signal(handle,signal.SIGCONT)
        except ProcessLookupError:pass
        os.close(handle)
    for p in base['owned']:
        if p.poll() is None:
            try:base['admin']({'op':'stop'})
            except BaseException:pass
            try:p.wait(timeout=3)
            except subprocess.TimeoutExpired:p.kill();p.wait(timeout=3)
    for p in owned_clients:
        if p.poll() is None:p.kill()
        p.communicate(timeout=3)
    if sys.exc_info()[0] is not None or not (options.results_dir/'result.json').exists():
        (options.results_dir/'result.json').write_text(json.dumps({'passed':False,'groups':groups,'mechanism':'SIGSTOP process supervision; no held filesystem syscall'},indent=2)+'\n')
