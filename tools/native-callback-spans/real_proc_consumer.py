#!/usr/bin/env python3
"""Frozen independent real-Linux-proc boundary consumer; no native Source or VM."""
import argparse,hashlib,importlib.util,json,os,re,select,stat,sys,time
from pathlib import Path

SPACING=50_000_000
CAP=524288
UNTIL=None

def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def active():
 if UNTIL is not None and now()>=UNTIL:raise TimeoutError('original consumer5s work bound')
def starttime(pid):return(Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()[19]
def oracle(pid,fd):
 p=Path('/proc')/str(pid);target=os.readlink(p/'fd'/str(fd));s=os.stat(p/'fd'/str(fd))
 with(p/'fdinfo'/str(fd)).open('rb')as f:info=f.read(4097)
 return {'target':target,'target_sha256':hashlib.sha256(os.fsencode(target)).hexdigest(),'stat':{'device':s.st_dev,'inode':s.st_ino,'rdev':s.st_rdev,'type':stat.S_IFMT(s.st_mode),'mode':stat.S_IMODE(s.st_mode),'UID':s.st_uid,'GID':s.st_gid},'fdinfo':info[:4096].decode(),'fdinfo_truncated':len(info)>4096}
def check_record(observer):
 r=observer.record;encoded=json.dumps(r,sort_keys=True,separators=(',',':')).encode();assert len(encoded)<=CAP
 assert r['optional_diagnostic_only']is True and r['Source_channel_accessed']is False and r['fd_lifetime_continuity_proven']is False
 assert len(r['photos'])<=2
 for p in r['photos']:
  assert len(p['rows'])<=64
  for row in p['rows']:
   if 'fdinfo'in row:assert len(row['fdinfo'].encode())<=4096
 return r

def photo(observer,ordinal):
 r=check_record(observer);p=r['photos'][ordinal-1];assert p['ordinal']==ordinal and p['PID']==os.getpid()
 assert p['proc_identity_verified']and p['pidfd_live_before']and p['pidfd_live_after']
 assert p['proc_starttime_before']==p['proc_starttime_after']==starttime(os.getpid())
 assert p['started_boottime_ns']<=p['completed_boottime_ns']<=now()
 return p

def row_for(p,fd):
 matches=[r for r in p['rows']if r.get('fd')==fd];assert len(matches)==1;return matches[0]
def compare(p,fd):
 actual=row_for(p,fd);expected=oracle(os.getpid(),fd)
 assert actual['stable_during_read']and actual['stat']==expected['stat']and actual['target_sha256']==expected['target_sha256']
 assert actual['fdinfo_truncated']==expected['fdinfo_truncated']
 if expected['target'].startswith(('pipe:[','socket:[','anon_inode:')):assert actual['target']==expected['target']
 else:assert actual['target']is None
 return actual

def later(observer):
 due=observer.record['second_due_boottime_ns'];active();time.sleep(max(0,(due-now())/1e9)+0.002);active();observer.watch()
 assert len(check_record(observer)['photos'])==2 and observer.record['photos'][1]['started_boottime_ns']>=due
 observer.watch();assert len(check_record(observer)['photos'])==2

def control():
 held=[];ep=None;pidfd=None
 try:
  r,w=os.pipe2(os.O_CLOEXEC|os.O_NONBLOCK);held.extend([r,w]);event=os.eventfd(0,os.EFD_CLOEXEC|os.EFD_NONBLOCK);held.append(event)
  ep=select.epoll();ep.register(r,select.EPOLLIN);pidfd=os.pidfd_open(os.getpid(),os.O_NONBLOCK)
  assert not select.select([pidfd],[],[],0)[0]
  assert oracle(os.getpid(),r)['stat']['inode']==os.fstat(r).st_ino
  assert oracle(os.getpid(),event)['target']=='anon_inode:[eventfd]'
  info=oracle(os.getpid(),ep.fileno());assert info['target']=='anon_inode:[eventpoll]'and re.search(r'tfd:\s*'+str(r)+r'\s',info['fdinfo'])
  print('CONTROL original-Linux-pidfd-pipe-eventfd-epoll-proc PASS',flush=True)
 finally:
  if pidfd is not None:os.close(pidfd)
  if ep is not None:ep.close()
  for fd in held:os.close(fd)

def target(cls):
 pid=os.getpid();stamp=starttime(pid);pidfd=os.pidfd_open(pid,os.O_NONBLOCK);held=[];extra=[];ep=None
 try:
  r,w=os.pipe2(os.O_CLOEXEC|os.O_NONBLOCK);held.extend([r,w]);event=os.eventfd(0,os.EFD_CLOEXEC|os.EFD_NONBLOCK);held.append(event)
  mem=os.memfd_create('pokelinux-fd-photo-control',os.MFD_CLOEXEC);held.append(mem);ep=select.epoll();ep.register(r,select.EPOLLIN)
  before=now();o=cls(UNTIL);assert check_record(o)['photos']==[]and before<=o.record['created_boottime_ns']<=now()
  o.bind_main(pid,pidfd,stamp);p1=photo(o,1);old_inode=compare(p1,r)['stat']['inode'];compare(p1,event);compare(p1,mem)
  ep1=compare(p1,ep.fileno());assert re.search(r'tfd:\s*'+str(r)+r'\s',ep1['fdinfo'])
  trigger=o.record['first_trigger_boottime_ns'];assert o.record['second_due_boottime_ns']==trigger+SPACING
  assert now()<o.record['second_due_boottime_ns']-1_000_000,'control missed actual pre50ms window'
  o.watch();assert len(check_record(o)['photos'])==1
  os.close(r);held.remove(r);r2,w2=os.pipe2(os.O_CLOEXEC|os.O_NONBLOCK);held.extend([r2,w2]);assert r2==r and os.fstat(r2).st_ino!=old_inode
  later(o);p2=photo(o,2);assert compare(p2,r2)['stat']['inode']!=old_inode
  o.bind_main(pid,pidfd,stamp);o.watch();assert len(check_record(o)['photos'])==2
  print('GROUP genuine-proc-two-triggers-and-real-fd-reuse PASS',flush=True)
  same=cls(UNTIL);same.bind_main(pid,pidfd,stamp);s1=compare(photo(same,1),event);later(same);s2=compare(photo(same,2),event)
  assert s1['stat']==s2['stat']and same.record['fd_lifetime_continuity_proven']is False
  print('GROUP identical-photos-do-not-prove-continuity PASS',flush=True)
  for _ in range(70):extra.append(os.eventfd(0,os.EFD_CLOEXEC|os.EFD_NONBLOCK))
  bounded=cls(UNTIL);bounded.bind_main(pid,pidfd,stamp);bp=photo(bounded,1);assert bp['fd_limit_exceeded']and len(bp['rows'])<=64;check_record(bounded)
  print('GROUP real-proc-descriptor-and-byte-caps PASS',flush=True)
  wrong=cls(UNTIL);wrong.bind_main(pid,pidfd,str(int(stamp)+1));wr=check_record(wrong)
  assert wr['errors']or any(p['errors']for p in wr['photos'])
  assert not any(p['proc_identity_verified']or p['rows']for p in wr['photos'])
  count=len(wr['photos']);wrong.bind_main(pid,pidfd,stamp);wrong.watch();assert len(check_record(wrong)['photos'])==count
  print('GROUP actual-starttime-mismatch-consumed-without-rebind PASS',flush=True)
 finally:
  if ep is not None:ep.close()
  for fd in extra+held:os.close(fd)
  os.close(pidfd)

def main():
 global UNTIL
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--helper',type=Path);p.add_argument('--control',action='store_true');a=p.parse_args();UNTIL=now()+5_000_000_000
 if a.control:control();active();return 0
 assert a.helper is not None;path=a.helper.resolve(strict=True);sys.path.insert(0,str(path.parent))
 spec=importlib.util.spec_from_file_location('fd_metadata_subject',path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 cls=getattr(module,'FDIdentityObserver',None)
 if cls is None:print('MISSING_OPTIONAL_FD_IDENTITY_OBSERVER_API',flush=True);return 3
 assert callable(cls);active();target(cls);active();return 0

if __name__=='__main__':raise SystemExit(main())
