# Native radio session owner

Status: the registration/publication authority is implemented in
`native-radio/src/owner.rs` and tested through real Binder on Harbour. It owns a
private callback inbox and the latest raw vendor response observation. It does
not yet own a transport worker, death watch, operation receipts or lifetime timers,
and it does not decode phone/IMS state. `prepare_until` bounds receive waiting
only; it is not an event-age, registration or publication deadline. The remaining
design below is unfinished.

A [held-registration Binder test](evidence/held-registration-provenance.json)
also proves that local stop remains usable while a synthetic registration is
held, and rejects activation after that call eventually returns success. The
scoped thread is test infrastructure, not the missing production worker.

The [execution record](evidence/radio-owner-provenance.json) separates the tested
publication boundary from the rest of the target architecture.

## One authority

Use one owner executor and one short owner-wide mutex for current session identity,
phase, first terminal reason, callback admission, pending operation, publication
revision and authoritative state. Keep parsing, assembly, allocation, Binder calls
and resource destruction outside that mutex. Do not expose arbitrary closures
that run under the lock.

The current in-process `SessionKey` is an opaque allocation identity held by an
`Arc`; prepared work also captures the exact owner allocation. Keys cannot be
recreated by another owner or a later attempt. Every callback endpoint permanently
captures its attempt identity. Numeric vendor registration IDs and response
serials are not session identities. Raw callback consumption is private to the
owner. A future cross-process observer/supervisor protocol must additionally
carry an owner incarnation and attempt identity; local pointer identity is not a
wire format and does not establish restart fencing.

A prepared result contains its originating attempt, applicable deadline, operation
or candidate identity and any base revision on which it depends. A fixed internal
commit method checks all of these under the same mutex used by invalidation and
replacement, then updates authoritative state and revision before unlocking.

- Invalidation first: reject the prepared result, even if it was already read.
- Commit first: preserve that earlier revision; invalidation then marks live
  session-derived state unavailable.
- Stop returns only after local admission and publication rights are revoked.
- Old endpoint failure cannot invalidate a replacement or advance its revision.
- Never drop the death watch under this lock: its destructor cancels and unlinks.

Current callers read point-in-time snapshots under the gate. A snapshot contains
a raw response, not a successful operation receipt or call model. Publication of
indications and segmented responses is rejected. This does not discard those
protocols from scope: semantic decoding and sound instance attribution remain
required before they can change phone state.

The callback sender has a storage mutex always taken after the publication gate;
there is no reverse lock path. All state validity decisions use the shared gate.
Poison recovery fails the current callback stream and closes its sender; a
poisoned owner cannot start another attempt. Failed revision reservations preserve
the current session. Live transitions retain room for terminal invalidation.

A prepared event retains its queue until dropped so old consumed work counts
against the retained-generation limit even if all endpoint wrappers are gone.
The owner itself retains a control. Publisher disappearance therefore means loss
of **all** strong queue owners, including controls, not merely endpoint-wrapper
drop. Stop, explicit cancellation and the retained service-death watch provide
lifecycle invalidation. Snapshot clones are historical caller-owned values; the
current generation cap is not a proof of total process-memory bounds.

A future watch API must send coalesced revision hints and read current snapshots. Hints must
not carry deferred mutations that can restore old state. A snapshot is a
point-in-time observation, not proof the session remains valid when the UI renders
it. Version the watch operation so waiter registration cannot lose a change.

## Registration and bounded transport work

Begin/replacement requires an expected current attempt (or idle), a free worker
slot, and an absolute deadline. Reserve resources, revoke old local state, then
install Registering. Arm death monitoring before registration. Keep callbacks
bounded and unpublished until guarded activation succeeds.

Use one owned transport worker and one reserved completion mailbox independent
of the callback queue. A terminal callback stream must not hide the fact that a
worker later returned. A late registration success cannot activate a cancelled,
expired or replaced attempt. It can still imply a remote side effect.

Separate logical session state from actual worker occupancy. Timeout or local
cancellation does not cancel `RadioTransport::register` or its synchronous
`submit_transact`. Do not free the worker slot, detach it or start another worker
until the call returns. Do not hide an indefinite join in Drop.

Production recovery from a permanently blocked call requires a supervised Rust
process boundary. Keep Binder endpoints, this gate and the authoritative reducer
together in that process. Revoke old process identity, terminate and reap before
starting a successor; if reaping stalls, remain unavailable. Parent/UI snapshots
must reject obsolete process/session identities. Reaping proves local resource
reclamation, not vendor cleanup or cancellation. This process lifecycle remains
to be built and tested; it is not a second production runtime.

Never send delayed cleanup using only a recycled vendor registration ID against
a new connection. Remote cleanup, registration overlap and producer reset rules
still need evidence from the actual vendor implementation.

## Deadlines and outcomes

Use absolute deadlines with a test-controlled clock. Phone-service expiry must
count suspend time; expiry after resume does not prove hardware wake support.
Check `now >= deadline` at activation, dispatch admission and final commit even
when timer delivery is delayed. The deadline governs admission at that locked
check, not realtime scheduling or UI delivery.

Initially permit one pending operation, one candidate and one preparing event.
An operation cancelled before dispatch permission is NotSent. Once permission is
granted, cancellation/failure before authoritative completion is OutcomeUnknown.
A completed result remains immutable. Never retry an unknown operation
implicitly or reuse an uncertain wire correlation key for a successor. Native
response attribution must use recovered wire contracts, not a local UUID alone.
These in-memory outcomes do not establish crash-safe durable admission.

Candidate lifetime starts at trusted first ingress so queue residence counts.
Duplicates and progress do not extend it. Timer work carries session/candidate
identity. Expiry or conflict invalidates the attempt rather than silently allowing
late fragments to seed a new candidate.

Bound payload sizes, queued and preparing events, candidate/output copies,
snapshots, pending/worker buffers, notifications and peer-retained old endpoints.
One bounded channel alone is not a whole-owner memory bound. Refuse replacement
when the configured retained-generation bound is reached.

## Producer identity remains unresolved

Fragments A0 and B1 from distinct messages can share message, slot and total,
produce contiguous indices and form a structurally valid hybrid. Local session
identity, deadlines and correct memory accounting cannot detect this. Keep
fragmented semantic publication disabled until producer serialization, wire
identity or a real reset/drain contract is established. Preserve assembly tests
as reconstruction evidence, not proof of message-instance identity.

## Required execution gates

Use barriers and a controlled clock for deterministic scheduling. Then exercise
the same production owner through the independent synthetic Binder peer.

1. Consume and prepare an event; invalidate or replace before commit; reject it.
   Test the opposite ordering and same-session base-revision conflict too.
2. Queue callbacks during registration; withhold publication until activation.
   Death, overflow, cancellation and expiry must defeat a late successful reply.
3. Withhold timer delivery; expired activation, dispatch and commit still fail.
4. Hold registration; expire it; repeatedly request replacement. Exactly one
   worker remains occupied and snapshots/stop stay responsive.
5. Release late registration success, reuse numeric IDs, and deliver old callbacks.
   None may resurrect old state or corrupt the replacement.
6. Race dispatch permission, cancellation and completion. Verify NotSent versus
   OutcomeUnknown and one immutable receipt.
7. Delay old revision hints until after replacement. They only trigger current
   snapshot reads, never apply stale state.
8. Exercise duplicate fragments at expiry and stale candidate timers. Preserve
   the A0/B1 ambiguity test as a known protocol limitation.
9. Reach aggregate limits including retired endpoints and worker-held buffers.
10. For supervised recovery, withhold process reaping and confirm no successor;
    after successful replacement, reject buffered old-process messages.

## Review and independent checks

[ChatGPT Pro review](https://chatgpt.com/c/6acd2549-3fc0-83e8-b79b-50860e459bdc)
completed in Chat mode at Pro power 5/5, marker
`PRO-BRIDGE-20261012-owner-c842`, zero context rounds. The completed response had
Copy/Regenerate controls with Stop absent. It is advisory, not execution evidence.

The low-level transport qualification factory still exposes before/after receive
checks; the new owner additionally fences final publication. `register` remains
synchronous without a cancellation API, and the existing runtime phone observer
decodes Android Telecom snapshots. Therefore raw radio data must not be presented
as native call state. Worker ownership and the native IMS semantic decoder must
be integrated before replacing that reference input.

The owner error paths keep lock guards local and prepared values as function
parameters; the guards therefore leave scope before the prepared-value destructor
reacquires the gate. This follows the independently checked
[Rust drop-scope rules](https://doc.rust-lang.org/reference/destructors.html#scopes-of-function-parameters)
and is also exercised by failed publication and wrong-owner tests.

A focused [implementation review](https://chatgpt.com/c/6acd2a5b-1b24-83e8-8e49-d7edc95e19cb)
completed at Pro power 5/5 with one context round, marker
`PRO-BRIDGE-20261012-owner-diff-a8d1`. Copy/Regenerate controls were present and
Stop was absent. The review found no remaining concrete ownership or concurrency
defect in the supplied excerpts. Revision exhaustion, poisoned-gate propagation
and retained prepared work were checked with failing tests before their fixes.
This review covers raw-response publication only; it does not establish native
IMS, call state or physical-device behavior.
