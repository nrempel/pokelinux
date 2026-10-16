"""Frozen actual facts-boundary epoch oracle; setup/cleanup failures never qualify RED."""
LEASE=100000000
CASES=['control-focus','control-client-death','control-lock','render-focus','render-client-death','render-lock','control-owner-stall']
def correct(sample,op):
 frame=sample['frame'];focus=frame['focus']
 if op['kind']=='focus':return focus is not None and focus['pid']==op['target_PID']and sample['has_owned_focus']and sample['focused_peer_matches_metadata']and sample['focused_peer_cloexec']
 if op['kind']=='client-death':return focus is None or focus['pid']!=op['target_PID']
 return frame['locked']is True and focus is None

def classify(result,actual_exit,epoch):
 assert epoch in ['RED','GREEN']and result['schema']==1
 assert result['setup_failure']is None and result['cleanup_errors']==[],'setup or cleanup failure, never feature RED'
 assert result['Source_lease_ns']==LEASE and result['Source_packets_authored_or_relayed']is False and result['Source_revival_attempted']is False and result['phone_accessed']is False
 assert result['ECHILD']and result['all_original_waits_and_EOF_qualified']
 assert all(x['pidfd_dead']and x['proc_absent']for x in result['descendant_closure'])
 assert [v['id']for v in result['cases']]==CASES
 failures=[]
 for row in result['cases']:
  assert row['setup_qualified']and 'cleanup_failure'not in row
  assert row['Pixel10_profile']['qualification']['qualified']and row['Pixel10_profile']['hardware_verified']is False
  previous=row['prepared_latest_frame'];assert previous['source']==row['source_incarnation']and previous['focus']['pid']==row['client_PIDs'][0]
  if row['kind']=='authority':
   assert row['actual_stopped_state'].startswith('State:')and 'T'in row['actual_stopped_state']and row['source_pidfd_still_live']
   begin=row['authority_stop_observed_ns'];end=row['resume_send_before_ns'];assert end-begin>LEASE and row['before_stall_frame']['sampled_at_ns']+LEASE>begin
   failed=row['failed_during_hold'];assert len(failed)==1 and failed[0]['error']=='timed out'and failed[0]['positive_state']is False and begin<row['failure_observed_ns']<end
   assert row['feature_passed']is True;continue
  op=row['operation'];assert op['kind']==row['kind']and op['target_live_before_issuance']and op['operation_validity_observed_at_ns']>=op['flushed_at_ns']
  if row['kind']=='focus':
   reply=op['actual_focus_response'];assert reply['actual_public_dispatch_reply'].strip()=='ok'and reply['target_app_id']=='render-stall-B'and reply['at_ns']>=op['flushed_at_ns']
  if row['kind']=='lock':
   accepted=op['actual_lock_acceptance'];assert set(accepted)=={'configure','locked'}and accepted['configure']['event']=='lock_configure_received'and accepted['locked']['event']=='protocol_locked_event'
   assert all(v['pid']==op['target_PID']and v['uid']==1000 and v['at_ns']>=op['flushed_at_ns']for v in accepted.values())
  if row['kind']=='client-death':assert op['actual_wait0_pidfd_dead_proc_absent']
  if not row['stalled']:
   sample=row['actual_correct_sample'];assert op['flushed_at_ns']<=sample['frame']['sampled_at_ns']<=op['flushed_at_ns']+LEASE and correct(sample,op)and row['feature_passed']is True;continue
  enter=row['enter'];release=row['release_send'];ret=row['returned'];assert enter['kind']==3 and ret['kind']==5 and enter['token']==ret['token']==1 and enter['status']==ret['status']==0
  assert enter['value0']==row['compositor_PID']and ret['value0']>LEASE and ret['value1']==1
  assert enter['at_ns']<op['flushed_at_ns']<release['before_ns']<=release['after_ns']<ret['at_ns']<ret['until_ns']
  assert release['before_ns']-op['flushed_at_ns']>LEASE and release['before_ns']-enter['at_ns']>LEASE and row['before_stall_frame']['sampled_at_ns']+LEASE>enter['at_ns']
  failed=row['failed_during_hold']
  if failed:
   assert len(failed)==1 and failed[0]['error']=='timed out'and failed[0]['positive_state']is False and enter['at_ns']<row['failure_observed_ns']<release['before_ns']
  passed=not failed and any(correct(v,op)and op['flushed_at_ns']<=v['frame']['sampled_at_ns']<=op['flushed_at_ns']+LEASE for v in row['during_samples'])
  assert row['feature_passed']==passed
  if not passed:failures.append(row['id'])
 if epoch=='RED':assert actual_exit==1 and failures,'behavioral RED needs a witnessed entered rendering-hold failure'
 else:assert actual_exit==0 and not failures,'GREEN requires all frozen cases'
 return {'epoch':epoch,'expected_epoch_qualified':True,'actual_case_count':7,'actual_render_feature_failures':failures,'cold_start_actor_permission_GPU_completion_or_phone_qualified':False}
