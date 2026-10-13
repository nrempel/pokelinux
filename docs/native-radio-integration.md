# Native radio integration: next execution gate

Status: bootstrap, native registration, raw/segmented requests, response delivery
and observed teardown pass across processes on the owned x86 emulator. Raw and
segmented indication delivery and shared inbox overflow now pass too. See
[execution evidence](evidence/radio-indication-provenance.json). Production session
recovery and the remaining failure gates below are not complete. Android is an
independent synthetic test peer. No physical phone was accessed.

## Test peer bootstrap

Use an instrumentation APK targeting its own test package. The native harness
starts it through the exact SDK37 IActivityManager interface and supplies a native
IInstrumentationWatcher. The APK passes its live synthetic Binder endpoint in a
status Bundle. The native watcher validates the expected test identity, retains
and queues the handle, acknowledges the watcher call, and returns. Registration
runs outside callback dispatch. Keep the peer alive until controlled shutdown.

Android documents [Binder values in Bundles](https://developer.android.com/reference/android/os/Bundle#putBinder(java.lang.String,%20android.os.IBinder))
and [instrumentation status delivery](https://developer.android.com/reference/android/app/Instrumentation#sendStatus(int,%20android.os.Bundle)).
The APIs informed the design; the recorded emulator execution now proves this
bootstrap on the pinned image.
Pin and verify the test-only framework transaction ABI against the installed
image. Do not copy it into the production radio client or guess transaction IDs.

Record the image, client/peer UIDs, SELinux domains and Binder device. Keep SELinux
enforcing; no root or permissive-domain shortcut. This tests handle transfer and
transport, not production vendor service discovery or access policy.

## Required end-to-end tests

Before accepting assembled indications, use the [original accumulator cases](evidence/indication-assembly-provenance.json) as evidence for valid index ordering. Bound bytes, fragment count, index range, active messages and incomplete-message lifetime separately. Reject conflicting duplicate payloads and totals explicitly; those are deliberate local validation rules, not claims about the producer. Test same-ID interleaving and late fragments. A local epoch or timeout does not establish vendor-side message identity or a flush boundary.

The enclosing indication adapter now passes seven original-code cases covering
slot filtering, ID reuse, interleaving, changed totals and exception retention.
See [evidence](evidence/indication-adapter-provenance.json). The accumulator
test alone cannot establish routing scope or cleanup. For example, an old index-0
fragment and a new index-1 fragment can have the same ID, slot and total as a valid
single message. Without another discriminator or a producer serialization rule,
no consumer algorithm can distinguish these histories.

- Two distinct callbacks, registration ID and error propagation, original wire
  fields/flags, raw and lossless segmented requests at 2048, 2049 and 4096 bytes.
- Callback before registration reply, callback with no concurrent client request,
  and simultaneous response/indication delivery. Start the Binder pool first;
  hold no session lock across remote calls or blocking queue operations.
- Death before/after registration and submission; new callback endpoints on
  reconnect. Reused client IDs and serials must not let late old-session events
  alter the replacement session. Retain the death recipient strongly.
- Local inbox overflow and driver queue pressure as separate cases. Terminal
  receiver failure must wake the owner and resolve pending operations explicitly;
  one-way handler errors are not acknowledgements to the remote sender.
- Malformed remote parcels, metadata length mismatch, bounds, trailing data and
  unknown transactions. Verify bounded resource use through the actual adapter.
- Withheld synchronous registration reply: bounded session-owner shutdown and
  exclusion of late results. A watchdog does not cancel an in-flight Binder call.
- Both callback flag modes in the synthetic peer. Actual vendor sender flags are
  still unknown. The current receiver writes no success exception header for
  response codes 1/2; a synthetic synchronous caller expecting readException()
  must expose this difference. The original receiver also lacks that header;
  do not report this conditional case as a proven vendor incompatibility.

Registration original-proxy execution is now covered by
[the five passing cases](evidence/registration-provenance.json). Separate-process
native radio registration now passes against the synthetic peer, along with raw
and segmented round trips and observed teardown. The rest of these gates remain
open except raw/segmented indication delivery and shared inbox overflow, now
covered by the native endpoint. Single-candidate assembly also passes the real Binder path. Session-level
reassembly lifetime, production owner integration, pending
operation failure and reconnect are still required. The overflow test polls the
endpoint failure state; it does not prove a production owner wakes correctly.
The separate blocking inbox wait now passes component wakeup tests and normal
real-Binder delivery. See [evidence](evidence/callback-wait-provenance.json).
Explicit callback cancellation and remote malformed-indication wakeup now pass
[the extended test](evidence/callback-cancel-provenance.json). Fresh endpoint state
is separate even when the synthetic peer reuses ID 42. Service death after
registration, watch-drop invalidation and already-dead link failure now pass
[real Binder tests](evidence/radio-death-provenance.json). Death during registration also passes
[the extended scenario](evidence/registration-death-provenance.json). Next:
restart/reconnect and late-callback epoch replacement, then deadline/aggregate-budget integration and
pending-operation resolution. These primitives are not yet a complete owner.

Late delivery to a still-live failed old endpoint now passes the
[replacement test](evidence/late-callback-provenance.json), using identical serials
and distinct payloads on old/new callbacks. This proves endpoint/queue separation.
That endpoint test alone does not fence already-consumed events. The raw-response
owner scenario below now covers consumed-event publication; assembly lifetime and
native phone-state integration remain separate gates.

The harness also has a `death-during-registration` scenario. It arms the watch
before submitting registration; the peer exits after validating the request and
before writing a reply. Acceptance requires both transaction `DeadObject` and
callback-stream `ServiceDied`, without assuming their delivery order. The harness
then force-stops its own package and runs the ordinary suite in fresh client and
peer processes. This is process-level test recovery, not in-process reconnect or
cancellation of an indefinitely hung synchronous call.

## Independent review

ChatGPT Pro review completed in Chat mode with Pro power selected, matching marker
PRO-BRIDGE-20261012-radio-42f7 and one context round. It recommended the
instrumentation bootstrap and the failure cases above. The first UI entry sent
only the marker; the complete brief was supplied and verified in the context
round. The review is advisory. Local source inspection independently confirms
weak death-recipient storage and the rsbinder 0.12 warning about late replies
after certain driver failures. The subsequent execution report above supplies bootstrap runtime evidence.

Passing this gate will establish emulator IPC and recovery only. ARM64 kernel
Binder, vendor permissions, modem/IMS behavior, GPU, camera and physical audio
remain separate qualifications.

An additional Pro review completed with Chat and Pro selected, marker
PRO-BRIDGE-20261012-segments-b731, zero context rounds and terminal response actions
observed. It recommended the enclosing-adapter matrix, absolute lifetime limits
that duplicates cannot extend, immutable callback epochs, and serialized
completion/invalidation. These are review recommendations, not execution evidence.
The original accumulator execution independently verifies ordering and malformed
index behavior; producer identity and reset guarantees remain unknown.

## Owner loss

Dropping `CallbackInbox` now invalidates both endpoints immediately with
`ConsumerGone`, preserving an earlier terminal reason. Its weak queue reference
does not retain the publisher or break publisher-loss wakeup. The regression
failed before implementation, then passed on Linux and ARM64/QEMU; the six
Binder dispatch tests and full radio IPC suite also passed on Harbour. See
[the execution record](evidence/inbox-drop-provenance.json). This closes implicit
owner teardown only; it does not fence events already taken from the inbox.

The [session-owner design](native-radio-owner.md) records the accepted publication
and recovery boundary, the completed independent review, and deterministic gates
for implementation. The publication portion is now exercised below; other transport operations, candidate/operation deadlines and semantic phone-state
integration remain unfinished. Owned registration and local supervision are covered below.

## Native publication owner integration

The `session-owner` IPC scenario uses the actual `RadioOwner`, endpoints and
transport. It verifies pre-activation refusal, publication of the initial raw
response, cancellation after a response has been consumed, replacement after
consumption with recycled vendor ID 42, old-endpoint rejection, and withdrawal on
stop. The original low-level IPC suite remains an independent transport test.

[The owner evidence](evidence/radio-owner-provenance.json) also covers failed
transition preservation, poison propagation, wrong-owner and stale-preparation
cleanup, overflow withdrawal and retained prepared work. This is an in-process
raw-response publication boundary. It does not establish worker supervision,
absolute deadlines, durable operation outcomes, notification delivery, native IMS
or physical phone behavior. These remain required in the
[owner plan](native-radio-owner.md).

## Held registration cancellation

The `cancel-during-registration` native scenario runs the actual Rust transport
and owner against a separate synthetic Binder peer. Test-only transactions 110
and 109 acknowledge a held registration and release it. The native caller stops
the owner before release, checks failed state and an unfinished registration
thread, then joins the returned call and rejects its successful ID 42. Both
barriers have bounded synthetic waits; the outer emulator harness also bounds
execution. No sleep establishes the ordering.

The first run failed with `UnknownTransaction` before the peer controls existed.
After adding those controls, all four scenario markers and the existing owner,
registration-death and ordinary IPC scenarios passed with SELinux enforcing.
See [source/binary evidence](evidence/held-registration-provenance.json).

This qualifies the existing cancellation/publication gate through a synchronous
Binder call. It does not implement the production worker, prove permanent-hang
recovery, cancel vendor registration, or establish cleanup/re-registration safety.
The added Java code is only the independent test peer.

## Suspend-aware session expiry

The `expire-during-registration` scenario holds the actual Rust registration in
Binder and advances a controlled absolute clock to the session deadline. No timer
is delivered. The owner snapshot fails with `Expired` while registration is still
blocked; the peer is then released, and successful ID 42 cannot activate. The
production clock uses Linux `CLOCK_BOOTTIME`; the controlled jump is synthetic
suspend-time evidence, not a hardware suspend/wake test. Four component regressions
cover expiry at activation, final publication, snapshots and attempted replacement.
See [execution provenance](evidence/radio-deadline-provenance.json).

The current held scenarios use production `TransportSession`, replacing the
earlier scoped test thread. Local permanent-hang reclamation is now covered below;
candidate/operation deadlines and vendor cleanup remain unfinished.

## Production registration lifecycle integration

Held cancellation and expiry now run through the owned Rust worker and retained
death watch. Sixteen replacement attempts return `WorkerOccupied`; repeated stop
and snapshot checks preserve the first terminal reason. Completion remains
available despite terminal callback state. Late successful ID 42 is returned as a
dispatched result with rejected activation; capacity is reclaimed only after
worker completion is joined. A further attempt returns `AlreadyAttempted`, without
sending `clear` or retrying registration.

`owned-service-death` proves watch retention after activation.
`owned-registration-death` proves watch/error invalidation during registration.
A component regression first exposed `DeadObject` being labelled InternalFailure
before watch delivery; the result now maps to ServiceDied while preserving earlier
terminal reasons. See [source and binary provenance](evidence/radio-worker-provenance.json).

These tests still have bounded synthetic registration barriers. A watchdog abort
regression is local process evidence, not supervised recovery from permanent
Binder blockage, vendor cleanup or modem/IMS correctness.

## Permanently held Binder registration

`supervised-blocked-registration` launches a separate native Rust child under the
production `Supervisor`. The independent peer permanently withholds registration,
with no release transaction. The child proves cancellation/snapshot responsiveness
and occupied-worker replacement refusal before reporting its process/attempt tag.
The parent revokes that tag, sends SIGKILL, retains occupancy until wait confirms
reaping, then rejects old output and all registration retries. A recreated
supervisor also refuses launch from the synced private admission marker.

The outer harness force-stops the synthetic peer after the scenario. That separate
test teardown is not evidence of vendor cleanup from client death. No successor
registration is allowed by this implementation. Safe reconnection needs recovered
reset/cleanup evidence. [Provenance](evidence/radio-supervisor-provenance.json)
separates real Binder reclamation, controlled reaping failures, ARM64/QEMU state
tests and the unresolved native ARM64 subprocess gate.

## ARM64 process qualification

The process boundary now has separate [ARM64 system-QEMU evidence](evidence/arm64-radio-process-provenance.json).
The guest runs the actual Rust component binary and native subprocess tests as an
unprivileged user under its own Arch Linux ARM kernel. It does not use Android or
exercise Binder. The x86 Android emulator remains the real Binder evidence class;
user-mode ARM64/QEMU and physical Pixel qualifications remain separate.

The shared `tools/native-process-vm.py` runner now also executes the exact
message-outbox and native-suspend artifacts. Prepare their ARM64 manifests,
including `tools/build-native-suspend.sh arm64-process`, before the guest run.
The integrated run preserves both radio process checks and adds27 suspend
checks under an actual ARM64 Arch kernel; this does not qualify Pixel Binder
or hardware suspend. See [native suspend](native-suspend.md).

## Owned request admission

The [owned request boundary](native-radio-requests.md) now qualifies six actual
x86 Binder scenarios: expiry with timer delivery withheld, cancel/stop/death before
dispatch, complete4096-byte segmentation, and successful one-way return followed
by cancellation and a held late raw echo. Guarded part admission records local
uncertainty before Binder; the echo does not accept the operation. Five additional
component checks pass on x86, ARM64 user-QEMU and under an actual ARM64 kernel.
Only one request attempt is allowed. Final semantic receipts, durable admission,
wire correlation/subsequent work, permanent request-call blockage/driver pressure,
installed native IMS and actual vendor/modem behavior remain open. Existing
registration supervision/reaping does not prove request cancellation or remote
cleanup. See [separate source/runtime provenance](evidence/radio-request-provenance.json).

The independently authored [bounded IMS reader](native-ims-reader.md) now matches
148 valid scalar/opaque-field observations from original pure reader execution.
It is not connected to response publication. A semantic response adapter must
qualify complete layouts and producer/method/error/slot identity, enforce full
packet completion, and pass the owner gate before exposing state. Raw snapshots
and successful getters alone remain insufficient.

## Exact factory adapter boundary

The current [factory recovery trace](native-radio-recovery-contract.md) separates
the original generated native V1 binding from the actual `libril-aidl.so` service
adapter and `libril_sitril.so` manager/producers. Their static identity/flags are
qualified separately from this synthetic peer. An original adapter fixture cannot
prove manager deletion or pending modem cleanup if it replaces those collaborators;
the report defines distinct adapter, manager/producer and pending/reset gates.
