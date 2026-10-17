"""Real StartupCustodyProbe TU: test macro control versus exact normal #error."""
import re
PHASE='native-x86-normal-profile-source-configuration-compile-red'
CLASSIFICATION='NORMAL_PROFILE_SOURCE_CONFIGURATION_RED'
MACRO='-DPOKELINUX_STARTUP_CUSTODY_TEST'
ERROR='#error "StartupCustodyProbe is main-linked test infrastructure only"'
def compiler_diagnostics(body,source_path,header_path):
 assert type(body)is str and 0<len(body.encode())<=1048576 and '\x1b'not in body and '\r'not in body
 rows=body.splitlines();assert len(rows)==3
 assert rows[0]=='In file included from '+source_path+':1:'
 match=re.fullmatch(re.escape(header_path)+r':3:([0-9]+): (error): (.+)',rows[1]);assert match
 assert int(match[1])==2 and match[2]=='error'and match[3]==ERROR
 assert rows[2]=='compilation terminated due to -fmax-errors=1.'
 return [{'kind':'error','message':ERROR,'file':header_path,'line':3,'column':2,'category':'UNGUARDED_TEST_ONLY_PROBE_HEADER'},{'kind':'termination','message':rows[2],'category':'EXACT_BOUNDED_COMPILER_TERMINATION'}]
def classify(c,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='RED'and expected_failure_case is None and controller_exit==0
 assert c['phase']==PHASE and c['epoch']=='RED'and c['passed']and c['setup_qualified']and not c['cleanup_errors']and 'failure'not in c
 assert c['classification']==CLASSIFICATION and c['baseline_syntax_exit']==0 and c['baseline_diagnostic_bytes']==0 and c['target_syntax_exit']==1
 assert c['syntax_only_no_link_or_execution']and not c['compositor_Source_UI_GL_actor_selected']and not c['production_semantic_RED']
 control,normal=c['actual_profile_compile_commands']['test_macro'],c['actual_profile_compile_commands']['normal']
 assert control[1]==MACRO and control[:1]+control[2:]==normal and normal.count('-fmax-errors=1')==1 and '-fsyntax-only'in normal and '-Werror'in normal and not any('POKELINUX_STARTUP_CUSTODY_TEST'in z for z in normal)
 assert c['same_authentic_source_warning_context_for_both_profiles']and c['full_test_macro_dependency_set_qualified']and c['only_compile_profile_delta_is_test_macro']
 assert c['actual_source_TU_sha256']=='914e2c17afeeab8eece5e3f5f3064b8833c1afa140b3de03c50eb4e8093dfece'and c['actual_consumer_inc_sha256']=='f07276f6f8bc66cb3494ee0a099e1d7467607b25198da1c7c7e0f5a7e9ff79d2'
 assert c['diagnostics']==compiler_diagnostics(c['actual_normal_diagnostic_text'],normal[-1],c['actual_test_only_header_path'])
 assert all(v['actual_direct_parent_wait']and v['wait_before_original_cutoff']and v['stdout_EOF']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in c['owned_process_receipts'])
 assert c['completed_boottime_ns']<c['original_close_until_ns']
 return {'qualified':True,'classification':CLASSIFICATION,'test_macro_profile_compiles':True,'normal_profile_reaches_exact_test_only_header_error':True,'production_semantic_RED':False,'link_native_startup_Wayland_Source_or_phone_qualified':False}
