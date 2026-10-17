# Frozen actual NORMAL recoverable DEVICE_CHANGE target

This is a new source-only behavioral target using the real normal main, actual
AQ broker/session, sole Rust executor and authenticated owner/public socket. It
is based on completed first-Stop GREEN2 (execution74325cf4..., casec4860a3a...),
whose actual first copied record was origin3/AQ_REVOKE, reason5/DEVICE_CHANGE,
same session generation; current safe terminal handling prevented backend Ready.
That earlier run did not execute this new recovery oracle and is not this RED.
No production implementation, stage, compiler, guest, runtime or oracle evaluation
is authorized by this package. Root reviews and launches any actual epoch.

## Fixed behavior and chronology

A recoverable same-session DEVICE_CHANGE must invalidate old native commit
permission immediately at the originating callback BEFORE event enqueue or any
fallible admission. It must block acceptance of copied discovery from older
work. Diagnostic success cannot gate this authority action. This operation is
revocation rather than terminal Stop. Every actually accepted suffix value and
owned FD capability must remain bounded and ordered; full queue or unsupported
suffix fails closed. Do not drop, reorder, silently coalesce, acknowledge or
pretend-apply an event to achieve Ready.

The corresponding actual CHANGE metadata carries genuine session/device lease/
generation and broker sequence, paired with its revocation fence. Rust must
retain it before the broker marks its prefix admitted. The worker exclusively
applies each admitted CHANGE exactly once in sequence. Reconciliation and fresh
worker discovery must cover the LATEST invalidation/admission and may be accepted
on control only if no newer invalidation intervened. A stale Ready, original
bootstrap discovery or a control-only boolean cannot satisfy this requirement.
The same original generation+20,000,000,000ns startup deadline remains in force;
for this INITIAL-startup target no recovery deadline, clock reset, second
executor/native graph or startup retry. Later post-Ready bursts are separate cases.

The successful public boundary is existing BACKEND_READY with actual raw Rust
ready=true, failed=false, stopping=false; worker alive, real join Busy, executor
and both borrowed watches retained, nonempty accepted copied discovery outputs
1..32, same original generation/deadline and observed before that deadline.
This is backend metadata acceptance, never shell/Wayland/GL/frame/Source Ready.
It ALSO requires genuine Rust functional state proving immediate revocation,
pending admission/application and latest refresh acceptance, with actual session
and sequence correlation. Ready/output count alone NEVER qualifies full GREEN.
The exact Rust-owned copied wire object is `session_reconcile` with precisely
these ten unsigned64 integer fields (JSON booleans do not count as integers):
`session_generation`, `invalidation_epoch`, `admitted_epoch`, `applied_epoch`,
`accepted_epoch`, `last_admitted_sequence`, `last_applied_sequence`,
`discovery_sequence`, `bootstrap_end_sequence`,
`reconcile_not_after_boottime_ns`.

The bootstrap seed is the actual final initial-session event sequence retained
by Rust from the captured/validated pack, not an assumed event count. Epochs
count genuine CHANGE/revocation pairs only. `invalidation_epoch` is the actual
functional callback fence; dirty means invalidation_epoch!=accepted_epoch,
including a pre-enqueue callback with no admitted queue head. Admitted/applied
watermarks are actual Rust custody/application transitions, not telemetry added
after the fact. `accepted_epoch` is the actual control acceptance gate for the
validated latest worker refresh. `discovery_sequence` belongs to that refreshed
candidate, never the original stale bootstrap discovery. These state changes
are required Rust policy state; C++ merely copies native facts and public values.

Full GREEN requires, on the actual authenticated Ready response, all of:

- session_generation == the original startup generation;
- bootstrap_end_sequence>=1, invalidation_epoch>=1;
- admitted_epoch == applied_epoch == accepted_epoch == invalidation_epoch;
- with checked no-overflow addition, last_admitted_sequence ==
  last_applied_sequence == discovery_sequence ==
  bootstrap_end_sequence + invalidation_epoch. This CHANGE-only route enforces
  strict contiguous actual sequence correlation, not merely matching counters;
- reconcile_not_after_boottime_ns == the UNCHANGED original startup_not_after_ns;
  actual server observation is strictly before that deadline. The startup
  watermark remains after acceptance/disarm; no reset to0 or refreshed20s;
- existing public/raw Rust Ready/noStop/error0/live-worker/Busy/watches/nonempty
  copied outputs1..32 checks above; first_stop is null and stop_requests==0.

The same assertions are frozen in device_change_recovery_oracle.py. Current RED
never reads the new object: existing Stop3/reason5 is its sufficient target
observation only after complete native/public prerequisites. Missing new object
on an otherwise real Ready is acceptance-unbound SETUP, never target RED or a
passing fallback. This preparation stages RED only; a future separately reviewed
provider binding must retain these exact case/oracle/contract assertions. No
production recovery implementation is permitted before this field-level freeze.
The planned actual C APIs create_v2/invalidate_session_v2/try_session_change_v2/
reconciliation_v2 are not declarations or compiler inputs in this preparation.
Post-Ready held-ticket/dirty-burst/deadline races are separate actual Rust tests;
this native tracer does not claim to exercise those by polling status.

## Current RED, with no future-field prerequisite

Launch exactly the selected whole NORMAL main11bf76... / SDKaff715... / RootQ
c3085713... / sole Rust0fdc60... and RootQfee9...; AQefa7... only. No test macro,
injection, client, plugin, proc helper, renderer/global substitute or manual
native initialization. Use the original seatd/udev/kernel/virtio setup; seatd
socket AND GID1000 readiness uses the SAME existing five-second cutoff.

Retain an actual clean STARTING/stage8 response before terminal state, with raw
Rust failed=false/ready=false/stopping=false, accepted outputs0, live worker,
join Busy2, executor/both borrowed watches retained, first_stop null and counter0.
All requests are raw UTF8 owner-startup snapshot, nonblocking, SHUT_WR EOF framed,
actual SO_PEERCRED matching the exact main PID/UID1000/GID1000, response EOF,
strict JSON with duplicate-key rejection. The actual server observation stamp
must lie within the original request interval; startup_not_after is invariant
and its generation is precisely deadline minus20s.

ONLY a subsequent spontaneous clean STOPPING/stage5 BEFORE original20s, before
any Ready or public Stop, with actual raw Rust stopping=true/failed=false/
ready=false, outputs0, live worker/joinBusy2/executor/watches, qualifies the target
branch. Validate the entire existing first-stop record; require origin3,
revoke.session==original generation, reason5, executor_present=true, known valid
post-authority observation clock before deadline, actual broker present,
successful probe and transport_failed=false. Pending may truthfully be null:
the real pre-enqueue callback can run before the CHANGE exists at the queue head.
No head at that observation is NOT an empty-queue or absent-event assertion.

That complete branch yields RECOVERABLE_DEVICE_CHANGE_TARGET_RED, with failure
DEVICE_CHANGE_TERMINATED_NORMAL_STARTUP_BEFORE_READY. Its existence is established
by current fields only: new Rust witness fields are not read on this branch.
Other Stop origins/reasons, missing native/bootstrap/peer/clock/transport facts,
failed/quarantined/native-exited main, missing GID or setup dependencies, deadline
or byte exhaustion are SETUP_UNQUALIFIED / unmet gate, never target RED. No public
Stop is sent by this test. A real Ready without the required ordered Rust witness
is retained as public evidence and classified acceptance-unbound SETUP, not GREEN.

## Custody, budgets and claims

Original ownership engines Scope/Descendants/host Children/QMP are retained.
Each actual owned process has original-cutoff wait and PIPE EOF receipts;
main/descendant SIGKILL disposal is process disposal, never native cleanup,
retirement, GL destruction, service retirement or successful worker join.
Outer actual CLI wait, exact CID absence, same daemon, shared input lock,
pre/post source/input hashes, QMP shutdown and socket EOF are required before
Root qualifies any expected RED. Case JSON alone cannot qualify outer closure.

Unchanged limits: case work35s/close45s; per request2s, response2048B, aggregate
public64KiB; exact64B request limit,250ms poll spacing and no renewed deadline.
Guest660/700, bootstrap720/740, QEMU780, boot800, host840; container8CPU/10GiB/
512pids/2GiB tmpfs, guest6CPU/6144MiB, KVM-only host device, network none, four
exact mounts with one readonly sole-Rust input. No Rust/AQ/main build here, so
no source-build16GiB gate is added or weakened. No automatic retry/fallback.
Pixel10 requested1080x2424@60 scale2.625 remains exact; no screenshot or actual
output/window geometry claim, no resize, phone, proprietary input or Source claim.
All approval/stage/run fields remain false. Failed prior firstStopGREEN1 fixture
is preserved by its predecessor; completed firstStopGREEN2 is distinct history.

## Pre-execution worker-stage criterion correction

No actual epoch or production recovery implementation has executed against
prep1. Worker stage is an instantaneous progress observation rather than a
Ready/discovery acceptance authority. BACKEND_READY therefore has no exact stage1
requirement: raw Ready/noStop/healthy custody, accepted outputs and every frozen
Rust watermark remain mandatory. While STARTING, healthy pending snapshots may
continue through idle/reconciling/other nonterminal progress rather than only
stage8. Their raw Ready/Stop flags remain false, outputs0, first_stop null and
count0. Destroying5/quarantined6/exited7 or unhealthy pending states remain unmet.
At least one actual stage8 snapshot is STILL mandatory before either target RED
or full GREEN can qualify the native bootstrap prerequisite. No stage is faked
or a missing stage8 supplied by inferred source execution. All ten watermark
assertions and original authority/deadline/budget/ownership constraints are exact.
