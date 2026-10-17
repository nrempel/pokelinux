"""Frozen five-group genuine CPU texture functional success; structural RED is separate."""
import re
PHASE='native-x86-cpu-texture-functional'
TEST_SHA='d8267bc27780d0143696690b62fa364b672c73ad2fb64218d665147276d65efa'
PASS_ROWS=['CPU_TEXTURE_PASS actual_Rust_pixels_after_producer_disposal','CPU_TEXTURE_PASS explicit_opacity_independent_of_AR24_XR24_type','CPU_TEXTURE_PASS sampler_metadata_and_creator_thread_refusals','CPU_TEXTURE_PASS fresh_factory_replacement_keeps_old_admitted_clone_stable','CPU_TEXTURE_PASS legacy_native_and_unsupported_factory_refusals','CPU_TEXTURE_ALL_PASS groups=5']
def thread_rows(rows,main_pid):
 assert len(rows)==8 and rows[:2]==PASS_ROWS[:2]and rows[4:]==PASS_ROWS[2:]
 successor=re.fullmatch(r'CPU_TEXTURE_OBSERVATION successor_tid=([1-9][0-9]*) retired_creator_tid=([1-9][0-9]*) actual_thread_joined=true cpp_identity_reused=(true|false)',rows[2])
 creator=re.fullmatch(r'CPU_TEXTURE_OBSERVATION creator_tid=([1-9][0-9]*) foreign_tid=([1-9][0-9]*) actual_thread_joined=true',rows[3])
 assert successor and creator
 stid,retired=int(successor[1]),int(successor[2]);mtid,ftid=int(creator[1]),int(creator[2])
 assert mtid==main_pid and retired==ftid and len({mtid,ftid,stid})==3 and all(0<t<=2147483647 for t in [mtid,ftid,stid])
 return {'main_tid':mtid,'first_creator_tid':ftid,'successor_tid':stid,'two_actual_joined_threads':True,'cpp_identity_reused_observed':successor[3]=='true'}
def classify(c,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='GREEN'and expected_failure_case is None and controller_exit==0
 assert c['phase']==PHASE and c['epoch']==epoch and c['passed']and c['setup_qualified']and not c['cleanup_errors']and 'failure'not in c
 assert c['frozen_consumer_sha256']==TEST_SHA and c['sole_actual_Rust_archive_sha256']=='246a6680b7f9ed8affd89bd9b022319a0903f5ebc58f03d7d76eaa85f8653311'
 assert c['all_four_actual_dependencies_qualified']and c['actual_three_production_TUs_selected']and c['real_link_and_loader_qualified']and c['actual_compiled_object_count']==8
 assert c['native_GL_EGL_and_renderer_call_symbols_absent']and c['only_Texture_header_overlay_qualified']
 assert c['actual_allowlisted_empty_renderer_wrappers']==['_ZN9Hyprutils6Memory14CUniquePointerIN6Render13IHyprRendererEED1Ev', '_ZN9Hyprutils6Memory14CUniquePointerIN6Render13IHyprRendererEED2Ev']and c['actual_forbidden_native_symbols']==[]
 assert c['structural_control_and_target_GREEN']and c['structural_consumer_sha256']=={'existing-texture-control.cpp':'e4951cb1c9c7b29c20a697564e2d80c20560c47938b9c6049d9b609d4957c981','cpu-texture-public-member.cpp':'1c2cc3e6d0641ba92c7dcf6a890312443a16f1a2b24343c92d1a6cd01bbe6c38'}
 assert set(c['structural_compile_results'])==set(c['structural_consumer_sha256'])and all(z=={'actual_exit':0,'diagnostic_bytes':0}for z in c['structural_compile_results'].values())
 assert all(k in c['dependency_closure']for k in ['signature0','signature1'])
 assert c['program_exit']==0
 receipt=c['program_receipt'];actual=thread_rows(c['program_rows'],receipt['pid']);assert actual==c['actual_joined_thread_observations']
 assert receipt in c['owned_process_receipts']and receipt['actual_direct_parent_wait']and receipt['stdout_EOF']and receipt['wait_before_original_cutoff']and receipt['EOF_before_original_cutoff']and not receipt['output_overflow']
 assert c['original_program_until_ns']-c['program_started_boottime_ns']==5000000000 and receipt['waited_at_boottime_ns']<c['original_program_until_ns']and receipt['stdout_EOF_at_ns']<c['original_program_until_ns']
 assert all(z['actual_direct_parent_wait']and z['stdout_EOF']and z['wait_before_original_cutoff']and z['EOF_before_original_cutoff']and not z['output_overflow']for z in c['owned_process_receipts'])
 return {'qualified':True,'classification':'REAL_CPU_TEXTURE_FIVE_GROUP_GREEN','two_actual_joined_threads':True,'full_Wayland_FramePlan_worker_GPU_Source_ARM_or_phone_qualified':False}
