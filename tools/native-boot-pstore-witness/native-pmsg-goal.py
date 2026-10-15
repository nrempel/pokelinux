"""Require complete native PMSG retention; absence must fail this fixed goal."""
from pathlib import Path
import argparse,hashlib,importlib.util,json
p=argparse.ArgumentParser();p.add_argument('--case-index',type=Path,required=True);p.add_argument('--private-result',type=Path,required=True);args=p.parse_args()
assert not args.private_result.exists()
rows=json.loads(args.case_index.read_text());cases={};observer=None;observer_hash=None
for row in rows:
 root=Path(row['path']);digest=lambda q:hashlib.sha256(q.read_bytes()).hexdigest()
 assert digest(root/'selection.json')==row['selection_sha256'] and digest(root/'command.json')==row['command_sha256']
 selection=json.loads((root/'selection.json').read_text())
 for name,h in selection['local_inputs'].items():assert digest(root/name)==h
 for name,h in selection['external_inputs'].items():assert digest(Path(name))==h
 h=digest(root/'source/observer.py')
 if observer is None:
  observer_hash=h;spec=importlib.util.spec_from_file_location('fixed_native_retention_observer',root/'source/observer.py');observer=importlib.util.module_from_spec(spec);spec.loader.exec_module(observer)
 else:assert h==observer_hash
 result=json.loads((root/'result.json').read_text());execution=json.loads((root/'execution.json').read_text());driver=json.loads((root/'Root-driver-execution.json').read_text())
 assert result['passed'] and execution['passed'] and execution['runtime_closure_passed']
 assert driver['actual_owned_driver_wait_exit']==0 and driver['stdout_EOF'] and driver['stderr_EOF']
 assert execution['qmp_terminal']['successful_quit_reply'] and execution['owned_disposal_and_input_closure_before_original_cutoff']
 raw=observer.inspect_native((root/'console.log').read_text(),json.loads((root/'qmp-records.json').read_text()),execution['qmp_commands'],row['case_id'])
 assert raw['native_pmsg_goal']==result['native_pmsg_goal']
 cases[row['case_id']]=result
classification=observer.classify_native_cases(cases,fixed_source_bindings_verified=True)
args.private_result.write_text(json.dumps({'observer_sha256':observer_hash,'case_index_sha256':hashlib.sha256(args.case_index.read_bytes()).hexdigest(),'actual_prerequisites_qualified':True,'classification':classification},indent=2)+'\n')
assert classification['qualified_GREEN'],classification['reason']
