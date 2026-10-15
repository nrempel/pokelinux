# Owned session restoration through the lifecycle actor

A sealed authenticated-and-closed completion now transfers one exact running
attempt into retained Armed state. Only a later matching protocol-unlock event
can request a durable clear. The actual service loop serializes this request
through its sole policy Worker and retains denial until the synced completion.
There is **no production completion constructor or authentication ingress**.
The only successful-result producer is a private `cfg(test)` fixture.

Two actual host service integrations pass after separate behavioral failures.
They use the real actor loop, root-installed daemon/launcher, policy worker,
kernel credentials, retained app pidfds, app socket requests and policy files.
Authentication, typed Source facts and returned history are authored test inputs.
This is a headless downstream restoration boundary, not genuine authentication,
Source IPC, original Omarchy/Qt Messages or a completed phone unlock tracer.
[Provenance](evidence/native-session-restoration-provenance.json) records the
selected sources, failures, artifacts, profiles and closure limits.

## Retained authority and storage ordering

Arming matches actor incarnation, attempt id and the original absolute BOOTTIME
deadline, revalidating the Source, accepted lock generation, retained live locker
and lock-admission epoch before moving the original context. It never releases
capacity and then looks up a lost attempt. A capacity-only closure stamp cannot
arm, release an Armed result or remove a successor. Replayed/inexact results fail.

Armed retains the original context, actual arm time and Source sequence baseline.
A historical locker exit can precede the matching protocol event; death alone
cannot unlock. A new lock, forced unlock without that event, Source failure or
expiry, suspend or degradation retires authority. Later fresh facts cannot revive
it. The original100ms Source lease and at-most30s attempt budget are unchanged.
A matching event needs the same generation, a later sequence and observation
strictly after arming.

Planning a clear moves Armed to Clearing exactly once. Memory remains locked and
durable pending remains true until the real worker returns its exact successful
completion. The write deadline is bounded by the original attempt and one second.
Completion rechecks actual BOOTTIME against the write's own deadline as well.
Known synced bytes remain truthful history even when publication is refused.
Stale/uncertain completion leaves denial and cannot receive a clean shutdown seal.

A new lock during Clearing fences memory immediately and refuses cached success.
The first unresolved new lock owns a bounded, nonrenewable deadline for a newer
true write. After a *known* old false commit, the service immediately dispatches
that true replacement through the same worker, before other publication/ingress.
It never retries an uncertain clear. Clean Stop also requires no required-true
intent. Grants and receipt history survive both replacements.

Queued lock success now retains the worker's admission sequence. Every send
rechecks it, even when another write started and completed between attempts and
storage is idle again. A changed sequence permanently replaces success with
`unavailable`. This complements the earlier planning/busy guards.

## Executed evidence

| Selection | Executed result | Scope |
|---|---|---|
| Missing completion/arming/clear APIs | Compile101, zero bodies | API prerequisite only |
| Initial authority implementation |47 host tests pass | Auth/closure and successful finish are models; existing storage components remain real |
| New lock during model-owned clear | One behavioral failure | Cached `Ok` instead of `Unavailable` |
| Missing required-true API | Compile101, zero bodies | Separate API prerequisite |
| Required-true authority repair |50 host tests pass | Models, including known-commit ordering and first new-lock deadline |
| Stale callback time after actual write-budget expiry | Two behavioral failures | Both clear and required-true returned `Ok`; actual-clock checks repair them |
| Formatting prerequisite | Exit1, zero bodies | Preserved separately from behavioral RED |
| Queued acknowledgement across completed real Replace | One behavioral failure | Real worker load/replace, idle pre-send check and actual socket packet still returned `Ok`; worker drained before assertion |
| Missing normal service scheduling | Two separately selected behavioral failures | Actual app remains denied; no clear dispatched or post-sync pause reached; normal cleanup precedes assertions |
| Current default host suite |53 pass,2 explicitly ignored | Strict formatting and default/all-target/all-feature Clippy pass |
| Current separately selected service cases |2 pass, plus clean restart subcase | Actual loop/storage/app boundary below |
| AArch64 daemon/library | Compile only | No new ARM execution or physical-device claim |

The first service case observes actual UID11010/GID11010, empty supplementary
groups and CapEff0. The same app connection receives `denied` before and after
arming, then the expected bounded history packet after the real false replacement
completes. Grants and retained receipts are unchanged.

The second case uses the existing `pause:1` hook after real file/directory sync
and before the worker response. While false bytes are present and the exact worker
is stopped, Snapshot responds, storage remains busy and the app receives
`unavailable`. A direct UID1000/GID1000 client with groups[] and CapEff0 receives
`unavailable` for its new lock. Resuming the retained pidfd within the original
write budget produces the required true replacement; ProcessView matches the
rediscovered worker against the original retained handle. Reads remain denied.
Normal Stop leaves a clean seal; a fresh actor loads the same policy directory
and denies a fresh app read despite authored unlocked/focused Source facts.

The two selected test processes have actual parent waits0, output EOF and proc
absence. Service and heartbeat threads are joined. Ordinary locker fixtures have
actual parent waits0. App/storage pidfd death and proc absence are observed after
service-owned drain; the numeric storage Child wait status is not independently
exposed and is not inferred. CLI wait0 and fresh same-daemon exact-CID absence
close the separate outer boundary. No fallback cleanup supplies a passing case.
The runtime has no network, a read-only root and only SETUID, SETGID, CHOWN, KILL
and DAC_OVERRIDE capabilities for the explicit isolated fixture.

## Remaining gates

The normal installed daemon has no successful authentication producer or new
success socket. Genuine whole-owner authentication composition, trusted broker
and locker handoff remain unqualified. Source expiry/forced-event/late-result
refusals have authority-model coverage; the full negative service matrix,
uncertain-clear Stop/restart, EAGAIN and complete-intervening-write timing through
the whole actor still need their own selections. The completed-write guard above
is a real Worker/socket component test, not a full actor or EAGAIN result.

Compose the qualified downstream path with real retained Messages history and
the normal installed app, then independently qualify genuine authentication.
The earlier queued ARM setup failure stays failed; no retry or relaxed predicate
is implied. Generic ARM execution of this selection, matched Google-kernel
composition, secure frame/configuration provisioning, suspend/wake and all
physical Pixel gates remain open. No phone was accessed or modified.
