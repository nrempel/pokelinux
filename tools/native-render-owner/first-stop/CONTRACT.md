# Frozen first-Stop observability target, version1

This is an actual NORMAL compositor owner/public-status behavior target, not a
native correctness bug or a general Rust progress API. No production change has
been authored by this package and no consumer/preparation has been executed.
The unsealed /proc prep3 and all126 baseline1 snapshots remain historical.
Baseline1 observed STARTING8 once, then clean STOPPING5 ~0.284s after generation,
19.716s before the original20s deadline. Stage5 is worker Destroying after the
bootstrap call returned. Outputs0 means no accepted copied discovery. Neither a
blocked bootstrap nor a deadline-caused Stop follows from those observations.

The existing CCompositor::ownedBackendStartupReply → COwnedBackendStartup::reply
path and exact raw UTF8 owner-startup snapshot/stop commands remain the boundary.
No fake SControl, manual globals, renderer, test macro, hook, special environment,
new C/Rust ABI, copied native FD, client, plugin, compiler or /proc diagnostic is
selected. The source-only case uses real Root seatd, actual virtio/udev, actual
NORMAL main/AQ15/Rust246, real wl_event_loop watches and existing public socket.

## Selected production behavior, to implement only after actual qualified RED

Add to every existing owner status JSON reply:

- first_stop: null before any accepted owner-thread Stop request; otherwise the
  immutable object with EXACT keys origin, boottime_ns, clock_errno,
  executor_present, error, event_mask, revoke, broker_present,
  broker_probe_failed, broker.
- stop_requests: exact u64 total accepted owner-thread authority Stop requests,
  initially0, incrementing once per stop/revoke entry and saturating at UINT64_MAX.
  Re-observation of an already terminal state may call Stop and increment this
  counter under the existing behavior. It is not a count of distinct causes.
  Snapshot reads do not reset it. Saturation must neither wrap nor alter authority.

origin is one of these exact numeric source-branch IDs:

| ID | Origin | Current actual source boundary |
|---|---|---|
|1|PUBLIC_STOP|reply("owner-startup stop"), OwnedBackendStartup.cpp665–666|
|2|EXTERNAL_STOP|COwnedBackendStartup::stop wrapper; Compositor.cpp461/477|
|3|AQ_REVOKE|SControl::revoke483; retain actual session and reason arguments|
|4|BROKER_WATCH_ERROR|brokerTick501 mask ERROR/HANGUP|
|5|BROKER_DISPATCH_EXCEPTION|brokerTick catch508|
|6|BROKER_SUFFIX|brokerTick506 observed transport failure or live pending head|
|7|RUST_WATCH_ERROR|rustTick535 ERROR/HANGUP mask|
|8|RUST_UNEXPECTED_COMPLETION|rustTick542 unexpected completion/error|
|9|OBSERVE_TRANSPORT|observe549 first unhealthy initial transport check|
|10|SNAPSHOT_ERROR|observe552 failed snapshot API result|
|11|RUST_FAILED|observe553 actual Rust snapshot.failed|
|12|RUST_STOPPING|observe554 snapshot.stopping while control not already terminal|
|13|DISCOVERY_ERROR|observe558 non-empty/non-busy discovery error|
|14|DISCOVERY_TRANSPORT|observe559 first discovery acceptance predicate|
|15|STARTUP_DEADLINE|same predicate, actual not-ready deadline comparison|
|16|DISCOVERY_EMPTY_OUTPUTS|same predicate, candidate.output_count==0|
|17|DISCOVERY_SESSION_MISMATCH|same predicate, candidate generation mismatch|
|18|OBSERVE_EXCEPTION|observe564 catch|
|19|BEGIN_EXCEPTION|COwnedBackendStartup::begin outer catch|
|20|TERMINAL_REOBSERVED|observe554 when control already terminal; must never be first|

Line references describe the unchanged input, not future edited line numbers.
Split the combined discovery predicate in its original left-to-right order;
record the actual first matched predicate, without changing authority behavior.
A fail helper carries its originating ID/error; it must not replace it with a
second generic Stop origin. Wrong-thread revoke remains unavailable and cannot
mutate this control-owned record.

The selected route is a narrow Rust-owned handle created before broker callbacks,
with fixed first-record storage and a saturating counter. C++ collects only the
actual native originating-branch value facts; it gains no durable new Stop policy
or service framework. Exact C ABI/layout and the frozen real C++→Rust component
consumer are a separate coordinated package. Preworker callbacks remain authentic
when no executor exists. Release the handle only after broker callbacks/sources
are retired, so no live callback can access a released record.

Copy origin/applicable direct arguments and actual pre-call executor presence
before revocation using fixed nonthrowing scalar stores only. THEN perform the
existing terminal latch/backend-ready clearing, authority invalidation and Rust
Stop FIRST. Only after that authority action record those prefacts and the actual
post-revocation observation timestamp/broker facts. Never clock before revoking.
Capture/storage/clock/probe/serialization success is not a prerequisite for Stop;
no observer failure may skip, retry, wait for or fail authority revocation.
boottime_ns is that post-authority observation time, NOT the native event's time,
exact transition time, kernel-entry bound or Stop completion. A raw nonthrowing
CLOCK_BOOTTIME observation supplies it; on failure store null and genuine positive
errno. Success stores errno0. No throw/string/log, blocking lock, callback, queue
mutation/admission, native call, FD operation, allocation or serialization belongs
to this capture. The single control owner does not reenter its event loop during
the Rust Stop call. A revoke callback records direct args before it returns to AQ
and before the subsequent native enqueue; no later queue reconstruction is used.


error is the actual originating int32 failure code for IDs4/5/7/8/10/11/13/18/19,
otherwise null. event_mask is the actual u32 mask only for IDs4/7, otherwise null.
revoke is null except ID3, where it is exactly {session:u64,reason:u32}, copied
DIRECTLY from the callback before it returns to AQ. Reasons1–5 retain the actual
AQ disable/transport-failure/shutdown/device-remove/device-change values. Callback
revocation can precede enqueue; a later queue head must NEVER replace or infer
these direct facts. Authentic ctor callbacks may have executor_present:false and
broker_present:false; preserve them, including on later native/Begin failure.

broker_present is the actual retained broker pointer-presence fact. If absent,
broker:null and broker_probe_failed:false. If present, bounded owner-local
transportFailed()/peekPendingRenderEvent() reads copy only scalar metadata into
{transport_failed:bool,pending:null|{type:u32,sequence:u64}}. No primary/render FD,
resource pointer, device path, seat string or native graph is serialized. An
exception leaves broker:null and broker_probe_failed:true; it never changes the
Stop result. pending:null means no head was observed by that accessor, NOT proof
of an empty queue: failed transport deliberately hides the head. Record all
available facts at this branch and never fill missing facts from a later event.
The original broker queue/FD/service-lease behavior remains untouched.

## Frozen actual RED and GREEN consumer

The same first_stop_case.py and first_stop_oracle.py are used unchanged in both
epochs. The original owned Scope launches one actual NORMAL main, UID/GID1000,
groups[], real seatd/virtio/udev and admitted matching main/SDK/AQ/Rust. It checks
actual main/AQ mappings, exact SO_PEERCRED PID/UID/GID, nonblocking socket I/O,
UTF8 raw request + shutdown(SHUT_WR), EOF response framing, duplicate-key rejection,
actual request/response timestamps and all actual status identity/custody bits.
No main native output/GL/client success is inferred from these prerequisites.

Require observed actual STARTING stage8, followed by clean spontaneous STOPPING
stage5 with failed/rust_failed:false, ready/rust_ready:false, stopping bits:true,
code/snapshot_code/first_error/startup_error0, worker_alive, joinBusy2,
executor retained and both actual borrowed watches registered. No public Stop is
sent before this target observation. These are native/public prerequisites;
missing/failed prerequisites, timeout, process exit, budget exhaustion, lack of
spontaneous Stop, clock/probe failure remain SETUP, never target RED.

Only THEN absence/null first_stop or absence stop_requests on this authenticated
snapshot is FIRST_STOP_RECORD_MISSING: narrow missing-observability RED. The
current main2/Q3 is exact RED authority. No public Stop, error, lost context or
app crash is an alternative passing RED. Other invalid new fields fail the
frozen target but do not count as the expected missing-field RED epoch.

For GREEN, require the full object schema and genuine integer/null types,
timestamp between actual startup generation and current observation, genuine
executor custody, valid branch-specific direct revoke/error/mask and copied
broker facts. A terminal re-observation or this consumer's public Stop cannot be
first. Send one actual owner-startup stop: actual stop code0 stays distinct from
startup_error. Then take two actual snapshots. All three must preserve the whole
first object EXACTLY; counter increases at least once for explicit Stop and never
regresses (min(previous+1,UINT64_MAX), then nondecreasing). All status snapshots
retain authentic native task/watch/custody observations. No fake default object or
counter synthesized by the consumer is accepted. The finite native case does
not force u64 saturation or independently induce each rare origin/clock fault;
those source invariants need Root review and may receive separate actual tests.

Both epochs require actual main/descendant pidfd disposal, parent wait and PIPE
EOF before the original45s close; process disposal is not native cleanup or join.
Guest collection carries immutable evidence only. The host finalizer qualifies
an expected RED/GREEN only after original nested KVM/QMP/CLI/CID/same-daemon,
shared input lock, hashes and every original wait/EOF/cutoff close successfully.
No native frame, normal managers, Source7, UI or hardware qualification follows.

## Bounds and future cohort

Keep response2048B, request64B (actual server), each request2s, aggregate public
response64KiB, case35s work/45s close; reserve8192B before each discovery poll for
that reply plus three post-Stop checks. Preserve250ms poll spacing. No retry of
semantic observations or native initialization. No automatic fallback. Counter
field/type widths and the fixed branch table keep the status wire bounded; Root
must retain the actual server2048B gate and inspect final maximum-width format.
No trace/helper/observer budget increases are selected.

Outer unchanged: container8CPU/10GiB/512pids/2GiB tmpfs; guest6CPU/6144MiB;
guest660/700s, bootstrap720/740s, QEMU780s, boot800s, host840s; original signed
images/runtime image, KVM-only device, four mounts including readonly soleRust246,
network:none, exact daemon/CID/input lock/pre+post hashes. No new compiler/build
or extra archive provider. Existing Pixel1080x2424/scale2.625 request is retained;
no UI/capture dimensions or screenshots are qualified.

Current Rust246/read-only mount/header/export assertions apply to RED only.
The selected durable Rust record adds real Rust API/code and therefore requires
NEW actual Rust static archive + public header/export/source cohort, and a NEW
complete matching NORMAL main/SDK/build-result/RootQ before GREEN. Do not reuse
Rust246's mount/hash or old23-export attestation as evidence for that new linked
main. No second provider or fake ABI implementation is permitted. This prep's
admission intentionally refuses GREEN; a separate source-reviewed cohort adapter
must bind the actual new sole provider, loader/exports/SDK and readonly mount once
those future artifacts exist. Such artifact-admission changes may not alter the
frozen first_stop_case.py/first_stop_oracle.py target or this behavior contract.
No future success/pins are supplied or approved here. Actual qualified RED comes
before production, then the new matching Rust/main rebuild and unchanged target.
A causal replay can establish the real first cause only from those new records.
If it is live ingress, implement bounded ordered ingestion with immediate
invalidation, never discard the queued event to create readiness.
