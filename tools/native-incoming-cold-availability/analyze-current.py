#!/usr/bin/env python3
"""Interpret diagnostic observations without converting failed epochs to passes."""
import argparse,hashlib,json
from pathlib import Path

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def summarize(out):
 rows=json.loads((out/'timings.json').read_text());result=json.loads((out/'result.json').read_text())
 stop_at=next((v['boottime_ns'] for v in result['source_events'] if v['event']=='diagnostic_source_stop'),None)
 if stop_at is not None:rows=[v for v in rows if v['values'][1 if v['role']=='observer' else 2]<stop_at]
 publishers=[v['values'] for v in rows if v['role']=='publisher'];polls=[v['values'] for v in rows if v['role']=='observer']
 assert all(v[0]==2 and len(v)==8 for v in publishers)
 assert all(v[0]==1 and len(v)==9 for v in polls)
 cutoff=next((v[1] for v in polls if v[4]==2),polls[-1][1])
 active=[v for v in polls if v[1]<=cutoff];sends=[v for v in publishers if v[2]<=cutoff]
 gaps=[{'begin_ns':v[1],'previous_begin_ns':v[3],'gap_ns':v[1]-v[3],'poll_duration_ns':v[2]-v[1],'result_kind':v[4],'sequence':v[5]} for v in active if v[3]]
 sendgaps=[{'sequence':b[1],'previous_sequence':a[1],'sampled_at_ns':b[2],'previous_sampled_at_ns':a[2],'gap_ns':b[2]-a[2],'send_began_ns':b[3],'send_ended_ns':b[4]} for a,b in zip(sends,sends[1:])]
 first=next((v for v in polls if v[4]==2),None)
 epoch_begin=next(v[1] for v in active if v[4]==1)
 source_gaps=[v for v in gaps if v['begin_ns']>=epoch_begin]
 report={'selected_label':out.name,'diagnostic_only':True,'explicit_current_core_head':json.loads((out/'selection.json').read_text())['core_head'],'provider_stop_at_ns':stop_at,'cleanup_polls_excluded':stop_at is not None,'unchanged_sample_budget_ns':100000000,'source_revived':False,'episode_passed':result['passed'],'all_tracked_children_reaped':result['all_tracked_children_reaped'],'timing_records':result['timing_records'],'trace_dropped':{'publisher':max(v[7] for v in publishers),'observer':max(v[8] for v in polls)},'maximum_active_poll_gap_ns':max(v['gap_ns'] for v in gaps),'maximum_source_epoch_poll_gap_ns':max(v['gap_ns'] for v in source_gaps),'source_epoch_began_ns':epoch_begin,'analysis_script_sha256':sha(Path(__file__)),'maximum_active_poll_duration_ns':max(v[2]-v[1] for v in active),'maximum_publisher_sample_gap_ns':max(v['gap_ns'] for v in sendgaps),'largest_poll_gaps':sorted(gaps,key=lambda v:v['gap_ns'],reverse=True)[:8],'largest_publisher_gaps':sorted(sendgaps,key=lambda v:v['gap_ns'],reverse=True)[:8],'source_failed':next((v for v in result['source_events'] if v['event']=='source_failed'),None),'files':{n:sha(out/n) for n in ['selection.json','result.json','timings.json','guest-timeline.json']}}
 if first:
  accepted=next((v for v in reversed(active[:-1]) if v[4]==1),None)
  last=accepted[6] if accepted else None
  report['first_failure_poll']={'began_ns':first[1],'ended_ns':first[2],'previous_begin_ns':first[3],'error_code':first[7],'last_accepted_sample_ns':last,'interval_since_sample_ns':first[1]-last if last else None}
  report['recent_observer_polls']=[v for v in active if v[1]>=first[1]-200000000]
  report['recent_publisher_sends']=[v for v in publishers if first[1]-250000000<=v[2]<=first[1]+250000000]
  fresh=[v for v in publishers if v[6]==0 and v[4]<=first[1] and last is not None and v[2]>last and first[1]<v[2]+100000000]
  report['fresh_successful_packets_sent_before_failure']=fresh
  report['discrimination']='Observation only: compare actual publisher sends with actor polling gaps. Trace instrumentation and host scheduling can perturb availability; no production fix is claimed.'
 return report

def main():
 p=argparse.ArgumentParser();p.add_argument('out',type=Path);a=p.parse_args();report=summarize(a.out)
 (a.out/'timing-analysis.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
