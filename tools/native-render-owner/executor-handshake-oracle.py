"""Frozen GL plus Rust-executor unload contract; absent public boundary is target RED."""

def _timed(row):
 assert 0<row['operation_enter_ns']<=row['operation_return_ns']<=row['collection_boottime_ns']<=row['Root_received_at_ns']

def classify(component,exit_code,epoch,expected_failure_case=None):
 assert epoch in ['RED','GREEN']
 assert component['schema']==1 and component['baseline_kind']=='OWNER_GL_NATIVE_LEAF_BEHAVIOR'
 assert component['Source_lease_ns']==100000000 and component['Source_packets_authored_or_relayed']is False
 assert component['availability']=='ACTUAL_NATIVE_SYNC_FILE_POLL_SUCCESS','actual native fence capability unavailable, never target RED'
 assert component['setup_failure']is None and not component['cleanup_errors']
 assert component['ECHILD']and component['all_original_waits_and_EOF_qualified']
 assert all(v['pidfd_dead']and v['proc_absent']for v in component['descendant_closure'])
 assert len(component['cases'])==1
 c=component['cases'][0];assert c['id']=='owner-gl-behavior'and c['setup_qualified']
 assert c['Pixel10_profile']['qualification']['qualified'] and c['render_node_inventory']['raw_open_qualified']
 assert c['actor_sender_UID_GID']==[1000,1000] and c['actor_sender_PID']==c['compositor_PID']
 assert c['actor_result_transport']=='POST_START_SEQPACKET_CONNECTION'
 assert c['actor_connection_peer_PID_UID_GID']==[c['compositor_PID'],1000,1000]
 assert c['actor_connection_CLOEXEC']and c['actor_connection_NONBLOCK']and c['no_actor_sender_FD_inherited_at_compositor_exec']
 assert c['source_healthy_before_and_after']and not c['source_terminal_errors']and c['actor_load_reply']=='ok'
 before,after=c['Source_before'],c['Source_after']
 for sample in [before,after]:
  assert sample['frame']['source']==c['source_incarnation']and sample['frame']['focus']['pid']==c['client_PID']
  assert sample['has_owned_focus']and sample['focused_peer_matches_metadata']and sample['focused_peer_cloexec']
 assert 0<=c['actor_load_before_ns']-before['frame']['sampled_at_ns']<50000000
 linkage=c['plugin_linkage'];assert linkage['unresolved_strong']==[]and linkage['production_primitive_CPP_explicitly_compiled']and linkage['production_resource_CPP_explicitly_compiled']
 assert linkage['plugin_only_local_headers']
 if epoch=='RED':assert not linkage['installed_Frame507_SDK_changed']and not linkage['native_rebuilt_cohort_claimed']
 else:
  assert linkage['matching_executor_native_cohort_qualified']and linkage['native_rebuilt_cohort_claimed']
  assert linkage['selected_Rust_staticlib_sha256']=='246a6680b7f9ed8affd89bd9b022319a0903f5ebc58f03d7d76eaa85f8653311'
  assert linkage['actual_AQ_SONAMEs']==['libaquamarine.so.15']and linkage['required_executor_exports_resolved_in_matching_main']
 records=c['actor_records'];assert 0<len(records)<=64 and 0<c['actor_total_bytes']<=65536
 tids={v['tid']for v in records};assert len(tids)==1;worker=next(iter(tids));assert worker>0 and worker!=c['compositor_PID']
 for v in records:
  assert v['Root_actual_sender_PID_UID_GID']==[c['compositor_PID'],1000,1000]
  assert v['run']==1 and 0<v['collection_boottime_ns']<=v['Root_received_at_ns']
  if v['case']=='GL_PROFILE':
   assert v['renderer'].startswith('virgl')and 'Mesa 26.2.4'in v['version']and v['vendor'];continue
  assert v['authority']=='OFFSCREEN_FIXTURE_IDS_ONLY'and v['admission_id']==v['run']
  assert all(v[k]==1 for k in ['security_generation','output_id','output_generation'])
  assert v['scene_sequence']==v['resource_id']==v['resource_generation']==v['commit_sequence']and v['resource_id']in [1,2]
  assert type(v['fence_sequence'])is int and 0<=v['fence_sequence']<=6
  assert type(v['command_sequence'])is int and 0<=v['command_sequence']<=3
 # Exact production-controlled prefix: actual capabilities and existing leaf
 # controls precede the first target failure or successful helper completion.
 prefix=[('CAPS','REAL_DEVICE'),('GL_PROFILE',None),('NATIVE_FENCE_CAPABILITY','EXTENSION_AND_PROCS_PRESENT_NOT_COMPLETION'),('NATIVE_FENCE_CAPABILITY','ACTUAL_SYNC_FILE_POLL_SUCCESS'),('CUSTODY_BYTES','EXACT_96_BYTES_AND_PADDING'),('CUSTODY_PRODUCER','SOURCE_SNAPSHOT_STORE_MAPPING_DROPPED'),('NGL-01','PASS'),('NGL-02','PASS'),('SAMPLER_CONTROL','BOUND_UNIT0_MIPMAP_SAMPLER_WITH_MISSING_LEVELS'),('SHM_DRAW','COMMANDS_ISSUED_NOT_COMPLETION')]
 assert [(v['case'],None if v['case']=='GL_PROFILE'else v['status'])for v in records[:10]]==prefix
 assert records[0]['value']==c['render_node_inventory']['actual_opened_rdev']
 _timed(records[8]);assert records[8]['value']>0 and records[8]['fence_sequence']==records[8]['command_sequence']==0
 assert all(v.get('resource_id',1)==1 for v in records[:10])
 _timed(records[3]);assert records[3]['value']>0
 assert records[4]['value']==0 and records[5]['value']==(c['compositor_PID']if epoch=='RED'else worker)
 _timed(records[5]);assert records[5]['operation_return_ns']<=records[9]['operation_enter_ns']
 for v in records[6:10]:_timed(v)
 assert records[6]['value']==records[7]['value']==0 and records[9]['command_sequence']==1 and records[9]['fence_sequence']==0
 assert expected_failure_case==('MISSING_EXECUTOR_STOP_FINALIZE'if epoch=='RED'else None)
 assert exit_code==(1 if epoch=='RED'else 0)
 assert c['outcome']==('MISSING_EXECUTOR_HANDSHAKE'if epoch=='RED'else'ALL_CASES_CLOSED')
 tail=[('FENCE_ISSUE','EXPORTED_NOT_COMPLETION'),('FENCE_FACT','SYNC_FILE_SUCCESS_OBSERVED'),('READBACK','EXACT_RGBA_192_BYTES'),('NGL-03','PASS'),('CUSTODY_BYTES','EXACT_96_BYTES_AND_PADDING'),('CUSTODY_PRODUCER','SOURCE_SNAPSHOT_STORE_MAPPING_DROPPED'),('SHM_DRAW','COMMANDS_ISSUED_NOT_COMPLETION'),('FENCE_ISSUE','EXPORTED_NOT_COMPLETION'),('FENCE_FACT','SYNC_FILE_SUCCESS_OBSERVED'),('READBACK','EXACT_RGBA_192_BYTES'),('NGL-04','PASS'),('WRONG_THREAD','REFUSED_WITHOUT_TRANSFER'),('FENCE_ISSUE','EXPORTED_NOT_COMPLETION'),('FENCE_FACT','SYNC_FILE_SUCCESS_OBSERVED'),('READBACK','EXACT_RGBA_192_BYTES'),('NGL-05','PASS'),('FENCE_ISSUE','EXPORTED_NOT_COMPLETION'),('FENCE_FACT','SYNC_FILE_SUCCESS_OBSERVED'),('READBACK','EXACT_RGBA_192_BYTES'),('NGL-06','PASS'),('FENCE_ISSUE','EXPORTED_NOT_COMPLETION'),('FENCE_FACT','SYNC_FILE_SUCCESS_OBSERVED'),('READBACK','EXACT_RGBA_192_BYTES'),('NGL-07','PASS'),('SHM_DRAW','COMMANDS_ISSUED_NOT_COMPLETION'),('FENCE_ISSUE','EXPORTED_NOT_COMPLETION'),('FENCE_FACT','SYNC_FILE_SUCCESS_OBSERVED'),('READBACK','EXACT_RGBA_192_BYTES'),('FINAL_PIXEL_ORACLE','PASS'),('NGL-08','PASS_COMPLETION_THEN_LOCAL_RETIREMENT_NO_PRESENTATION'),('WORKER_CLEANUP','CONTEXT_CLOSED_AFTER_OBSERVED_FENCE_LOCAL_RETIREMENT'),('DONE','EIGHT_CASES_CLOSED_NO_SOURCE_OR_PRESENTATION_CLAIM')]
 assert [(v['case'],v['status'])for v in records[10:]]==tail and len(records)==42
 assert c['actor_done_before_EOF']
 assert records[-1]['Root_received_at_ns']<=c['actor_SEQPACKET_EOF']<after['frame']['sampled_at_ns']
 stop=c['executor_stop'];assert records[-1]['Root_received_at_ns']<=c['actor_SEQPACKET_EOF']<=stop['entered_ns']<=stop['returned_ns']<after['frame']['sampled_at_ns']
 assert stop['returned_ns']<=component['completed_at_ns']
 if epoch=='RED':
  assert stop['raw_reply']=='unknown request'and 'parsed'not in stop and not c['executor_finalize']
  assert not c['actor_unload_attempted']and c['actor_unload_reply']is None
  disposal=c['owned_process_disposal'];assert disposal['signal']=='SIGKILL'and disposal['actual_wait_exit']==-9 and disposal['exact_retained_PID']==c['compositor_PID']
  assert stop['returned_ns']<after['frame']['sampled_at_ns']<=disposal['entered_boottime_ns']<=disposal['waited_boottime_ns']<disposal['original_close_until_ns']
  assert disposal['native_cleanup_or_worker_join_qualified']is False
 else:
  keys={'schema','run','status','join_code','joined','worker_alive','released','watches_removed','control_tid','observed_boottime_ns'}
  def reply(v):
   d=v['parsed'];assert set(d)==keys and d['schema']==d['run']==1 and d['control_tid']==c['compositor_PID']
   assert type(d['join_code'])is int and all(type(d[k])is bool for k in ['joined','worker_alive','released','watches_removed'])
   assert v['entered_ns']<=d['observed_boottime_ns']<=v['returned_ns']
  reply(stop);d=stop['parsed'];assert d['status']=='STOP_REQUESTED'and d['join_code']==-1 and not d['joined']and not d['released']and not d['watches_removed']
  assert 0<len(c['executor_finalize'])<=128
  previous=stop['returned_ns']
  for index,v in enumerate(c['executor_finalize']):
   reply(v);assert previous<=v['entered_ns'];previous=v['returned_ns'];d=v['parsed']
   if index+1<len(c['executor_finalize']):assert d['status']=='BUSY'and d['join_code']==2 and not d['joined']and not d['released']and not d['watches_removed']
   else:assert d['status']=='JOINED_AND_RELEASED'and d['join_code']==0 and d['joined']and not d['worker_alive']and d['released']and d['watches_removed']
  assert previous<=c['executor_joined_released_before_unload_ns']<=c['actor_unload_returned_ns']<after['frame']['sampled_at_ns']
  assert c['actor_unload_attempted']and c['actor_unload_reply']=='ok'and c['worker_task_absent_after_unload']and c['owned_process_disposal']is None
 facts={};issued={};lastFact=None;readbacks=[];custody={};commands=[]
 for v in records:
  case=v['case']
  if case=='GL_PROFILE':continue
  if case in ['CUSTODY_BYTES','CUSTODY_PRODUCER']:
   custody[(v['resource_id'],case)]=v
   if case=='CUSTODY_BYTES':assert v['value']==0
  if case=='SHM_DRAW':
   _timed(v);commands.append(v);assert v['command_sequence']==len(commands)
  if case=='FENCE_ISSUE':
   _timed(v);seq=v['fence_sequence'];assert seq not in issued and seq==len(issued)+1;issued[seq]=v
  if case=='FENCE_FACT':
   _timed(v);seq=v['fence_sequence'];assert seq in issued and seq not in facts and v['value']>0
   assert issued[seq]['operation_return_ns']<=v['operation_enter_ns']and v['command_sequence']==issued[seq]['command_sequence']
   facts[seq]=v;lastFact=v
  if case=='READBACK':
   _timed(v);assert lastFact and v['fence_sequence']==lastFact['fence_sequence']and v['command_sequence']==lastFact['command_sequence']
   assert lastFact['operation_return_ns']<=v['operation_enter_ns']and v['value']==0;readbacks.append(v)
  if case.startswith('NGL-')or case=='FINAL_PIXEL_ORACLE':
   _timed(v);assert v['value']==0
  if case=='WRONG_THREAD':assert v['value']>0 and v['value']not in [worker,c['compositor_PID']]
 assert len(commands)==3 and len(facts)==len(issued)==len(readbacks)==6
 for seq in range(1,7):
  v=facts[seq];assert v['resource_id']==(1 if seq==1 else 2)and v['command_sequence']==(1 if seq==1 else 3 if seq==6 else 2)
 assert commands[0]['resource_id']==1 and commands[1]['resource_id']==commands[2]['resource_id']==2
 for resource in [1,2]:
  v=custody[(resource,'CUSTODY_PRODUCER')];_timed(v)
  assert v['operation_return_ns']<=next(x for x in commands if x['resource_id']==resource)['operation_enter_ns']
  assert v['value']==(c['compositor_PID']if epoch=='RED'else worker)if resource==1 else v['value']not in [worker,c['compositor_PID']]
 return {'qualified':True,'kind':('ACTUAL_GL_WITH_MISSING_EXECUTOR_HANDSHAKE_RED'if epoch=='RED'else'ACTUAL_RUST_EXECUTOR_OFFSCREEN_GL_AND_PRE_UNLOAD_JOIN_COMPONENT_GREEN'),'real_CLEAR_RECT_readback_controls':True,'CPU_retained_96_pixel_and_padding_bytes_per_format':True,'AR24_XR24_actual_exact_RGBA_readback':True,'wrong_owner_context_and_allocation_identity_refusals':True,'actual_correlated_sync_file_completion_before_readback':True,'worker_local_helper_retirement_and_context_cleanup':True,'GPU_wide_drain_or_presentation_qualified':False,'Source_repair_qualified':False,'native_AQ_or_full_owner_or_device_retirement_qualified':False}
