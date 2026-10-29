"""Pure desired policy oracle for actual owned normal worker traces; performs no I/O."""
import re
MARKER='IDLE_EMPTY_RECEIVE_MUST_NOT_RETIRE_PERSISTENT_WORKER'
CONTEXT=re.compile(r'Error: Custom \{ kind: TimedOut, error: WorkerFailure \{ stage: ReceiveCommand, original_until_ns: Some\(([0-9]+)\), detected_at_ns: ([0-9]+), source: Kind\(TimedOut\) \} \}')
def need(value,message):
 if not value:raise ValueError(message)
def common(r):
 need(r['prerequisites_qualified']and r['custody_qualified']and not r.get('error')and not r['cleanup_errors'],'actual setup/cleanup must qualify')
 need(not r['retry_or_reconnect_or_rollback']and not r['Settings_or_VM_or_phone_executed'],'only selected headless no-retry case')
 need(r['initial_private_inspect']['state']['configured']is False and not r['initial_private_inspect']['state']['pending'],'initial exact idle state')
 need(r['initial_independent_readback']['ipv4']==[]and r['all_handles_retired']and r['namespace_FDs_retained_through_all_waits_EOF'],'independent absence and retained namespace custody')
def context(log):
 found=CONTEXT.findall(log);need(len(found)==1,'one exact actual ReceiveCommand/TimedOut diagnostic required');until,detected=map(int,found[0]);need(detected>=until,'actual late guard detection');return until,detected

def qualify_idle(r,log):
 common(r);need(r['case']=='idle'and r['actual_effect_requests']==0,'idle has zero effect submissions')
 t=r['trace'];need(t['one_fixture_owned_worker_only']and not t['packet_payload_or_process_memory_read']and not t['syscall_results_changed'],'only actual metadata trace')
 need(t['detached_at_actual_nonterminal_stop']and t['normal_Main_retains_terminal_wait'],'tracer does not consume Main terminal wait')
 stops=t['trace_stops'];need(len(stops)==1 and stops[0]['label']=='empty-idle-before-auth'and stops[0]['requested_hold_ns']==80_000_000,'one exact fixed empty-idle stop')
 stop=stops[0];need(stop['actual_resume_started_at_ns']>=stop['original_resume_at_ns'],'actual controlled interval')
 gate=t['idle_gate_attribution'];need(gate['prior_private_observation_send']['bytes']==47 and gate['loop_sleep_exit_ns']<stop['kernel_stopped_at_ns'],'actual send/sleep/thread-check ordering')
 need(r['after_trace_independent_readback']['ipv4']==[]and r['functional_completed_before_original_case'],'no address effect and original case bound')
 desired=bool(r.get('same_worker_fresh_after_idle')and not r['worker_terminal_before_cleanup']and t.get('post_hold_observation_send'))
 behavior_red=False;detail={}
 if not desired:
  until,detected=context(log)
  need(stop['kernel_stopped_at_ns']<until<=stop['kernel_stopped_at_ns']+50_000_000 and stop['actual_resume_started_at_ns']>=until,'same original50ms envelope straddles exact trace stop')
  need(r['worker_terminal_before_cleanup']and t.get('exit_event_before_terminal_wait')and t['post_hold_private_receives']==[],'old worker dies without a post-stop receive/payload consumption')
  need(r['actual_Main_worker_closure']['worker_exit_code']==1 or r['actual_Main_worker_closure']['worker_exit_signal']==9,'actual final worker outcome')
  behavior_red=True;detail={'original_envelope_until_ns':until,'detected_at_ns':detected,'exact_empty_no_receive_control':True}
 return {'passed':desired,'behavior_RED_qualified':behavior_red,'actual_outcome_confirmed':True,'detail':detail}

def qualify_consumed(r,log):
 common(r);need(r['case']=='consumed'and r['actual_effect_requests']==1,'exactly one explicit effect request')
 t=r['trace'];stops=t['trace_stops'];need(len(stops)==1 and stops[0]['label']=='consumed-receive-envelope-expiry'and stops[0]['requested_hold_ns']==80_000_000,'one exact consumed stop')
 frame=t['actual_consumed_frame'];stop=stops[0];need(frame['bytes']==19 and not frame['contents_inspected'],'kernel actual private19-byte receive; no payload inspection')
 need(frame['exit_stop_at_ns']<stop['actual_resume_started_at_ns']<frame['original_action_until_ns'],'controlled resume must precede the unchanged original100ms action cutoff')
 request=t['one_apply'];need(request['request']['until_ns']==str(frame['original_action_until_ns'])and request['actual_Main_pending_before_receiver_resume']['state']['pending'],'exact submitted original action and prior dispatch')
 until,detected=context(log);need(frame['exit_stop_at_ns']<until<=stop['actual_resume_started_at_ns']<=detected<frame['original_action_until_ns'],'actual worker detection must exceed unchanged50ms envelope yet precede original100ms action cutoff')
 need(r['worker_terminal_before_cleanup']and t.get('exit_event_before_terminal_wait')and not t.get('post_hold_observation_send'),'late consumed frame cannot become successful/fresh')
 need(r['actual_Main_worker_closure']['worker_exit_code']==1 and r['actual_Main_worker_closure']['worker_exit_signal']is None,'supervisor SIGKILL cannot establish worker post-receive validation')
 need(r['after_trace_independent_readback']['ipv4']==[]and r['functional_completed_before_original_case'],'independent absence under original aggregate')
 need(t['detached_at_actual_nonterminal_stop']and t['normal_Main_retains_terminal_wait'],'actual parent retains final wait')
 return {'passed':True,'actual_consumed_bytes':19,'original_action_until_ns':frame['original_action_until_ns'],'no_late_success_or_effect_retry':True}

def qualify_parent_death(r):
 common(r);need(r['case']=='parent-death'and r['actual_effect_requests']==0 and r['parent_death_control_passed'],'idle parent death with no effect')
 need(r['actual_Main_exit']==-9 and r['actual_Main_wait_and_EOF_before_adopted_worker_wait'],'actual owned parent death and shared output closure')
 v=r['known_worker_adopted_actual_wait'];need(v['pid']==r['worker_pid']and v['returncode']==-9 and r['independent_worker_pidfd_terminal_after_Main_wait'],'kernel parent death kills the exact retained worker')
 return {'passed':True,'actual_known_worker_parent_death_wait_signal':9,'ordinary_traced_Main_wait_claimed':False}

def qualify_all(idle,consumed,death,*,all_outer_cleanup_qualified):
 need(all_outer_cleanup_qualified and consumed['passed']and death['passed']and idle['actual_outcome_confirmed'],'all controls and actual outer closure')
 return {'passed':bool(idle['passed']),'behavior_RED_qualified':bool(idle['behavior_RED_qualified']),'consumed_expiry_control_passed':True,'idle_parent_death_control_passed':True}
def assert_desired_after_cleanup(review):
 assert review['passed'],MARKER
