"""Same genuine probe TU profiles after only a byte-exact outer test guard."""
PHASE='native-x86-normal-profile-source-configuration-compile-green'
CLASSIFICATION='NORMAL_PROFILE_SOURCE_CONFIGURATION_GREEN'
MACRO='-DPOKELINUX_STARTUP_CUSTODY_TEST'
def classify(c,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='GREEN'and expected_failure_case is None and controller_exit==0
 assert c['phase']==PHASE and c['epoch']=='GREEN'and c['passed']and c['setup_qualified']and not c['cleanup_errors']and 'failure'not in c
 assert c['classification']==CLASSIFICATION and c['baseline_syntax_exit']==0 and c['baseline_diagnostic_bytes']==0 and c['target_syntax_exit']==0 and c['target_diagnostic_bytes']==0 and c['diagnostics']==[]
 assert c['syntax_only_no_link_or_execution']and not c['compositor_Source_UI_GL_actor_selected']and not c['production_semantic_RED']
 control,normal=c['actual_profile_compile_commands']['test_macro'],c['actual_profile_compile_commands']['normal']
 assert control[1]==MACRO and control[:1]+control[2:]==normal and normal.count('-fmax-errors=1')==1 and '-fsyntax-only'in normal and '-Werror'in normal and not any('POKELINUX_STARTUP_CUSTODY_TEST'in z for z in normal)
 assert c['same_authentic_source_warning_context_for_both_profiles']and c['full_test_macro_dependency_set_qualified']and c['only_compile_profile_delta_is_test_macro']
 assert c['actual_source_TU_sha256']=='ebcf1ecf92f1d77cc56a56cc0600a010f6b84242f652b786ca1faf2155c8c822'and c['actual_consumer_inc_sha256']=='f07276f6f8bc66cb3494ee0a099e1d7467607b25198da1c7c7e0f5a7e9ff79d2'and c['byte_exact_original_TU_inside_outer_guard']
 assert len(c['dependency_closure']['test-macro-profile']['selected_private_SDK_headers'])==16
 normal_deps=c['dependency_closure']['normal-profile'];assert normal_deps['selected_private_SDK_headers']=={}and normal_deps['actual_source_only']and not normal_deps['consumer_inc_selected']and c['normal_dependency_file_emitted']
 assert normal_deps['actual_dependency_paths']==[normal[-1],'/usr/include/stdc-predef.h']and normal_deps['implicit_toolchain_predefinition_header']['path']=='/usr/include/stdc-predef.h'and normal_deps['implicit_toolchain_predefinition_header']['bytes']>0
 assert all(v['actual_direct_parent_wait']and v['wait_before_original_cutoff']and v['stdout_EOF']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in c['owned_process_receipts'])
 assert c['completed_boottime_ns']<c['original_close_until_ns']
 return {'qualified':True,'classification':CLASSIFICATION,'test_macro_profile_compiles_with_original_test_body':True,'normal_profile_compiles_with_project_source_and_exact_toolchain_predefinition_only':True,'production_semantic_or_native_startup_Wayland_worker_Source_or_phone_qualified':False}
