#!/usr/bin/env python3
"""Index the actual chronological held-initialization launch RED precisely."""
import argparse
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'artifacts/native-display-app-generations'
def sha(path):
 digest=hashlib.sha256()
 with Path(path).open('rb') as stream:
  while block:=stream.read(1048576):digest.update(block)
 return digest.hexdigest()
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--label',required=True);parser.add_argument('--output',required=True,type=Path);args=parser.parse_args();assert not args.output.exists()
 out=BASE/args.label;s=json.loads((out/'selection.json').read_text());r=json.loads((out/'result.json').read_text())
 assert r['desired_boundary_red'] and r['prerequisites_qualified'] and not r['passed'] and r['all_tracked_children_finally_absent_or_directly_waited']
 assert len(r['groups'])==5 and sum(g['passed'] for g in r['groups'])==4
 actual=r['actual_premature_launch'];assert actual['display']['state']=='Initializing' and not actual['display']['admission'] and actual['display']['occupied']
 assert actual['rpc']['reply']=={'result':'ok'} and actual['snapshot']['reply']['principals']==1 and actual['app_uid']==11010
 assert not any(v['event']=='display' and v['state']=='Ready' for v in r['source_events'])
 source=r['groups'][0];assert [source[k]['revision'] for k in ['first','second','third']]==[1,2,3]
 assert all(source[k]['until']==source[k]['sampled_at_ns']+100000000 for k in ['first','second','third'])
 for n,d in s['files'].items():assert sha(out/n)==d,n
 for n,d in s['rust_selection']['sources'].items():assert sha(out/'source/rust-build'/n)==d,n
 own=Path(__file__).parent
 value={'scope':r['boundary_scope'],'selected_label':args.label,'result_sha256':sha(out/'result.json'),'selection_sha256':sha(out/'selection.json'),'source':r['source'],'actual_source_updates':{k:source[k] for k in ['first','second','third']},'actual_premature_launch':actual,'Stop_ACK':r['whole_Stop_ACK'],'groups':{'passed_controls':4,'desired_readiness_boundary_failed':1},'old_service_epoch_contract_not_retroactively_invalidated':True,'production_changes_after_actual_RED':True,'selected_service_elf_sha256':s['service_elf_sha256'],'selected_source_library_sha256':s['rust_selection']['sources']['native-session/src/lib.rs'],'normal_Qt_ELF_sha256':s['normal_qt_elf_sha256'],'no_mapped_Qt_or_history_view_claim':True,'cleanup':{'tracked_unique_pids':len({v['pid'] for v in r['cleanup']}),'all_direct_children_waited_or_helpers_finally_absent':r['all_tracked_children_finally_absent_or_directly_waited'],'qualification':'Python actually waits its direct children, including the service. For SDK/app/policy/writer/reader helpers this case records pidfd death and proc absence after service completion; no helper parent wait syscall or individual app-parent inference is independently qualified here. Generic inherited row wording referring to app-parent inference does not extend this evidence.'},'proof_checksums':{str(q.relative_to(ROOT)):sha(q) for q in sorted(out.rglob('*')) if q.is_file()},'current_authored_tools_checksums':{str(q.relative_to(ROOT)):sha(q) for q in sorted(own.glob('init-red-*')) if q.is_file() and q!=args.output.resolve()},'limits':['Only actual Source focus revisions, policy readiness, held SDK Initializing, premature Launch/principal and responsive Snapshot/Stop are selected.','No fake SDK/SourceFrame, live ptrace, Source reset/reload, physical phone or vendor runtime.','Actual SIGSTOP is not an unreapable drain; native Stop kills/reaps rather than waiting for a cooperative resume.','Observed RPC times qualify this epoch; no broader hard realtime bound or exhausted-budget case.']}
 args.output.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n');print(json.dumps({'evidence':str(args.output),'sha256':sha(args.output),'proof_paths':len(value['proof_checksums'])},indent=2))
if __name__=='__main__':main()
