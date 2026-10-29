# Trusted native Omarchy session ingress contract

The installed Messages path now uses a small adapter in the matching
open-source compositor and a native Rust bridge into the existing lifecycle
actor, with separately selected evidence linked below. Hyprland 0.56.2 exposes
actual lock state, but its public `activewindow`
query does not establish actual keyboard focus. The adapter exports compositor
facts; the existing actor retains the only permission policy and final data
publication gate. Java or an Android framework service is unnecessary.

This document records public original-source inspection and the session contract.
Its [static provenance](evidence/native-session-contract-provenance.json) qualifies
source inspection only. The [matching-build observation](native-session-build-boundary.md)
executes eleven original API/protocol/kernel groups. The current
[owned-focus protocol](native-session-owned-focus.md) has separate executed
identity, observer, actor and original-compositor selections. Historical
[native ingress prototypes](native-session-ingress.md) separately qualify17 prepared
compositor/observer groups,13 synthetic-source/current-authority ARM groups,
16 process/socket/exec cases and one typed policy regression. Production launch,
retained-kernel identity, trusted locker containment, secure presentation and
hardware remain separate gates. The later [installed app](native-session-installed-app.md)
and [current read split](native-session-receive-read-combined.md) qualify a joint
local-history path. [Display containment](native-display-containment.md) separately
qualifies the restricted listener and supervised SDK child. The
[service-epoch actor composition](native-session-display-composition.md) now owns
source stop/occupied and qualifies exact app-focus/independent-grant data through
that listener. Per-app context generations/relaunch and production privileged
runtime ownership remain open. The static inspection's
unexecuted case records describe its original checkpoint, not these later runs.

The existing [lifecycle authority](native-lifecycle.md) and
[installed Messages path](native-messages-service.md) qualify separate launched
principals and current read checks. The desktop fixture manually changes trusted
root lifecycle controls after reading the compositor's active-window PID. That
fixture does not qualify production lock/focus/session ingress.

The current Source wire also requires [owned mapped-locker state](native-session-owned-locker.md).
Its retained kernel handle is a fact about the mapped peer, distinct from
protocol completion, secure presentation and authentication. It introduces no
permission or trusted restoration path.

## Exact original selection

| Original | Selection |
| --- | --- |
| Omarchy | v4.0.4, commit `c668141e9c42b13c80c9ca4ea108e11708c5e8a5` |
| Omarchy archive | SHA256 `8cc6b1d9d903c606600395e3b3d80ad9f15bd3ab78f0f19b0f49041b1df82013` |
| Hyprland | v0.56.2 resolves to `efb50993780079460b0cbed1363e2166a2de1d9f` |
| Hyprland protocol submodule | `bd153e76f751f150a09328dbdeb5e4fab9d23622` |
| Earlier installed guest inventory | Hyprland `0.56.2-4`, Quickshell `0.3.2-1`, Qt6.12, Arch ARM kernel7.1.6 |

The package inventory was inherited from earlier renderer execution. The separate
build observation now pins the executed package ELF, all497 matching installed
headers, compiler and authored probe. It also identifies the unresolved exact
Arch ARM recipe and C++ patch set; the upstream tag is not assumed unpatched.
The production adapter and trusted launcher need their own artifact selection.
Public source trees stay in private ignored or temporary inspection storage;
only this authored report, URLs, pins, hashes and observations are publishable.

## Lock state, notification and original Omarchy

The [pinned HyprCtl implementation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/debug/HyprCtl.cpp#L1967)
registers the actual `locked` command. `j/locked` returns a JSON boolean from
`SessionLockManager::isSessionLocked`, which reads the protocol's `m_locked`.
[Current upstream documentation](https://wiki.hypr.land/Configuring/Advanced-and-Cool/Using-hyprctl/)
also documents the command and warns that queries execute synchronously in the
compositor. A Rust client can use the fixed read-only request directly, with
bounded nonblocking IO; executing a user-selected `hyprctl` is unnecessary.

The [pinned lock-notify XML](https://github.com/hyprwm/hyprland-protocols/blob/bd153e76f751f150a09328dbdeb5e4fab9d23622/protocols/hyprland-lock-notify-v1.xml)
and [notifier implementation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/protocols/LockNotify.cpp)
provide real Wayland lock notifications. A new notification object receives
`locked` if the notifier already considers the session locked; there is no
initial unlocked event. Silence must never grant authority. This is not a
socket2 lock event, and no such event is assumed.

Immediate denial and secure presentation are different boundaries.
[SessionLock.cpp](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/protocols/SessionLock.cpp#L185)
sets the protocol lock state during lock admission. Its ordinary resource
destruction does not clear that state. The
[manager](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/managers/SessionLockManager.cpp#L103)
sends completed-lock notification after output lock frames, or its five-second
fallback. A locker dying before that point can leave the protocol locked without
a completed notification. An observer restart needs current state; notification
history alone is insufficient. The fallback's existence is not an executed proof
that every active output has presented a secure frame.

Protocol unlock and force-unlock clear the compositor lock state. Locker death
retains denial. Restore, rejected-lock and inert-object paths must be covered by
the adapter without inferring unlock from process exit, `finished`, or missing
lock surfaces. Compositor death/restart invalidates the entire source incarnation.

Pinned Omarchy uses its
[Quickshell/PAM service](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/shell/plugins/lock/Service.qml),
not Hyprlock. The
[lock command](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/bin/omarchy-system-lock)
requests shell lock; the service marks its UI request, then waits for screen
stabilization before requesting ext-session-lock. User-owned QML state, PAM UI
strings and shell IPC status are not lifecycle authority. Its
[monitor-blocker helper](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/bin/omarchy-hyprland-session-locked)
infers lock from monitor reasons and can return undetermined. Its comment claiming
no direct lock query is obsolete for the selected compositor. Do not copy that
inference or treat undetermined as unlocked.

Separate [Hyprlock v0.9.6 source inspection](https://github.com/hyprwm/hyprlock/blob/b222d9b1f87e980cac379371df57913a53b99d7f/src/core/hyprlock.cpp#L760)
shows its own ext-session-lock acquire/release sequence. It is not installed-lock
execution or evidence for the selected Omarchy Quickshell implementation.

## Actual focus and kernel identity

`activewindowv2` carries an address. `activewindow` queries the remembered
`FocusState::window`, which can differ from actual keyboard surface focus.
[FocusState](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/desktop/state/FocusState.cpp#L223)
can clear keyboard focus without clearing that window, and
[interactive layers](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/desktop/view/LayerSurface.cpp#L185)
can take keyboard focus through a non-null surface without an active-window event.
A shell menu can therefore own keyboard input while the Messages PID remains in
`activewindow`. A query/event bridge using only those interfaces does not satisfy
this project's actual app-focus policy.

The minimal source-backed focus observation is the actual
[SeatManager keyboard focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/managers/SeatManager.cpp#L182)
and its `keyboardFocusChange` signal. The central setter handles null/non-null
focus and focused-surface destruction. Reading only the higher-level FocusState
input event misses other direct seat focus mutations. Export a principal only
when the focused surface belongs to an eligible native app window. Null focus,
shell/layer surfaces, lock surfaces, XWayland, unsupported delegation and missing
state deny. Define and test native popup ownership explicitly; do not infer it
from title, class, app ID or window address.

Native [window PID retrieval](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/desktop/view/Window.cpp#L469)
uses `wl_client_get_credentials`; XWayland follows a separate stored-PID path.
The adapter must obtain PID/UID/GID from the actual focused native `wl_client`.
[Wayland's primary API documentation](https://wayland.freedesktop.org/docs/html/apc.html)
specifies connection-time kernel `SO_PEERCRED`, with PID reuse and
compositor-created socketpair caveats. This is not per-request authentication;
Wayland FD inheritance/delegation remains a separate qualification gate. The
Rust actor must still match the exact installed app generation and live pidfd.
The executed build probe additionally obtains `SO_PEERPIDFD` from the actual
Wayland socket, requires live pidfs identity, and matches its device/inode against
an independently opened current task pidfd. The current [owned-focus protocol](native-session-owned-focus.md) transfers that
actual socket peer handle to compare with the already owned app pidfd through a
pinned protected procfs view and joint final liveness; scalar identity is unused.
Source incarnation, semantic revisions and original boot-time leases remain bound. Opening a pidfd from an observed numeric PID can alias a recycled PID;
unsupported socket identity must deny. Delegation and numeric PID reuse have
not been qualified by that probe.

## Trusted locker and protocol containment

Distinct app UIDs and a Wayland socket ACL do not restrict privileged Wayland
protocols. The pinned compositor's
[global filter](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/Compositor.cpp#L262)
exposes globals to clients not classified sandboxed; its
[protocol allowlist](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/managers/ProtocolManager.cpp#L348)
restricts privileged globals for security-context clients. The current Messages
fixture's direct socket ACL does not establish that classification.

[Omarchy's configuration](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/default/hypr/looknfeel.lua#L113)
enables session-lock restoration. The pinned manager's restore condition does
not visibly require that the old locker died. This suggests that an unrestricted
Wayland client could take lock ownership and unlock its replacement. This is a
static inference requiring a real protocol test, not an executed vulnerability
finding. Checking compositor lock state does not itself prove trusted unlocking.

Trusted presentation and restoration must select the exact trusted locker
kernel identity and launched generation. Neither caller app metadata, a PAM UI
string nor general UID1000 membership is sufficient. Restrict privileged protocol
access for apps, and constrain restoration to the trusted owned locker and the
qualified old-owner state. Qualify the selected Quickshell client/delegation
contract. Do not authorize a replacement merely because the old lock process
seems absent. Lock-client failure keeps data denied.

## Minimal adapter and Rust ingress

Use a small matching-source adapter or patch in Hyprland, plus a Rust bridge into
the existing actor. Pin any necessary C++ ABI glue to the exact compositor build;
this is not an abstract service framework or a compatibility layer.

The following are proposed fields and behavior, not existing Hyprland APIs:

1. Export one bounded snapshot containing source incarnation/revision, immediate
   lock-admission state, completed-lock state separately, and eligible focused
   native credentials or no principal. Obtain the snapshot in the compositor's
   event loop. Observe central seat changes and all lock/restore/force/teardown
   paths; invalidate when their ordering or identity cannot be established.
2. Use a private root-provisioned channel with monotonically increasing sequence,
   bounded packets/pending output and bounded reconciliation. Overflow, malformed
   packets, missing sequence, disconnect, revision exhaustion or stale completion
   fences the source. Never interpret an event-stream absence as positive state.
3. Bind the source to an exact root-trusted launched compositor and owned pidfd.
   For genuine compositor-created Unix listeners, validate `SO_PEERCRED` against
   that expected identity on every connection. Path ownership, environment
   instance signature, executable name and UID1000 alone are insufficient.
4. A root-created inherited socketpair has creation-time peer credentials. If
   used for adapter output, enable kernel `SCM_CREDENTIALS` and verify each packet
   against the expected compositor PID/UID and generation. Close parent copies,
   set close-on-exec, and transfer exactly one owned focus pidfd when focused.
   Refuse unexpected descriptors and truncation. FD possession alone is
   insufficient. General app/policy IPC still refuses all descriptors; the
   dedicated source receiver now has separately executed owned-focus evidence.
5. The trusted launch profile must fix executable, security configuration and
   approved adapter/plugin identities, and prevent untrusted modification of the
   compositor. Process credentials prove identity, not code/config integrity.
   Appearance customization must not alter this security boundary.
6. Keep IO nonblocking or in one supervised owned Rust helper. Use absolute
   BOOTTIME deadlines across admission, send, receive, completion and publication.
   Limit query/event work per actor turn. A stopped compositor/helper or a flooded
   source must leave snapshot/stop responsive and data denied. Unconfirmed child
   cleanup retains ownership under the existing bounded cleanup contract.

The original
[watchdog FD](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/Compositor.cpp#L127)
provides immediate manager lock/unlock lines through a launched descriptor. It
lacks focus, snapshot, revision and a complete bounded-output contract. It is
supporting integration evidence, not a sufficient authority protocol.

## Actor ordering, session and publication limits

Startup and every unknown/stale/dead source state remain locked and
background-denied. Source messages provide facts only: they cannot grant
`MessagesRead`, launch arbitrary apps, select executable/UID/PID, or expose the
root admin operations to desktop apps. Match observed native focus to the actor's
current launched app generation, background all other principals and apply that
focus change in one actor transition. A new Messages generation still starts
denied and requires fresh focus plus separately owned policy.

The [native lock entry](native-lock-admission.md) now fences lifecycle authority
in that same actor before requesting/starting the original Omarchy locker. The
fixed session UID1000 route accepts only `{"op":"lock"}`, checks each packet's
kernel `SCM_CREDENTIALS` and supplies no parameters, grants, launch or unlock.
The fixed Rust client requires a reply from the connected root actor's exact
PID/UID/GID within its original two-second BOOTTIME budget, then executes the
exact retained original action. Failed or delayed admission refuses that action;
failed or delayed locker startup retains denial. This denial-only route does not
establish trusted locker identity, PAM success or secure presentation.

The actor revokes current memory authority before storage admission. A successful
ACK requires the owned storage child to confirm file sync, replacement and directory
sync of required-schema `Durable.lock_pending`. This monotonic `Gate::Lock` supplies
no effect stamp and tolerates irrelevant focus revisions during sync. Restore loads
the pending intent before Ready; fresh compositor `locked:false`, root admin false,
UI death and display/Source replacement cannot clear it. Repeated pending locks
use the already confirmed durable state. The healthy same-directory actor replacement
and changed original display/Source epoch now retain fresh normal Qt-page denial.
Whole-actor SIGKILL separately refuses uncertain unclean recovery. Clearing pending
intent requires a qualified trusted unlock/reconciliation path, which is absent;
a fresh unlocked session requires an explicit root-owned fresh storage lifecycle.
No old-schema compatibility or implicit policy deletion/reset is provided.

Bind a current read to source incarnation/revision/freshness in addition to the
existing principal, policy, lifecycle, worker, connection and BOOTTIME bounds.
Before each nonblocking publication attempt, including every EAGAIN retry,
consume available source transitions/failures and recheck authority. A late
query or worker reply cannot refresh its own source lease or read stamp.

The current [timing-recovery contract](native-session-timing-recovery.md) withdraws
authority on observed100ms sample expiry or producer gaps before exporting any
replacement. The same owned Source must establish a fresh advancing pair after
actual loss discovery, with independent actor validation. Every semantic revision
is processed before coalescing; malformed identity/wire/protocol history, backwards
or future timestamps, sequence failures and Source death remain terminal. Existing
work retains its captured deadline and cannot revive. The earlier
[bounded observer correction](native-session-freshness-chain.md) preserves its
34 actual process cases per host/ARM kernel as historical evidence, separate from
the current recovery work and actor-held-read evidence.

An asynchronous event/query bridge still has a whole-publication race: the
compositor can change after a successful query or event drain but before the
actor sends to the app. A source revision is not an atomic cross-process send
barrier. The honest guarantee is fencing after ordered authority ingress, with
bounded source freshness. Do not claim zero publication after an unobserved
external transition. Stronger ordering for external ext-session-lock clients
requires a source-backed pre-transition barrier or a policy restricting all
lock entry through the trusted coordinator, including qualified failure behavior.
Focus transitions need their own stated ordering guarantee. Bytes already
successfully sent or queued cannot be withdrawn, and a later Qt callback is not
a new service authorization check.

Session activity can provide an additional deny condition from the authenticated
system logind owner and the exact installed session mapping. `LockedHint` is
caller-set hint state, not compositor lock proof. Current
[systemd's primary contract](https://github.com/systemd/systemd/blob/main/man/org.freedesktop.login1.xml)
documents `Active`, `LockedHint` and sleep preparation. Bus owner changes, session
removal/inactivity and unknown state fence the source. Resume requires fresh
compositor/session reconciliation; a sleep-ended signal grants neither unlock
nor focus. This design does not qualify suspend authority or actual device sleep.

## Required original-protocol red/green plan

This table preserves the initial plan. Later owned-focus, installed-app and
freshness reports qualify their separate source boundaries; the
[native lock report](native-lock-admission.md) now qualifies denial before the
original locker and healthy same-directory persistence. It does not qualify
trusted restore, PAM, secure frames or an atomic external-transition barrier.
For remaining cases, preserve the failing implementation and exact binaries before
qualifying the boundary through installed processes and selected original protocols.
Synthetic actor injection cannot substitute for original source execution.

| Planned case | Failure to expose / required observation |
| --- | --- |
| Interactive shell layer | Actual keyboard focus leaves Messages while public active-window PID remains Messages; native reads deny |
| Null focus, surface destruction and native popup | Central seat changes fence old focus; popup ownership follows an explicit tested rule |
| Lock before first frame | Delay lock surfaces and kill the locker before notification; immediate denial persists |
| Initial/restarted observer | Unlocked, locked, early-lock and stranded states never infer permission from missing notifications |
| Trusted lock intent | Hold/fail locker launch; actor acknowledgement fences pending reads before launch |
| Restore and external lock client | Attempt ownership replacement with actual Wayland requests; app cannot acquire trusted unlock authority |
| Actual render state | Record lock admission and completed presentation separately, including no-output/fallback paths |
| Held read and real EAGAIN | Lock/unlock/refocus/revocation fence the old pending page at every retry |
| Scheduling race | Force a source change between observation and publication; demonstrate the stated ingress guarantee and unresolved stronger boundary |
| Source transport identity | Foreign UID1000 server, socket replacement, wrong launched PID, transferred FD and old incarnation cannot restore authority |
| Source stop/death/flood | Actual SIGSTOP/death, overflow, malformed output, sequence gaps and deadline expiry deny while snapshot/stop respond |
| App generation and unsupported paths | Old app death/replacement, XWayland metadata and unsupported FD/process delegation deny |
| Session change/restart | Session inactivity/removal, bus-owner replacement, compositor restart and resume require fresh exact source binding |

Later ARM64 installed replay must record the real guest kernel and matching
compositor/adapter ELF identities. None of these steps authorizes phone access,
modem operations or a stronger physical secure-lock/publication claim.

## Current per-app context ownership

The [per-app actor checkpoint](native-display-app-generations.md) gives each
reserved launch its own trusted SDK context, listener path and original one-second
setup deadline within the two-second RPC. Exact pending-client EOF is checked
before app spawn; app/context retirement revokes data authority and withdraws the
path before SDK signals, retaining the occupied slot until actual parent waits.
It leaves the plain Source Observer and Messages writer independent. Four ARM
lifetime groups and one held-init Stop group pass. Pending EOF passes narrowly,
but the successor aggregate fails after Source InvalidData of unknown predicate.
The Source100ms lease is unchanged. Metadata grants no data/exclusive PID authority;
accepted clients, secure frames, installed PAM restoration and wake remain gates.

The [current readonly authority selection](../tools/native-current-readonly-authority/README.md)
records15 host groups against its frozen actor/Observer code. Genuine held-reader
tests signal the retained readonly pidfd and observe T-state; the independent
writer is not the held-read boundary. Source expiry/EOF/death retire the actual
app and refuse a successor while Snapshot/Stop respond. This uses synthetic
credentialed session facts, not the original compositor. Archived test failures
and the v1 fixture error are retained separately; see the
[independently verified proof](evidence/native-current-readonly-authority-provenance.json).
