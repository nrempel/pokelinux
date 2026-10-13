# Native session facts and current read authority

The native Rust observer and the existing lifecycle actor now compose through a
small internal facts boundary. The separately qualified matching Hyprland adapter
exports actual central focus and lock observations. Three independent selections
qualify observer/process behavior, synthetic facts into actual Messages authority,
and prepared original-compositor facts into the observer. They do not qualify a
joint original-compositor-to-installed-Messages path or production session launch.
[Clean provenance](evidence/native-session-ingress-provenance.json) keeps those
selections and their exact source/ELF/proof hashes separate.

The observer owns the private nonblocking seqpacket endpoint and source pidfd.
Each bounded packet must have the expected actual kernel sender PID/UID, source
incarnation and next sequence. Socketpair creation credentials alone are
insufficient. Wrong/truncated/malformed packets, delegated senders, bounded-drain
overflow, original-sample expiry, disconnect or source death clear positive state
and close the channel. Source liveness never renews the100ms BOOTTIME sample.
Both transferred owned descriptors have CLOEXEC restored before later child exec.
There is no detached reader thread or implicit source revival.

Every drained meaningful lock/completion/focus change advances a semantic revision
before coalescing. Lock-to-unlock/refocus within one drain invalidates an old read
even if the final facts equal the previous facts. Identical heartbeats preserve
revision. The actor captures the original sample deadline in its non-cloneable
read stamp; later heartbeats cannot extend that stamp.

Only the existing actor owns grants, principal generations and publication. When
facts are required, initial/unknown/lost state denies data even if trusted fixture
controls request unlock or foreground. Focus must match the actual registered
PID/UID/GID, its living already-owned generation pidfd, and the observed pidfs
device/inode. This prototype requires PIDFS_MAGIC and refuses shared anon-inode
identities. It never opens a pidfd from the observed numeric focus PID. All other
principals become background-denied in the same transition. Independent root lock
and background intents remain denial conditions; source false cannot clear them.
Explicit resume discards the sample and requires later sampling; established
source-loss history remains terminal across that fence.

Facts are drained at each actor turn and before every request/publication attempt,
including EAGAIN retries. Current grants, lifecycle, exact live app generation,
worker incarnation, source revision and both original deadlines still gate each
send. SQLite/Binder remain in their separately supervised child. This establishes
ordered ingress and bounded freshness, not zero publication after an external
transition that the actor has not observed. Successfully queued bytes cannot be
withdrawn. Trusted pre-lock coordination and source-backed admission containment
remain necessary for stronger lock guarantees.

## Executed boundaries

| Selection | Evidence |
| --- | --- |
| Observer actual processes/sockets |16 cases pass, including inherited pidfd absence after a real later exec. Original pre-fence13-case selection failed11; unchanged-heartbeat and inherited-FD behavior each have separate executed reds |
| Resume history component | Real BOOTTIME and the internal typed policy boundary reproduce Some→explicit resume→None→fresh Some reopening the old source; the corrected latch passes. This is not an IPC source or physical-resume claim |
| Synthetic source→authority |13 corrected groups pass on actual ARM kernel7.1.6-1, with UID1000 source and UID11010 app, exact pidfs identity, coalesced transitions, original leases, real source SIGSTOP/EOF/death, EAGAIN and root-intent fencing. Two actual ARM baseline failures reproduce missing ingress and manual-control bypass |
| Host unsupported identity | Three denial groups pass on Harbour's6.8 kernel; its shared anon-inode pidfds cannot establish scalar identity. This selection precedes the small inherited-FD/loss-history corrections |
| Actual original compositor→observer |17 groups pass on the matching package and prepared320x240 virtual graphics. Identical fixture recipes reproduce the first-admission unlocked-value red; the latch preserves denial until later event-loop reconciliation |

The actor's source/service/app/messages-worker/policy-child reaps are observed with
owned pidfds and process disappearance. Some synthetic source processes required
bounded SIGKILL fallback; successful service exit does not imply every peer exited
gracefully. The compositor selection records41 reap entries for40 distinct owned
children. No fixture process is deliberately left detached.

The compositor's initial callbacks precede its actual locked flag. The corrected
adapter sets a denial latch in admission callbacks and keeps it through the
intervening focus event, clearing that local latch only on later timer reconciliation.
It exposes no file logger, public control command, grant, launch or unlock operation.
The frozen diagnostic probe provides only isolated test controls.

Cold virtual graphics produced source sample gaps above100ms and terminal Rust
expiry at800x900 and320x240. Local logging and the default socket buffer did not
remove the cold failure. The passing fixture maps/render-prepares a warmup window
before the first adapter load and before any accepted frame; this is preparation,
not revival. Exact cold renderer/driver cost remains untraced. Prepared unload,
pressure and death epochs observed bounded terminal denial with `TimedOut`; they
do not establish immediate EOF or a particular death-error path. The100ms contract
has not been relaxed.

## Retained-kernel and production gates

The selected Pixel profile still records matched module release
`6.6.127-android15-8-gee206ffe8752-ab16254410-4k`; its exact kernel image/config and
complete authenticated set remain unavailable. The scalar-pidfs prototype is
qualified on the newer Arch guest, not that Pixel set. Public
[Android15-6.6 filesystem sources](https://android.googlesource.com/kernel/common/+/refs/heads/android15-6.6/fs/)
do not establish pidfs support for the retained set. No matched kernel is replaced
or assumed upgraded to make this prototype work.

The next unified identity design transfers the focused socket's owned
`SO_PEERPIDFD` over the private credentialed channel and retains that handle through
publication. Linux6.6's [socket implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/net/core/sock.c)
uses the retained peer process reference. Comparing both already-owned live
handles through one pinned protected procfs namespace view can remove the pidfs
scalar dependency; this is an architectural inference awaiting executed tests.
It requires handle-aware revisions, bounded ancillary parsing/FD cleanup and
qualified namespace configuration. Numeric PID lookup, an anon-inode scalar
comparison or an unqualified compatibility fallback is not admitted.

Other remaining offline gates are trusted compositor executable/configuration and
plugin provisioning; genuine Omarchy Quickshell/PAM lock admission/restoration;
privileged Wayland containment, popup/delegation rules, secure output presentation;
a joint compositor/actor/installed-app run; logind/power/resume ingress; notifications
and wake. The normal installed Messages entrypoint still selects local history
without a production source selector. Kernel/KMI/SELinux/device behavior and
physical suspend/input/display remain explicit hardware/backend gates. No physical
phone, vendor executable or proprietary input was used by these session fixtures.
