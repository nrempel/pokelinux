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
The next owner gate must fence already-consumed events and assembly/publication
work against replacement; it must not treat this test as proof of that race.

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
