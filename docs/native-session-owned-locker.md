# Owned mapped-locker facts

The compositor exports the actual mapped lock-surface peer to the native Rust
observer. This supplies owned process identity for subsequent trusted-unlock
work. It does not authenticate a password, clear durable lock intent or grant a
Messages read. Existing permissions and the100ms Source lease remain unchanged.

Every current Source frame requires a `locker` object:

```json
{"state":"unavailable"}
```

or, while locked, a mapped peer's PID/UID/GID with `state:"mapped"`. A mapped
locker requires exactly one actual SCM_RIGHTS pidfd and no focused app. An
unlocked frame requires unavailable locker state; its focused app may carry the
one pidfd instead. Old frames without `locker`, contradictory states, extra or
missing descriptors, non-pidfds and a mapped PID that does not match its live
kernel handle are refused terminally. General application IPC still rejects all
descriptors.

The adapter checks every enabled, DPMS-active output. Each must have a mapped
lock surface belonging to the same actual Wayland client. It obtains that
client's SO_PEERPIDFD and transfers the owned handle atomically with its facts.
No active output, an unmapped surface or inconsistent owners reports unavailable.
There is no PID reopening or scalar pidfs identity check in production.

Rust retains the handle in the internal lifecycle Snapshot. It compares live
identity through the existing pinned protected procfs view and increments the
semantic revision when ownership changes or becomes unavailable, including
intermediate changes drained in one turn. Stop and Source failure release the
observer's retained reference. Snapshot copies own their references; consuming
one later requires a fresh liveness/identity check. Source acceptance establishes
sample-time identity, not future authorization.

Mapped ownership is distinct from protocol completion. Hyprland's completion
flag can be set after a timeout without secure frames. Neither flag identifies
an exact accepted lock-protocol generation. Late first attachment can establish
the mapped process but cannot reconstruct an earlier protocol object. Trusted
PAM, exact-generation unlock evidence, Source restoration and durable policy
completion remain required before an unlock may restore native reads.

## Executed checks

An actual original Hyprland/Omarchy first-attachment RED observes the completed
mapped lock and independently retained Quickshell pidfd, then proves its fresh
accepted Source packet has neither locker metadata nor a transferred handle.
No authentication or unlock is selected. A separate host process RED refuses a
new mapped-owner packet before the repair; the corrected host suite passes38
groups using actual kernel credentials, pidfds and SCM_RIGHTS.

The new controls include11 malformed/contradictory locker cases,100 same-owner
heartbeats with stable descriptor count, CLOEXEC across an actual unrelated exec,
queued ownership disappearance/reappearance, and a dead locker packet followed
by a refused attempt to reactivate the failed Source. Existing focused-app
death/replacement and queue/deadline behavior remain covered. Two earlier host
build attempts lack dependency/lock prerequisites; they are not feature REDs.

A fresh actual Arch ARM run passes the38 process groups and five original
Hyprland/Omarchy integration groups. It compiles the current adapter against the
selected installed headers before attaching Source, then observes one matching
SCM_RIGHTS locker pidfd in the authentic packet and retained Rust snapshot.
Independent mapped-surface and already owned Quickshell handles agree. Scalar
pidfs comparison is an independent diagnostic in this Arch7.1 run only; the
production implementation uses retained handles and ProcessView.

Root independently rehashes163 local and111 external bindings, checks the raw
recvmsg trace, accepted snapshot, actual compilation and teardown.31 direct guest
waits include25 exits0, five explicitly checked not-ready Original status polls
before Source, and one expected shell SIGTERM. Four service/actor deaths have
pidfd/proc-absence observations, separate from independently traced parent waits.
Actual Docker CLI wait0 and fresh same-daemon exact-CID absence pass.

The prior run passes its narrow checks but remains a failed aggregate: test ELF
copies exceed the unchanged256MiB overlay cap. Staging only component binaries
on qualified guest `/run` tmpfs produces a fresh passing run within that cap;
Source, compiler flags, policy and deadlines are unchanged. The actual Rust build
also retains a failed final-copy step because its image lacks Python; its fresh
successor uses an exact-one executable shell copy. These are fixture failures,
not production behavior REDs.

See [bounded provenance](evidence/native-session-owned-locker-provenance.json).
This qualifies prepared first attachment to an existing mapped lock. It does not
qualify continuous lock transitions, Source recovery, original PAM, protocol
unlock, secure frames, a normal Messages window or Google6.6/physical Pixel
operation. No phone has been accessed. Trusted unlock and production session
provisioning remain open. A separate headless
[Original PAM reference prerequisite](native-original-pam-prerequisite.md) now
passes wrong/correct typed results. It does not add an exact lock-protocol
generation, native owned worker or restoration authority to these facts.
