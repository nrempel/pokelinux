#!/usr/bin/env python3
"""Disposable installed kernel credential/process/SQLite qualification. No modem."""
import argparse,array,hashlib,json,os,shutil,signal,socket,sqlite3,struct,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--bins',type=Path,default=Path('/bins'));p.add_argument('--out',type=Path,default=Path('/evidence'));p.add_argument('--phase',default='green');p.add_argument('--retained-order',action='store_true');p.add_argument('--retained-fd',action='store_true');p.add_argument('--runtime-suffix',default='');a=p.parse_args()
assert os.getuid()==0 and os.environ.get('NATIVE_MESSAGES_ISOLATED')=='1'
assert not a.runtime_suffix or a.runtime_suffix=='foreign-fd'
suffix='-'+a.runtime_suffix if a.runtime_suffix else''
I=Path('/opt/pokelinux-messages'+suffix);R=Path('/run/messages'+suffix);S=Path('/var/lib/messages'+suffix);owned=[];writers=[];groups=[];retained_clients=[]
def connect(path):
 s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.settimeout(2);s.connect(str(path));return s
def call(path,value):
 with connect(path)as s:s.send(json.dumps(value,separators=(',',':')).encode());return json.loads(s.recv(4096))
def admin(v):return call(R/'admin.sock',v)
def driver(v):return call(R/'app'/'driver.sock',v)
def wait(f,timeout=3):
 end=time.monotonic()+timeout
 while time.monotonic()<end:
  try:
   v=f()
   if v:return v
  except (FileNotFoundError,ConnectionRefusedError):pass
  time.sleep(.01)
 raise AssertionError('bounded installed fixture wait')
def receive():return wait(lambda:(v if (v:=driver({'driver':'receive','channel':0})).get('driver_result')!='pending'else False))
def request(q,ttl=1000):return {'op':'messages_read','request':{'query':q,'ttl_ms':ttl}}
Q={'query':'conversations','after':None,'limit':8}
def send(q=Q,ttl=1000):assert driver({'driver':'send','channel':0,'request':request(q,ttl)})=={'driver_result':'sent'}
def record(s):groups.append(s);print('PASS '+s,flush=True)
def worker_pid():
 for x in Path('/proc').iterdir():
  if x.name.isdecimal():
   try:
    c=(x/'cmdline').read_bytes().split(b'\0')
    if b'--messages-worker'in c:return int(x.name)
   except (OSError,ProcessLookupError):pass
 raise AssertionError('owned Messages child missing')
def start():
 c=subprocess.Popen([str(I/'messages-service'),str(R),str(S/'policy'),str(I/'profile.json'),str(I/'lifecycle-launcher'),str(S/'raw'),str(S/'inbox')],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);owned.append(c)
 wait(lambda:(R/'admin.sock').exists());wait(lambda:admin({'op':'snapshot'}).get('ready'));wait(lambda:admin({'op':'messages_status'}).get('available'));return c
def launch():
 (R/'app'/'driver.sock').unlink(missing_ok=True)
 pid=os.fork()
 if pid==0:
  try:
   os.setgroups([]);os.setgid(1000);os.setuid(1000)
   assert call(R/'launch-messages.sock',{'op':'launch_messages'})=={'result':'ok'}
   os._exit(0)
  except BaseException:os._exit(4)
 assert os.waitpid(pid,0)[1]==0,'desktop credential launch-only ingress failed'
 wait(lambda:(R/'app'/'driver.sock').exists())
 assert admin({'op':'background','app':'org.omarchy.Messages','background':False})=={'result':'ok'}
def main():
 for d,m in [(I,0o755),(R,0o755),(S,0o700),(S/'policy',0o700),(R/'app',0o700)]:d.mkdir(parents=True);d.chmod(m)
 os.chown(R/'app',11010,11010)
 for b in ['messages-service','messages-seed-fixture','lifecycle-launcher','lifecycle-client']:
  shutil.copyfile(a.bins/b,I/b);(I/b).chmod(0o755)
 executable=str(I/'lifecycle-client');arguments=[str(R/'app.sock'),str(R/'app'/'driver.sock')]
 if a.retained_order or a.retained_fd:
  import sys
  script=I/'retained-fd-app.py';shutil.copyfile(Path(__file__).with_name('native-messages-retained-fd-app.py'),script);script.chmod(0o644)
  executable=str(Path(sys.executable).resolve());arguments=[str(script),*arguments]
 (I/'profile.json').write_text(json.dumps({'apps':[{'app':'org.omarchy.Messages','uid':11010,'executable':executable,'args':arguments}]}))
 subprocess.run([str(I/'messages-seed-fixture'),str(S/'raw')],check=True,timeout=3)
 c=start()
 if a.retained_order or a.retained_fd:
  import importlib.util
  spec=importlib.util.spec_from_file_location('retained_desktop_fixture',Path(__file__).with_name('native-messages-recovery-order-fixture.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  retained=module.RetainedDesktop(R/'launch-messages.sock');retained_clients.append(retained)
 launch()
 send();assert receive()=={'result':'denied'},'default lock did not deny current Messages read'
 assert admin({'op':'locked','locked':False})=={'result':'ok'}
 assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':True})=={'result':'ok'}
 send();page=receive();assert page.get('page')=='conversations' and len(page['items'])==1,'installed read did not release bounded conversation page'
 cid=page['items'][0]['id'];mq={'query':'messages','conversation':cid,'after_ordinal':0,'limit':2}
 send(mq);msg=receive();assert msg.get('page')=='messages' and len(msg['items'])==2 and msg['items'][0]['receipt']!=msg['items'][1]['receipt'],'local occurrence projection missing'
 ids=[m['receipt']for m in msg['items']];record('closed_window_background_projection_distinct_occurrences_and_current_read')
 if a.retained_order or a.retained_fd:
  detail=retained.verify_reopen(c,R,worker_pid,send,receive,admin,driver,mq,ids,a.retained_fd)
  a.out.mkdir(parents=True,exist_ok=True);(a.out/'retained-desktop-order.json').write_text(json.dumps(detail,indent=2)+'\n')
  record('retained_preconnected_desktop_before_old_read_'+('foreign_fd_dead_principal_recovery'if a.retained_fd else'exit_retirement_recovery'))
  retained.close()
  if a.phase=='recovery-only':return
 w=worker_pid();os.kill(w,signal.SIGSTOP);send(mq);wait(lambda:admin({'op':'messages_status'}).get('busy'))
 start_time=time.monotonic();assert admin({'op':'snapshot'}).get('ready');assert time.monotonic()-start_time<.5,'held storage blocked snapshots'
 assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':False})=={'result':'ok'}
 os.kill(w,signal.SIGCONT)
 assert receive()=={'result':'denied'},'late SQLite read published after live permission revocation'
 record('actual_SIGSTOP_read_revocation_and_responsive_snapshot')
 if a.phase=='red':return
 assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':True})=={'result':'ok'}
 for op in [{'op':'locked','locked':True},{'op':'suspended','suspended':True},{'op':'background','app':'org.omarchy.Messages','background':True}]:
  w=worker_pid();os.kill(w,signal.SIGSTOP);send(mq);wait(lambda:admin({'op':'messages_status'}).get('busy'));assert admin(op)=={'result':'ok'};os.kill(w,signal.SIGCONT);assert receive()=={'result':'denied'},'late read survived lifecycle transition'
  reset=dict(op);reset[next(k for k in op if k in ('locked','suspended','background'))]=False;assert admin(reset)=={'result':'ok'}
 record('lock_suspend_background_invalidate_held_publication')
 # Fill the real kernel reply queue without receiving already authorized pages.
 before=admin({'op':'messages_status'})['backpressure']
 blocked=False
 for _ in range(80):
  send(mq);time.sleep(.05)
  status=admin({'op':'messages_status'})
  if status['backpressure']>before:blocked=True;break
 assert blocked,'real nonblocking private reply did not reach EAGAIN'
 assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':False})=={'result':'ok'}
 saw_denied=False
 for _ in range(90):
  value=receive()
  if value=={'result':'denied'}:saw_denied=True;break
 assert saw_denied,'blocked reply retry published stale page after live revocation'
 assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':True})=={'result':'ok'}
 record('actual_socket_EAGAIN_revalidates_current_read_before_retry')
 # Closing the actual launched app abandons its held read; explicit desktop
 # reopen must recover pure local storage without a privileged restart command.
 old_worker=worker_pid();os.kill(old_worker,signal.SIGSTOP);send(mq);wait(lambda:admin({'op':'messages_status'}).get('busy'))
 try:driver({'driver':'exit'})
 except (OSError,json.JSONDecodeError):pass
 wait(lambda:admin({'op':'snapshot'}).get('principals')==0)
 launch()
 wait(lambda:admin({'op':'messages_status'}).get('available'))
 assert worker_pid()!=old_worker and not Path(f'/proc/{old_worker}').exists(),'reopen replaced a worker before actual reaping'
 send(mq);reopened=receive();assert [m['receipt']for m in reopened['items']]==ids,'reopen replayed or lost exact local occurrence history'
 record('held_read_actual_app_close_reap_desktop_reopen_local_recovery')
 w=worker_pid();os.kill(w,signal.SIGSTOP);send(mq,80);wait(lambda:admin({'op':'messages_status'}).get('busy'));time.sleep(.15)
 # Connection deadline closes the channel; no late read reply can become text.
 try:driver({'driver':'receive','channel':0})
 except (OSError,ConnectionError,json.JSONDecodeError):pass
 wait(lambda:not admin({'op':'messages_status'}).get('occupied'))
 assert admin({'op':'messages_restart'})=={'result':'ok'};wait(lambda:admin({'op':'messages_status'}).get('available'))
 # The driver can have exited after EOF. Revoke/reap and launch the same installed identity anew.
 admin({'op':'terminate','app':'org.omarchy.Messages'});wait(lambda:admin({'op':'snapshot'}).get('principals')==0)
 (R/'app'/'driver.sock').unlink(missing_ok=True);launch();send(mq);again=receive();assert [m['receipt']for m in again['items']]==ids,'explicit reaped local restart duplicated projection'
 record('deadline_reaped_local_restart_preserves_exact_history_without_remote_replay')
 # An external isolated real SQLite writer leaves a hot rollback journal.
 # It is test infrastructure, not an incoming producer or production mutation API.
 os.kill(worker_pid(),signal.SIGKILL);wait(lambda:not admin({'op':'messages_status'}).get('occupied'))
 dbpath=S/'inbox'/'inbox.sqlite3';before=hashlib.sha256(dbpath.read_bytes()).hexdigest();readfd,writefd=os.pipe();writer=os.fork()
 if writer==0:
  os.close(readfd)
  try:
   db=sqlite3.connect(dbpath);db.execute('PRAGMA cache_size=1');db.execute('BEGIN IMMEDIATE');db.execute('CREATE TABLE private_rollback_probe(payload BLOB)');db.execute('INSERT INTO private_rollback_probe VALUES(zeroblob(100000))');os.write(writefd,b'ready');time.sleep(10);os._exit(8)
  except BaseException:os._exit(9)
 writers.append(writer);os.close(writefd);assert os.read(readfd,5)==b'ready';os.close(readfd)
 journal=dbpath.with_name('inbox.sqlite3-journal');assert journal.stat().st_size>512 and any(journal.read_bytes()[:8]),'SQLite did not persist a hot journal';assert hashlib.sha256(dbpath.read_bytes()).hexdigest()!=before,'uncommitted SQLite pages never reached storage'
 os.kill(writer,signal.SIGKILL);assert os.waitpid(writer,0)[1]==signal.SIGKILL;writers.remove(writer)
 assert admin({'op':'messages_restart'})=={'result':'ok'};wait(lambda:admin({'op':'messages_status'}).get('available'));send(mq);again=receive();assert [m['receipt']for m in again['items']]==ids,'recovery changed exact local occurrences'
 assert not journal.exists(),'native SQLite did not finish journal recovery'
 with sqlite3.connect(dbpath)as check:assert check.execute("SELECT count(*) FROM sqlite_master WHERE name='private_rollback_probe'").fetchone()[0]==0
 record('actual_SIGKILL_hot_journal_recovery_and_exact_local_reprojection')
 with connect(R/'app.sock')as s:s.send(json.dumps(request(mq)).encode());assert json.loads(s.recv(4096))=={'result':'denied'}
 pid=os.fork()
 if pid==0:
  try:
   os.setgroups([]);os.setgid(11011);os.setuid(11011)
   assert call(R/'launch-messages.sock',{'op':'launch_messages'})=={'result':'denied'}
   with connect(R/'app.sock')as s:
    try:s.sendmsg([b'{"op":"query"}'],[(socket.SOL_SOCKET,socket.SCM_CREDENTIALS,struct.pack('3i',os.getpid(),11010,11011))])
    except PermissionError:os._exit(0)
   os._exit(8)
  except BaseException:os._exit(9)
 assert os.waitpid(pid,0)[1]==0,'kernel did not reject forged credential or foreign launch'
 record('actual_foreign_UID_unregistered_PID_and_forged_SCM_denied')
 assert admin({'op':'stop'})=={'result':'ok'};c.wait(timeout=4);assert c.returncode==0
 assert not (R/'messages-storage.sock').exists() and not (R/'app.sock').exists(),'owned cleanup failed'
 c=start();assert admin({'op':'locked','locked':False})=={'result':'ok'};launch();send(mq);again=receive();assert [m['receipt']for m in again['items']]==ids,'ordinary service restart did not recover qualified SQLite history'
 assert admin({'op':'stop'})=={'result':'ok'};c.wait(timeout=4);assert c.returncode==0
 record('ordinary_service_restart_exact_SQLite_recovery_no_new_arrivals')
def run():
 try:
  main();a.out.mkdir(parents=True,exist_ok=True);(a.out/'result.json').write_text(json.dumps({'passed':True,'groups':groups,'evidence':'installed Linux kernel credentials/process + real SQLite','remote_modem_qualified':False})+'\n')
 finally:
  for retained in retained_clients:retained.close()
  for writer in writers:
   try:os.kill(writer,signal.SIGKILL)
   except ProcessLookupError:pass
   os.waitpid(writer,0)
  for c in owned:
   if c.poll()is None:c.kill()
   c.wait(timeout=4)
if __name__=='__main__':run()
