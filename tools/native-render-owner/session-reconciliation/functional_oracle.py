"""Frozen stdout checks only; matching real provider and outer closure are separate."""
import argparse
import hashlib
import json
import re
from pathlib import Path

TEST_SHA256 = '5026e2f9cdbdee378df0adb780bec531ec2e066e6b8eeb74b43cc574c4adcf6d'
GROUPS = [
 'ABI2_required_apply_and_atomic_refusals',
 'held_bootstrap_three_copied_changes_latest_only_Ready',
 'postReady_real_fixed20s_burst_no_renewal',
 'immediate_unsealed_ticket_revocation_prepare_and_execute_held',
 'alreadysealed_custody_survives_reconcile_and_Stop',
 'real_original_startup_timer_late_apply_never_Ready',
]
APPLY_COUNTS = [0,2,0,3,2,1,1,1,1]


def assess(raw, actual_exit, actual_main_pid, selected_consumer_sha256):
 """No target invocation. Inputs must come from actual owned process receipts."""
 result = {'schema':1, 'qualified':False,
           'consumer_assertions_passed':False,
           'matching_Rust_provider_and_outer_scope_closure_pending':True,
           'native_or_normal_readiness_frame_Source_phone_qualified':False}
 try:
  assert type(actual_main_pid) is int and actual_main_pid > 0
  assert type(actual_exit) is int and actual_exit == 0
  assert selected_consumer_sha256 == TEST_SHA256
  assert type(raw) is bytes and 0 < len(raw) <= 65536
  text = raw.decode('utf-8')
  assert text.endswith('\n')
  lines = text.splitlines()
  assert len(lines) == 29
  bounds = re.fullmatch(r'SESSION_RECONCILE_BOUNDS main_tid=(\d+) started_ns=(\d+) work_until_ns=(\d+) close_until_ns=(\d+)',lines[0])
  assert bounds
  main,started,work,close = map(int,bounds.groups())
  assert main == actual_main_pid and 0 < started < work < close
  assert work-started == 5_000_000_000 and close-started == 8_000_000_000
  workers,apply_rows,groups,foreign = [],[],[],[]
  for line in lines[1:-1]:
   m = re.fullmatch(r'SESSION_RECONCILE_APPLY worker_tid=(\d+) ordinal=(\d+) sequence=(\d+) copied408_equal=true',line)
   if m:
    w,n,sequence=map(int,m.groups())
    assert w>0 and w!=main and n==len(apply_rows)+1 and sequence==3+n
    apply_rows.append({'worker_tid':w,'ordinal':n,'sequence':sequence})
    continue
   m = re.fullmatch(r'SESSION_RECONCILE_OBSERVATION worker_tid=(\d+) control_tid=(\d+) callbacks=(\d+) applies=(\d+) actual_join=true task_absent_at_ns=(\d+)',line)
   if m:
    w,control,callbacks,applies,absent=map(int,m.groups())
    assert w>0 and w!=main and control==main and 6<=callbacks<=128
    assert len(workers)<len(APPLY_COUNTS) and applies==APPLY_COUNTS[len(workers)]
    assert len(apply_rows)==applies and all(r['worker_tid']==w for r in apply_rows)
    assert started<absent<close
    if len(workers)==1:
     assert len(foreign)==1 and foreign[0]!=w
    workers.append({'worker_tid':w,'callbacks':callbacks,'applies':applies,'task_absent_at_ns':absent})
    apply_rows=[]
    continue
   m = re.fullmatch(r'SESSION_RECONCILE_OBSERVATION foreign_tid=(\d+) control_tid=(\d+) actual_join=true wrong_thread_refusals=3',line)
   if m:
    f,control=map(int,m.groups())
    assert len(workers)==1 and not foreign and f>0 and f!=main and control==main
    foreign.append(f)
    continue
   m = re.fullmatch(r'SESSION_RECONCILE_PASS (\S+)',line)
   assert m and len(groups)<len(GROUPS) and m.group(1)==GROUPS[len(groups)]
   # First group owns three workers; fourth owns two, all others one.
   assert len(workers)==[3,4,5,7,8,9][len(groups)] and not apply_rows
   groups.append(m.group(1))
  final=re.fullmatch(r'SESSION_RECONCILE_ALL_PASS groups=6 completed_ns=(\d+)',lines[-1])
  assert final and started<int(final.group(1))<close
  assert groups==GROUPS and len(workers)==9 and len(foreign)==1 and not apply_rows
  result.update(consumer_assertions_passed=True,
                classification='CPU_SESSION_RECONCILIATION_ASSERTIONS_PASS',
                actual_main_pid=main, actual_worker_joins=9,
                actual_foreign_Cpp_thread_joins=1,
                actual_apply_callbacks=11, groups=groups,
                observations=workers,
                output_sha256=hashlib.sha256(raw).hexdigest())
 except (AssertionError,UnicodeError,ValueError,TypeError,IndexError) as error:
  result.update(classification='UNQUALIFIED_COMPONENT_OUTPUT',
                failure_type=type(error).__name__)
 return result


def main():
 p=argparse.ArgumentParser()
 p.add_argument('--log',required=True,type=Path)
 p.add_argument('--actual-exit',required=True,type=int)
 p.add_argument('--actual-main-pid',required=True,type=int)
 p.add_argument('--selected-consumer-sha256',required=True)
 args=p.parse_args()
 raw=args.log.read_bytes()
 result=assess(raw,args.actual_exit,args.actual_main_pid,args.selected_consumer_sha256)
 print(json.dumps(result,sort_keys=True))
 return 0 if result['consumer_assertions_passed'] else 1

if __name__=='__main__':
 raise SystemExit(main())
