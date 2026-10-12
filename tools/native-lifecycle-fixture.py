#!/usr/bin/env python3
"""Installed Linux boundary qualification; isolated container paths only."""
import argparse
import array
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import struct
import subprocess
import sys
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary-root',type=Path,default=Path('/bins'))
parser.add_argument('--results-dir',type=Path,default=Path('/evidence'))
parser.add_argument('--install-root',type=Path,default=Path('/opt/pokelinux-lifecycle'))
parser.add_argument('--runtime-root',type=Path,default=Path('/run/native-lifecycle'))
parser.add_argument('--policy-root',type=Path,default=Path('/var/lib/native-lifecycle'))
options = parser.parse_args()
assert os.getuid() == os.geteuid() == 0 and os.environ.get('NATIVE_LIFECYCLE_ISOLATED') == '1', 'run only in the explicit disposable container or guest'
INSTALL = options.install_root
RUNTIME = options.runtime_root
POLICY = options.policy_root
OUT = options.results_dir
owned = []
observed = []

def packet(sock, value):
    sock.send(json.dumps(value, separators=(',', ':')).encode())
    data = sock.recv(4096)
    return json.loads(data)

def connect(path):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    s.settimeout(2)
    s.connect(str(path))
    return s

def until(function, timeout=3):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        try:
            result = function()
            if result:
                return result
        except (FileNotFoundError, ConnectionRefusedError):
            pass
        time.sleep(.01)
    raise AssertionError('bounded fixture wait expired')

def admin(value):
    with connect(RUNTIME / 'admin.sock') as s:
        return packet(s, value)

def driver(app, value):
    with connect(RUNTIME / app / 'driver.sock') as s:
        return packet(s, value)

def query(app):
    return driver(app, {'op': 'query'})

def prepare(app='one', ttl=5000):
    r = driver(app, {'op': 'prepare', 'permission': 'capture', 'ttl_ms': ttl})
    assert r['result'] == 'prepared', r
    return r['permit']

def admit(app, permit):
    return driver(app, {'op': 'admit', 'permit': permit})

def launch(app):
    result = subprocess.run([str(INSTALL / 'lifecycle-launcher'), str(RUNTIME), app], capture_output=True, timeout=3)
    assert result.returncode == 0, result.stderr.decode()
    until(lambda: (RUNTIME / app / 'driver.sock').exists())
    assert query(app) == {'result': 'ok'}

def snapshot():
    return admin({'op': 'snapshot'})

def daemon():
    child = subprocess.Popen([str(INSTALL / 'lifecycle-daemon'), str(RUNTIME), str(POLICY), str(INSTALL / 'profile.json'), str(INSTALL / 'lifecycle-launcher')], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    owned.append(child)
    until(lambda: (RUNTIME / 'admin.sock').exists())
    return child

def stop(child):
    try:
        admin({'op': 'stop'})
    except (OSError, AssertionError):
        pass
    child.wait(timeout=4)
    assert child.returncode == 0, child.stderr.read().decode()

def record(name):
    observed.append(name)
    print('PASS ' + name, flush=True)

def main():
    for p,mode in [(INSTALL,0o755),(RUNTIME,0o755),(POLICY,0o700)]:
        p.mkdir(parents=True,exist_ok=True); p.chmod(mode)
    for name in ['lifecycle-daemon','lifecycle-launcher','lifecycle-client']:
        shutil.copyfile(options.binary_root / name, INSTALL / name)
        (INSTALL / name).chmod(0o755)
    apps=[]
    for app,uid in [('one',10000),('two',10001)]:
        d=RUNTIME / app; d.mkdir(); d.chmod(0o700); os.chown(d,uid,uid)
        apps.append({'app':app,'uid':uid,'executable':str(INSTALL/'lifecycle-client'),'args':[str(RUNTIME/'app.sock'),str(d/'driver.sock')]})
    (INSTALL/'profile.json').write_text(json.dumps({'apps':apps}))
    child=daemon()
    assert snapshot()['locked'] is True
    assert admin({'op':'locked','locked':False}) == {'result':'ok'}
    for app in ['one','two']:
        assert admin({'op':'grant','app':app,'permission':'capture','allowed':True}) == {'result':'ok'}
        launch(app)
    # Runtime red: a prepared operation must lose authority even after unlocking.
    old=prepare()
    assert admin({'op':'locked','locked':True}) == {'result':'ok'}
    assert admin({'op':'locked','locked':False}) == {'result':'ok'}
    assert admit('one',old) == {'result':'denied'}, 'stale permit survived actual lock/unlock lifecycle transition'
    record('real_process_lock_generation_fence')
    for field,value in [('pid',os.getpid()),('uid',10001),('app','two')]:
        assert driver('one',{'op':'query',field:value}) == {'result':'denied'}
    with connect(RUNTIME/'app.sock') as s:
        assert packet(s,{'op':'query'}) == {'result':'denied'}
    record('claimed_pid_uid_app_and_unregistered_kernel_principal_denied')
    # Kernel itself rejects forged SCM_CREDENTIALS from a separate unprivileged child.
    pid=os.fork()
    if pid == 0:
        try:
            os.setgroups([]); os.setgid(10001); os.setuid(10001)
            with connect(RUNTIME/'app.sock') as s:
                try:
                    s.sendmsg([b'{"op":"query"}'],[(socket.SOL_SOCKET,socket.SCM_CREDENTIALS,struct.pack('3i',os.getpid(),10000,10001))])
                except PermissionError:
                    try:
                        s.sendmsg([b'{"op":"query"}'],[(socket.SOL_SOCKET,socket.SCM_CREDENTIALS,struct.pack('3i',os.getppid(),10001,10001))])
                    except PermissionError:
                        os._exit(0)
            os._exit(4)
        except BaseException:
            os._exit(5)
    assert os.waitpid(pid,0)[1] == 0
    record('kernel_rejected_real_child_credential_forgery')
    # Child process driver can export the actual connection; sender identity must change.
    with connect(RUNTIME/'one'/'driver.sock') as s:
        s.send(b'{"driver":"export_socket"}')
        _,anc,flags,_=s.recvmsg(4096,socket.CMSG_SPACE(4))
        assert not flags & socket.MSG_CTRUNC
        fds=array.array('i'); fds.frombytes(anc[0][2][:4])
        fd=fds[0]
    delegated=os.fork()
    if delegated == 0:
        try:
            os.setgroups([]); os.setgid(10000); os.setuid(10000)
            with socket.socket(fileno=fd) as s:
                s.settimeout(2)
                result=packet(s,{'op':'query'})
                os._exit(0 if result == {'result':'denied'} else 6)
        except BaseException:
            os._exit(7)
    os.close(fd)
    assert os.waitpid(delegated,0)[1] == 0
    record('inherited_actual_socket_cannot_delegate_pid_authority')
    permit=prepare('one')
    assert admit('two',permit) == {'result':'denied'}
    assert admit('one',permit)['result'] == 'admitted_uncertain'
    assert admit('one',permit) == {'result':'denied'}
    record('cross_principal_permit_refused_and_admission_once')
    for operation,enter,leave in [('background',{'app':'one','background':True},{'app':'one','background':False}),('suspended',{'suspended':True},{'suspended':False})]:
        p=prepare()
        assert admin(dict(op=operation,**enter)) == {'result':'ok'}
        assert driver('one',{'op':'prepare','permission':'capture','ttl_ms':5000}) == {'result':'denied'}
        assert admin(dict(op=operation,**leave)) == {'result':'ok'}
        assert admit('one',p) == {'result':'denied'}
    record('background_suspend_and_resume_fence')
    p=prepare(); assert admin({'op':'grant','app':'one','permission':'capture','allowed':False}) == {'result':'ok'}
    assert admit('one',p) == {'result':'denied'}
    assert driver('one',{'op':'prepare','permission':'capture','ttl_ms':5000}) == {'result':'denied'}
    assert admin({'op':'grant','app':'one','permission':'capture','allowed':True}) == {'result':'ok'}
    p=prepare(ttl=20); time.sleep(.05); assert admit('one',p) == {'result':'denied'}
    record('durable_revocation_and_absolute_deadline')
    outstanding=[prepare() for _ in range(4)]
    assert driver('one',{'op':'prepare','permission':'capture','ttl_ms':5000}) == {'result':'denied'}
    assert admin({'op':'locked','locked':True}) == {'result':'ok'}
    assert admin({'op':'locked','locked':False}) == {'result':'ok'}
    assert snapshot()['permits'] == 0
    record('per_principal_capacity_and_prompt_snapshot')
    with connect(RUNTIME/'one'/'driver.sock') as socket_export:
        socket_export.send(b'{"driver":"export_socket"}')
        _,anc,_,_=socket_export.recvmsg(4096,socket.CMSG_SPACE(4))
        retained_fds=array.array('i'); retained_fds.frombytes(anc[0][2][:4])
    retained_connection=socket.socket(fileno=retained_fds[0]); retained_connection.settimeout(2)
    p=prepare(); assert admin({'op':'terminate','app':'one'}) == {'result':'ok'}
    until(lambda:snapshot()['principals'] == 1)
    assert packet(retained_connection,{'op':'query'}) == {'result':'denied'}
    retained_connection.close()
    (RUNTIME/'one'/'driver.sock').unlink()
    launch('one')
    assert admit('one',p) == {'result':'denied'}
    record('real_death_reaping_and_fresh_principal_generation')
    receipt=admit('one',prepare())
    assert receipt['result'] == 'admitted_uncertain'
    assert admin({'op':'grant','app':'one','permission':'capture','allowed':False}) == {'result':'ok'}
    retained=snapshot()['receipts']; assert retained == 2
    # A duplicate service cannot open the durable owner lock or launch successors.
    duplicate=subprocess.run([str(INSTALL/'lifecycle-daemon'),str(RUNTIME),str(POLICY),str(INSTALL/'profile.json'),str(INSTALL/'lifecycle-launcher')],capture_output=True,timeout=3)
    assert duplicate.returncode != 0
    record('single_durable_owner_and_revocation_retains_uncertainty')
    p=prepare('two')
    stop(child)
    for app in ['one','two']:
        (RUNTIME/app/'driver.sock').unlink()
    child=daemon()
    assert snapshot()['receipts'] == retained
    assert snapshot()['locked'] is True
    assert admin({'op':'locked','locked':False}) == {'result':'ok'}
    launch('one'); launch('two')
    assert driver('one',{'op':'prepare','permission':'capture','ttl_ms':5000}) == {'result':'denied'}
    assert admit('two',p) == {'result':'denied'}
    record('restart_policy_uncertainty_persist_and_old_permit_fenced')
    while snapshot()['receipts'] < 128:
        assert admit('two',prepare('two'))['result'] == 'admitted_uncertain'
    assert admit('two',prepare('two')) == {'result':'unavailable'}
    record('durable_uncertain_receipt_capacity_never_evicts_or_retries')
    # Idle app connections cannot occupy privileged snapshot/stop capacity.
    idle=[connect(RUNTIME/'app.sock') for _ in range(10)]
    time.sleep(.1)
    assert snapshot()['principals'] == 2
    for sock in idle: sock.close()
    record('reserved_control_capacity_with_idle_app_clients')
    # Oversized data and unexpected FDs close only the offending connection.
    with connect(RUNTIME/'app.sock') as oversized:
        oversized.send(b'x'*4097)
        try: assert oversized.recv(4096) == b''
        except ConnectionResetError: pass
    with connect(RUNTIME/'app.sock') as rights:
        with open('/dev/null','rb') as fd_fixture:
            rights.sendmsg([b'{"op":"query"}'],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[fd_fixture.fileno()]*3))])
            try: assert rights.recv(4096) == b''
            except ConnectionResetError: pass
    assert snapshot()['principals'] == 2
    record('truncated_packet_and_unexpected_descriptors_fail_closed')
    # Durable IO fails before return; the same owner never silently recovers.
    (POLICY/'policy.pending').write_text('owned failure fixture')
    assert admin({'op':'grant','app':'two','permission':'capture','allowed':False}) == {'result':'unavailable'}
    assert driver('two',{'op':'prepare','permission':'capture','ttl_ms':5000}) == {'result':'unavailable'}
    assert snapshot()['degraded'] is True
    (POLICY/'policy.pending').unlink()
    assert admin({'op':'grant','app':'two','permission':'capture','allowed':True}) == {'result':'unavailable'}
    record('durable_failure_latches_unavailable_without_reentry')
    (POLICY/'policy.pending').write_text('owned unreconciled pending fixture')
    start=time.monotonic(); stop(child); assert time.monotonic()-start < 2
    assert not list(RUNTIME.glob('*.sock'))
    record('owned_children_stopped_reaped_and_socket_cleanup')
    pending_restart=subprocess.Popen([str(INSTALL/'lifecycle-daemon'),str(RUNTIME),str(POLICY),str(INSTALL/'profile.json'),str(INSTALL/'lifecycle-launcher')],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    owned.append(pending_restart)
    time.sleep(.1)
    assert pending_restart.poll() is not None and pending_restart.returncode != 0, 'unreconciled durable pending file allowed owner restart'
    record('unreconciled_durable_pending_blocks_restart')
    stored=json.loads((POLICY/'policy.json').read_text())
    assert all(isinstance(r,str) and len(r) == 49 for r in stored['receipts'])
    assert set(stored) == {'grants','receipts'}
    record('bounded_private_receipts_contain_no_pid_uid_app_payload')
    OUT.mkdir(exist_ok=True)
    (OUT/'result.json').write_text(json.dumps({'passed':True,'groups':observed,'kernel':os.uname().release,'machine':os.uname().machine},indent=2)+'\n')

try:
    main()
finally:
    for p in owned:
        if p.poll() is None:
            try: admin({'op':'stop'})
            except BaseException: pass
            try: p.wait(timeout=3)
            except subprocess.TimeoutExpired: p.kill(); p.wait(timeout=3)
    OUT.mkdir(exist_ok=True)
    if not (OUT/'result.json').exists():
        (OUT/'result.json').write_text(json.dumps({'passed':False,'groups':observed},indent=2)+'\n')
