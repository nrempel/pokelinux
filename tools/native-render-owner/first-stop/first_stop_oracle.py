"""Frozen first-Stop target checks; no owner/native stand-in or code execution here."""
U64_MAX = (1 << 64) - 1
ORIGINS = {
 1:'PUBLIC_STOP', 2:'EXTERNAL_STOP', 3:'AQ_REVOKE',
 4:'BROKER_WATCH_ERROR', 5:'BROKER_DISPATCH_EXCEPTION', 6:'BROKER_SUFFIX',
 7:'RUST_WATCH_ERROR', 8:'RUST_UNEXPECTED_COMPLETION', 9:'OBSERVE_TRANSPORT',
 10:'SNAPSHOT_ERROR', 11:'RUST_FAILED', 12:'RUST_STOPPING',
 13:'DISCOVERY_ERROR', 14:'DISCOVERY_TRANSPORT', 15:'STARTUP_DEADLINE',
 16:'DISCOVERY_EMPTY_OUTPUTS', 17:'DISCOVERY_SESSION_MISMATCH',
 18:'OBSERVE_EXCEPTION', 19:'BEGIN_EXCEPTION', 20:'TERMINAL_REOBSERVED',
}
FIRST_KEYS = {'origin','boottime_ns','clock_errno','executor_present','error','event_mask',
              'revoke','broker_present','broker_probe_failed','broker'}
ERROR_ORIGINS = {4,5,7,8,10,11,13,18,19}
MASK_ORIGINS = {4,7}
class TargetFailure(RuntimeError): pass
class DiagnosticUnavailable(RuntimeError): pass

def integer(value,low=0,high=U64_MAX):
 return type(value) is int and low <= value <= high

def clean_snapshot(value,pid,wire,generation=None):
 """Actual wire/process/bootstrap prerequisites; these are never target RED."""
 assert type(value) is dict
 for key in ('schema','run','main_tid','observed_boottime_ns','startup_not_after_ns','code','first_error','startup_error','stage','snapshot_code','join_code','outputs'):
  assert type(value[key]) is int,key
 assert value['schema']==1 and value['run']==1
 assert value['main_tid']==pid and value['snapshot_code']==0 and value['code']==0
 assert value['first_error']==0 and value['startup_error']==0
 for key in ('rust_failed','rust_ready','rust_stopping','failed','ready','stopping',
             'worker_alive','executor_retained','both_borrowed_watches_registered',
             'native_retirement','shell_ready'):
  assert type(value[key]) is bool,key
 assert value['rust_failed'] is False and value['failed'] is False
 assert value['rust_ready'] is False and value['ready'] is False
 assert value['worker_alive'] and value['executor_retained'] and value['both_borrowed_watches_registered']
 assert value['join_code']==2 and value['outputs']==0
 assert value['native_retirement'] is False and value['shell_ready'] is False
 assert integer(value['observed_boottime_ns']) and wire['enter_ns']<=value['observed_boottime_ns']<=wire['return_ns']
 assert integer(value['startup_not_after_ns'],20_000_000_001)
 actual_generation=value['startup_not_after_ns']-20_000_000_000
 assert actual_generation<=value['observed_boottime_ns']
 if generation is not None:assert actual_generation==generation
 return actual_generation

def first_record(value,generation):
 if 'first_stop' not in value or value['first_stop'] is None or 'stop_requests' not in value:
  raise TargetFailure('FIRST_STOP_RECORD_MISSING')
 first=value['first_stop'];count=value['stop_requests']
 if type(first) is not dict or set(first)!=FIRST_KEYS or not integer(count,1):
  raise TargetFailure('FIRST_STOP_RECORD_OR_COUNTER_INVALID')
 origin=first['origin']
 if not integer(origin,1,20) or origin in (1,20):
  raise TargetFailure('FIRST_STOP_ORIGIN_NOT_SPONTANEOUS_OR_REPLACED')
 if not integer(first['clock_errno'],0,(1<<31)-1):raise TargetFailure('CLOCK_ERRNO_INVALID')
 if first['clock_errno']!=0:
  if first['boottime_ns'] is not None:raise TargetFailure('CLOCK_FAILURE_INVENTED_TIMESTAMP')
  raise DiagnosticUnavailable('ACTUAL_FIRST_STOP_CLOCK_UNAVAILABLE')
 if not integer(first['boottime_ns'],generation,value['observed_boottime_ns']):
  raise TargetFailure('FIRST_STOP_TIMESTAMP_INVALID')
 if first['executor_present'] is not True:raise TargetFailure('FIRST_STOP_EXECUTOR_CUSTODY_INVALID')
 if origin in ERROR_ORIGINS:
  if not integer(first['error'],-(1<<31),(1<<31)-1):raise TargetFailure('ACTUAL_ERROR_MISSING')
 elif first['error'] is not None:raise TargetFailure('INAPPLICABLE_ERROR_NOT_NULL')
 if origin in MASK_ORIGINS:
  if not integer(first['event_mask'],1,(1<<32)-1):raise TargetFailure('ACTUAL_EVENT_MASK_MISSING')
 elif first['event_mask'] is not None:raise TargetFailure('INAPPLICABLE_EVENT_MASK_NOT_NULL')
 if origin==3:
  r=first['revoke']
  if type(r) is not dict or set(r)!={'session','reason'} or not integer(r['session'],1) or r['session']!=generation or not integer(r['reason'],1,5):
   raise TargetFailure('DIRECT_REVOKE_FACTS_MISSING')
 elif first['revoke'] is not None:raise TargetFailure('INAPPLICABLE_REVOKE_NOT_NULL')
 if type(first['broker_present']) is not bool or type(first['broker_probe_failed']) is not bool:
  raise TargetFailure('BROKER_PRESENCE_FLAGS_INVALID')
 if first['broker_probe_failed']:
  if first['broker'] is not None:raise TargetFailure('FAILED_PROBE_INVENTED_BROKER_FACTS')
  raise DiagnosticUnavailable('ACTUAL_FIRST_STOP_BROKER_PROBE_UNAVAILABLE')
 broker=first['broker']
 if first['broker_present']:
  if type(broker) is not dict or set(broker)!={'transport_failed','pending'} or type(broker['transport_failed']) is not bool:
   raise TargetFailure('COPIED_BROKER_FACTS_INVALID')
  pending=broker['pending']
  if pending is not None:
   if type(pending) is not dict or set(pending)!={'type','sequence'} or not integer(pending['type'],1,7) or not integer(pending['sequence'],1):
    raise TargetFailure('COPIED_PENDING_FACTS_INVALID')
  if origin==6 and not (broker['transport_failed'] or pending is not None):
   raise TargetFailure('BROKER_SUFFIX_ORIGIN_WITHOUT_OBSERVED_SUFFIX_OR_FAILURE')
 elif broker is not None:raise TargetFailure('ABSENT_BROKER_INVENTED_FACTS')
 return first,count

def preserved(value,first,minimum_count):
 if value.get('first_stop')!=first or not integer(value.get('stop_requests'),minimum_count):
  raise TargetFailure('FIRST_STOP_REPLACED_OR_COUNTER_REGRESSED')
 return value['stop_requests']
