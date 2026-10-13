# Development handoff

## Goal and constraints

Finish the full Omarchy smartphone system described in `smartphone-goal.md`.
The earlier call-bridge goal was expanded to native Rust phone userspace: calls,
cellular/IMS, SMS, camera, power, audio, connectivity, sensors and background/lock
behavior, with secure local controls, packaging and Omarchy theme integration.
Pixel 10 is the first target; common services must support separate hardware
backends. Retain matched Google kernel/modules/firmware initially. Android/Java
components are reference and test infrastructure, not the intended final runtime.
Use open source first and REA for closed interfaces. No backwards compatibility.

Build and test everything possible offline. Do not access or modify a physical
phone. Keep synthetic emulator, original-code, ARM64/QEMU and physical evidence
separate. Do not equate registration, submission or API success with carrier
acceptance, delivery, native call state or working physical hardware.

## Current stopping point

Per-app display ownership is now implemented in the current actor. Three actual
ARM chronology REDs precede fresh context/listener generations, asynchronous
Ready-before-app launch and exact-client EOF cancellation. Four lifetime groups
and one held-initialization Stop group pass with unchanged normal Qt, Source610aa,
original Hyprland and the same independent history writer. Pending EOF passes
its boundary; its subsequent cold-successor aggregate fails after Source
InvalidData. The rejecting predicate is not retained in that old epoch. Preserve
the failure and investigate using separately pinned diagnostics; do not retry,
relax the100ms lease or classify it as hardware-only. Root independently verifies
2,740 nested bindings/1,206 paths and eight correctly run host lifecycle tests.
A separate exact-code host selection passes27 distinct headless process groups:
18 installed lifecycle, seven storage and two cleanup. Root verifies93 bindings;
the unsupported admin.sock-only startup attempt remains preserved. See
[evidence](evidence/native-display-app-headless-regression.json).
Another current-code host selection qualifies15 readonly/Source authority groups:
14 v1 passes plus one narrow v2 coalesced-transition pass. Actual retained readonly
pidfd-directed SIGSTOP holds the reader; expiry/EOF/death retire the app while
Snapshot/Stop remain available. Root independently verifies443 bindings. The
archived11pass/4fail suite and v1 fixture error remain failed. See the
[current wrapper](../tools/native-current-readonly-authority/README.md) and
[proof](evidence/native-current-readonly-authority-provenance.json).
See [per-app display](native-display-app-generations.md) and its provenance.
A separately pinned diagnostic-only epoch identifies its own103.208749ms
publisher sample-chain gap, exceeding the unchanged100ms limit by3.208749ms.
Authenticated trace closure has zero drops; this does not retroactively diagnose
the old failure or establish scheduling causality. Production Source is unchanged.

Future UI runs/previews must use the [Pixel 10 display profile](native-phone-display-profile.md),
1080 ×2424 at422PPI with matched logical density420. A fresh actual ARM measurement
at the correct resolution exposes incorrect virtual physical dimensions; its
negative evidence is preserved. The simulation-only physical-size adapter and
native-buffer capture remain unqualified until their actual positive selection.
Do not resume historical320/800 preview geometry or resize an image to qualify.

The native raw-response publication gate, absolute suspend-aware session expiry,
owned registration worker/death watch, and local process reclamation are implemented.
They are not native IMS semantics or a complete phone service.

- `RadioOwner` fences session/revision/preparation identity and checks Linux
  `CLOCK_BOOTTIME` expiry under the gate. Withheld timer delivery cannot permit
  late activation or publication. Assembly candidate and durable operation policies
  remain open; the bounded single-request checkpoint is described below.
- `TransportSession` owns one registration worker and retained death watch. Stop
  and snapshots do not join or unlink. Cancellation/expiry retain occupied worker
  capacity; completion is independent of the failed callback stream. Late success
  cannot activate, and every second attempt is refused.
- `Supervisor` revokes process output before SIGKILL and retains ownership until
  reaping. Its exclusive, synced private admission marker persists uncertainty
  before launch and survives supervisor restart. It never removes that marker or
  treats reaping as remote cleanup. No successor registration is permitted.
- Real Binder passes held cancellation/expiry, death before/after activation,
  permanent blocked-child termination/reaping, stale-tag refusal and durable
  restart refusal. Component red/green tests include worker detachment, death-result
  ordering, output revocation and stalled reaping. Existing IPC and six Binder
  dispatch-wrapper tests also pass on Harbour with SELinux enforcing.

Evidence: `evidence/radio-deadline-provenance.json`,
`evidence/radio-worker-provenance.json`, `evidence/radio-supervisor-provenance.json`.
Java changes are synthetic peer controls only; no physical phone was accessed.
Harbour now has authenticated `gh` write access and the repository-local existing
author identity. Authentication remains outside source control.

ARM64 user-mode QEMU passes component state, journal and provenance tests, but
its failed-exec subprocess test unexpectedly returns a successful child for a
missing executable. The two process execution tests are now compiled for ARM64
and explicitly ignored by the user-mode suite. `tools/native-process-vm.py` runs
them under an Arch Linux ARM guest kernel in system-QEMU, as the unprivileged
`alarm` user. Both pass, along with the component tests. See
`evidence/arm64-radio-process-provenance.json`. This closes the process-specific
gate; matched Pixel kernel/policy and physical hardware remain open.

Native radio Binder now has nine actual Rust/manual scenarios under the unchanged
Arch ARM7.1.6 kernel and three separate genuine Android Binder/Bionic NDK interop
scenarios. Actual transferred objects/callbacks/death, withheld-timer cancellation
and expiry, request uncertainty and permanently blocked child reclamation are
qualified. Original runtime libraries are separate from NDK link stubs and the
public AOSP synthetic startup/property fixtures. No original vendor service ran;
matched Pixel policy, vendor cleanup/reset, installed ownership and modem/carrier
remain open. See `native-arm64-binder.md` and its separate evidence classes.

Native messaging now has a device-neutral SMS-SUBMIT encoder and bounded multipart
planner. Twenty-seven byte comparisons match actual AOSP encoder execution in the
owned SDK37 emulator, including forced UTF-16 ASCII fragments. The comparison
found and fixed an upstream form-feed mapping omission. Five planner tests and
x86/ARM64 user-QEMU builds pass. The durable outbox now reserves references/payloads atomically, commits uncertain
state before dispatch permission, fences stale owners and preserves cancellation
after late acceptance. Twelve actual SQLite tests and x86/ARM64-kernel SIGKILL
restart tests pass. This does not implement send/receive, delivery, permissions,
a messaging service or UI. See `native-messaging.md` and
`evidence/sms-submit-provenance.json` and `evidence/sms-outbox-provenance.json`.

Incoming SMS text decoding now matches 33 actual AOSP parser outputs for text,
sender, timestamp, reply-path and8/16-bit concatenation. Five bounds/malformed
tests and x86/ARM64 user-QEMU builds pass. Parts are parsed only; durable incoming
deduplication/assembly, vendor acknowledgement, delivery and UI remain open.
See `evidence/sms-receive-provenance.json`.

The frozen standard SMS messaging V3 contract is now recovered. An isolated
original Shannon APK oracle passes50 codec/tracker outcomes and records8 message
IDs without radio dispatch or service startup. Signed response lengths, token
reuse, message-reference ambiguity and removal before failed acknowledgement
are explicit observations. Last-incoming GSM acknowledgement has no durable
receive identity; replay after reconnect remains forbidden without qualification.
See `native-sms-transport.md` and its provenance. The frozen Binder submission
and receipt owners now qualify these local fences as described below; production
modem identity, reset and acknowledgement policy remain open.

Focused REA/public-source investigations now recover camera AIDL4, audio core4,
Power7 and Sensors3. Selected audio/power/sensor frozen hashes match shipped
libraries; camera hash equality remains open. Camera failed-return/count and
flush semantics, AoC call audio, two sensor wake obligations and potentially
wedged haptic completion remain explicit boundaries. Twenty-five public power
source checks and AIDL compiler generation passed, separately from runtime.
See `native-device-contracts.md` and its provenance; raw factory/REA records stay
private. The camera error-reply/FD fixture now passes through real Binder using original
generated NDK proxy/dispatcher and an independent Rust client. Both pass
64-cycle FD budgets, transfer/reverse-transfer lifetime, error-count omission,
DEAD_OBJECT and death notification. See `native-camera-boundary.md`; pipe FDs
do not prove gralloc/sync_file/DMA or capture. Native power arbitration now passes18 tests on x86/ARM64 user-QEMU and18
outcomes against actual pinned public RequestGroup/Node/FileNode execution.
Owned leases, cancellation, boot-time expiry, uncertain application, retry and
stale completion fencing are implemented. Authentication, actual backend IO
ownership and a deployed suspend authority remain open; see `native-power.md`.
Native audio now passes five policy tests on x86/ARM64 user-QEMU and16 actual
PipeWire/WirePlumber stages plus an authenticated stop run. One owned backend
process isolates original-stack IO from responsive status/stop. Nonblocking
worker pipes, boot-time deadlines and intent tokens fence late replies; expired
work is killed, retained until reaped and requires explicit recovery. `ok:true`
admits intent; observed application and pending cleanup remain separate. Actual
stalls measured status2.3ms/stop0.64ms, without claiming hard latency. Capture
revocation, capacity reservation, parent death and graph-cookie restoration pass
through real daemons. This qualifies a virtual mono graph; installed service
management, PCM brokerage, durable call recovery, physical mute and Pixel AoC/DSP
routing remain open; see `native-audio.md` and its process provenance.

Native camera ownership now passes nine checks with exact frozen cameraV4
requests/results and original generated NDK callbacks over real Binder. Buffer
return, capture completion and release readiness are distinct. Cached reuse
preserves older capture accounting; duplicate/resolved receipts cannot reclassify
acceptance. Held-call revocation/configuration fences stay responsive, and peer
death retains quarantine; EOF is not a release signal. Three actual runtime reds
preceded fixes. Production publication/storage, durable quarantine/reset,
multi-stream/FMQ, sync_file/gralloc/DMA and physical capture remain open. See
`native-camera-owner.md` and its evidence.

The read-only native connectivity adapter now pins the original NetworkManager
owner and bus identity, targets unique-name RPCs, and checks absolute boot-time
expiry before publication. Eight real-daemon/private-D-Bus stages pass with x86
and ARM64 user-QEMU Rust clients; the daemon stays x86 in both runs. No host NIC
or bus is accessed. Installed authority, authenticated permissions, mutations,
BlueZ and device behavior remain open; see `native-connectivity.md`.

The native suspend coordinator now has one cooperating writer, owned wake
objects, atomic responsive stop/snapshots, abort-before-counter ordering and
terminal uncertainty. Absolute CLOCK_BOOTTIME budgets fence late successful FD
operations without resetting after EINTR. Twenty-seven checks pass on x86 and
under an actual ARM64 Arch kernel, including three process/Unix lifetime cases;
24 component checks pass under ARM64 user-QEMU. Eight actual unmodified Android17
source cases qualify the reference ordering/lifetime separately. Pixel kernel
count/abort controls and driver-blocked process supervision remain open; see
`native-suspend.md`.

The retained frozen V1 suspend Binder boundary now passes18 actual x86 outcomes:
original generated NDK/Rust interoperability in both directions, explicit and
final-reference release, strong-object transfer across client exit, second-client
death, and a retained authority death recipient permanently fencing the native
coordinator. Both unchanged original libpower cache branches execute actual RPCs
through a link-time discovery adapter. A held-name success after service death
does not prove a current wake blocker. SELinux stays enforcing; default is occupied
and custom registration is denied. Installed standard discovery/permissions,
ARM64 Binder and actual suspend remain open; see `native-suspend-boundary.md`.

Native SMS submission ownership now drives the durable outbox through exact
frozen messaging V3 original NDK/Rust Binder. Ten live groups and seven additional
SQLite owner tests pass. One-way return/intermediate ACK cannot accept a part;
only a matching final qualified result can. Malformed acknowledgement hex,
wrong endpoint/method/serial/generation, duplicates, cancellation and death
cannot consume or revive work. Both live defects went red before their fixes.
This remains a synthetic SOLICITED/NONE/errorCode-1 qualification, with modem
mapping, receive identity/ACK replay, delivery and installed integration open.
See `native-sms-transport.md` and `evidence/native-sms-binder-provenance.json`.

Native sensor ownership now passes11 actual x86 SensorsV3 Binder/original FMQ
and EventFlag groups. The fixed96-byte Event ABI is probed by original NDK code;
Rust copies opaque aligned bytes and validates them before normalization.
Authenticated subscriptions, reserve-before-read, processing-before-ACK, exact
split counts, full-queue retention, revocation, stale FD generations, malformed
Event ownership,32 FD cycles and peer-death quarantine pass after two live reds.
This11-group baseline uses a synthetic source wake counter. The subsequent
10-group checkpoint below now acquires actual native coordinator reader leases
and shares frozen V1 blockers. USF200ms, top-level HAL1s cleanup, physical suspend,
app publication/lifecycle, durable reconciliation, calibration and ARM64/device
proof remain open; see `native-sensors-boundary.md` and `native-sensor-wake.md`.

Native GSM status-report parsing now passes57 field comparisons against actual
original AOSP execution plus12 focused tests on x86 and ARM64 user-QEMU. Both
receipt/discharge timestamps, numeric addresses, status, optional PID/DCS and
qualified text are bounded. AOSP parsed41 deliberately rejected native cases;
that divergence remains explicit. TPDU/full-PDU limits are164/177 bytes. Parsing
provides no unique outbox delivery key or modem acknowledgement authority.
Incoming receipt ownership now persists distinct raw observations before any test
ACK permit and commits uncertainty before dispatch. Cancellation revokes the source
before I/O; late callbacks stay quarantined and restart never reconstructs windows.
Nineteen actual SQLite tests pass on x86 and an actual ARM64 kernel, including
SIGKILL/reap/restart, and twelve frozen V3 NDK/Rust Binder groups pass. The ACK
adapter remains explicitly test-only: producer identity/reset, modem success/cause
policy, receive-worker/storage supervision and installed apps remain open. See
`native-sms-status-report.md` and `native-sms-receipt-owner.md`.

Native radio request ownership now shares the session gate and exact registered
transport. Six real x86 Binder scenarios and five new component checks qualify
absolute per-part admission, NotSent/OutcomeUnknown, cancellation/stop/death,
complete4096-byte segmentation and a held late raw callback that cannot accept the
operation. The expiry defect failed at actual peer count1 before the fix. x86,
ARM64 user-QEMU and actual ARM64-kernel component/process gates pass. One request
attempt is intentionally retained; durable admission, final semantic receipts,
subsequent operations, driver pressure and installed native IMS remain open. See
`native-radio-requests.md` and `evidence/radio-request-provenance.json`.

Native permission/lifecycle authority now passes18 installed Linux groups plus4
real pidfd/durable-file checks on x86 and under an actual ARM64 Arch kernel.
The owned launcher assigns distinct app UIDs; each packet checks real credentials
and exact launched PID, rejecting inherited-socket delegation and kernel-forged
identity attempts. Policy/lifecycle/principal generations, boot-time expiry,
revocation, child death/reaping and durable uncertainty are qualified after four
executed reds. Receipt history is not later effect permission. Omarchy ingress,
production app/package/service-manager integration, full storage/kernel closure and
atomic backend dispatch remain open; see `native-lifecycle.md` and its provenance.

Native sensor wake ownership now connects consumed events to actual coordinator
wake objects. Ten original-FMQ/frozen-V1 Binder groups pass after two live reds:
acquire before read, retain through processing/full ACK queue, share retained V1
blockers and release only after qualified queue+signal handoff. A real guest-thread
futex EPERM can coexist with peer-visible ACK count; it retains quarantine and
cannot retry. Actual blocked Unix-socket acquire keeps snapshot/revoke responsive.
Source death/last-owner drop fence authority before lease destruction. Injected
power descriptors do not prove hardware suspend or source wake acquisition;
USF200ms/HAL1s, installed lifecycle/app delivery, durable recovery and ARM64/device
integration remain open; see `native-sensor-wake.md` and its provenance.

Native IMS decoding now has an independent allocation-free bounded reader. REA
method inspection and247 original pure APK reader calls qualify148 valid values
and remaining counts;96 bounded refusals and3 oversized original inputs are
explicit native policy differences. CP "String" getters are byte-array diagnostics,
so the native API returns opaque bytes and never invents text. Ten focused tests
plus all prior codec tests pass on x86 and ARM64 user-QEMU after executed runtime
reds. Whole response layouts/producer identity, semantic publication and native
IMS/call/bearer/media integration remain open; see `native-ims-reader.md`.

The first whole registration-indication payload decoder now matches populated
fields from 37 complete ranges among 121 pure original APK probes. It explicitly
refuses 84 malformed/over-limit ranges that the original returns from normally.
Missing fields become None; opaque body bytes retain NUL/non-UTF8, without old
object values, UNKNOWN text or lossy conversion. Eight runtime reds preceded
implementation; all 24 native checks pass per x86/ARM64 user-QEMU target. Producer
identity, semantic publication and installed IMS remain open. See
`native-ims-registration.md` and its separate provenance.

Native inbox projection now qualifies private durable conversation storage with
20 actual SQLite/access/process tests on x86 and an ARM64 guest kernel and33
recorded original AOSP fixture comparisons. Exact local occurrence IDs import
idempotently; equal bytes under distinct IDs stay separate. Durable deletion and
retention keys prevent resurrection; local leases fence storage-time publication.
This is a synchronous component, not OS/app authorization. The next integration
priority is an installed Messages service/window through the existing lifecycle
owner, supervised storage and real lock/revoke ingress; no production modem
ACK/delivery capability is inferred. See `native-inbox.md`.

Lifecycle policy IO now runs in one owned storage process, retaining source/token
and boot-time deadline identity through actual reap. Twenty-seven installed/process
groups plus seven pidfd/file components pass on x86 and an actual ARM64 kernel.
SIGSTOP storage keeps snapshots/stop responsive; post-sync late replies cannot
restore admission. Healthy quiescent shutdown writes an epoch+exact-policy-hash
seal; uncertainty forbids automatic successor/replay. Stop and fatal cleanup share
a two-second absolute budget and abort with ownership retained. Injected wait
failures are distinct from uninterruptible Linux tasks. Existing historical receipts
still grant no later backend capability. See `native-lifecycle-storage.md`.

Original Omarchy appearance now executes the pinned v4.0.4 scripts and shell
against production Qt view/theme components in a private ARM64 Hyprland guest.
Eight original/native comparisons pass, including live font changes and token
overrides without replacing the window/draft. Six real Qt/process cases qualify
boot-time result fencing and abort before uncertain helper ownership is lost.
Historical theme proof and the corrected worker replay have separate provenance;
injected wait faults are not uninterruptible-task evidence. Installed native
service composition, complete desktop provisioning and hardware remain open.
See `native-theme.md` and `native-theme-worker.md`.

Native deployment admission now passes29 groups on x86/ARM64 user-QEMU after
independent review exposed and corrected ELF load geometry and cumulative payload
retention. Actual192 MiB authored inventory conforms under a96 MiB process limit;
recorded RSS is an observation, not a universal service budget. Current profile
refuses deployment with nine offline requirements, one physical gate and three
null kernel/module/firmware-set pins. Complete factory integrity, transitive ABI,
init/SELinux/backend/service closure remain offline work. Original21-test evidence
is preserved separately from correction evidence. See `native-deployment.md`.

Factory radio recovery now identifies the actual `libril-aidl.so` adapter and
native V1 bindings, separately from `libril_sitril.so` manager/producers. Fifty-two
static REA records and seven target hashes qualify scoped clear/death/locking
facts. Clear is one-way; version/hash lookup is not a manager cleanup barrier.
Locked producer paths do not establish complete serialization across unlocked
registration output and other caller paths. No vendor code was executed and no
retry/publication fence was relaxed. The next original runtime boundaries are
adapter, actual manager/producers, then pending work/reset, kept separate. See
`native-radio-recovery-contract.md`.

Installed Messages local history now qualifies one actor, an owned raw/inbox
child and an actual UID11010 Qt window in original Omarchy. Selected correction
passes11 ARM64 desktop groups, nested10 service groups and2 retained-descriptor
cases; current x86 permission/EAGAIN/recovery,27 lifecycle cases,4 components per
architecture and7 actual Qt/socket worker checks pass. Persistent desktop launch
ordering and transferred old-read descriptors retain source-bound local recovery
until actual reap; successor app/worker identities require fresh authorization.
The Qt worker checks stop/boot-time exchange completion and observes thread exit
before destruction, aborting if cleanup remains uncertain. Historical ff8d source
and proofs remain separate. Ongoing arrivals,>32 receipt enumeration, actual
lock/focus/session ingress, notifications/wake, send/ACK, production session ACLs
and Wayland/device policy remain open. See `native-messages-service.md`,
`native-messages-ui-worker.md` and `evidence/native-messages-correction-provenance.json`.

Synthetic incoming Messages now qualifies18 genuine ARM64 Binder/storage/authority
integration groups plus3 kernel identity cases. Two equal callbacks commit and
project distinct IDs before any app/read. Unsolicited ingestion retains original
boot-time deadlines; queue/socket/storage/source failures fence receiving while
stop and snapshots remain responsive. Durable attempt markers precede Binder
initialization and survive child/whole-service/systemd restart; recovery reads
history without registering again. Three actual reds establish raw-only projection
failure and old stop ACK returning before service exit/systemd healthy completion.
The new stop helper owns the exact socket-origin pidfd and observes service exit;
Main status and private policy seal/startup validation qualify separately. All16
fixture peer/manager contexts actually reap. Normal installed service is still
history-only;33rd-occurrence refusal, held-read close failing the source, callback
allocation/completion limits, modem identity/ACK/reset, session/notifications/wake
and hardware remain open. See `native-messages-incoming.md` and its provenance.

The next native session source now has an executed build boundary: eleven groups
in original Arch ARM Hyprland qualify matching package/header loading, central
keyboard focus, socket-derived live pidfs identity and original early lock signal
ordering. Callbacks precede the protocol locked flag; admission must retain denial
until later reconciliation. Public observation APIs do not provide trusted locker
admission/restore containment. See `native-session-build-boundary.md` and
`native-session-contract.md`; the production facts adapter and actor bridge remain
separate from this probe.

Complete bounded raw enumeration now removes the33rd-occurrence stop within the
append-only4096 receipt limit. Fixed-upper owner-bound pages and eight-import
projection turns preserve exact IDs and expose Ready only after full coverage plus
current tail. Fresh component/live-Binder reds precede six actual ARM greens;
22 receipt and6 projection cases pass per architecture. Four independent peer/
manager contexts actually reap. The frozen `f733d3b` actor/source is explicit;
new session composition remains separate. Raw retention, inbox capacity and large
cold backlog availability under original deadlines remain open. See
`native-messages-enumeration.md` and its clean provenance.

Native session identity now uses one owned focused-socket pidfd protocol on
anon-inode/pidfs kernels, with no scalar fields or observed numeric PID lookup.
14 leaf and26 observer checks pass on host and actual ARM; eight host policy/storage
components pass.15 host/14 ARM synthetic actor groups preserve independent grants,
original leases, held transitions, exact handle matching and responsive ownership.
17 prepared original-compositor/observer groups pass with the same current wire;
its prior adapter fails the actual missing-descriptor boundary. All selected
children reap; source/reap/vendor cleanup remain separate. Actor baseline replay
was after code drafting, explicitly regression evidence rather than chronological
TDD. Core source/leaf reds preceded implementation. See
`native-session-owned-focus.md` and clean provenance.

The actual original compositor now composes with the current observer/actor and
installed native Messages window. Six normal production-app groups and three
independent10-group diagnostic epochs pass. Unknown source, separate grants,
central null/layer focus, root intents and source unload/stop/death fence actual
reads. The separate fixture numeric-FD alias RED/GREEN prevents double adoption;
the guarded entry passes a fresh normal ARM app replay. Local synthetic history,
normal conversation visibility and diagnostic literal body remain separate.
See `native-session-installed-app.md` and its authored provenance.

The original100ms lease and locked-graphics availability gate remain: actual lock
can expire the source, and no failed source is revived. Trusted launch/procfs/common
namespaces, genuine Quickshell/PAM restoration, privileged Wayland containment,
secure frames, resume/wake and notifications remain open. Matched Pixel6.6
image/config/KMI remains unqualified. The fresh replay below adds the explicit
read-worker dispatch and qualifies the writer/readonly local-history path.
No matched kernel replacement or physical hardware completion follows.

Independent Messages read ownership now preserves the same live synthetic
receiver through app close, expiry, reader failure and actual held SQLite read
locks. Exact occurrence-bound projection permits wait for transaction completion
or actual reap without renewing the one-second arrival deadline. Two actual ARM
reds precede correction;17 ARM groups and51 focused cases per x86/actual ARM
kernel pass. Read identity, incoming source identity and whole Stop stay separate.
See `native-messages-receive-isolation.md` and its frozen-base proof.

A fresh combined replay now uses the current owned-focus actor and writer/readonly
split through original Hyprland and an actual UID11010 Qt window. The missing
reader-selector failure ran before its three-line fixture correction. Six normal
production-app groups and ten separately labeled diagnostic groups pass; held
readonly work is reaped after source unload while the writer stays alive until
whole Stop. Normal conversation screenshots and diagnostic literal-body evidence
remain separate. See `native-session-receive-read-combined.md`. This local-history
replay does not attach Binder; the incoming proof remains independent.

A separate frozen incoming/session/normal-Qt composition now attaches authored V3
callbacks to the same writer/readonly and owned-focus actor. Two epochs qualify
six narrow checks each, including background commit without an app and actual held
read plus normal close/reap with receiving preserved. Both full matrices fail on
cold Qt reopening under the unchanged100ms source lease. A later source-loss-only
epoch passes eight groups: actual plugin unload denies reads and reaps the held
reader while the same receiver commits callback4; actual retained Binder peer
death then fences receiving. No source revival or reattachment occurs. These
runtime selections remain based on518954b; normal conversation screenshots,
database bodies and failed reopen groups retain separate scope. See
`native-incoming-session-installed-app.md`. Broad cold availability remains offline work,
alongside production transport/ACK/reset, privileged display and trusted restore.

Native display containment now uses open-source Rust security-context-v1 bindings
through the unchanged original compositor filter. Fifteen actual ARM protocol/
normal Qt identity observations and22 supervised-owner observations in six ARM
epochs pass. A held-helper responsiveness red ran before the native supervisor.
The parent never dispatches the SDK;128MiB/64FD/core0 limits and kernel parent-death
kill precede setup. Original initialization deadlines, current source/child watches,
queued Ready fencing and actual reap ownership stay separate from remote close
processing. Per-app context generations/relaunch, production privileged runtime
and Quickshell/PAM provisioning, complete data/protocol restrictions, secure frames
and Pixel qualification remain open. See `native-display-containment.md`.

A new service-epoch display composition now joins the supervised Rust SDK child
to the current session actor. Two executed integration reds precede required source
stop/occupied, admission fencing and actual reaping under the same original actor
cleanup deadline. Two original ARM/normal Qt epochs pass25 groups: filtered binds,
actual focus plus independent grants, held-policy Stop and held-SDK death with
writer preservation. Current host observer/lifecycle checks pass26+8 cases.
An independent syscall-traced preflight red establishes SDK fencing after Messages
drain; the fixture correction fences first and shares one fallback deadline.
Runtime selections and current API build adaptations remain distinct. See
`native-session-display-composition.md`. Per-app display contexts, production
privileged source/configuration, PAM/restore and secure presentation remain open.

The next freshness checkpoint preserves both actual host/ARM reds before changing
the observer:30 cases pass and four desired boundaries fail per kernel. The bounded
drain now examines timely authenticated queued successors before cached expiry,
requiring a monotonic continuous100ms sample chain and rejecting expired
intermediates. Mutable snapshots permanently latch observed expiry/death; failed
authority cannot revive. Fresh builds pass34/34 per kernel, with original negative
controls, deadlines and assertions retained.

An exact published10ce2f1 original-compositor cold RED independently reproduces
fresh queued samples rejected before draining. Immutable base plus the reviewed
single-file patch then passes nine actual ARM incoming/session/normal-Qt matrix
groups, including successful cold reopen, source unload with held reader and the
same receiver, and actual peer-death fencing. The accepted queued successor follows
an expired cache by a99.177139ms sample gap; producer availability remains narrow.
Root verifies1,600 proof files and1,575 nested pins and independently reviews four
normal frames. Historical failures and earlier25 display groups remain frozen.
See `native-session-freshness-chain.md` and its publication provenance.

## Next work

1. Continue the qualified synthetic incoming Messages path with raw retention/capacity
   and cold backlog availability. Held-read cancellation now preserves shared
   receiving through the qualified occurrence permit and owned readonly child. Retain original ingestion deadlines and durable source uncertainty;
   history recovery never permits implicit backend reattachment. The transferred focus-peer handle now passes host/ARM interface checks; the joint
   original-compositor/current-authority/installed-app local-history path now passes separately. A fresh
   local-history replay combines the reader/writer split with that path. Next qualify
   broader cold incoming/session/Qt availability beyond the newly qualified nine-group
   replay and the failed per-app cancellation successor beyond the newly qualified
   lifetime/held-initialization Stop boundaries, then trusted lock admission/restore,
   notifications and wake. Producer/ACK/reset,
   sending and hardware gates remain separate. See `native-integration-map.md`,
   `native-messages-service.md` and `native-messages-incoming.md`.
2. Recover and test the actual vendor cleanup/reset and registration-overlap
   contract. `clear` is one-way. Do not remove uncertainty markers, retry, use a
   recycled numeric registration ID for delayed cleanup, or call local reaping
   vendor cleanup. Keep registration unavailable without proven remote recovery.
3. Extend the existing single-attempt request transport to durable operation
   receipts, safe subsequent operations, assembly candidate deadlines, notification
   watches and aggregate budgets. Recover producer identity before enabling fragmented semantic output.
4. Recover native IMS state/response contracts with original-code comparison and
   integrate the shared phone service. Continue calls, SMS, camera, power, audio,
   connectivity, sensors, background/lock behavior, permissions, apps, deployment
   and genuine Omarchy integration in the full smartphone matrix.

An advisory Pro worker-lifecycle consultation was stopped at the user's handoff
request before a terminal answer. It is not a completed review or accepted design.
Earlier completed owner reviews are recorded in `native-radio-owner.md`.

## Verification and publication

Use the existing prepared Linux checkout and its private inputs. Follow
`AGENTS.md`, strict red-green TDD, and `remote-development.md`. Update README and
affected docs each work turn when implementation or evidence changes. Run:

```sh
bash tools/build-native-radio.sh build
bash tools/build-native-messaging.sh build
bash tools/build-power-reference.sh build
bash tools/build-native-power.sh build
bash tools/build-suspend-reference.sh build
bash tools/build-native-suspend.sh build
bash tools/build-native-suspend.sh arm64-process
bash tools/build-native-suspend-boundary.sh build
bash tools/build-native-suspend-boundary.sh arm64-check
bash tools/suspend-boundary-suite.sh green
bash tools/build-native-sensors-boundary.sh
bash tools/sensors-boundary-suite.sh green
bash tools/build-native-audio.sh build
bash tools/build-native-connectivity.sh build
bash tools/build-native-audio.sh integration artifacts/native-audio/NEW_RUN
bash tools/framework-suite.sh sms-codec
bash tools/framework-suite.sh sms-transport-oracle
bash tools/framework-suite.sh sms-binder
bash tools/framework-suite.sh binder-kernel
bash tools/framework-suite.sh radio-ipc
bash tools/framework-suite.sh camera-owner
# In the prepared network-disabled graphics simulation container:
python3 tools/native-process-vm.py
```

These are focused native-radio gates, not proof that the full smartphone goal is
complete. Keep scope-specific evidence and explicit remaining gates. Public source
is on `main` at https://github.com/nrempel/pokelinux. Commit and push source/docs
as work progresses, after checking staged files. Preserve private artifacts and
pre-existing untracked files; never stage them wholesale. Check GitHub write
authentication on a new host without copying credentials from another machine.
