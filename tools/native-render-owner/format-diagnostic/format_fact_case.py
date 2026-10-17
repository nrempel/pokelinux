"""Proposed separately classified case; retains unchanged actual wire target."""
from pathlib import Path
from owned import now
import device_change_recovery_case as original
from device_change_recovery_oracle import snapshot,clean_bootstrap
from format_fact_oracle import validate,need,FactsMissing,SetupUnqualified,REFUSAL,MAX_LOG

def run_case(scope,base,main_path,guest_work,guest_close,cohort=None,*,normal_log_path):
 native=original.run_case(scope,base,main_path,guest_work,guest_close,cohort)
 out={'schema':1,'phase':'same-owner-format-facts','classification':'SETUP_UNQUALIFIED','diagnostic_facts_qualified':False,'native_wire_target_passed':False,'native_wire_case':native,'completed_before_original_case_close':native['completed_before_original_case_close'],'all_owned_descendants_closed':native['all_owned_descendants_closed'],'outer_closure_pending':True,'native_frame_path_qualified':False,'phone_accessed':False}
 try:
  need(native['completed_before_original_case_close']and native['all_owned_descendants_closed'],'owned case closure incomplete')
  need(native.get('main_process_disposal')and not any(k.endswith('disposal_error')for k in native),'owned disposal incomplete')
  requests=native['public_requests'];values=native['startup_snapshots']
  need(len(values)>=2 and len(values)==len(requests),'authenticated actual snapshots absent')
  generation=snapshot(values[0],native['main_PID'],requests[0]);clean_bootstrap(values[0])
  need(native['classification']=='SETUP_UNQUALIFIED' and not native.get('full_ordered_Rust_recovery_witness_mapping_bound'),'native wire target disposition changed')
  for r in requests:
   need(r['actual_server_SO_PEERCRED']==[native['main_PID'],1000,1000] and r['response_EOF']is True and r['return_ns']<r['original_until_ns'],'actual request identity/EOF/deadline')
  last=values[-1]
  need(last['phase']=='UNAVAILABLE'and last['stage']==5 and last['rust_failed']is True and last['rust_ready']is False and last['first_error']==9,'expected genuine callback failure absent')
  need(last['observed_boottime_ns']<last['startup_not_after_ns']==native['startup_not_after_ns'],'failure outside original20s')
  first=last['first_stop'];need(type(first)is dict and first['origin']==11 and first['error']==9 and generation<=first['boottime_ns']<=last['observed_boottime_ns'],'actual callback-to-Rust terminal witness')
  p=Path(normal_log_path);need(p.is_file()and p.stat().st_size<=MAX_LOG,'owned normal log absent/oversize')
  body=p.read_bytes();need(REFUSAL.encode()in body,'unchanged aggregate refusal missing')
  out['actual_failed_native_prerequisites_qualified']=True
  out.update(validate(body,native['main_PID'],generation))
 except FactsMissing as error:
  out.update(classification='DIAGNOSTIC_FACTS_MISSING_RED',diagnostic_target_failed=True,missing_gate=str(error))
 except Exception as error:
  out.update(classification='SETUP_UNQUALIFIED',error={'type':type(error).__name__,'detail':str(error)[:512]})
 finally:
  out['completed_at_ns']=now()
  out['completed_before_original_case_close']=out['completed_at_ns']<native['close_until_ns']
  if not out['completed_before_original_case_close']:
   out.update(classification='SETUP_UNQUALIFIED',diagnostic_facts_qualified=False,error={'type':'Deadline','detail':'original45s case close exceeded'})
 return out
