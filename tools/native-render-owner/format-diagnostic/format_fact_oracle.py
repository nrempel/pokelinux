"""Pure bounded same-owner numeric fact oracle. No DRM calls or capability policy."""
import json
MARKER='owned-device-change-format-v1: '
REFUSAL='owned-device-change: primary-format-modifier-unavailable errno=0 connector='
MAX_LOG=2*1024*1024
MAX_PACKET=32768
U32=(1<<32)-1
U64=(1<<64)-1
class FactsMissing(RuntimeError):pass
class SetupUnqualified(RuntimeError):pass
def need(value,reason):
 if not value:raise SetupUnqualified(reason)
def integer(value,low=0,high=U64):return type(value)is int and low<=value<=high
def unique(pairs):
 out={}
 for k,v in pairs:
  need(k not in out,'duplicate packet JSON key');out[k]=v
 return out
HEADER={'schema','worker_tid','event_sequence','session_generation','connector_id','fd_rdev','selected_format','selected_modifier','cap','crtc_count','connector_encoder_id','connector_current_crtc_id','compatible_crtc_mask','plane_count','planes','deny_display_before','deny_display_after','complete','unchanged_predicate_result'}
PLANE={'id','current_crtc_id','possible_crtcs','route_eligible','format_count','selected_format_present','type_present','type_value','in_formats_present','blob_id','blob_errno','blob_length','selected_pair_match'}
def validate(log,main_pid,generation):
 need(type(log)is bytes and len(log)<=MAX_LOG,'log absent/oversize')
 text=log.decode('utf-8','strict');lines=text.splitlines()
 refusals=[v for v in lines if REFUSAL in v]
 need(len(refusals)==1,'actual aggregate format refusal missing/ambiguous')
 selected=[v.split(MARKER,1)[1] for v in lines if MARKER in v]
 if not selected:raise FactsMissing('selected tuple/plane/cap diagnostic packet was not emitted')
 need(len(selected)==1 and len(selected[0].encode())<=MAX_PACKET,'packet count/size')
 d=json.loads(selected[0],object_pairs_hook=unique)
 need(type(d)is dict and set(d)==HEADER and d['schema']==1,'packet header schema')
 for k in ['worker_tid','event_sequence','session_generation','connector_id','fd_rdev','selected_format']:
  need(integer(d[k],1,U32 if k in ('worker_tid','connector_id','selected_format')else U64),k)
 need(d['worker_tid']!=main_pid and d['session_generation']==generation,'actual worker/session binding')
 need(refusals[0].endswith(str(d['connector_id'])),'aggregate connector mismatch')
 need(integer(d['selected_modifier']),'modifier u64')
 cap=d['cap'];need(type(cap)is dict and set(cap)=={'ret','errno','value'},'cap shape')
 need(type(cap['ret'])is int and integer(cap['errno'],0,U32),'cap result fields')
 need(cap['ret']==0 and cap['errno']==0 and integer(cap['value'],0,1),'actual cap query did not qualify')
 need(integer(d['crtc_count'],1,32),'crtc bounds')
 for k in ['connector_encoder_id','connector_current_crtc_id']:need(integer(d[k],0,U32),k)
 need(integer(d['compatible_crtc_mask'],1,(1<<d['crtc_count'])-1),'compatible route mask')
 need(integer(d['plane_count'],1,128) and type(d['planes'])is list and len(d['planes'])==d['plane_count'],'plane enumeration')
 seen=set();primary=0;matches=0
 for p in d['planes']:
  need(type(p)is dict and set(p)==PLANE,'plane shape')
  need(integer(p['id'],1,U32) and p['id']not in seen,'plane identity');seen.add(p['id'])
  for k in ['current_crtc_id','possible_crtcs']:need(integer(p[k],0,U32),k)
  need(type(p['route_eligible'])is bool and p['route_eligible']==bool(p['possible_crtcs']&d['compatible_crtc_mask']),'route mask fact')
  need(integer(p['format_count'],0,4096) and type(p['selected_format_present'])is bool,'format membership fact')
  extra=['type_present','type_value','in_formats_present','blob_id','blob_errno','blob_length','selected_pair_match']
  if not p['route_eligible']:
   need(all(p[k]is None for k in extra),'excluded plane falsely observed');continue
  need(type(p['type_present'])is bool and type(p['in_formats_present'])is bool,'property presence')
  need(p['type_present'] and integer(p['type_value'],0,U64),'plane type unavailable')
  if p['type_value']==1:primary+=1
  if not p['in_formats_present']:
   need(all(p[k]is None for k in ['blob_id','blob_errno','blob_length','selected_pair_match']),'absent blob fabricated');continue
  need(integer(p['blob_id'],0,U32),'blob id')
  if p['blob_id']==0:
   need(all(p[k]is None for k in ['blob_errno','blob_length','selected_pair_match']),'zero blob fabricated');continue
  need(p['blob_errno']==0 and integer(p['blob_length'],1,1024*1024) and type(p['selected_pair_match'])is bool,'blob query incomplete')
  matches+=p['type_value']==1 and p['selected_format_present']and p['selected_pair_match']
 need(d['deny_display_before']is True and d['deny_display_after']is True and d['complete']is True,'query/scope incomplete')
 need(d['unchanged_predicate_result']is False and matches==0,'unchanged predicate/facts disagree')
 return {'classification':'DIAGNOSTIC_FACTS_PASS','diagnostic_facts_qualified':True,'native_wire_target_passed':False,'primary_candidates':primary,'explicit_selected_pair_matches':matches,'facts':d,'capability_fallback_qualified':False,'Ready_frames_Source_hardware_phone_qualified':False}
