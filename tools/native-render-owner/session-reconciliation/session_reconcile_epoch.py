"""Frozen four-name missing-interface structural RED, never semantic/native proof."""
import re
NAMES=['pr_executor_create_v2','pr_executor_invalidate_session_v2','pr_executor_try_session_change_v2','pr_executor_reconciliation_v2']
def diagnostics(body,source,path,header):
 assert 0<len(body.encode())<=1048576 and '\x1b'not in body and '\r'not in body
 lines=source.splitlines();locations={name:next(i+1 for i,line in enumerate(lines)if '&'+name+';'in line)for name in NAMES}
 declared=set(re.findall(r'\b(pr_[A-Za-z0-9_]+)\s*\(',header));out=[]
 for raw in body.splitlines():
  if raw==path+': At global scope:':continue
  match=re.fullmatch(r'(.+):(\d+):(\d+): (error|fatal error|warning|note): (.+)',raw);assert match,'unexpected compiler output: '+raw
  file,line,col,kind,message=match.groups();message=message.replace('‘',"'").replace('’',"'").replace('`',"'")
  missing=re.fullmatch(r"'([A-Za-z0-9_]+)' was not declared in this scope(?:; did you mean '([A-Za-z0-9_]+)'\?)?",message)
  assert missing and kind=='error'and file==path and missing[1]in NAMES and int(line)==locations[missing[1]]and 1<=int(col)<=len(lines[int(line)-1])+1
  assert missing[2]is None or missing[2]in declared
  out.append({'name':missing[1],'line':int(line),'column':int(col),'message':message,'category':'MISSING_PUBLIC_SESSION_RECONCILIATION_FUNCTION'})
 assert len(out)==4 and {v['name']for v in out}==set(NAMES)
 return out
