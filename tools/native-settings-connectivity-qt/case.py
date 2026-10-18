"""One authored-root socket peer for real unshown Qt input; no actor/kernel peer."""
import errno,json,os,select,socket,struct,time,traceback
from pathlib import Path
from owned import now
from closure import close_owned
from peer import reduce_caps

def run_case(scope,out,runtime,executable,theme,work,close,expect_red):
    start=now();case=min(work,start+6_000_000_000);endpoint=runtime/'app.sock'
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET|socket.SOCK_NONBLOCK|socket.SOCK_CLOEXEC)
    listener.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);listener.bind(str(endpoint));endpoint.chmod(0o666);listener.listen(8)
    connections={};child=None;configured=False;sets=[];packets=[];eofs=[];resets=[];readonly_send_failures=[];connection_serial=0;role_counts={};r={'passed':False,'behavior_RED_qualified':False,'setup_qualified':False,'original_case_until_ns':case,'original_work_until_ns':work,'original_close_until_ns':close,'authored_root_peer_only':True,'actor_OSgrant_kernel_packets_normal_Main_window_or_capture_claimed':False}
    environment=dict(os.environ,HOME=str(runtime/'user/home'),XDG_RUNTIME_DIR=str(runtime/'user/xdg'),XDG_CACHE_HOME=str(runtime/'user/home/.cache'),QT_QPA_PLATFORM='offscreen',QT_QUICK_BACKEND='software',QSG_RENDER_LOOP='basic',QT_SCALE_FACTOR='2.625',QML_DISABLE_DISK_CACHE='1')
    try:
        child=scope.start(['python3','-B','/runner/inside.py','qt-child',str(executable),str(endpoint),str(theme),str(case)],'actual-Qt-click-child.log',64*1024**2,env=environment,cutoff=case,wait_cutoff=close,stderr_regular=out/'actual-Qt-click-child.stderr')
        r['root_peer_actual_caps']=reduce_caps(224)
        while now()<case:
            scope.pump();scope.check()
            while True:
                try:connection,_=listener.accept()
                except BlockingIOError:break
                connection.setblocking(False);connection.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1)
                credentials=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
                assert credentials==(child['child'].pid,11050,11050),credentials
                connection_serial+=1
                connections[connection]={'credentials':credentials,'role':None,'index':connection_serial}
            for connection in list(connections):
                try:body,ancillary,flags,_=connection.recvmsg(4096,socket.CMSG_SPACE(12),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
                except BlockingIOError:continue
                except OSError as error:
                    if error.errno!=errno.ECONNRESET:raise
                    resets.append({'connection_id':connections[connection]['index'],'errno':error.errno,'at_ns':now()})
                    continue # Same FD retained: drain queued packets and actual read0 under original case.
                if not body:
                    eofs.append({'role':connections[connection]['role'],'at_ns':now()});connection.close();del connections[connection];continue
                assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC)
                assert len(ancillary)==1 and ancillary[0][:2]==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS)
                credentials=struct.unpack('3i',ancillary[0][2]);assert credentials==connections[connection]['credentials']
                request=json.loads(body);assert isinstance(request,dict);at=now();assert at<case
                role='brightness'if request.get('op')=='brightness_status'else 'connectivity'
                previous=connections[connection]['role'];assert previous is None or previous==role,'request role changed within retained socket'
                connections[connection]['role']=role;role_counts[role]=role_counts.get(role,0)+1
                packets.append({'request':request,'credentials':list(credentials),'at_ns':at,'role':role,'connection_id':connections[connection]['index']})
                if role=='brightness':
                    assert request=={'op':'brightness_status'};reply={'result':'unavailable'}
                    encoded=json.dumps(reply,separators=(',',':')).encode()
                    try:assert connection.send(encoded,socket.MSG_DONTWAIT|socket.MSG_NOSIGNAL)==len(encoded)
                    except OSError as error:
                        if error.errno not in [errno.EPIPE,errno.ECONNRESET]:raise
                        readonly_send_failures.append({'role':role,'errno':error.errno,'at_ns':now()})
                    connection.close();del connections[connection];continue
                assert request.get('op')in ['connectivity_status','connectivity_set']
                if request['op']=='connectivity_set':
                    assert set(request)=={'op','request'}and set(request['request'])=={'connected','until_ns'}
                    wanted=request['request'];assert wanted['connected']is True and isinstance(wanted['until_ns'],str)
                    until=int(wanted['until_ns']);assert str(until)==wanted['until_ns']and at<until and until-at<=1_000_000_000
                    assert len(sets)==0,'exactly one mutation from one mouse pair; no retries'
                    sets.append({'request':request,'received_ns':at});configured=True
                else:assert request=={'op':'connectivity_status'}
                observed=now();state={'connected':True,'available':True,'configured':configured,'pending':False,'observed_at_ns':str(observed),'until_ns':str(observed+250_000_000)}
                reply={'result':'connectivity_completed','state':state}if request['op']=='connectivity_set'else {'result':'connectivity_state','state':state,'can_control':True}
                encoded=json.dumps(reply,separators=(',',':')).encode()
                try:assert connection.send(encoded,socket.MSG_DONTWAIT|socket.MSG_NOSIGNAL)==len(encoded)
                except OSError as error:
                    if request['op']!='connectivity_status'or error.errno not in [errno.EPIPE,errno.ECONNRESET]:raise
                    readonly_send_failures.append({'role':role,'connection_id':connections[connection]['index'],'errno':error.errno,'at_ns':now()})
            if not scope.live(child)and child['eof']and not connections:break
            time.sleep(.001)
        code=scope.finish(child,close);assert code==0,('Qt setup or local close failed',code)
        assert now()<case,'whole original six-second component cutoff'
        body=(out/'actual-Qt-click-child.log').read_text();lines=[v[len('QT_CLICK_CASE '):]for v in body.splitlines()if v.startswith('QT_CLICK_CASE ')];assert len(lines)==1
        qt=json.loads(lines[0]);r['Qt_observations']=qt;assert not qt.get('error'),qt
        assert qt['setup_qualified']and qt['actual_click_qualified']and qt['all_quick_windows_hidden']and qt['Qt_version']=='6.8.2'
        assert qt['click']['clicked']==1 and qt['click']['press_delivered']and qt['click']['release_delivered']and qt['click']['all_quick_windows_hidden']
        deadline=int(qt['original_until_ns']);assert qt['deadline_calls']==1 and int(qt['capture_before_ns'])+1_000_000_000<=deadline<=int(qt['capture_after_ns'])+1_000_000_000
        assert int(qt['click']['before_ns'])<=int(qt['capture_before_ns'])<=int(qt['capture_after_ns'])<=int(qt['click']['after_ns'])
        assert int(qt['observation_until_ns'])<deadline
        assert qt['async_close_qualified']and qt['closed_signals']==1 and qt['event_heartbeat']>0 and not qt['closed_inline']and qt['all_joins_and_free_after_beginClose_return']
        assert int(qt['close_started_ns'])<=int(qt['close_completed_ns'])<int(qt['close_until_ns'])and int(qt['close_until_ns'])-int(qt['close_started_ns'])==200_000_000
        calls=qt['actual_FFI_calls'];assert sum(v['operation']=='deadline'for v in calls)==1
        stops=[n for n,v in enumerate(calls)if v['operation']=='stop'];joined=[n for n,v in enumerate(calls)if v['operation']=='join'and v['result']==0];freed=[n for n,v in enumerate(calls)if v['operation']=='free'and v['result']==0]
        assert len(stops)==len(joined)==len(freed)==1 and stops[0]<joined[0]<freed[0]
        assert calls[stops[0]]['result']==0 and all(v['result']in [0,2]for v in calls if v['operation']=='join')
        assert all(int(v['before_ns'])>=int(qt['close_call_returned_ns'])for v in calls if v['operation']in ['join','free'])
        assert role_counts.get('connectivity',0)>0 and role_counts.get('brightness',0)>0
        assert len({v['connection_id']for v in packets if v['role']=='connectivity'})==1,'one readonly-established retained connectivity transport'
        assert any(v['role']=='connectivity'for v in eofs),'actual connectivity zero-byte socket EOF after Stop'
        exact=len(sets)==1 and sets[0]['request']['request']['until_ns']==str(deadline)and qt['configured_true_readback']
        if expect_red:
            assert not sets and not qt['configured_true_readback']and qt['end_configured']is False and qt['end_available']and qt['end_canControl'],'only actual missing Set after healthy fresh state is desired RED'
        else:assert exact
        r.update(setup_qualified=True,actual_click_qualified=True,original_deadline_capture_qualified=True,async_close_qualified=True,actual_child_wait_EOF=True,exact_Set_and_configured_readback=exact,actual_Set_packets=sets,actual_socket_packets=packets,actual_socket_EOFs=eofs,actual_socket_resets=resets,readonly_reply_transport_failures=readonly_send_failures,role_counts=role_counts,component_child_exit=code)
    except BaseException as error:r.update(error=repr(error),traceback=traceback.format_exc())
    finally:
        for connection in connections:connection.close()
        listener.close();endpoint.unlink(missing_ok=True)
        r['prior_parent_closure']=close_owned(scope,close,reap_adopted=True)
        r['prior_parent_handles_closed']=all(v['wait']is not None and v['eof']and v['log'].closed and v['child'].stdout.closed and v['pidfd']is None and(v['stderr_file']is None or v['stderr_file'].closed)for v in scope.children)
        r['authored_peer_listener_and_connections_closed']=True
    proof=out/'qt-case-proof.json';proof.write_text(json.dumps(r,indent=2)+'\n');proof.chmod(0o644)
    if not r.get('error')and r['setup_qualified']and r['prior_parent_handles_closed']and r['prior_parent_closure']['closed']and r['prior_parent_closure']['wait_all_ECHILD']:
        # The same compiled consumer asserts only after the actual Qt process and
        # authored peer have closed, with all original parent handles closed.
        code,body=scope.run([str(executable),'--assert-result',str(proof)],'final-compiled-click-assertion.log',close,limit=64*1024**2,identity={'user':11050,'group':11050,'extra_groups':[]})
        r['final_compiled_assert_exit']=code;r['final_compiled_assert_stdout']=body.decode()
        if expect_red:
            assert code==-6 and b'SETTINGS_CONNECTIVITY_QML_ENABLE_CLICK_MUST_SEND_EXACT_SET_AND_READBACK'in body,(code,body)
            r['behavior_RED_qualified']=True;r['classification']='ACTUAL_NORMAL_QML_ENABLED_CLICK_MISSING_SET_RED'
        else:
            assert code==0 and b'PASS normal Settings real QML click/exact wire/readback/async close'in body,(code,body)
            r['passed']=True;r['classification']='ACTUAL_NORMAL_QML_CLICK_EXACT_SET_READBACK_ASYNC_CLOSE_GREEN'
    return r
