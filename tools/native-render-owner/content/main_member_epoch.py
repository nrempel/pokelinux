"""Exact frozen real CPU custody/geometry functional epoch; setup errors never count as RED."""
PHASE='native-x86-cpu-surface-content-functional'
TEST_SHA='16f7aaec47daf5e913608867f0e68f77501dfe8f06435c5074fc1ed14e5ad3b3'
PREREQUISITES='CPU_CONTENT_PREREQUISITES_PASS actual_Rust_snapshot=true client_buffer_absent=true producer_overwritten_unmapped_closed=true width=5 height=3 stride=32 retained_bytes=96'
RED_ROWS=['CPU_CONTENT_PASS empty_zero',PREREQUISITES,'CPU_CONTENT_GEOMETRY name=retained_shm_geometry expected=5,3 actual=0,0','CPU_CONTENT_FAIL retained_shm_geometry']
GREEN_ROWS=['CPU_CONTENT_PASS empty_zero',PREREQUISITES,'CPU_CONTENT_PASS retained_shm_geometry_and_producer_disposal','CPU_CONTENT_PASS no_attach_update_and_pending_reset','CPU_CONTENT_PASS viewport_rotation_scale_geometry','CPU_CONTENT_PASS attach_null_fresh_import_and_current_reset','CPU_CONTENT_ALL_PASS groups=5']
def classify(component,controller_exit,epoch,expected_failure_case=None):
 assert epoch in ['RED','GREEN']and expected_failure_case is None and controller_exit==0
 assert component['phase']==PHASE and component['epoch']==epoch and component['passed']and component['setup_qualified']and not component['cleanup_errors']and 'failure'not in component
 assert component['frozen_consumer_sha256']==TEST_SHA and component['sole_actual_Rust_archive_sha256']=='246a6680b7f9ed8affd89bd9b022319a0903f5ebc58f03d7d76eaa85f8653311'
 assert component['all_four_actual_dependencies_qualified']and component['actual_three_production_TUs_selected']and component['real_link_and_loader_qualified']and component['legacy_GL_upload_discarded']
 assert component['program_rows']==(RED_ROWS if epoch=='RED'else GREEN_ROWS)and component['program_exit']==(1 if epoch=='RED'else 0)
 r=component['program_receipt'];assert r in component['owned_process_receipts']and r['actual_direct_parent_wait']and r['stdout_EOF']and r['wait_before_original_cutoff']and r['EOF_before_original_cutoff']and not r['output_overflow']
 assert component['original_program_until_ns']-component['program_started_boottime_ns']==5000000000 and r['waited_at_boottime_ns']<component['original_program_until_ns']and r['stdout_EOF_at_ns']<component['original_program_until_ns']
 assert all(z['actual_direct_parent_wait']and z['stdout_EOF']and z['wait_before_original_cutoff']and z['EOF_before_original_cutoff']and not z['output_overflow']for z in component['owned_process_receipts'])
 return {'qualified':True,'classification':'REAL_CPU_SURFACE_GEOMETRY_RED'if epoch=='RED'else'REAL_CPU_SURFACE_CONTENT_FIVE_GROUP_GREEN','real96byte_Rust_custody_prerequisites_passed':True,'full_Wayland_GPU_Source_or_phone_qualified':False}
