"""One frozen missing-API compile RED; never a production semantic/runtime result."""
import re
from pathlib import Path

PHASE='native-x86-aq-initial-batch-compile-red'
CLASSIFICATION='STRUCTURAL_COMPILATION_RED'

def compiler_diagnostics(value, source, source_path):
    assert type(value) is str and 0 < len(value.encode()) <= 1048576
    assert "\x1b" not in value and "\r" not in value
    lines=source.splitlines(); raw=value.splitlines(); assert 0<len(raw)<=512
    flat=[]; headings=[]; missing_type_unused=False
    diagnostic=re.compile(r"^(.+):([0-9]+):([0-9]+): (fatal error|error|warning|note): (.+)$")
    context=re.compile(re.escape(source_path)+r": (?:At global scope:|In function '.+':|In lambda function:)$")
    for record in raw:
        if record=='cc1plus: all warnings being treated as errors':
            assert missing_type_unused, 'unexpected warning summary'
            continue
        matched=diagnostic.fullmatch(record)
        if matched is None:
            assert context.fullmatch(record), 'unparsed compiler output: '+record
            headings.append(record); continue
        path,line,column,kind,msg=matched.groups();line=int(line);column=int(column)
        assert path==source_path and kind=='error', 'external/warning/note/fatal diagnostic'
        assert 1<=line<=len(lines) and 1<=column<=len(lines[line-1])+1
        assert 0<len(msg)<=2048
        text=lines[line-1]; normalized=msg.replace('‘',"'").replace('’',"'").replace('`',"'")
        missing_enum='eSessionInitialRenderEventsResult' in normalized and ('does not name a type' in normalized or 'not been declared' in normalized or 'not declared' in normalized)
        missing_member='peekInitialRenderEvents' in normalized and ('not a member' in normalized or 'no member named' in normalized)
        symbol=re.fullmatch(r"(?:'?(?:Result|Signature|wrongResult)'? (?:does not name a type|has not been declared|was not declared in this scope)|expected type-specifier before '?Result'?)", normalized)
        template=re.fullmatch(r'template argument [12] is invalid',normalized)
        dependent=bool(symbol) and any(v in text for v in ['Result','Signature','wrongResult'])
        dependent_template=bool(template) and ('static_assert' in text or 'Signature' in text or 'Result' in text)
        dependent_semicolon=normalized=="expected ';' before 'wrongResult'" and 'Result wrongResult=' in text
        dependent_namespace=normalized=="'Result' is not a class, namespace, or enumeration" and 'Result::' in text
        dependent_unused=normalized=="'void {anonymous}::refusal(Aquamarine::CSession&, int, size_t)' defined but not used [-Werror=unused-function]" and text=='void refusal(CSession& broker,Result expected,size_t capacity){'
        if dependent_unused:
            assert not missing_type_unused
            missing_type_unused=True
        assert missing_enum or missing_member or dependent or dependent_template or dependent_semicolon or dependent_namespace or dependent_unused, 'unclassified compiler error: '+normalized
        assert not missing_enum or 'eSessionInitialRenderEventsResult' in text, 'enum diagnostic not on primary reference'
        assert not missing_member or 'peekInitialRenderEvents' in text, 'member diagnostic not on primary reference'
        flat.append({'kind':'error','message':msg,'file':path,'line':line,'column':column,'source_line':text,'category':'MISSING_ENUM' if missing_enum else 'MISSING_MEMBER' if missing_member else 'DEPENDENT_MISSING_TYPE'})
    assert 0<len(flat)<=256
    assert any(v['category']=='MISSING_ENUM' for v in flat), 'missing primary enum diagnostic'
    assert any(v['category']=='MISSING_MEMBER' for v in flat), 'missing primary member diagnostic'
    return flat

def classify(component, controller_exit, epoch, expected_failure_case=None):
    assert epoch=='RED' and expected_failure_case is None and controller_exit==0
    assert component['phase']==PHASE and component['epoch']=='RED'
    assert component['classification']==CLASSIFICATION and component['setup_qualified'] is True
    assert component['passed'] is True and component['cleanup_errors']==[] and 'failure' not in component
    assert component['compile_only_current_consumer'] is True and component['missing_api_compile_exit']==1
    assert component['target_object_absent'] is True and component['consumer_executed'] is False
    assert component['compositor_Source_UI_GL_actor_selected'] is False
    assert component['baseline_actual_exit']==0 and component['baseline_record']['event']=='EXISTING_API_INCLUDE_LINK_SETUP_PASS'
    assert component['Rust_archive_occurrences_in_baseline_link']==1 and component['AQ15_loader_qualified'] is True
    categories={v['category'] for v in component['diagnostics']}
    assert {'MISSING_ENUM','MISSING_MEMBER'} <= categories
    receipts=component['owned_process_receipts']; assert receipts
    assert all(v['actual_direct_parent_wait'] and v['wait_before_original_cutoff'] and v['stdout_EOF'] and v['EOF_before_original_cutoff'] and not v['output_overflow'] for v in receipts)
    assert component['completed_boottime_ns'] < component['original_close_until_ns']
    return {'qualified':True,'classification':CLASSIFICATION,'production_semantic_RED':False,'runtime_initial_batch_PASS':False,'Source_GREEN':False}
