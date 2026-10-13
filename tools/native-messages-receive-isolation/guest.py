#!/usr/bin/env python3
import json,os,platform,pwd,shutil,signal,subprocess,sys,time,traceback
from pathlib import Path
ROOT=Path('/workspace');BASE=ROOT/'artifacts/native-messages-receive-isolation';out=Path('/results')
sys.path.insert(0,str(ROOT/'tools/native-messages-incoming-binder'))
import peer_session
peer_session.BASE=BASE
peer_session.LOADER='/private/artifacts/runtime/apex/com.android.runtime/bin/linker64'
peer_session.LIBRARIES='/private/artifacts/runtime/system/lib64:/private/artifacts/runtime/apex/com.android.runtime/lib64/bionic'
# Own staging paths, separate from every main/private cache input.
for name in ['incoming-peer','property-info','property-init']:shutil.copyfile(BASE/'input'/name,out/name);(out/name).chmod(0o755)
for name in ['incoming-manager','incoming-probe']:shutil.copyfile(BASE/'input'/name,out/name);(out/name).chmod(0o755)
# Existing fixture expects flat peer helpers plus target Rust programs.
for name in ['incoming-peer','property-info','property-init']:
 target=Path('/run/receive-isolation-input')/name;target.parent.mkdir(exist_ok=True);shutil.copyfile(BASE/'input'/name,target);target.chmod(0o755)
# Redirect the fixture to a guest-only tree; the host worktree/cache remain readonly.
guest=Path('/run/receive-isolation-bins');guest.mkdir()
for name in ['incoming-peer','property-info','property-init']:shutil.copyfile(BASE/'input'/name,guest/name);(guest/name).chmod(0o755)
bins=guest/'target/aarch64-unknown-linux-gnu/debug';bins.mkdir(parents=True)
for name in ['incoming-manager','incoming-probe']:shutil.copyfile(BASE/'input'/name,bins/name);(bins/name).chmod(0o755)
for name in ['messages-incoming-fixture-service','messages-service','messages-stop','lifecycle-client','lifecycle-launcher']:shutil.copyfile(BASE/'target/aarch64-unknown-linux-gnu/debug'/name,bins/name);(bins/name).chmod(0o755)
peer_session.BASE=guest
import integration
result={'passed':False,'architecture':platform.machine(),'kernel':platform.release(),'qemu_mode':'system','network':'none','phone_accessed':False,'vendor_execution':False,'session_provider':False,'checks':{}}
session=None;service=None
try:
 assert platform.machine()=='aarch64'
 Path('/dev/binderfs').mkdir();subprocess.run(['mount','-t','binder','-o','max=8,stats=global','binder','/dev/binderfs'],check=True)
 phase=next(x.split('=',1)[1] for x in Path('/proc/cmdline').read_text().split() if x.startswith('pixel.receive-isolation='))
 session=peer_session.PeerSession(pwd.getpwnam('alarm'),out,'held-close');session.start();service=integration.Service(out,session,fault='hold-read-snapshot' if phase in ['lock-red','lock-green'] else 'none')
 if phase=='errors-green':(service.r/'messages-read.sock').write_text('authored readonly bootstrap socket obstruction')
 service.start(available=phase!='errors-green');integration.wait(lambda:service.status()['receiving']);writer=service.worker()
 def readers():
  found=[]
  for path in Path('/proc').iterdir():
   if path.name.isdecimal():
    try:
     if set((path/'cmdline').read_bytes().split(b'\0')) & {b'--messages-read-worker',b'--messages-held-read-fixture-worker'}:found.append(int(path.name))
    except OSError:pass
  return found
 assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_completed']==1)
 if phase=='errors-green':
  assert service.status()['receiving'] and not service.status()['available'] and not readers()
  assert service.arrival(slot=1).returncode==0;integration.wait(lambda:service.status()['incoming_completed']==2)
  (service.r/'messages-read.sock').unlink();time.sleep(.08);assert not readers() and not service.status()['available'],'failed bootstrap must not implicitly retry'
  service.launch();integration.wait(lambda:service.status()['available']);service.authorise();assert service.worker()==writer
  conversations=service.query();q={'query':'messages','conversation':conversations['items'][0]['id'],'after_ordinal':0,'limit':2};held=readers()[0]
  result['checks']['readonly_bootstrap_os_error_is_local_single_explicit_recovery']={'passed':True,'same_writer_pid':writer,'same_generation':session.generation,'raw_occurrences':2,'new_reader_pid':held,'no_implicit_retry':True}
  os.kill(held,signal.SIGSTOP);service.send(q);integration.wait(lambda:service.status()['busy']);os.kill(held,signal.SIGKILL)
  try:reply=service.receive();assert reply in [{'result':'unavailable'},{'result':'denied'}]
  except (OSError,ValueError):reply={'channel_closed':True}
  integration.wait(lambda:not Path('/proc/'+str(held)).exists());assert service.status()['receiving'] and service.worker()==writer
  assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_completed']==3)
  service.close_app();service.launch();integration.wait(lambda:service.status()['available']);service.authorise();assert service.query(q)['page']=='messages'
  raw=service.raw();assert raw[0][2]==raw[2][2]
  counts=[json.loads(integration.probe('counters',slot).stdout) for slot in range(2)];assert all(row[:2]==[1,1] and row[3:]==[0,0,0,0] for row in counts)
  result['checks']['pending_reader_death_fences_reply_and_preserves_receiving']={'passed':True,'reader_reaped':held,'same_writer_pid':writer,'raw_occurrences':3,'reply':reply,'same_slot_source_incarnation':True,'registration_counters':counts}
  held=readers()[0];os.kill(held,signal.SIGSTOP);service.send(q);integration.wait(lambda:service.status()['busy']);started=time.monotonic();service.stop();stop_ms=(time.monotonic()-started)*1000
  assert not readers() and not Path('/proc/'+str(writer)).exists()
  result['checks']['whole_stop_reaps_held_reader_and_writer']={'passed':True,'stop_ms':stop_ms,'writer_reaped':writer,'reader_reaped':held}
  result['passed']=True;raise SystemExit(0)
 if phase=='enumeration-green':
  for count in range(2,34):
   assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_completed']==count)
  rows=service.raw();assert len(rows)==33 and len(service.records())==33 and service.worker()==writer and service.status()['receiving']
  service.launch();service.authorise();conversations=service.query();assert conversations['items'][0]['messages']==33
  q={'query':'messages','conversation':conversations['items'][0]['id'],'after_ordinal':0,'limit':2};ids=[]
  while True:
   page=service.query(q)['items']
   if not page:break
   ids.extend(item['receipt'] for item in page);q['after_ordinal']=page[-1]['ordinal']
  assert ids==[row[0] for row in rows]
  result['checks']['live_binder33_exact_ids_and_same_generation']={'passed':True,'raw_occurrences':33,'exact_unique_ids':len(set(ids)),'same_writer_pid':writer,'same_generation':session.generation}
  for manifest in ['projection-arm-tests.jsonl','receipt-arm-tests.jsonl','inbox-arm-tests.jsonl']:
   ran=False;logs=[]
   for line in (BASE/'target'/manifest).read_text().splitlines():
    row=json.loads(line)
    if row.get('reason')=='compiler-artifact' and row.get('executable') and row['profile']['test']:
     executable=BASE/'target'/Path(row['executable']).relative_to('/target');run=subprocess.run([str(executable),'--include-ignored'],capture_output=True,text=True,timeout=75);logs.append(executable.name+'\n'+run.stdout+run.stderr);assert run.returncode==0;ran=True
   assert ran;(out/(manifest+'.log')).write_text('\n'.join(logs));result['checks'][manifest]={'passed':True}
  result['passed']=True;raise SystemExit(0)
 initial=service.raw();service.launch();service.authorise();conversations=service.query();q={'query':'messages','conversation':conversations['items'][0]['id'],'after_ordinal':0,'limit':2}
 if phase not in ['lock-red','lock-green']:assert service.query(q)['items'][0]['receipt']==initial[0][0]
 held=writer if phase=='red' else readers()[0]
 if phase in ['lock-red','lock-green']:
  service.send(q);integration.wait(lambda:Path('/proc/'+str(held)+'/stat').read_text().split()[2]=='T')
  import sqlite3
  check=sqlite3.connect(service.s/'inbox/inbox.sqlite3',timeout=0)
  locked=False
  try:check.execute('BEGIN EXCLUSIVE')
  except sqlite3.OperationalError as error:locked='locked' in str(error)
  finally:check.close()
  assert locked,'actual SQLite SHARED transaction must be held'
  assert service.arrival().returncode==0
  if phase=='lock-red':
   service.unavailable_reaped()
   result['checks']['actual_shared_read_transaction_arrival']={'actual_sqlite_shared_lock':True,'raw_receipts':len(service.raw()),'projected_records':len(service.records()),'status':service.status(),'same_generation':session.generation}
   result['expected_red']=True;result['signature']='held_sqlite_reader_stops_background_projection'
   assert service.status()['receiving'] and len(service.records())==2,'ordinary held read must not permanently quarantine background receiving'
   raise AssertionError('lock red unexpectedly passed')
  integration.wait(lambda:service.status()['incoming_raw_committed']==2)
  assert service.status()['receiving'] and len(service.records())==1
  held_status=service.status();started=time.monotonic();assert service.admin({'op':'snapshot'})['ready'];snapshot_ms=(time.monotonic()-started)*1000
  service.close_app();integration.wait(lambda:not Path('/proc/'+str(held)).exists());integration.wait(lambda:service.status()['incoming_completed']==2)
  assert service.worker()==writer and service.status()['receiving'] and len(service.raw())==2 and len(service.records())==2
  result['checks']['actual_shared_read_transaction_arrival_cancel']={'passed':True,'actual_sqlite_shared_lock':True,'held_status':held_status,'same_writer_pid':writer,'same_generation':session.generation,'read_child_actually_reaped':held,'raw_occurrences':2,'projection_complete_after_reap':True,'snapshot_ms':snapshot_ms}
  service.launch();integration.wait(lambda:service.status()['available']);service.authorise();service.query();held=readers()[0]
  service.send(q);integration.wait(lambda:Path('/proc/'+str(held)+'/stat').read_text().split()[2]=='T');assert service.arrival(slot=1).returncode==0
  integration.wait(lambda:service.status()['incoming_raw_committed']==3);assert len(service.records())==2 and service.status()['receiving']
  os.kill(held,signal.SIGCONT);reply=service.receive();assert reply['page']=='messages' and len(reply['items'])==2
  integration.wait(lambda:service.status()['incoming_completed']==3);assert service.worker()==writer and len(service.records())==3
  result['checks']['graceful_transaction_completion_releases_exact_occurrence']={'passed':True,'original_read_upper_items':2,'raw_occurrences':3,'same_reader_pid':held,'same_writer_pid':writer}
  # Reader death is local; one explicit launch recovers only the readonly lifetime.
  os.kill(held,signal.SIGKILL);integration.wait(lambda:not Path('/proc/'+str(held)).exists());assert service.status()['receiving'] and service.worker()==writer
  assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_completed']==4)
  service.close_app();service.launch();integration.wait(lambda:service.status()['available']);service.authorise();service.query();held=readers()[0]
  result['checks']['reader_death_local_recovery_preserves_source']={'passed':True,'raw_occurrences':4,'same_writer_pid':writer,'new_reader_pid':held}
  # A dead but unobserved child cannot release the parent's projection gate.
  service.send(q);integration.wait(lambda:Path('/proc/'+str(held)+'/stat').read_text().split()[2]=='T');assert service.arrival().returncode==0
  integration.wait(lambda:service.status()['incoming_raw_committed']==5);assert len(service.records())==4
  parent=service.child.pid;os.kill(parent,signal.SIGSTOP);os.kill(held,signal.SIGKILL)
  integration.wait(lambda:Path('/proc/'+str(held)+'/stat').read_text().split()[2]=='Z')
  start=time.monotonic();time.sleep(1.15);assert len(service.records())==4 and len(service.raw())==5
  assert Path('/proc/'+str(held)).exists(),'unreaped child retains gate'
  os.kill(parent,signal.SIGCONT);service.unavailable_reaped();failure=service.status()['incoming_failure']
  assert failure in ['arrival_deadline','worker_or_arrival_deadline','child_exit_1']
  assert len(service.records())==4 and len(service.raw())==5
  counts=[json.loads(integration.probe('counters',slot).stdout) for slot in range(2)]
  assert all(row[:2]==[1,1] and row[3:]==[0,0,0,0] for row in counts)
  result['checks']['unobserved_reap_withholds_permit_original_arrival_deadline']={'passed':True,'actual_zombie_before_parent_reap':held,'withheld_seconds':time.monotonic()-start,'raw_occurrences':5,'projected_records':4,'failure':failure,'one_source_registration':counts}
  result['passed']=True
  raise SystemExit(0)
 os.kill(held,signal.SIGSTOP);service.send(q);integration.wait(lambda:service.status()['busy'])
 started=time.monotonic();assert service.admin({'op':'snapshot'})['ready'];snapshot_ms=(time.monotonic()-started)*1000
 service.close_app()
 if phase=='red':
  service.unavailable_reaped();before=service.status();arrival_result=service.arrival();time.sleep(.05)
  after=service.raw();counts=json.loads(integration.probe('counters',0).stdout)
  result['checks']['held_close_next_actual_binder_arrival']={'before_receipts':len(initial),'after_receipts':len(after),'writer_pid_before':writer,'receiving_after_close':before['receiving'],'generation':session.generation,'registration_counters':counts,'next_arrival_returncode':arrival_result.returncode,'snapshot_ms':snapshot_ms,'source':session.evidence}
  result['expected_red']=True;result['signature']='held_read_close_loses_same_live_background_source'
  assert before['receiving'] and arrival_result.returncode==0 and len(after)==2,'next actual arrival must commit under the same still-live generation after held app close'
 else:
  integration.wait(lambda:not Path('/proc/'+str(held)).exists())
  assert service.worker()==writer and service.status()['receiving'] and not readers()
  assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_completed']==2)
  rows=service.raw();assert len(rows)==2 and rows[0][0]==initial[0][0] and rows[1][2]==initial[0][2]
  counts=[json.loads(integration.probe('counters',slot).stdout) for slot in range(2)]
  assert all(row[:2]==[1,1] and row[3:]==[0,0,0,0] for row in counts)
  result['checks']['held_close_next_actual_binder_arrival']={'passed':True,'raw_occurrences':2,'same_writer_pid':writer,'same_generation':session.generation,'same_slot_source_incarnation':True,'read_child_actually_reaped':held,'registration_counters':counts,'snapshot_ms':snapshot_ms}
  # Fresh desktop launch gives one local read recovery, never a new source attachment.
  service.launch();integration.wait(lambda:service.status()['available']);service.authorise()
  new_reader=readers()[0];assert new_reader!=held and service.worker()==writer
  assert [row['receipt'] for row in service.query(q)['items']]==[row[0] for row in rows]
  result['checks']['fresh_authority_reads_exact_ids_with_same_background_source']={'passed':True,'new_read_pid':new_reader,'writer_pid':writer,'ids':[row[0] for row in rows]}
  # Late read completion still rechecks current publication authority.
  os.kill(new_reader,signal.SIGSTOP);service.send(q);integration.wait(lambda:service.status()['busy'])
  assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_raw_committed']==3)
  assert service.admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':False})=={'result':'ok'}
  os.kill(new_reader,signal.SIGCONT);assert service.receive()=={'result':'denied'}
  integration.wait(lambda:service.status()['incoming_completed']==3);assert service.status()['receiving'] and service.worker()==writer
  service.authorise()
  result['checks']['late_read_reply_denied_while_background_occurrence_completes']={'passed':True,'raw_occurrences':3,'same_writer_pid':writer}
  # Original absolute read expiry kills/reaps only the held reader; source keeps receiving.
  os.kill(new_reader,signal.SIGSTOP);service.send(q,ttl=500)
  try:
   reply=service.receive();assert reply in [{'result':'unavailable'},{'result':'denied'}]
  except (OSError,ValueError):reply={'channel_closed':True}
  integration.wait(lambda:not Path('/proc/'+str(new_reader)).exists())
  assert service.worker()==writer and service.status()['receiving']
  assert service.arrival(slot=1).returncode==0;integration.wait(lambda:service.status()['incoming_completed']==4)
  assert len(service.raw())==4 and len(service.records())==4
  service.close_app();service.launch();integration.wait(lambda:service.status()['available']);service.authorise()
  result['checks']['absolute_read_expiry_preserves_writer_and_next_slot_arrival']={'passed':True,'expired_reader_reaped':new_reader,'raw_occurrences':4,'writer_pid':writer}
  # Policy denial is still publication authority, independent of background receiving.
  assert service.admin({'op':'locked','locked':True})=={'result':'ok'}
  assert service.arrival().returncode==0;integration.wait(lambda:service.status()['incoming_completed']==5)
  assert service.query(q)=={'result':'denied'}
  assert service.admin({'op':'locked','locked':False})=={'result':'ok'}
  assert service.worker()==writer and service.status()['receiving']
  result['checks']['locked_reads_denied_background_projection_continues']={'passed':True,'raw_occurrences':5}
  live_readers=readers();started=time.monotonic();service.stop();stop_ms=(time.monotonic()-started)*1000
  assert not readers() and not Path('/proc/'+str(writer)).exists()
  result['checks']['whole_stop_owns_and_reaps_both_lifetimes']={'passed':True,'writer_reaped':writer,'readers_reaped':live_readers,'stop_ms':stop_ms}
  result['passed']=True
except Exception:result['error']=traceback.format_exc()
finally:
 if service:service.stop()
 if session:session.close()
 (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2),flush=True);subprocess.run(['systemctl','poweroff'],check=False)
