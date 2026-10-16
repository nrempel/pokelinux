"""Actual frozen real-broker consumer phase only, independent of compile RED."""
PHASE='native-x86-aq15-initial-render-batch-real-broker'
CONSUMER_SHA='2976ed3145590f574476e3fcfef237847722623fedecd1cd14a69de0ea91f423'
RUST='/rust/libpokelinux_render.a'

def classify(component,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='GREEN' and expected_failure_case is None and controller_exit==0
 assert component['schema']==1 and component['phase']==PHASE and component['passed']is True
 assert component['classification']=='REAL_BROKER_INITIAL_BATCH_CONSUMER_GREEN_ONLY'
 assert component['consumer_sha256']==CONSUMER_SHA and component['cleanup_errors']==[] and 'setup_or_runtime_failure'not in component
 assert component['phone_accessed']is False and component['main_GL_actor_or_Source_selected']is False
 assert component['worker_mirror_output_discovery_or_service_retirement_qualified']is False
 assert component['strict_baseline_compile_only']is True and component['actual_consumer_exit']==0
 assert component['consumer_LD_interposition_variables_absent']is True and component['seatd_socket_GID']==1000
 argv=component['actual_compile_argv'];assert argv.count(RUST)==1 and [v for v in argv if v.endswith('.a')]==[RUST]
 assert '-lhyprutils'in argv and '-laquamarine'in argv and not any('libaquamarine.so.14'in v for v in argv)
 assert component['actual_AQ_library']['sha256']=='323e1cb3120162338ee46cde029038677c0abb4214e152ce8848245d4d16b148'
 launch=component['original_consumer_outer_started_ns'];until=component['original_consumer_outer_until_ns'];assert until==launch+20_000_000_000
 rows=component['consumer_records'];assert len(rows)==4 and [v.get('event',v.get('status'))for v in rows]==['SETUP','RUST_CAPTURE','PREWORKER_REVOKE','PASS']
 setup,capture,revoke,passed=rows;assert setup['qualified']is True and 0<setup['actual_service_devices']<=32
 start=setup['work_until_ns']-5_000_000_000;assert launch<=start<until and setup['close_until_ns']==start+8_000_000_000
 assert capture['code']==0 and capture['no_queue_mark_yet']is True and capture['at_ns']<setup['work_until_ns']
 assert revoke['result']==3 and revoke['terminal_latched']is True and revoke['service_retirement_qualified']is False
 assert passed['Rust_capture_before_marks']is True and passed['worker_or_output_discovery_qualified']is False and passed['service_lease_retirement_qualified']is False and passed['events']==passed['devices']+2
 receipts=component['owned_process_receipts'];assert 0<len(receipts)<=64
 assert all(v['actual_direct_parent_wait']and v['wait_before_original_cutoff']and v['stdout_EOF']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in receipts)
 assert all(v['returncode']==0 for v in receipts if v['role']!='private-seatd')
 actual=component['actual_consumer_wait_EOF_receipt'];assert actual in receipts and actual['role']=='actual-real-broker-consumer'and actual['returncode']==0
 assert actual['original_cutoff_ns']==actual['original_wait_cutoff_ns']==actual['spawn_admission_cutoff_ns']==until
 assert actual['waited_at_boottime_ns']<until and actual['stdout_EOF_at_ns']<until
 assert component['completed_boottime_ns']<component['original_close_until_ns']
 return {'qualified':True,'classification':'REAL_BROKER_INITIAL_BATCH_CONSUMER_GREEN_ONLY','runtime_initial_batch_PASS':True,'production_semantic_RED':False,'Source_GREEN':False,'worker_or_output_discovery_qualified':False,'service_lease_retirement_qualified':False}
