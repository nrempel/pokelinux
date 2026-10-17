"""Frozen two missing CPU texture freefunctions: structural compilation RED only."""
import re
PHASE='native-x86-cpu-texture-public-api-compile-red'
CLASSIFICATION='STRUCTURAL_COMPILATION_RED'
FUNCTIONS=['createCPUTexture','borrowCPUTextureSnapshot']
def compiler_diagnostics(body,source,source_path):
 assert type(body)is str and 0<len(body.encode())<=1048576 and '\x1b'not in body and '\r'not in body
 lines=source.splitlines();allowed={};primary={}
 def own(statement,messages):
  matches=[i+1 for i,line in enumerate(lines)if line==statement];assert len(matches)==1;allowed[matches[0]]=messages;return matches[0]
 for alias,name in [('Factory',FUNCTIONS[0]),('Borrow',FUNCTIONS[1])]:
  line=own('using '+alias+' = decltype(&Render::'+name+');',{"'"+name+"' is not a member of 'Render'"});primary[line]=name
 for statement,alias in [
  ('static_assert(std::is_same_v<Factory, int32_t (*)(const Snapshot&, bool, SP<Texture>&) noexcept>);','Factory'),
  ('static_assert(std::is_same_v<Borrow, const Snapshot* (*)(const Texture&) noexcept>);','Borrow'),
  ('static_assert(!std::is_invocable_v<Factory, const Aquamarine::SDMABUFAttrs&, bool, SP<Texture>&>);','Factory'),
  ('static_assert(!std::is_invocable_v<Factory, std::span<const float>, bool, SP<Texture>&>);','Factory')]:
  own(statement,{"'"+alias+"' was not declared in this scope",'template argument 1 is invalid'})
 raw=body.splitlines();assert 0<len(raw)<=64;out=[];parse=re.compile(r'^(.+):([0-9]+):([0-9]+): (fatal error|error|warning|note): (.+)$')
 for text in raw:
  if text==source_path+': At global scope:':continue
  match=parse.fullmatch(text);assert match,'unparsed compiler output: '+text
  path,line,column,kind,message=match.groups();line=int(line);column=int(column);message=message.replace('‘',"'").replace('’',"'").replace('`',"'")
  assert path==source_path and line in allowed and 1<=column<=len(lines[line-1])+1 and kind=='error'
  assert message in allowed[line],'unrelated compiler diagnostic: '+message
  out.append({'kind':kind,'message':message,'file':path,'line':line,'column':column,'source_line':lines[line-1],'category':'MISSING_PUBLIC_FUNCTION'if line in primary else'DEPENDENT_MISSING_ALIAS','function':primary.get(line)})
 assert {v['function']for v in out if v['category']=='MISSING_PUBLIC_FUNCTION'}==set(FUNCTIONS)
 return out

def classify(component,controller_exit,epoch,expected_failure_case=None):
 assert epoch=='RED'and expected_failure_case is None and controller_exit==0
 assert component['phase']==PHASE and component['epoch']=='RED'and component['passed']is True
 assert component['classification']==CLASSIFICATION and component['setup_qualified']is True
 assert component['cleanup_errors']==[]and 'failure'not in component
 assert component['baseline_syntax_exit']==0 and component['baseline_diagnostic_bytes']==0
 assert component['target_syntax_exit']==1 and component['syntax_only_no_link_or_execution']is True
 assert component['both_dependency_sets_qualified']is True and component['compositor_Source_UI_GL_actor_selected']is False
 assert {v['function']for v in component['diagnostics']if v['category']=='MISSING_PUBLIC_FUNCTION'}==set(FUNCTIONS)
 assert all(v['actual_direct_parent_wait']and v['wait_before_original_cutoff']and v['stdout_EOF']and v['EOF_before_original_cutoff']and not v['output_overflow']for v in component['owned_process_receipts'])
 assert component['completed_boottime_ns']<component['original_close_until_ns']
 return {'qualified':True,'classification':CLASSIFICATION,'missing_public_functions':FUNCTIONS,'production_semantic_RED':False,'link_or_main_startup_GREEN':False,'worker_or_Source_GREEN':False}
