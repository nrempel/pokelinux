# Exact lock-operation facts

The compositor Source identifies an observed accepted lock-protocol object with
a checked, nonzero counter scoped to its Source incarnation. Rust validates that
counter and an exact-object protocol-unlock occurrence before retaining them in
its internal Snapshot. These facts add no authentication, permission, command or
durable restoration capability. Root lock intent still denies native app reads.

Every current frame requires two explicitly nullable fields:

```json
{"lock_generation":1,"protocol_unlock":null}
```

After a genuine request on that exact accepted object:

```json
{"lock_generation":null,"protocol_unlock":{"generation":1,"sequence":11,"observed_at_ns":123456789}}
```

These are field examples, not complete packets. A known current generation
requires locked state. An exact occurrence requires unlocked state and names its
first publication sequence and immutable BOOTTIME observation. Ordinary unlocked
heartbeats retain the same record. Its historical timestamp does not renew an
authentication attempt or the unchanged100ms current-sample lease. Earlier frames
omitting either field are refused, including when its current value is null.

The ABI adapter observes new protocol candidates using only weak references.
It associates a candidate's actual lock-surface objects with every current
mapped manager wrapper on enabled, DPMS-active outputs. Only that association
and a live mapped peer promote the candidate and increment the counter. The
counter is independent of process IDs, pointer values, Source sequence and Rust
semantic revision. A known generation can remain known while mapping becomes
unavailable; any future authorization still needs current mapping and live owned
identity separately. Initial attachment to an already mapped lock supplies owned
process facts but leaves its generation unknown for that object.

Hyprland's generic manager-unlock listener runs before the exact object's
`unlockAndDestroy` listener. Source may publish an interim unlocked sample with
no occurrence. A short pending association allows only that same eligible object
to supply the exact event; an unanswered association is retired on the next timer
turn. Replacement invalidates the old association. In the pinned original code,
forced unlock makes lock objects inert, so a later request on that old object
cannot emit the exact signal. Destruction alone supplies no positive cause.
Candidate/listener storage is bounded and reclaimed outside callbacks. Source
never keeps protocol objects alive. Counter, storage or send failure closes the
channel without a new incarnation or implicit retry.

Rust inspects every actual packet before coalescing. It refuses zero, skipped,
regressed or reused generations; an occurrence without an observed eligible
accepted generation; impossible sequence/time or state; rewritten sticky records;
and loss of a record on an unlocked heartbeat. A new unknown lock retires the old
operation. Generation and event changes advance semantic revision, including
intermediate transitions drained in one turn. Later ordinary heartbeats preserve
that revision. Source expiry/death remains terminal. General app IPC still
rejects descriptors and has no protocol-unlock operation.

The lifecycle actor independently checks the typed Provider boundary. Impossible
protocol facts, changed generations or changed unlock records at a repeated
sequence or unchanged semantic revision fail the Source terminally. Cached exact
samples and stable generation/sticky-event heartbeats remain accepted. Three
negative RED groups execute18 mutations before the repair; four focused controls
and all16 lifecycle components pass on host and an actual Arch ARM kernel.
This selection runs no Source IPC or authentication. See
[actor provenance](evidence/native-session-protocol-actor-provenance.json).

## Evidence and remaining work

The independent original-compositor RED qualifies an accepted mapped object,
real protocol request, owner liveness and fresh unlocked Source, then fails the
required generation/cause equivalence. Three prerequisite groups pass and the
feature aggregate remains failed. No Rust observer or authentication runs in that
selection. Its prepared first version remains unrun after review caught an
incorrect native-pixel configure assumption and an absence-as-PASS classification.
The corrected fixture observes actual411x923 surface-local configuration while
native output stays1080x2424 at DPR2.625.

A separate real-packet host RED compiles17 cases against the prior wire: two pass
and15 fail, including all four required positive admissions. Its original runner
misclassifies the failures because of a log-marker parser bug. That frozen report
remains unchanged; a separate postrun review qualifies the actual runtime failure
witnesses. Negatives failing at their valid-prefix prerequisite are not new
semantic-refusal evidence.

After the repair, all55 host packet cases pass, including the original38 and17
new groups. They use actual credentials, pidfds and SCM_RIGHTS, with72 actual
fixture-child waits. Both crates pass formatting and clippy. An extra lifecycle
library test invocation under UID1000 fails seven explicit root prerequisites;
that whole build aggregate remains failed. A fresh compile-only continuation
uses the same37 source files and produces all five static ARM executables. That
compilation alone supplies no ARM execution or original integration result.

The actual current adapter then compiles against508 installed public headers and
feeds its private channel exclusively to the current Rust observer in an Arch
ARM guest. The eight-group run passes55 packet cases and12 lifecycle components
along with the live protocol checks. One living client performs two operations: generation1 supplies a
genuine exact event retained unchanged across three heartbeats, then generation2
clears it. Public forced unlock followed by a real request on the now inert
object produces no exact event. A second compositor/Source starts after its
client already mapped; generation and occurrence stay unknown even through its
real protocol unlock. No Python receiver competes with or relays the channel.

Twenty-nine direct guest waits and72 component peer waits exit0. The actual
subreaper reaches ECHILD with no adopted descendants. Both observers report
terminal timeout during owned shutdown and exit0; the driver observes no failure
before explicitly starting closure. The logs do not independently timestamp a
causal Source-death transition. Actual Docker wait0, fresh same-daemon exact-CID
absence and93 local/host plus93 external hash bindings pass. The256MiB overlay
cap is unchanged. See [bounded provenance](evidence/native-session-lock-generation-provenance.json).

An unwired internal actor-attempt primitive now retains one inert handle with
actor/Source identity, accepted generation, live exact locker, lock-admission epoch
and an original absolute BOOTTIME deadline. Retirement is sticky across observed
context excursions and never frees the occupied slot. A missing-API compile RED
precedes33 passing host components and strict Clippy; see the bounded selection in
[the lifecycle README](../native-lifecycle/README.md). This is typed Provider and
real-pidfd evidence only, without Source IPC, PAM or a broker result. No closure,
candidate, unlock, reset or durable-clear operation is provided.

Native owned authentication, broker/actor result binding, exact one-shot
restoration authority, synced durable policy restoration and a subsequent normal
Messages read remain open. Neither protocol completion nor a mapped frame proves
secure presentation. Production launcher/procfs/namespace provisioning, Google6.6
compositor qualification and physical Pixel tests remain separate gates. No
physical phone has been accessed.
