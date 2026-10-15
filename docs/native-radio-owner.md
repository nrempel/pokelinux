# Native radio session owner

The installed production radio service/status tracer now passes on an actual
generic ARM kernel after a genuine missing-history-projection RED. A one-line
service repair preserves all five other ELF bytes and the outer oracle. The
packaged unit, native worker and real restricted status client carry one raw
callback into diagnostics, then peer loss becomes unavailable with retained
history. Native worker wait9, systemd Main exit0, empty unit cgroup and same-journal
restart exit78 qualify separately; no new registration occurs. All45 host tests,
strict checks and full guest/outer/input closure pass. This is public synthetic
Binder and operator diagnostics, not native IMS or hardware qualification.
Exact installed blocked-call/pressure, device policy/producer/reset and phone
gates remain. See [installed service](native-radio-service.md) and the preceding
[production worker tracer](native-radio-production-worker.md).

A later [cooperative bootstrap checkpoint](native-radio-bootstrap.md) adds a new
production-library parent-bound worker entry before Binder, qualified separately
by actual generic ARM Binder parent-death and live wrong-parent refusal controls.
It preserves durable uncertainty and the installed service-manager requirement;
that historical selection alone provides no complete production worker, vendor
cleanup, IMS or phone qualification.
The historical execution selections below retain their original scope.

Status: the registration/publication authority is implemented in
`native-radio/src/owner.rs` and tested through real Binder on Harbour. It owns a
private callback inbox and the latest raw vendor response observation. It now enforces an explicit absolute session deadline using Linux `CLOCK_BOOTTIME`
at every gate acquisition, including activation, callback admission, preparation,
publication and snapshots. Expiry counts suspend time and withdraws local state
even with timer delivery withheld. `begin_until` replaces unbounded owner begin.
`TransportSession` now owns one registration worker, its reserved JoinHandle
completion and retained death watch. `RadioOwner` remains the publication gate.
One bounded owned request now distinguishes queued, NotSent and OutcomeUnknown
under this gate. Qualified final semantic receipts and timer delivery remain
unfinished, and it does not decode phone/IMS state. `prepare_until` still bounds
receive waiting only; candidate/event-age and operation deadlines remain open.
See [deadline execution](evidence/radio-deadline-provenance.json).

A [held-registration Binder test](evidence/held-registration-provenance.json)
also proves that local stop remains usable while a synthetic registration is
held, and rejects activation after that call eventually returns success. The earlier
scoped thread was test infrastructure; current held scenarios use the production
owned lifecycle and supervisor described below.

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
owner. The supervisor now checks a random 128-bit owner incarnation, the source
connection identity and attempt 1 (one attempt per process). A future authenticated
app observer protocol must carry those identities too; local pointer identity is not a
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
reclamation, not vendor cleanup or cancellation. Local termination/reaping and
durable uncertainty now pass real Binder testing, described below. Installed
service-manager integration and safe successor registration remain unfinished.

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

An advisory ChatGPT Pro review
completed in Chat mode at Pro power 5/5, marker
`PRO-BRIDGE-20261012-owner-c842`, zero context rounds. The completed response had
Copy/Regenerate controls with Stop absent. It is advisory, not execution evidence.

The low-level transport qualification factory still exposes before/after receive
checks; the new owner additionally fences final publication. `register` remains
synchronous without a cancellation API, and the existing runtime phone observer
decodes Android Telecom snapshots. Therefore raw radio data must not be presented
as native call state. Supervised process recovery, candidate/operation deadlines and the native IMS semantic decoder must
be integrated before replacing that reference input.

The owner error paths keep lock guards local and prepared values as function
parameters; the guards therefore leave scope before the prepared-value destructor
reacquires the gate. This follows the independently checked
[Rust drop-scope rules](https://doc.rust-lang.org/reference/destructors.html#scopes-of-function-parameters)
and is also exercised by failed publication and wrong-owner tests.

A focused implementation review
completed at Pro power 5/5 with one context round, marker
`PRO-BRIDGE-20261012-owner-diff-a8d1`. Copy/Regenerate controls were present and
Stop was absent. The review found no remaining concrete ownership or concurrency
defect in the supplied excerpts. Revision exhaustion, poisoned-gate propagation
and retained prepared work were checked with failing tests before their fixes.
This review covers raw-response publication only; it does not establish native
IMS, call state or physical-device behavior.

## Owned registration worker

`TransportSession` performs one registration attempt per supervised process. The
worker arms and retains the death watch, takes dispatch permission under the owner
gate, and executes synchronous registration outside it. Its owned JoinHandle is
the reserved completion mailbox, independent of the callback stream. The service
only joins after `is_finished`; stop and snapshots never wait for registration or
unlink the watch. Cancellation and expiry preserve occupied worker capacity. A
late result records dispatch and Binder outcome separately from guarded activation.

The worker and watch remain owned. Accidentally dropping an unfinished worker
explicitly aborts its process; it does not detach or hide an indefinite join in
Drop. `Supervisor` now provides tested parent ownership; installed service-manager
integration is still required before production deployment. A
unit test starts a disposable child and proves SIGABRT; ARM64 user-mode QEMU tests exercise completion ownership; the ARM64 system-QEMU
guest now also runs the subprocess signal test under its own kernel.

Even after completion, this lifecycle refuses another registration attempt. No
`clear` is sent automatically. Its one-way acknowledgement cannot establish remote
cleanup. Supervisor termination/reaping and a durable admission marker across process
restart are now implemented. A qualified vendor reset/cleanup contract remains
required before permitting any successor registration. The [owned request boundary](native-radio-requests.md) now qualifies local
admission and one-way submission; final outcomes, durable admission, safe
subsequent operations and native IMS semantics remain open.

## Supervised reclamation and durable uncertainty

`Supervisor` revokes output admission before signalling its child. `poll_reap`
retains the owned process on a pending or failed wait, without blocking or creating
an output-reader thread. Native output reads use a nonblocking pipe and caller
bounds. Process identity, source connection and attempt must agree before a frame
can be accepted. This is provenance fencing, not app authentication or a complete
native phone IPC/reducer contract.

Before spawn, the supervisor creates a mode-0600 admission marker in a private
directory belonging to the service UID, then syncs the file and directory. Marker
creation is exclusive. It records uncertainty conservatively before the child can
register and survives spawn failure, termination, reaping and supervisor restart.
No code here deletes that marker or sends `clear`. A marker blocks every new
registration launch. This implements safe unavailability while remote state is
unknown; it does not restore vendor registration or define a reset protocol.

The synthetic peer's permanent-registration branch has no release control. The
actual Rust child retains its blocked worker while stop, snapshots and replacement
refusal remain responsive. The Rust parent sends SIGKILL, reaps it, rejects its old
tag, and proves restart refusal from the persisted marker. Deferred reaping and
kill/wait failures have separate controlled component tests. These do not emulate
an unkillable kernel task. See [execution](evidence/radio-supervisor-provenance.json).

The native x86-64 failed-exec subprocess test passes. Under pinned ARM64 user-mode
QEMU, spawning a nonexistent executable unexpectedly returns success; the
fail-closed destructor then aborts when the assertion unwinds. That subprocess test is now compiled for ARM64 but explicitly ignored under
user-mode QEMU. Journal, provenance and lifecycle state tests still run there.
The system-QEMU guest runs both native process tests under an Arch Linux ARM
kernel, unprivileged, and passes. The matched Pixel kernel/policy and installed
service-manager behavior remain open; scoped ARM64 Binder qualification now has
its separate report below.

## ARM64 kernel process execution

`tools/native-process-vm.py` boots the existing prepared Arch Linux ARM image using
a temporary disk overlay, a read-only project share and a fresh results share. It
uses no network, physical phone or host device transport. The guest selects the
exact test executable from Cargo's recorded artifact manifest, runs the components
as `alarm`, then explicitly executes the two ARM64 process tests. The first VM
run failed because those tests were excluded from the ARM64 binary. After replacing
architecture exclusion with an explicit user-mode ignore, all gates pass.

This demonstrates SIGABRT ownership behavior and failed-exec/durable-admission
semantics under an ARM64 Linux kernel in system-QEMU. It does not demonstrate
ARM64 Binder or Pixel kernel behavior. The earlier pinned user-mode failed-exec
observation remains recorded rather than being reclassified as a native failure.
See [the separate evidence class](evidence/arm64-radio-process-provenance.json).

## Vendor cleanup checkpoint

Focused REA static evidence shows client-list/callback cleanup and separately
managed pending modem requests. One-way clear and local reaping establish no
acknowledged remote reset. The durable uncertainty fence remains unchanged.
See [cleanup contract](native-radio-cleanup.md) and
[provenance](evidence/vendor-radio-cleanup-provenance.json).

## Factory recovery evidence

A public-only containment precursor now passes six actual Arch ARM7.1.6 kernel
groups. The unchanged production supervisor first allowed an authored child's
outside-root read and independently observed Unix connection, establishing the
new containment requirement's RED. An isolated authored owner then enforces
namespaces, final UID/GID/groups/capabilities, no-new-privileges, seccomp and
resource limits. Held controls, forbidden access, attempted second exec, task
limits, queued readiness after the original BOOTTIME deadline and parent death
pass with actual child/thread waits and an empty cgroup. The largest recorded
status/stop sample is2.413108ms, an observation rather than a realtime guarantee.

The earlier bootstrap failure and guest-pass/wrapper-failure labels remain
failed. Only a bounded actual QEMU wait corrects the latter wrapper; it changes
no owner/child bytes. Root independently verifies289 preserved bindings. See
the [public authored tools](../tools/native-radio-load-only/README.md) and
[separate evidence](evidence/native-radio-containment-provenance.json).
Production radio code is unchanged. Original linker/Bionic/vendor execution,
loader namespace/maps and property behavior remain a separate next boundary;
these harmless controls authorize no registration, reset or successor retry.

The [factory recovery contract](native-radio-recovery-contract.md) identifies the
actual native V1 service adapter and manager/complete producers through static
REA evidence. Its scoped locks and one-way clear do not establish remote drain,
pending modem cleanup, safe successor registration or all-path producer identity.
No original vendor code was executed. Retain current uncertainty and fragmented
semantic-publication fences until the specified original runtime gates qualify.

## ARM64 Binder checkpoint

The owned production transport/registration/request and process-supervisor
boundaries now execute through actual Binder in an unchanged Arch ARM7.1.6
system-QEMU guest. Nine Rust/manual peer scenarios cover real callback transfer,
cancellation/absolute expiry with timers withheld, capacity retention, late
registration refusal, death, request uncertainty and permanently blocked child
reaping with durable successor refusal. Three separate genuine Android NDK
interop scenarios cover callback/metadata and death. Successful join/reap is
local reclamation; no vendor cleanup or retry authorization follows. See
[scope and pinned evidence](native-arm64-binder.md). Pixel policy, actual vendor
manager/reset and installed IMS/call service remain unqualified.
