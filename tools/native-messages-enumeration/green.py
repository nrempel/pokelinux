"""Actual bounded enumeration composition, using the frozen incoming actor/peer."""
import json,os,signal,sqlite3,subprocess
from pathlib import Path
import integration as legacy
from peer_session import PeerSession,BASE

def ids(service):return [row[0] for row in service.raw()]
def records(service):return [row[0] for row in service.records()]
def complete(service,count):legacy.wait(lambda:service.status()['incoming_completed']==count);legacy.wait(lambda:not service.status()['busy'])
def read_all(service):
    service.launch();assert service.query()=={'result':'denied'};service.authorise()
    conversations=service.query()['items'];seen=[]
    for conversation in conversations:
        after=0
        for _ in range(2049):
            page=service.query({'query':'messages','conversation':conversation['id'],'after_ordinal':after,'limit':2})['items']
            assert len(page)<=2
            if not page:break
            after=page[-1]['ordinal'];seen.extend(row['receipt'] for row in page)
        else:raise AssertionError('finite app page bound exhausted')
    return seen

def run(account,out,result):
    checks=result['checks'];session=PeerSession(account,out,'enum-live');service=None
    try:
        session.start();service=legacy.Service(out,session).start();worker=service.worker()
        for count in range(1,65):
            assert service.status()['receiving'];assert service.arrival().returncode==0;complete(service,count)
            if count==33:assert len(service.raw())==33 and len(service.records())==33 and service.status()['available']
        raw_ids=ids(service);assert len(set(raw_ids))==64 and records(service)==raw_ids
        assert service.admin({'op':'snapshot'})['principals']==0
        assert read_all(service)==raw_ids;service.close_app();assert service.worker()==worker and service.status()['receiving']
        checks['live64_closed_window_all_ids_and_current_authority']={'passed':True,'actual_live33rd_callback_projects':True,'raw_occurrences':64,'distinct_equal_receipts':True,'all_ids_across_bounded_app_pages':True,'default_denied_before_fresh_authorization':True,'same_owned_storage_child':True,'fixture':session.evidence}
        for slot,kind,raw in [(1,0,legacy.PDU),(0,1,'00'),(0,0,'01')]:assert service.arrival(slot,kind,raw).returncode==0
        complete(service,67);all_ids=ids(service);assert records(service)==all_ids;assert len([r for r in service.records() if r[3]=='text'])==65
        text_ids=[r[0] for r in service.records() if r[3]=='text'];assert set(read_all(service))==set(text_ids);service.close_app()
        checks['closed_close_two_slots_and_exclusions_beyond32']={'passed':True,'all_raw_ids_projected':67,'text_records':65,'malformed_status_excluded':True,'source_preserved_on_clean_close':True,'fixture':session.evidence}
        # Explicitly fail the owned source, then reopen local history after actual reap.
        service.arrival(0,0,legacy.PDU,1);service.unavailable_reaped();service.history_restart();assert ids(service)==all_ids and records(service)==all_ids
        assert set(read_all(service))==set(text_ids);service.close_app();store=service.s;service.stop();service=legacy.Service(out,session,store=store).start()
        assert not service.status()['receiving'] and ids(service)==all_ids and records(service)==all_ids
        assert all(json.loads(legacy.probe('counters',slot).stdout)[:2]==[1,1] for slot in range(2))
        checks['child_whole_restart_all_ids_without_reattach_or_callback_recreation']={'passed':True,'exact_raw_projection_ids':True,'source_attempt_remains_one':True,'receiving_after_restart':False,'fixture':session.evidence}
        assert all(json.loads(legacy.probe('counters',slot).stdout)[3:]==[0,0,0,0] for slot in range(2))
    finally:
        if service:service.stop()
        session.close()
    for name,fault in [('raw-gap-deadline','hold-after-raw'),('completion-loss','exit-before-complete')]:
        session=PeerSession(account,out,name);service=None
        try:
            session.start();service=legacy.Service(out,session,fault=fault)
            seed=subprocess.run([str(BASE/'target/aarch64-unknown-linux-gnu/debug/enumeration-component'),str(service.s/'seed'),str(out/(name+'-seed.json'))],capture_output=True,text=True,timeout=8)
            # The component's private root is nested; use its exact raw/inbox paths.
            assert seed.returncode==0;os.rename(service.s/'seed/raw',service.s/'raw');os.rename(service.s/'seed/inbox',service.s/'inbox')
            service.start();before=ids(service);assert len(before)==33
            assert service.arrival().returncode==0
            if fault=='hold-after-raw':
                legacy.wait(lambda:service.status()['incoming_raw_committed']==1)
                worker=service.worker();os.kill(worker,signal.SIGSTOP)
                assert service.admin({'op':'snapshot'})['ready']
            service.unavailable_reaped();after=ids(service);assert len(after)==34 and after[:-1]==before
            assert len(records(service))==(33 if fault=='hold-after-raw' else 34)
            service.history_restart();assert ids(service)==after and records(service)==after
            assert json.loads(legacy.probe('counters',0).stdout)[:3]==[1,1,1]
            checks[name]={'passed':True,'committed_ids_recovered_across_pages':34,'no_arrival_recreation_or_registration_retry':True,'actual_reaped_storage_owner':True,'fixture':session.evidence}
        finally:
            if service:service.stop()
            session.close()
    # Actual external writer keeps cold full reconstruction unavailable, no partial Ready.
    session=PeerSession(account,out,'cold-sqlite-failure');service=None;writer=None
    try:
        session.start();service=legacy.Service(out,session)
        seed=subprocess.run([str(BASE/'target/aarch64-unknown-linux-gnu/debug/enumeration-component'),str(service.s/'seed'),str(out/'cold-seed.json'),'--raw-only'],capture_output=True,text=True,timeout=8);assert seed.returncode==0
        os.rename(service.s/'seed/raw',service.s/'raw');os.rename(service.s/'seed/inbox',service.s/'inbox')
        expected=None
        writer=sqlite3.connect(service.s/'inbox/inbox.sqlite3',timeout=0);writer.execute('BEGIN IMMEDIATE')
        service.start(available=False);service.unavailable_reaped();expected=ids(service);assert len(expected)==33
        writer.rollback();writer.close();writer=None;service.history_restart();assert ids(service)==expected and records(service)==expected
        assert all(json.loads(legacy.probe('counters',slot).stdout)[:2]==[0,0] for slot in range(2))
        checks['external_sqlite_failure_no_partial_ready_full_history_recovery']={'passed':True,'all_ids':33,'actual_external_writer':True,'no_source_attachment_before_complete_ready':True,'fixture':session.evidence}
    finally:
        if writer:writer.rollback();writer.close()
        if service:service.stop()
        session.close()
