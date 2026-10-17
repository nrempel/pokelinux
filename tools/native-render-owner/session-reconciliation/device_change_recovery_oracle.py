"""Frozen recovery prerequisites using real existing owner/public Rust snapshots.
No future reconciliation fields are read to classify the unchanged current RED.
"""
from first_stop_oracle import first_record,integer,U64_MAX

class RecoveryUnmet(RuntimeError):
 def __init__(self,gate,detail):self.gate=gate;super().__init__(detail)

def snapshot(value,pid,wire,generation=None):
 assert type(value)is dict
 for key in ('schema','run','main_tid','observed_boottime_ns','startup_not_after_ns','code','first_error','startup_error','stage','snapshot_code','join_code','outputs'):
  assert type(value[key])is int,key
 assert value['schema']==1 and value['run']==1 and value['main_tid']==pid
 assert value['code']==value['snapshot_code']==value['first_error']==value['startup_error']==0
 for key in ('rust_failed','rust_ready','rust_stopping','failed','ready','stopping','worker_alive','executor_retained','both_borrowed_watches_registered','native_retirement','shell_ready'):
  assert type(value[key])is bool,key
 assert value['failed']is False and value['rust_failed']is False
 assert value['worker_alive']is True and value['executor_retained']is True and value['both_borrowed_watches_registered']is True
 assert value['join_code']==2 and integer(value['outputs'],0,32)
 assert value['native_retirement']is False and value['shell_ready']is False
 assert integer(value['observed_boottime_ns']) and wire['enter_ns']<=value['observed_boottime_ns']<=wire['return_ns']
 assert integer(value['startup_not_after_ns'],20_000_000_001)
 actual=value['startup_not_after_ns']-20_000_000_000
 assert actual<=value['observed_boottime_ns']
 if generation is not None:assert actual==generation
 return actual

def clean_pending(value):
 assert value['phase']=='STARTING'
 assert integer(value['stage'],0,(1<<32)-1) and value['stage']not in (5,6,7)
 assert value['ready']is value['rust_ready']is value['stopping']is value['rust_stopping']is False
 assert value['outputs']==0 and 'first_stop'in value and value['first_stop']is None
 assert type(value.get('stop_requests'))is int and value['stop_requests']==0

def clean_bootstrap(value):
 clean_pending(value)
 assert value['stage']==8

def device_change_terminal(value,generation):
 assert value['phase']=='STOPPING' and value['stage']==5
 assert value['ready']is value['rust_ready']is False
 assert value['stopping']is value['rust_stopping']is True and value['outputs']==0
 if value['observed_boottime_ns']>=value['startup_not_after_ns']:
  raise RecoveryUnmet('DEVICE_CHANGE_NOT_OBSERVED_BEFORE_ORIGINAL_STARTUP_DEADLINE','terminal observation reached original20s deadline')
 first,count=first_record(value,generation)
 if first['origin']!=3 or first['revoke']!={'session':generation,'reason':5}:
  raise RecoveryUnmet('STOP_CAUSE_IS_NOT_SAME_SESSION_DEVICE_CHANGE','another actual firstStop cause cannot substitute')
 if first['broker_present']is not True or first['broker_probe_failed']is not False or first['broker']['transport_failed']is not False:
  raise RecoveryUnmet('DEVICE_CHANGE_TRANSPORT_PREREQUISITE_UNMET','actual broker/transport is unavailable')
 assert generation<=first['boottime_ns']<=value['observed_boottime_ns']<value['startup_not_after_ns']
 return first,count

def public_ready(value):
 assert value['phase']=='BACKEND_READY'
 assert value['ready']is value['rust_ready']is True
 assert value['stopping']is value['rust_stopping']is False and integer(value['outputs'],1,32)
 if value['observed_boottime_ns']>=value['startup_not_after_ns']:
  raise RecoveryUnmet('BACKEND_READY_AFTER_ORIGINAL_STARTUP_DEADLINE','no renewed recovery deadline')
 # These existing fields alone cannot prove ordered application or revocation.
 return True


RECONCILE_KEYS={'session_generation','invalidation_epoch','admitted_epoch','applied_epoch','accepted_epoch','last_admitted_sequence','last_applied_sequence','discovery_sequence','bootstrap_end_sequence','reconcile_not_after_boottime_ns'}

def ordered_recovery(value,generation):
 """Fixed full GREEN criterion; unreachable on the current Stop3/reason5 RED branch."""
 public_ready(value)
 if 'session_reconcile'not in value:
  raise RecoveryUnmet('RUST_ORDERED_RECOVERY_ACCEPTANCE_UNBOUND','actual Ready lacks required genuine Rust policy watermarks')
 witness=value['session_reconcile']
 assert type(witness)is dict and set(witness)==RECONCILE_KEYS
 assert all(integer(v)for v in witness.values())
 assert witness['session_generation']==generation
 seed=witness['bootstrap_end_sequence'];epoch=witness['invalidation_epoch']
 assert seed>=1 and epoch>=1 and seed<=U64_MAX-epoch
 assert witness['admitted_epoch']==witness['applied_epoch']==witness['accepted_epoch']==epoch
 assert witness['last_admitted_sequence']==witness['last_applied_sequence']==witness['discovery_sequence']==seed+epoch
 assert witness['reconcile_not_after_boottime_ns']==value['startup_not_after_ns']
 assert generation<=value['observed_boottime_ns']<witness['reconcile_not_after_boottime_ns']
 assert 'first_stop'in value and value['first_stop']is None
 assert type(value.get('stop_requests'))is int and value['stop_requests']==0
 return witness
