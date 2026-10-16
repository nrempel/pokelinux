"""Frozen public CCompositor member compilation RED only; no execution or link."""
import re
PHASE='native-x86-main-startup-public-member-compile-red'
CLASSIFICATION='STRUCTURAL_COMPILATION_RED'

def compiler_diagnostics(body, source, source_path):
 assert type(body)is str and 0<len(body.encode())<=1048576 and '\x1b'not in body and '\r'not in body
 lines=source.splitlines();raw=body.splitlines();assert 0<len(raw)<=32;out=[]
 expected='static_assert(std::is_same_v<decltype(&CCompositor::ownedBackendStartupReply), OwnedStartupReply>);'
 target=[i+1 for i,text in enumerate(lines)if text==expected];assert len(target)==1
 parse=re.compile(r'^(.+):([0-9]+):([0-9]+): (fatal error|error|warning|note): (.+)$')
 for text in raw:
  if text==source_path+': At global scope:':continue
  match=parse.fullmatch(text);assert match,'unparsed compiler output: '+text
  path,line,column,kind,message=match.groups();line=int(line);column=int(column)
  assert path==source_path and line==target[0]and 1<=column<=len(expected)+1 and kind=='error'
  message=message.replace('‘',"'").replace('’',"'").replace('`',"'")
  primary=message=="'ownedBackendStartupReply' is not a member of 'CCompositor'"
  dependent=message=='template argument 1 is invalid'
  assert primary or dependent,'unrelated compiler diagnostic: '+message
  out.append({'kind':kind,'message':message,'file':path,'line':line,'column':column,'source_line':expected,'category':'MISSING_PUBLIC_MEMBER'if primary else'DEPENDENT_INVALID_DECLTYPE'})
 assert any(v['category']=='MISSING_PUBLIC_MEMBER'for v in out)
 return out

def classify(component,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='RED'and expected_failure_case is None and controller_exit==0
 assert component['phase']==PHASE and component['epoch']=='RED'and component['passed']is True
 assert component['classification']==CLASSIFICATION and component['setup_qualified']is True
 assert component['cleanup_errors']==[]and 'failure'not in component
 assert component['baseline_syntax_exit']==0 and component['baseline_diagnostic_bytes']==0
 assert component['target_syntax_exit']==1 and component['syntax_only_no_link_or_execution']is True
 assert component['both_dependency_sets_qualified']is True
 assert component['compositor_Source_UI_GL_actor_selected']is False
 assert any(v['category']=='MISSING_PUBLIC_MEMBER'for v in component['diagnostics'])
 assert all(v['actual_direct_parent_wait']and v['wait_before_original_cutoff']and v['stdout_EOF']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in component['owned_process_receipts'])
 assert component['completed_boottime_ns']<component['original_close_until_ns']
 return {'qualified':True,'classification':CLASSIFICATION,'production_semantic_RED':False,'link_or_main_startup_GREEN':False,'worker_or_Source_GREEN':False}
