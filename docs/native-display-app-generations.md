# Owned display contexts for each app generation

The native Rust lifecycle actor now reserves one display context for each app
launch. It waits for the owned SDK child to become Ready before creating the app,
and retains the app and SDK until their actual parent waits complete. The
protected listener pathname belongs to that fresh generation. App death,
pending-client cancellation, Source failure and Stop revoke data authority and
withdraw the pathname before signalling the SDK.

This is a partial engineering checkpoint. Four actual ARM lifetime groups and
one held-initialization Stop group pass. Pending-client EOF cancellation passes
its narrow boundary, but that epoch's subsequent cold successor fails after
Source InvalidData. Its exact rejection predicate remains unknown; the full
matrix is failed. These results do not finish installed smartphone deployment.

## Actor ownership

`native-lifecycle::display::Factory` supplies a trusted Context to the actor's
reserved launch slot. Fallible setup before helper creation may return an error;
post-spawn failure must return its owned terminal Context. Context polling must
be bounded and nonblocking. Root's isolated provisioner uses the existing owned
SDK Supervisor and an independent plain Source Observer. Context retirement
does not reset that Observer or the Messages history writer.

Setup has one original BOOTTIME second within the existing two-second RPC.
The exact pending connection checks actual queued EOF/additional requests before
it alone can trigger app creation. General app polling never spawns. After spawn,
the actual Child is retained before pidfd registration and another context and
deadline check. Every successful launch reply is checked against its current
slot before publication; a retired instance cannot publish a stale success.
Headless callers use the same actor path without a display Factory.

`native-display::listener::Listener` creates a distinct Root-owned directory
and socket under protected Root-owned ancestors. Initial mode0700 protects ACL
setup; final directory0710 and socket0660 admit the installed app group. It pins
parent, directory and socket identities, refuses collisions/replacements, and
withdraws only its own pathname. The isolated provisioner replaces only the
normal Qt profile's WAYLAND_DISPLAY argument; its other five arguments stay
unchanged. Instance metadata supplies no data permission or exclusive PID
admission. Accepted clients are not promised disconnected by pathname withdrawal.

Retirement retains the occupied app slot until both actual Child and Context
waits finish. No successor, detached worker, replay or Source revival follows
from a dead pidfd. Final actor cleanup shares its original two-second budget
across app, SDK, policy and Messages owners. An unreaped slot cannot be silently
dropped. Listener identity/withdrawal failure is an error and retains ownership;
ordinary retirement does not delete replacement files.

## Chronological actual boundaries

Three original-compositor/normal-Qt ARM REDs ran against the unchanged historical
service1a4d before their respective implementation/refinement. The old listener
still admitted a separate same-UID registry after exact app death; held SDK
initialization still allowed an app/principal; client EOF before SDK Ready still
created a principal. These are new per-app requirements, consistent with the
older service-epoch contract. The earlier tracefs setup failure remains unselected.

The corrected build selects Source610aa and fixture ELFbb1032, while normal Qt
remains4aa50f. It uses the original Hyprland and adapter in an actual Arch ARM
Linux7.1.6 guest kernel, read-only backing/snapshot, no NIC, four guest CPUs within
six CPU/eight GiB caps. Offline builds use eight CPUs/twelve GiB and a mutable RAM
cache. No physical phone or original radio service is invoked.

| Selected actual epoch | Result |
| --- | --- |
| Lifetime | Four groups pass,26 tracked cleanup outcomes. Qt383/SDK382 retire after graceful close; a different same-UID process gets ENOENT on the old listener. Explicit Qt401/SDK400 use a fresh path/instance while Source337 and writer379 stay live. Separate app-read/admin/raw denial preserves2raw/2inbox Root0600 rows. |
| Held initialization and Stop | One group passes,17 tracked cleanup outcomes. SDK394 remains Initializing, zero apps/principals/replies, four actual Source focus revisions. Snapshot6.539ms, Stop12.764ms; service exit observed43.170ms after Stop start. |
| Cancellation and successor | Pending EOF boundary passes: zero app/principal/reply, SDK391 actually retires, path disappears, Source/writer stay healthy. Snapshot8.225ms. Explicit successor SDK392 becomes Ready, then Source InvalidData at42.883624843s fences it; later empty app command line causes a secondary driver error. Full epoch fails;18 tracked cleanup outcomes complete. |

Timing observations are not hard realtime bounds. Service-owned reap is qualified
from trusted compiled `Child::try_wait` ownership, occupied=false and recorded
kernel absence; no independent parent wait syscall trace is available. Python
directly waits its own children. SuccessorSDK392's native terminal occupied=false
event is separate from that failed epoch's18 tracked cleanup rows.

Root independently verified2,740 nested bindings covering1,206 retained paths,
current runtime inputs and the three RED/fresh build selections. Eight host
lifecycle component tests pass under their required isolated Root runner; the
display library has zero unit tests. The initial wrong-UID component invocation
is an unselected harness failure. Three normal lifetime PNGs were independently
viewed: unknown-source denial and two granted local conversation lists showing
+15550100 and modem unavailability. Literal body visibility is unqualified.

The [independent provenance](evidence/native-display-app-generations-provenance.json)
and [authored tools](../tools/native-display-app-generations/green-notes.md)
preserve exact inputs/results. Current docs are not substituted into archived
builds. The [failed-successor diagnosis](../tools/native-display-app-generations/green-source-failure-diagnosis.md)
records why no particular Source predicate follows from the old traces: there
are no mirrored frames, first rejected packet or last accepted heartbeat.

## Current shared headless regressions

The same new app-slot algorithm also runs without a display Factory. A separate
exact-code host Linux6.8.0 process/IPC selection passes18 installed lifecycle,
seven durable-storage and two cleanup groups. These27 distinct groups include
real app death/principal removal and fresh launch, stale permits across Stop and
restart, delegated socket/SCM denial, held storage Snapshot/Stop and uncertainty.
Root independently verifies93 private bindings, unchanged test scripts and exact
current core/binaries in the [headless regression proof](evidence/native-display-app-headless-regression.json).
They are separate from the eight host component tests and ARM display groups.

The old standalone test initially sends Grant after admin.sock exists while
storage is still loading; it fails before executing any lifecycle group. That
prerequisite failure is preserved. The unmodified storage-aware runner already
waits for snapshot.ready and then runs the same18 lifecycle assertions. No test,
deadline or production startup contract was relaxed. Its service-owned app wait
is inferred from current Child::try_wait plus principal removal/slot reuse; the
storage cases record real pidfd death/proc absence, and cleanup subreaper cases
actually wait adopted children after required owner abort. No unkillable syscall,
original compositor, Source-terminal or held Messages read qualification follows
from these27 headless groups.

## Separately pinned failure diagnosis

One new diagnostic-only ARM cancellation/successor epoch changes only a frozen
copy of Source610aa. The approved patch preserves all original guards and adds
fault-only nonblocking authenticated trace records. Root independently checks
its zero-fuzz application,77 unchanged candidate files and terminal trace closure.
Consecutive publisher sequences101/102 contain CLOCK_BOOTTIME samples
43.920184896s and44.023393645s:103.208749ms apart,3.208749ms beyond the unchanged
100ms chain limit. The original guard rejects this gap before the later own-age
or descriptor-type check. Two actual service386/Root-authenticated records include
the terminal summary, with zero dropped or malformed callbacks.

That new aggregate also fails. Its specific chain-gap predicate is qualified;
the earlier production aggregate remains failed with unknown predicate. Added
bookkeeping can perturb timing, and cold successor correlation does not prove
kernel scheduler/compositor causality. The production Source stays byte-identical.
See the [diagnostic result](../tools/native-display-app-generations/diagnostic-result.md)
and [separate evidence](../tools/native-display-app-generations/diagnostic-evidence.json).

## Remaining gates

Resolve broad cold-successor availability under the unchanged100ms Source lease;
qualify incoming Binder together with per-app restricted display; complete
installed trusted source/listener/executable provisioning, privilege restrictions,
PAM lock admission/restoration, secure presentation, delegated clients, background
notifications and suspend/wake. Retained matched Pixel6.6 kernel/config/KMI,
Google modules/firmware and physical hardware remain separate gates. Local
reaping is never vendor cleanup. The full smartphone goal remains active.
