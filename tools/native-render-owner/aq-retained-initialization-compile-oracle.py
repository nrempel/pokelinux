"""Frozen public AQ retained-initialization members compile RED only."""
import re
PHASE='native-x86-aq-retained-initialization-compile-red'
CLASSIFICATION='STRUCTURAL_COMPILATION_RED'

def compiler_diagnostics(body,source,source_path):
 assert type(body)is str and 0<len(body.encode())<=1048576 and '\x1b'not in body and '\r'not in body
 lines=source.splitlines();raw=body.splitlines();assert 0<len(raw)<=32;out=[]
 expected={
  'createRenderSessionHolder':'static_assert(std::is_same_v<decltype(&Aquamarine::CBackend::createRenderSessionHolder), ExpectedHolderFactory>);',
  'initializeRenderSession':'static_assert(std::is_same_v<decltype(&Aquamarine::CBackend::initializeRenderSession), ExpectedInitialize>);'}
 positions={}
 for name,text in expected.items():
  found=[i+1 for i,v in enumerate(lines)if v==text];assert len(found)==1;positions[found[0]]=name
 parse=re.compile(r'^(.+):([0-9]+):([0-9]+): (fatal error|error|warning|note): (.+)$')
 for record in raw:
  if record==source_path+': At global scope:':continue
  matched=parse.fullmatch(record);assert matched,'unparsed compiler output: '+record
  path,line,column,kind,message=matched.groups();line=int(line);column=int(column)
  assert path==source_path and line in positions and 1<=column<=len(lines[line-1])+1 and kind=='error'
  name=positions[line];message=message.replace('‘',"'").replace('’',"'").replace('`',"'")
  primary=message in ["'"+name+"' is not a member of 'Aquamarine::CBackend'","'"+name+"' is not a member of 'CBackend'"]
  dependent=message=='template argument 1 is invalid'
  assert primary or dependent,'unrelated compiler diagnostic: '+message
  out.append({'kind':kind,'message':message,'file':path,'line':line,'column':column,'source_line':lines[line-1],'category':'MISSING_'+name if primary else'DEPENDENT_INVALID_DECLTYPE'})
 for name in expected:assert any(v['category']=='MISSING_'+name for v in out),'missing primary diagnostic for '+name
 return out

def classify(component,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='RED'and expected_failure_case is None and controller_exit==0
 assert component['phase']==PHASE and component['epoch']=='RED'and component['passed']is True
 assert component['classification']==CLASSIFICATION and component['setup_qualified']is True
 assert component['cleanup_errors']==[]and 'failure'not in component
 assert component['baseline_syntax_exit']==0 and component['baseline_diagnostic_bytes']==0
 assert component['target_syntax_exit']==1 and component['syntax_only_no_link_or_execution']is True
 assert component['both_dependency_sets_qualified']is True and component['compositor_Source_UI_GL_actor_selected']is False
 for name in ['createRenderSessionHolder','initializeRenderSession']:assert any(v['category']=='MISSING_'+name for v in component['diagnostics'])
 assert all(v['actual_direct_parent_wait']and v['wait_before_original_cutoff']and v['stdout_EOF']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in component['owned_process_receipts'])
 assert component['completed_boottime_ns']<component['original_close_until_ns']
 return {'qualified':True,'classification':CLASSIFICATION,'both_missing_API_members':True,'production_semantic_RED':False,'native_custody_or_runtime_GREEN':False,'Source_GREEN':False}
