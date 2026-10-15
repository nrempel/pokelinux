# Queued lock acknowledgements across storage work

The actor now rechecks each queued successful lock acknowledgement immediately
before every nonblocking send. If storage is busy, an operation is unresolved,
the actor is unavailable or Stop has begun, the reply becomes `unavailable`.
That downgrade is permanent, including a later retry after EAGAIN. Only a
successfully serialized typed lock `Ok` sets the private connection mark;
successful send clears it. This adds no authentication or unlock authority.

The preceding [planning-time guard](native-lock-storage-barrier.md) could queue
an idle lock `Ok`, then process a later connection's grant write in the same
pass. The next pass sent the queued `Ok` while storage was busy. The new actual
daemon/socket regression reproduces that behavior before the repair and passes
afterward. [Bounded provenance](evidence/native-lock-queued-storage-provenance.json)
keeps the behavioral RED, formatting prerequisite and successful runs separate.

A retained UID1000 client first completes a denied handshake, then a later root
admin connection completes a snapshot round trip. The actor is stopped in its
actual main poll: `/proc/PID/syscall` must show poll/ppoll with at least five fds,
excluding one-fd liveness polls. Both requests are queued before resume. There
is no retry or filtering of a failed stop. Before and after observing the reply,
the fixture requires a ready, nondegraded, busy actor, the exact storage child
SIGSTOP-held after ordinary grant sync, no grant reply and no successor.

The prior code returns `ok`; the repaired code returns `unavailable`. The grant
connection stays open and the original one-second bound is unchanged. Only
after Stop, grant refusal, actual client/actor parent waits0, stdout EOF and exact
storage pidfd death/proc absence does the fixture assert the desired reply.
Exceptional cleanup cannot pass. The test observes storage death; the numeric
storage Child wait belongs to the actor and is not independently traced.

All 40 host lifecycle regressions and both preceding planning-time cases also
pass on the repaired binary, with formatting and strict default/all-target/
all-feature Clippy. The first build label fails formatting before any test;
a fresh corrected label supplies GREEN. AArch64 daemon/library compilation is
separate from execution. The older planning-time code from `1106ab2` separately
passes both ordinary cases on an actual Arch ARM kernel; that does not execute
this newer queued-reply repair. See the [ARM proof](evidence/native-lock-storage-barrier-arm-provenance.json).

The new queued case is x86_64 host evidence. It induces no send EAGAIN, actual
suspend, authenticated clear or intervening write that starts and completes
before the send. A future clear needs stronger epoch/serialization rules and
truthful durable history; a busy check alone cannot qualify those transitions.
Original scope/PAM composition, broker result provenance, atomic attempt-to-armed
receipt, exact unlock, synced clear and normal Messages reopening remain gates.
These headless controls run no Source, Messages UI, original action or phone.
Source100ms and all original deadlines remain unchanged.
