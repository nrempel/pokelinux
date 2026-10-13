# Owned native focus identity

The session boundary now transfers the actual focused Wayland socket's owned
`SO_PEERPIDFD` reference. The Rust observer retains that reference, and the existing
lifecycle actor compares it with the pidfd it already owns for the launched app.
One protocol works on the tested Linux6.8 anon-inode and ARM7.1 pidfs kernels;
scalar device/inode equality and the earlier wire fields have been removed.
[Clean provenance](evidence/native-session-owned-focus-provenance.json) separates
actual host, actual ARM kernel, synthetic actor and original compositor evidence.
This does not qualify Pixel's exact retained kernel or production session launch.

## Ownership and authority

A focused frame contains only PID/UID/GID metadata and exactly one actual
`SCM_RIGHTS` pidfd. Null or locked focus carries no descriptor. A dedicated
credentialed source receiver checks packet size, truncation, source PID/UID,
incarnation, exact sequence, descriptor cardinality and the actual kernel pidfd
type. General app/policy IPC continues to reject every descriptor. All received
handles become owned values with atomic close-on-exec; every rejected handle and
queued reference is released when its owner closes. The frame limit remains1024
bytes and the per-turn drain limit16. There is no detached reader or source retry.

`OwnedFocus` separates serializable metadata from an internal shared owned handle.
The observer and actor share one retained reference without duplicating an FD on
every actor poll. The adapter holds its socket-derived temporary pidfd through one
atomic nonblocking `sendmsg`, then closes it on success and all failures. Failed
`SO_PEERPIDFD` calls never transfer ownership of a prospective descriptor number.
Backpressure closes the source output instead of queuing or renewing an attempt.

The unsafe-free `ProcessView` pins actual procfs, its kernel-generated self view,
and this process's descriptor/fdinfo directories. It rejects use after fork and
checks procfs magic and the fixed kernel pidfd type. Each bounded fdinfo read must
contain one canonical positive `Pid`. Both retained handles are read through that
same view; a joint final liveness poll follows both reads. Equal positive PIDs in
one qualified namespace and two still-living retained handles establish the same
kernel task. Keeping a pidfd does not prevent numeric PID reuse. Neither observed
PID reopening nor shared anon-inode scalar equality is an identity operation.
See [the leaf contract and primary kernel sources](../native-kernel-identity/README.md).

The actor also requires the registered PID/UID/GID, independent current grant and
app generation. Source facts cannot grant permission, clear a root lock/background
intent or launch an app. Nonselected principals become background-denied together.
A correctly typed dead app handle can remain a source fact; it supplies no positive
app authority and does not itself kill the healthy source. The actual adapter
exports null on its next observation of a dead socket peer. Synthetic repeated
unprovable dead-focus frames can keep advancing revisions; grant availability
under that stream is not qualified.

Every meaningful drained transition advances revision before coalescing. Keeping
revision across focused heartbeats requires live handle equivalence, not identical
numeric metadata. Repeated observation of the same retained object can validate
cached-sample consistency after app death; that reference equality never authorizes
publication. Old reads capture source/revision and the original100ms BOOTTIME
sample deadline. Heartbeats cannot extend the stamp. Final read validation checks
both request and sample deadlines again after identity IO. Source loss is terminal,
including after explicit resume discarded the prior sample. Resume requires later
sampling; real logind/power coordination remains open.

## Executed selections

| Boundary | Result and scope |
| --- | --- |
| Identity leaf, Harbour6.8 |14 checks pass after six actual-kernel, four component and two type-check failing cases. Malformed fdinfo is component injection; final-poll exit, zombie/reap, typed handles, inherited view and descriptor replacement use real processes/kernel objects |
| Source socket, Harbour6.8 |26 checks pass. The initial desired selection passed19 and failed valid owned transfer plus missing-handle refusal before implementation. Actual truncation cleanup, later exec, bounded FD counts and changed-handle/same-metadata fencing are included |
| Lifecycle components, host | Eight root-isolated policy/storage checks pass, including established source loss across resume and actual worker reap ownership |
| Synthetic source to Messages actor, host |15 groups pass with actual UID1000 source, UID11010 launched app, original SQLite owners and real seqpacket/pidfds; shared anon-inode identities do not prevent a valid read |
| Synthetic source to Messages actor, ARM kernel |14 groups pass on7.1.6-1-aarch64-ARCH. The same53 production Rust/manifest inputs match the selected host run |
| Original Hyprland to observer, ARM kernel |17 prepared original-compositor groups pass with the changed wire and current Rust observer. The prior adapter actually fails the focused-handle requirement. The same guest separately executes the14 leaf and26 socket/process checks |

The immutable84ac3f8 actor/observer baseline passes two unknown-state controls and
fails two desired positive admissions. This actor replay occurred after current
code drafting and is regression evidence, not chronological test-first evidence.
The observer and identity-leaf behavioral reds preceded their implementations.
A further actual source-owner test passed25 and failed1 before the constructor
was bound to its expected source PID; the corrected26-case selection passes.
The constructor now validates the retained source handle in its pinned view,
preventing an unrelated live task from becoming the source death watch.
Intermediate fixture setup, lint and runner failures remain private and are not
counted as behavioral reds. The authority fixture's initial grant during repeated
dead-focus changes was corrected without changing shared production Rust.

The original compositor is the matching installed Hyprland0.56.2-4 package and
headers. This preserves the earlier immediate-admission latch: original callbacks
precede the actual locked flag, so denial remains set through their intervening
focus callbacks until a later event-loop reconciliation. Lock completion and
presentation remain separate. Unload, stopped-source, pressure and death groups
retain bounded terminal denial; their observed error is `TimedOut`, not proof of
immediate EOF or a specific death-error path.

The compositor run records45 reap entries for44 distinct direct children. Host
actor records109 entries for94 distinct recorded children; ARM actor102 for88.
Service-owned children have owned-pidfd death and process absence; direct children
have wait results. Some cleanup uses bounded SIGKILL. Reaping is local process
ownership evidence and does not imply vendor cleanup, graceful shutdown or hardware
health. No child is intentionally detached.

## Remaining qualification

The selected Pixel module release remains
`6.6.127-android15-8-gee206ffe8752-ab16254410-4k`. Upstream6.6 interfaces and the
executed6.8/7.1 tests do not authenticate its exact image/config/KMI. Deployment must
protect procfs and qualify one common PID/user namespace view for the compositor,
launcher, credentials and actor. `NSpid` is a translation list, not a namespace ID.
The self-PID check alone does not qualify that deployment. No matched asset or
physical phone was accessed or changed.

The passing original-compositor epochs use320x240 virtual graphics prepared before
the first adapter load and accepted frame. Earlier cold render gaps above100ms
remain unresolved. Preparation is not source revival and the lease was not relaxed.
Bounded bytes/work do not prove a hard wall-clock bound for kernel/procfs IO.

The subsequent [actual original compositor → actor → installed Messages path](native-session-installed-app.md)
now passes six normal production Qt groups and three separate10-group diagnostic
epochs for local synthetic history. Its locked-graphics expiry remains an
availability gate; combined receiving/read isolation needs a fresh replay.
Trusted launch/configuration/plugin identity, genuine Omarchy
Quickshell/PAM admission/restoration, privileged Wayland containment, delegation/
popup ownership and secure output presentation remain separate gates. Forced
numeric PID reuse, translated namespaces, physical suspend/input/display, wake,
notifications and Pixel hardware remain unqualified. Ordered observation and
freshness do not withdraw queued bytes or promise zero publication after an
external transition the actor has not observed. The normal installed Messages
entrypoint still supplies local history without a production session-source selector.
The earlier [scalar ingress](native-session-ingress.md) remains historical evidence.

Harbour capacity measured after these jobs finished is16 logical CPUs and62.4GiB
RAM,41.8GiB available. The selected runs retain their original quotas. Future Rust
build/observer jobs allow12 CPUs/12GiB, and compositor containers10 CPUs/8GiB;
internal guest SMP and evidence deadlines stay explicit in each selection.
Independent jobs should run concurrently within host capacity. This operational
allowance is not a new hardware or scheduling qualification.
