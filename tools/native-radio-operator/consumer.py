"""Source-only phases for the EXISTING owned installed-radio guest envelope.
No __main__, VM driver, production-library import, fake C ABI or in-process owner.
Read INTEGRATION.md before wiring; Root must review all source before execution.
"""
import json, os, signal, stat
from pathlib import Path
from wire_assertions import status,first_response,retained


def run(g,case,baseline=False):
    assert case in ('accepted','expired-before-admission','permit-expires','stop-before-consumption','writer-held')
    if baseline:
        # The original current executable has no future writer/channels/Ready.
        # Never convert structural unknown-command refusal into setup failure.
        assert case=='accepted'
    else:
        # Real metadata/process/channel verifier, still an explicit integration
        # prerequisite. This branch is never used to prepare the original RED.
        g['assert_proposed_ready']()
    now=g['now'];command=g['command'];until=g['case_until'];digest=g['expected']['payload_sha256']
    socket=g['SOCKET'];pid=g['main_pid'];inc=g['incarnation'];fixture=g['FIXTURE']
    assert now()<until and os.geteuid()==0
    record=[]; stopped=None; first=None

    def helper(action,label):
        action_until=min(until,now()+3_000_000_000)
        code,data,_=command(['setpriv','--reuid=1000','--regid=1000','--clear-groups',fixture/'operator-controller',action,str(action_until)],label,
            action_until,expected=0,user=False,pipe=True)
        v=json.loads(data);assert v['controller_uid']==1000 and v['kind']==action
        record.append(v);return v

    def capture(count):
        v=helper('capture','independent-capture')
        assert v['epoch']==0 and v['count']==v['total']==count
        assert v['owner_pid']==g['worker_pid'] and v['owner_uid']==1500
        if count:
            assert [v[k] for k in ('serial','client_id','request','slot')]==[1,42,105,1]
            assert v['calling_pid']==0 and v['calling_uid']==1500
            assert v['payload_hex']==g['expected']['payload_hex']
        return v

    def current(label):
        v=g['read_current_status'](label,until)
        return v,status(v,pid,inc,digest)

    def admit(deadline,label,user=False):
        body={'schema':1,'incarnation':inc,'attempt':1,'serial':1,'request':105,'slot':1,
            'payload_sha256':digest,'deadline_ns':deadline}
        rpc_until=min(until,now()+3_000_000_000)
        code,data,_=command(['python3',fixture/'operator_client.py','--socket',socket,
            '--server-pid',str(pid),'--until-ns',str(rpc_until),'--admit',
            json.dumps(body,separators=(',',':'),sort_keys=True)],label,rpc_until,expected=None,user=user,pipe=True)
        value=json.loads(data);record.append({'code':code,'operator':value})
        assert code in (0,101),value
        return code,value

    def op_file():
        return Path(str(g['JOURNAL'])+'.add-pdn')

    # Healthy current real executable/socket/Binder setup is mandatory first.
    plain=g['read_current_status']('healthy-current-status',until)
    assert plain['server_pid']==pid and plain['incarnation']==inc and plain['phase']=='observed-active'
    if 'request_history' in plain:assert plain['request_history'] is None
    capture(0)
    # Existing config is data only. At least two real visits and peer snapshots
    # prove startup/config/status never automatically emits105.
    plain2=g['read_current_status']('config-alone-no-dispatch',until)
    assert plain2['phase']=='observed-active';capture(0)
    # Existing named peer emits one137 response during registration. Require the
    # actual current tagged history before freezing a stable transport revision.
    assert plain2['last_observation']['kind']=='raw'
    assert plain2['last_observation']['serial']==7
    assert not op_file().exists()

    # A current diagnostics principal cannot obtain operation authority.
    denied_code,denied=admit(min(until,now()+2_000_000_000),'diagnostic-admit-denied',user=True)
    assert denied_code==101 or denied['reply']['admitted'] is False
    assert not op_file().exists();capture(0)

    deadline=min(until-5_000_000_000,now()+8_000_000_000)
    if case=='expired-before-admission':deadline=now()-1
    if case in ('permit-expires','stop-before-consumption','writer-held'):
        role='writer' if case=='writer-held' else 'worker'
        stopped=g[role+'_watch']
        assert g['alive'](stopped)
        signal.pidfd_send_signal(stopped,signal.SIGSTOP)
        # Original bound pidfd only; never reopen or signal a numeric PID.
        g['wait_stopped_role'](role,until)
    try:
        code,reply=admit(deadline,'one-root-admit')
        if baseline:
            assert code==101 and reply['classification']=='refused-or-missing-command'
            capture(0);assert not op_file().exists()
            closure=g['close_baseline_unit_and_refuse_restart']()
            assert closure['native_radio_worker_wait'] and closure['Main_exit']==0
            assert closure['restart_exit']==78 and closure['no_successor_lookup_or_registration_or105']
            return {'closure':closure,'classification':'structural-functional-missing-command','oracle_satisfied':False,
                'healthy_actual_setup':True,'zero105':True,'records':record}
        assert code==0 and reply['classification']=='operator-reply'
        r=reply['reply']
        assert r['incarnation']==inc and r['attempt']==1
        if case=='expired-before-admission':
            assert r['admitted'] is False and r['reason']=='expired-before-admission'
            v,q=current('expired-command-no-admission');assert q is None
            assert not op_file().exists();capture(0)
        else:
            assert r['admitted'] is True and r['reason']=='admitted-pending-durability'
            if case=='writer-held':
                # No fabricated sync ACK, no release. Main must serve three real
                # clients while the owned writer is actually stopped.
                for index in range(3):
                    v,q=current('held-writer-responsive-'+str(index))
                    assert q is not None and q.outcome=='outcome-unknown' and q.response is None
                    assert v['revision']==plain2['revision']  # request-only status change
                capture(0);first=q
            else:
                g['wait_verified_writer_sync_and_reap'](deadline)
                # Guest may inspect marker data; production Main must not perform
                # blocking filesystem rereads during its live event loop.
                m=op_file().lstat();assert stat.S_ISREG(m.st_mode) and stat.S_IMODE(m.st_mode)==0o600
                assert m.st_uid==m.st_gid==1500
                g['assert_exact_intent_record'](op_file(),inc,deadline,digest)
                v,q=current('admitted-history')
                assert q is not None and q.deadline==deadline and q.outcome=='outcome-unknown'
                first=q
                if case=='accepted':
                    g['wait_for'](lambda:capture_if_one(g,helper),deadline,'actual sole105')
                    capture(1)
                    for control in (141,142,143,144):
                        assert helper(str(control),'wrong-response-'+str(control))['ok']
                        _,after=current('unattributed-response-'+str(control))
                        assert after.response is None
                    assert helper('145','matching-response')['ok']
                    after=g['wait_request_response'](deadline)
                    first=first_response(q,status(after,pid,inc,digest))
                    transport_revision=after['revision']
                    assert helper('146','duplicate-response')['ok']
                    _,after_duplicate=current('duplicate-preserves-first')
                    retained(first,after_duplicate)
                    denied_code,denied=admit(deadline,'second-admit-refused')
                    assert denied_code==0 and denied['reply']['admitted'] is False
                    assert denied['reply']['reason']=='operation-slot-occupied';capture(1)
                    # request history has its own revision; no condition compares
                    # request_revision to transport_revision.
                    record.append({'first_request_revision':first.revision,'transport_revision':transport_revision})
                elif case=='permit-expires':
                    capture(0)
                    g['wait_for'](lambda:now()>=deadline,until,'original request deadline')
                    signal.pidfd_send_signal(stopped,signal.SIGCONT);stopped=None
                    final=g['wait_not_sent'](until)
                    _,q=current('expired-permit-not-sent')
                    assert q.outcome=='not-sent' and q.response is None;capture(0);first=q
                    # Late matching callback is acknowledged only if the actual
                    # worker receiver remains live; never invent ACK after loss.
                    late=helper('145','late-expired-response')
                    _,late_q=current('late-expired-unattributed')
                    assert late_q.response is None and late_q.outcome=='not-sent'
                else:
                    assert case=='stop-before-consumption';capture(0)

        # Single Stop path retains custody even in assertion failures (finally in
        # envelope must call its selected-unit cleanup; this consumer never reaps
        # or kills Main's radio worker / intent writer).
        snapshot=g['stop_with_snapshot'](until)
        if case!='expired-before-admission':
            final=status(snapshot,pid,inc,digest)
            assert final is not None
            if first.response is not None:retained(first,final)
            else:
                assert final.response is None
                assert final.outcome==('not-sent' if case=='permit-expires' else 'outcome-unknown')
        else:assert snapshot['request_history'] is None
        # Required adapter uses EXISTING native worker wait + writer role wait,
        # PID1 outcome/empty cgroup, client EOF, unchanged marker and restart78.
        closure=g['close_unit_and_refuse_restart'](first)
        assert closure['native_radio_worker_wait'] and closure['native_writer_wait']
        assert closure['Main_exit']==0 and closure['restart_exit']==78
        assert closure['unchanged_markers'] and closure['no_successor_lookup_or_registration_or105']
        return {'classification':'actual-boundary-case','case':case,'oracle_satisfied':True,
            'records':record,'closure':closure,'response_crash_persistence_claimed':False}
    finally:
        if stopped is not None and g['alive'](stopped):
            # Only CONT an exact still-owned stopped child so parent disposal can
            # progress; do not substitute an independent kill/wait as evidence.
            signal.pidfd_send_signal(stopped,signal.SIGCONT)


def capture_if_one(g,helper):
    v=helper('capture','capture-until-one')
    assert v['count'] in (0,1) and v['total']==v['count'] and v['epoch']==0
    return v if v['count']==1 else None
