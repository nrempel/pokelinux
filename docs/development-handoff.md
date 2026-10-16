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

The [fixed Sounds desktop endpoint](native-sounds-desktop.md) now passes six
actual host UID/socket/owned-child cases after a genuine three-failure dispatch
RED. The matching fixed Rust client, desktop metadata, default64-test suite and
five ARM binary builds qualify within their recorded scopes. A separate actual
public native-client invocation passes with the actor's actual child wait; it
uses a sleep app, without desktop activation or Source/audio session installation.

The [normal permission-aware ARM join](native-audio-arm-lifecycle.md) now passes
all four groups: denied quietness, current-grant normal touch and440Hz routing,
explicit revoke/regrant/reconnect without replay, and fresh selection/call
preemption/revoke without restored media followed by window close. Independent
qualification recomputes14 PCM windows/336,000 measured frames. Original native
captures qualify1080×2424 and QWindow DPR2.625. The selected d982 actor cohort
predates the fixed desktop endpoint; root-authored grant/focus controls remain
explicit. The32 prior component passes are reused, not rerun in this epoch.

Earlier setup failures remain failed. A private actual-parent-wait diagnostic
observed SIGXFSZ25 under a global4MiB file limit, without observing the particular
file/write. The successful fixture repair restores the original actor and bounds
capture files in a separate8MiB tmpfs instead of constraining all app files.
Functional assertions, sample thresholds and deadlines stay unchanged; this is
fixture repair, without a production-bug TDD claim.

Next implement the installed native session-owner/profile/desktop assembly using
the existing Observer, per-app display supervisor and audio owner. Its parent
must retain and actually wait the compositor Child; Observer owns handles only.
Qualify genuine Source focus and independent grant, null-focus background lease
continuation, then terminal Source loss without app/audio revival. Do not reopen
an auth/PAM or broad static/diagnostic campaign. Hardware remains untested.

The [lifecycle MediaRoute boundary](native-audio-lifecycle.md) now owns current
selection admission and background Media leases through the existing sole audio
owner.64 library tests, strict Rust/ARM compilation, three caller builds and four
actual host permission/audio groups pass. The matching target Sounds client
passes30 socket behaviors plus setup/cleanup. The finite four-group normal ARM
join now qualifies separately above; preserve the historical direct-stream tracer
below. Production Source/PAM and hardware remain open.

The [normal direct-stream ARM Sounds tracer](native-audio-arm-ui.md) now passes
all five groups,18 audio socket behaviors and seven paired PCM stages
(336,000 measured frames), with native1080×2424/DPR2.625 captures and owned
closure. Live themes use original theme code in an authored theme-only host.
Keep this completed selection frozen and distinct from the later lifecycle
MediaRoute ARM join above. Source/PAM, production provisioning and hardware
remain open.

The [Sounds input repair](native-audio-readonly-input.md) prevents read-only
polls from cancelling a press. Seven real socket REDs precede18 behavioral host
passes plus Qt setup/cleanup. One queued explicit choice retains its original
deadline and fresh-state checks. The completed normal ARM tracer uses this exact cohort;
its UI/audio/theme result remains separately bound from the host proof.

The [media-only Rust audio client](native-audio-client.md) is now qualified as a
standalone transport:24 default host passes/one root-only ignored case,25 in
isolated root, and ARM compilation. Two real original-deadline failures precede
the source-only repair. Its poll is read-only and revoke closes the connection;
current lifecycle permission, normal Qt and graph integration remain separate.
Continue the finite ARM tracer and matching actor integration below.

Sounds now uses the [current schema3 appearance client](native-audio-appearance-client.md),
with11 unchanged audio/29 actual appearance IPC cases and an exact installed-QML
Ready/create check. The candidate is one matching current cohort, without a
private-HOME reader or old-schema fallback. The later normal ARM tracer above qualifies this separate UI/audio/theme
selection; its host component results retain their original scope.


The [additional audio checkpoint](native-audio-pcm-arm-foundation.md) passes five
actual host PCM stages (240,000 measured frames) and eleven unchanged Qt socket
cases under a generic ARM kernel. Signed32-package preparation preserves all353
existing versions and exports a closed read-only layer. These are separate
proofs; no normal Sounds GUI/audio/theme composition is claimed. Continue the
finite normal ARM tracer using the current matching schema3 appearance cohort.


The [native output-selector foundation](native-audio-output-selector.md) now
passes host logical catalog→actual graph and fresh same-cookie application
RED→GREEN checks, final original16-stage regression and Stop.12 Rust host tests,
ARM ELF compilation and11 actual Qt socket cases pass; native Sounds compiles.
The Qt peer is authored and no normal Sounds window or PCM result is claimed.

Continue the finite normal installed ARM output selector: actual UI input,
private Rust/PipeWire routing and independent sample flow, call interruption,
close/lease cleanup, loss/no replay and genuine Omarchy theme change. Official
ARM package provisioning and the existing qualified typed appearance client are
preparation; compose and qualify the exact final cohort. Keep private HOME closed,
use the Pixel10 metadata/native-capture guards, and avoid auth/PAM expansion.

The preceding CPIF checkpoint below retains its own completed evidence.

The [native CPIF local policy](native-sit-cpif-policy.md) now adds bounded
poll/read work, an absolute BOOTTIME timer and idle-zero handling. A compiled
2-pass/9-missing-policy RED precedes86 host/86 ARM user-QEMU component passes.
Separate actual host/generic ARM natural-worker0/70 cases pass after the fixture's lost-wait
repair, with exact history and journal refusal; original three controls retain
worker9. All11 policy tests and all five process cases also pass in one actual
generic ARM epoch with23 owned waits and input/container closure. The proofs
remain distinct; CPIF source/record custody, production admission and hardware
remain open.

After this finite local policy/lifecycle checkpoint, continue the installed
native media-output selector: existing Rust audio owner through normal Qt to
actual PipeWire/WirePlumber virtual links and sample flow, including close/loss
and a live Omarchy theme switch. Keep physical audio and CPIF custody gates
explicit; do not reopen a broad static/diagnostic or auth/PAM campaign.

The [retained CPIF checkpoint](native-cpif-receive-boundary.md) now binds a fresh
4K module extraction to the authenticated selected image and qualifies exact
read/poll, selected FMT queue producers and ordinary static factory paths. Reads
can retain a packet tail or return zero on an empty queue. This advances the native receive
contract; complete-record/channel custody, source/KMI, provider/APN/IMS and
hardware remain open. That static checkpoint changes no native behavior; no
phone execution is claimed.

The [owned SIT descriptor transport](native-sit-owned-transport.md) now adds
one-shot native Rust writes and owned raw reads beneath the attempt/process
owner. Strict missing-dispatch and missing-read REDs precede75 component passes
per host/ARM user-QEMU architecture and actual host/generic ARM kernel
completion/expiry/cancel process integration. The primitive exposes no descriptor or retry, retains
uncertainty, and uses the original BOOTTIME deadline. Three additional direct
syscall controls pass on host/ARM user-QEMU: actual write error, partial16 and
late-read refusal. CPIF record/source admission, provider/APN/IMS and hardware remain open;
the later natural-worker Main repair is qualified above.

One [current diagnostic bundle](native-boot-ack-and-current-bundle.md) now joins
the native build, complete8MiB image, exact original-unpacked generic and normal
child to actual Arch and full-stream Google-kernel consumers. Both pass four
intact readback ACKs, matching child wait37, shutdown and owned/input closure.
The complete selected vendor prefix, both vendor_boot fragments and padding
are preserved. Current init also passes five separate PMSG regression boots.

Two preceding Arch ACK corruptions remain the actual integration RED. The new
ACK helper makes one preformatted direct write and the unchanged Arch checker
passes. Host packet controls and host syscall observations already pass before
repair; they establish neither a packet RED nor a proven interleaving mechanism
or universal serial atomicity. Earlier evidence keeps its original bindings.

The [offline bundle](native-boot-development-bundle.md) remains unsigned and local.
Pixel loader/slot/DT selection, current-state restoration and independent recovery
remain open. Built-in PMSG/ramoops and both stock DTBs are favorable static facts;
real PowerOff may lose DRAM. A RAM-preserving transition and first compatible
recovery snapshot/export remain unqualified. No phone was accessed. Full IMS,
graphics and all remaining smartphone services remain separate; full deployment
is refused. Keep this diagnostic candidate frozen while continuing the native
service matrix rather than adding more synthetic diagnostic variations.

The [owned SIT receive copy](native-sit-received-chunk.md) now bridges raw
receive storage to the strict attempt parser. Twelve original Bionic/user-QEMU
children qualify full source-overwrite/free comparisons. A compiled42-pass/eight
missing-copy RED precedes50 codec and15 attempt passes per x86/ARM user-QEMU
architecture. Additional actual host and generic ARM kernel completion/expiry/
cancel controls pass with unchanged983/144-byte/report and lifecycle oracles;
the worker copies before overwriting/checking its entire reusable buffer. This
is opaque byte ownership, not complete-record framing, runtime modem identity,
provider/APN acquisition, production CPIF transport or Pixel/carrier proof.

Earlier codec and attempt milestones below retain their recorded evidence.

The [native SIT codec](native-sit-codec.md) retains its six complete983-byte
format-2 encoder comparisons, earlier host RED and eight encoder tests per
host/ARM64 user-QEMU target. The original constructor9 and encoder8 children
remain their own evidence. Earlier format0 supplies no current fixture.

The new [base144-byte response decoder](native-sit-codec.md#base144-byte-response-decoder)
initially passed22 tests per host/ARM64 user-QEMU target after genuine compiled/linted
host RED (16pass/six missing comparisons). Only response source changes; tests
and fixtures stay fixed. A separate original load plus nine complete parser
controls has ten actual QEMU waits0/stdout EOF; native tests compare six full
response/getter observations and eight refusal groups. Root qualifies the native RED/GREEN receipts; the original parser reference
also has separate independent bounded qualification.
Tokens and reported activity are local wire observations. The later independent-family
repair below identifies the uint32 MTUV4 mapping; the final byte remains opaque. These are user-mode codec tests, not ARM kernel/modem execution.
Original CLOCK_MONOTONIC cutoffs do not qualify suspend. Extended/error-remap
behavior, owned dispatch correlation, context/channel/subscription binding,
provider/APN lifecycle, deactivate/reset and hardware/carrier remain open.

The [independent-server repair](native-sit-codec.md#independent-server-families)
now passes23 host and23 ARM64 user-QEMU tests after genuine data-loss RED:
22 prior tests pass, but IPv6 DNS is lost for an IPv4-local cross-selector case.
A complete original modern report retains those servers. Pre-RED API/getter
harness adaptation is explicit; all six original getter fixture bytes stay fixed.
Only decoder source changes for GREEN. Canonical local/DNS/P-CSCF families are
independent, with explicit narrower getter projections and no compatibility alias.
This remains pure wire observation; the modern report port and owned dispatch/
provider/APN/channel lifecycle are not established by it.

The [native modern SIT report projection](native-sit-report-projection.md) now
composes the canonical base parser with owned emitted report observations.
Fourteen captured original cases match after compiled missing-report RED:
23 prior tests plus three controls pass, fourteen comparisons failUnsupported;
only the report source changes for fixed-test40 host/40 ARM64 user-QEMU GREEN.
Separate original runs have12 and7 actual child waits0/EOF, with independent
lifecycle/mapping reviews. The second run establishes zero-local/server filtering
and compatible/mapped/tied-zero IPv6 presentation. Fixed empty containers do not
establish complete264-byte ABI equivalence. The separate owned-attempt checkpoint
below adds local lifecycle/process composition; factory transport, provider/
channel/APN ownership and hardware remain open.

The [initial native SIT attempt](native-sit-attempt.md) now composes the983-byte
request,144-byte reply and owned modern report with one supervised source.
A compiled13-pass/two-missing-completion baseline and actual host/ARM process
RED precede a source-only completion repair:15 host and15 ARM user-QEMU tests,
then identical actual host/ARM kernel0/0/0 completion/expiry/cancel controls pass.
Held-worker snapshots remain responsive; Stop actually waits worker9 and the
unchanged journal refuses a successor. Current/history are local observations,
not APN/provider authority. Factory complete-record/runtime owner identity, channel/driver
binding, installed deployment and Pixel/carrier remain open.

The separate [original context-application probe](native-sit-context-application.md)
now closes seven memory-only context controls/14 update calls plus a load check:
eight actual QEMU waits0/stdout EOF. Matching dual/inactive/PDP-failure cases
copy the complete352-byte fixed DataCall prefix with bit1; matched Non-IP copies
with bit0, while CID mismatch/null leave it unchanged. Every identical second
call returns0 and the actual full-prefix memcmp is unchanged. State0 and null
APN/TD/controller pointers persist; original destructors and guards close.
Root and independent review qualify this additional original integration. No native behavior changes or22-test rerun are claimed. Handler/test
hooks, normal context/channel selection, activation, populated state and
APN/provider/kernel/modem/carrier qualification remain open.

The [native Data service tracer](native-cellular-service.md) now records one
complete generic ARM production Main/worker/operator integration pass.24 service
and nine shared host tests pass. Its34 guest direct waits/EOF include Main0,
Stop0 and unchanged-journal restart78; Main separately actually waits worker9.
One standard named Data endpoint/one setup yields exact historical18-field
provider report, a fresh same-observer association with an independently created
TUN, then real sole-close/independent-absence withdrawal retaining history.
All eight phases and outer CID/input closure pass. This is additional integration,
not a manufactured full-tracer RED; preceding host components and natural named
fixture RED→GREEN remain separate. Root independently rehashes all264 selected
and1247 original inputs and qualifies the raw phase/wait/kernel evidence.

Shared `native-process-owner` supplies final-credential cooperative bootstrap,
actual one-child wait and conservative synced admission. The new host receipt
checkpoint passes nine shared tests separately from its historical eight-shared/
59-radio result. No grant, vendor cleanup or automatic retry follows from reaping.
Normal installation/current authorization, provider/slot/APN identity, hardware,
blocked/output-pressure/expiry controls and service-manager fallback remain open.

The [native Data/kernel owner](native-cellular-data-owner.md) retains its fixed
actual ARM Binder/kernel RED→GREEN: wrong-serial refusal, accepted provider
history, a fresh same-observer join and real link-loss withdrawal. Three later
actual ARM runs of unchanged production code now exercise ordinary public
`poll()` setup dispatch and separate link, empty-list and endpoint-death loss.
The list/death cases withdraw a positive association while an independent
controller confirms the same kernel link and addresses still present; accepted
history remains intact. The earlier 42 waits/EOF and the additional 71
(21/25/25) qualify separately, with input/container closure.
[Public-poll evidence](evidence/native-cellular-data-public-poll-provenance.json).

This controlled fixture association does not prove the provider owns or created
the link or acquires an APN. Unknown-serial ordinary dispatch, current permissions/normal installation and
matched Pixel/carrier behavior remain open. The separately selected Data service
above adds its own conservative journal and process lifecycle qualification; it
does not upgrade these old owner evidence pins.

The [public Data V3 ABI](native-cellular-data-abi.md) now composes generated Rust
bindings, a dedicated Rust fixture manager and an independently generated NDK
C++ peer through actual ARM kernel Binder. One setup preserves all10 arguments,
21 profile fields and18 result fields; held/wrong-serial/right-serial callbacks,
populated/empty lists and actual endpoint death pass with eight child waits/EOF
and complete input/container closure. This is an ABI prerequisite: wrong serial
is copied, not owner-correlated in that ABI run. The later owner qualification
above is separate; production APN acquisition, slot/subscription mapping,
permissions/lifecycle and matched Pixel/carrier behavior remain open.

The [native kernel network observer](native-network-observer.md) now passes an
actual ARM kernel RED→GREEN with one fixed oracle: UID1000 Rust observes an
independently created TUN link, its IPv4 local/peer and IPv6 addresses, then
withdraws after actual sole-descriptor closure. Namespace retention, continuous
local lifetime, complete matching dumps and all32 direct child waits/EOF qualify
on the generic Arch kernel. Only the observer implementation changes; no Data
provider result is associated with this link and no APN readiness is granted.
The later controlled Data-owner join above is a separate qualification.
Error/reuse/overflow/suspend controls, installed permissions/lifecycle and
matched Pixel kernel/policy remain open.

The [typed add-PDN transport tracer](native-radio-add-pdn.md) now composes the
original-qualified encoder, current-session native owner and actual ARM Binder.
A genuine Unsupported RED precedes exact code105/slot1/663-byte receipt and two
pre-dispatch controls in three fresh VMs. Dispatch remains OutcomeUnknown through
post-dispatch cancellation, peer death and subsequent Stop; cancellation or real
BOOTTIME expiry before dispatch is
NotSent. All78 host radio tests pass, with8 ignored. Registration stays unavailable.
The [pure encoder](native-ims-add-pdn-native.md) retains its separate five-vector
x86/ARM user-QEMU qualification. Installed dispatch needs durable uncertainty and
current permissions; response meaning, APN/network acquisition and hardware remain
gates. The synthetic fixture reset clears only its preflight capture table.

The [native registration observation](native-registration-state.md) now has a
data-only shared type, production Gate mapper and a private actual ARM Binder
RED→GREEN test. Library67pass/1ignored and full host78pass/7ignored qualify the
component; no installed shared phone-state delivery is added. Source origin pairs
are local to one RadioOwner and require provider/worker-incarnation/subscription
binding plus current permission checks before any future export.
APN readiness injection is cfg(test)-only, with no production context writer or
assume-ready configuration, so ordinary owners cannot affirm RegistrationView.
Existing installed status remains transport/wire diagnostics (including its
ObservedActive phase); new view export and APN ownership/acquisition remain gates.
The separate [original admission probe](native-ims-registration-admission.md)
retains its conditional SDK37 scope; native conservative Unavailable/history
policy is explicit, without original loss/retry equivalence or cleanup authority.
Source-only REA still identifies an add-PDN-ready early return on the reason0
loss path. The new admission probe explicitly fixes PDN-readyfalse and closes the
original live request HandlerThread through actual quit/join and task absence.
The original asynchronous disconnected/loss path remains unexecuted.

The [installed slot1 registration history tracer](native-radio-registration-installed.md)
now qualifies actual ARM Binder callback→native owner→typed worker IPC→restricted
status after a genuine missing-projection RED. Required `SLOT_ID=1` reaches the
worker and owner. One current `last_observation` is tagged `raw` or
`registration-wire`; typed revision4 remains historical when peer loss makes
status unavailable at revision5. Native worker wait9, systemd Main exit0, empty
cgroup and unchanged-journal refusal78 pass without another registration. All59
host library tests, strict checks and guest/outer/input closure pass. This is
public synthetic Binder evidence, without semantic IMS or phone qualification.

The earlier [registration-wire owner checkpoint](native-radio-registration-owner.md)
qualified exact raw3012 receipt→Unsupported RED→typed publication GREEN with52
host tests, raw↔typed replacement, five refusals and consumed-valid cancellation.
Its identical peer/fixture/oracle and full closure remain component evidence.

The [original raw registration route](native-ims-registration-routing.md) separately
qualifies17 dispatch cases and8 parser calls in SDK37 x86: class code3012, exact
slot filtering and full-array delivery. Its original-code scope stays separate
from the authored ARM peers above. Full helper/channel semantics, producer/reset,
remote cleanup acknowledgement, endpoint policy, semantic IMS and hardware remain
open. Installed malformed-indication continuation has not run; the explicit
worker `InvalidIndication` skip is source policy.

The earlier [permanently held registration control](native-radio-service-blocked.md)
qualified three responsive Initializing statuses, native Stop/wait9, Main exit0,
empty cgroup and unchanged-journal refusal78 with its then-identical production
ELFs and45 host regressions. That historical execution is not a rerun of the
current typed component set and grants no remote-cleanup or retry authority.

The earlier [installed RAW service tracer](native-radio-service.md) qualified
raw callback diagnostics after a separate one-line history repair, with45 host
tests, peer-loss withdrawal, native/PID1 waits and durable refusal. Its old schema
and evidence remain historical. The preceding [production worker tracer](native-radio-production-worker.md)
retains its own narrower boundary; neither is relabeled as the new slot1 run.

The preceding bootstrap policy checkpoint follows.

Four additional actual ARM bootstrap policy controls now pass against one retained
public Rust Binder peer: live wrong-parent refusal, actual READY/STOP/T/queued GO
before expiry then CONT after expiry with typed TimedOut/wait1, parent polling
withheld past its deadline, and Stop during Initializing. Parent actual waits are
exit1 and SIGKILL9; durable same/recreated-owner refusal remains. Recorded actual
wait cutoffs and external parser checks close; late GO wait is29.715879ms after
its original deadline. All37 host tests and strict/ARM compile checks pass. Root
checks67 source bindings and production runtime source unchanged from9b1948b.
All four guest clients wait0, the peer stays live with registrations0, and actual
outer wait/exact-CID/frozen-input closure passes. These are additional coverage,
not a new manufactured RED or actual suspend. Exact queued READY at parent
expiry/Stop remains open, alongside production worker/discovery, installed
supervision, vendor/IMS and hardware. No phone, vendor/Android runtime, private
REA or authentication/PAM executes. See [policy controls](native-radio-bootstrap-policy.md).

The preceding cooperative bootstrap implementation follows.

The production radio library now has a selected cooperative worker bootstrap
before Binder setup. A genuine new-requirement ARM baseline leaves its held worker
orphaned after supervisor death; the repaired selection uses the unchanged outer
oracle and has actual adopted SIGKILL wait7.528539ms without a driver worker signal.
The peer stays live, uncertainty survives restart, and explicit Stop regresses
successfully. A separate actual ARM control refuses a live wrong-parent pidfd
before READY/body with child wait1 and peer registrations0. Review's reap-error
readmission defect has its own35-pass/1-fail component RED, then36 host tests pass.
Strict checks and ARM compilation pass; Root verifies67 source pins and three
runtime selections with complete actual-wait/exact-CID/input closure. No phone,
Android/vendor runtime, PAM or new authentication execution occurs. Queued READY
expiry/Stop controls, late GO, installed service-manager/cgroup, complete worker,
vendor cleanup/IMS and hardware remain. See [bootstrap report](native-radio-bootstrap.md).
The generic start primitive retains only its existing explicit-Stop contract.

The preceding normal GUI checkpoint follows.

The normal full QtQuick Messages tracer now passes on an actual Arch ARM kernel
in original Hyprland/Omarchy, through the current Rust actor, retained native
history and actual owned display SDK. Initial/armed denial, exact later authored
Source clear, two hello pages and actual UID1000 durable relock denial reach the
same mapped window. The real baseline closes natively but has app owner wait15;
only production Qt main.cpp then gains early TERM masking/signalfd→normal quit.
The unchanged test ELF/other helpers/assertions/harness pass with actual app
owner wait0, complete native/guest/outer closure and frozen input checks.53 default
host tests,13 selected host cases/16 reports, strict checks and full normal Qt
host/ARM compilation pass. Six ARM compiler jobs finish in30.768s. Actual surface
protocol/independent1080x2424 captures qualify narrowly; direct QWindow API,
exact commit-to-capture association and source attestation remain separate. Auth,
Source and SMS seed are authored, with no production auth ingress/PAM action or
phone access. Live appearance, normal GUI same-ID reopen, secure presentation,
production provisioning and hardware gates remain. See [GUI report](native-session-restoration-gui.md).

The preceding normal app Stop checkpoint follows.

Explicit normal app Stop now gives Running apps one immutable BOOTTIME TERM
grace capped inside the original2s actor cleanup; other retirement stays forced.
Actual Qt wait9 and premature TERM-ignore kill REDs precede53 default tests plus13
selected service cases passing (16 actor reports). Repeated Stop preserves its
first reply/storage shutdown and cannot renew grace; actual protected reads deny,
snapshots respond, and the TERM-ignoring app has owner wait9 after death observed
511.570189ms with34 probes/maxgap16.248268ms. Explicit Terminate remains prompt
wait9. A held authored Terminal Context retains the actor after actual Qt wait0
and policy-worker closure until explicit fixture release. Four Qt epochs have
normal owner wait0; the unchanged production client destructor/thread wait now
executes in the private QCore host. Actual waits come from a bounded cfgtest witness
of normal Apps::poll, not a second reaper. Strict checks and ARM compilation pass;
there is no new ARM runtime/GUI/original SDK/capture/auth ingress or phone access.
Root rehashes98 source and18 runtime bindings. Source-level500ms/TERM-once and the
Forced-latch error caveat remain distinct from measured bounds. Preparing Stop,
forced upgrade/error controls and real GUI/compositor/hardware remain open.
[Current app Stop report](native-app-stop.md).

The preceding consumer checkpoint links the unchanged production MessagesClient into a
private lifecycle-owned UID11010 QCore process. It actually decodes two retained
Messages bodies after clear, clears its lists after real relock, then returns the
same receipt IDs from a fresh actor/Qt process over unchanged paths without reseed.
53 default tests and9 separately selected actual service tests pass, with12 actor
reports and9 actual test-parent waits0/EOF/proc absence. The missing-client baseline
RED and a zero-body compile prerequisite failure remain separate. Root rehashes98
source and18 runtime bindings. Strict C++/Rust checks and separate ARM compilation
pass. The normal actor Stop uses SIGKILL, so the helper's signalfd/destructor path
is source-only; Qt graceful shutdown is not claimed. No QML/Wayland/original Source,
production authentication ingress, new ARM runtime or capture is added. Auth,
typed Source and SMS seed remain authored. Continue into normal installed Qt and
original compositor composition, keeping graceful lifecycle and secure presentation
open. [Current Qt client report](native-session-restoration-qt-client.md).

The preceding headless tracer uses the actual native Messages Projector writer
and independent readonly child after a preserved real-query behavioral RED.
53 default tests pass with8 explicitly ignored integrations separately selected
and passing. Native pages reach the actual UID11010 app only after the real policy
worker clear. A relock/clean-close/reopen over the same raw/inbox paths requires
fresh admission/attempt/later Source and returns exactly the same two receipt IDs,
without reseeding. There are8 actual test-parent waits0 and10 successful actor
reports. Source/auth/SMS seed are authored; no production auth constructor/ingress
or normal Qt is added. The dev-only facade bridges crate identity while delegating
full Provider ownership/deadlines. A linker resource prerequisite failure is kept
separate; corrected affinity/PID capacity preserves all original deadlines.
Strict lifecycle/facade checks and AArch64 compilation pass; no new ARM execution
is claimed. Astra finds no blocking architectural issue and keeps early-error
Drop source-reviewed. See [retained history](native-session-restoration-history.md).
The preceding6 stress cases use authored history and do not become real-history-
specific held-clear qualification. Normal UI/genuine authentication/device gates
remain open; no phone was accessed.

The preceding downstream restoration milestone passes53 default host tests, with6
explicitly ignored root integrations selected separately and passing. A private
cfgtest authenticated-and-closed completion atomically retains Armed context;
only its later exact Source event requests a real sole-worker synced clear.
The same UID11010 app stays denied before/after arm and reads the authored history
packet only after completion. A held post-sync false clear remains denied and
Snapshot-responsive; an actual UID1000 new lock refuses ACK, the same retained
worker writes true, and a clean same-policy restart denies the fresh app.
The first new lock's storage budget is not renewed. Queued lock success retains
storage admission identity, including a complete intervening write. Separate
actual service and Worker/socket REDs, authority ordering/clock REDs and zero-body
compile/format prerequisites are preserved. Strict checks and AArch64 compilation
pass. No new ARM execution is inferred. Authentication/Source/history inputs are
models; there is no production success constructor or ingress, real history/Qt
unlock tracer, whole-owner auth composition or phone proof. See
[restoration](native-session-restoration.md) and its bounded provenance.

Four additional actual-loop cases now preserve forced-sample-before-later-event
ordering, actual100ms expiry with responsive Snapshot and app retirement, old
completion refusal after an actual successful idle UID1000 new lock, and normal
Stop during held synced false. The latter leaves no clean seal and a fresh actual
storage worker refuses the unchanged policy/live-owner bytes; a full unclean
actor reopen is not executed. A bounded private Source-observation barrier carries
no auth authority. All6 cases and the clean full-actor restart subcase pass with
actual test waits0/EOF/proc closure. Host daemon/launcher and AArch64 daemon bytes
match the preceding selection exactly; these are coverage additions, not a new
production repair. New ARM runtime and genuine authentication remain unqualified.

The preceding production source `90ced3a` now runs all 40 lifecycle library tests and
both planning socket cases under the actual generic Arch ARM kernel. The combined
run is failed: the later queued fixture sees stopped syscall-1 without nfds and
refuses setup before sending either test packet. A source-supported ARM64 signal
explanation is an inference, not qualification. Actor/peer fallback waits-9 and
storage pidfd death do not establish storage reaping; its proc entry was still
present. Actual QEMU/CLI/container wait0 and fresh CID absence close only the
outer boundary. Root rehashes120 inputs. No rerun or weakened guard occurs.
The native-ABI fixture still passes its separate host case. See
[current ARM report](native-lock-storage-arm.md).

The queued lock-success timing hole now has its own actual host socket RED and
repair. A poll-phase-qualified actor queues an idle UID1000 lock before a later
ordinary grant; the old binary sends `ok` while storage is held. Current code
marks typed queued lock success and rechecks before every nonblocking send,
permanently downgrading to unavailable on busy/unavailable/Stop. The unchanged
queued fixture, two planning controls and all 40 host regressions pass; strict
Clippy and AArch64 compilation pass. A formatting-only zero-test build failure
is retained. [Queued report](native-lock-queued-storage.md).
The older planning code `1106ab2` separately passes two actual Arch ARM cases,
with normal fixture/actor/QEMU/CLI/container waits and unchanged read-only image;
no later queued-repair ARM runtime is inferred. No send EAGAIN or complete
intervening write is induced. Those earlier controls do not qualify arming or clear. The newer downstream
selection above still leaves genuine authentication and normal Messages reopening
open, with no phone accessed.

The actual headless lifecycle lock socket now refuses a cached successful ACK
if an actor/storage operation remains unresolved at planning time. A genuine production-daemon
RED returns `ok` with a ready actor and exact storage child held after an ordinary
grant sync; the unchanged pending-true/false fixture passes after the guard.
Memory is fenced before refusal, idle already-durable ACK stays successful,
snapshot/Stop remain responsive and normal actor waits/storage death close.
All 40 host regressions and strict Clippy pass; AArch64 daemon compilation is
separate from execution. A pre-start empty-CID runner failure is preserved with
its missing CLI wait explicitly unqualified. See [storage barrier](native-lock-storage-barrier.md).
That earlier source still allowed idle queued success after another write began;
the later publication guard above qualifies its own separate regression. It implements
no authentication result or clear. Those earlier tests qualify no authenticated clear. The newer private
restoration selection above remains separate from genuine authentication; no
phone was accessed.

The safe Rust native authentication scope now passes two actual Arch ARM
synthetic controls after a preserved wait-scope defect and detached-descendant
RED. Cancel/Stop and the original15s deadline retire locally while the exact owned
supervisor is stopped; after resume, recursive termination closes all three actual
workload waits, ECHILD, EOF and the empty leaf without independent SIGKILL. Both
controls close five retained pidfds/proc entries, native supervisor/parent waits0
and all127 frozen input bindings. The component remains isolated-feature-only.
A later actual missing-handle setup RED proves the parent can block on its child
pipe before NONBLOCK conversion. The minimal repair disarms both endpoints while
retaining Child/pidfd and sticky occupied Setup fault. The first actual GREEN's
initial-report scheduling gap is preserved; a hardened fresh control now passes,
as does the new normal two-group regression. Driver9/9 teardown in the setup
control is not native cleanup. See [setup report](native-auth-setup.md) and
[proof](evidence/native-auth-setup-provenance.json). An actual privileged-helper-profile self-migration RED now precedes a
private mount/root direct-path GREEN on Arch ARM, preserving the real helper IDs,
audit-only capabilities and NNP0. Owned cgroup kill, wait9, EOF, empty leaves and
ECHILD close without fallback. [Containment report](native-auth-containment.md).
This is an authored direct-path fixture, not original PAM or native scope cleanup.
A later ordinary host wire/parser RED precedes13 passing host components for
hidden prompts and a standalone zeroizing response frame. A new extracted
Conversation selection reproduces three actual host failures before repair:
failed initialization reads a retained blocking pipe, an expired direct call
writes22 public dummy bytes, and a late candidate reaches pure protocol state.
Nineteen host components now pass, including two existing-wire prompt rounds,
queued-response cancellation under real EAGAIN and invalid response admission.
The blocked ordinary helper is actually killed/waited before RED failure; GREEN
exits0 naturally. Strict Clippy and separate ARM64 compilation pass. Local
deadline checks do not prove atomic timing across syscalls. Attempt retains real
custody, but no actual Attempt/cgroup, original PAM or ARM runtime executes in
this historical selection. See
[current conversation I/O](native-auth-conversation-io.md) and the historical
[primitives](native-auth-conversation.md).

The earlier isolated parent/supervisor forwarding loops compile, using
PKSCOPE3 credential-free prompt records and bounded zeroizing command frames.
A real-pipe production record-decoder RED passes19/fails1 after an accepted
baseline. A separate new-API legacy-command extraction passes1/fails11 before
the codec repair; it is not original full-fixture execution. The final frozen
host suite passes40 components and strict Clippy, plus separate ARM64 compilation.
Exact parent queue/command composition crosses real pipes. Both owners retain
their ingress decoder so retirement drops partial response bodies before cgroup
work; fixture read buffers drop before decoded intent is applied. No actual
ScopeChild/Attempt or complete forwarding loop runs in this selection. See
[forwarding report](native-auth-parent-forwarding.md) and
[bounded proof](evidence/native-auth-parent-forwarding-provenance.json).
A separate unchanged library selection now passes the same40 components on an
actual Arch ARM7.1.6 kernel as UID/GID1000, groups[] and CapEff0. All41 entries are
listed; its one ignored helper is explicitly invoked by the parent regression
and reaches natural wait0/EOF/dead pidfd/proc absence. Guest test, QEMU, CLI and
container waits0, fresh exact-CID absence and frozen input closure pass. Earlier
compile/postprocessing and no-boot temporary-directory attempts remain separate
zero-test failures. No fixture binary, ScopeChild/Attempt, PAM, cgroup workload,
unlock or physical phone executes. See [ARM components](native-auth-components-arm.md)
and [proof](evidence/native-auth-components-arm-provenance.json).
A later positive host process regression now constructs actual ScopeChild from
one ordinary Child/pidfd and private pipes. Production pending/respond/poll APIs
transfer the expected-length public response prefix to that peer. Native parent
poll observes actual peer wait0/EOF, dead pidfd/proc absence and reconciles closed
bookkeeping. Worker/cgroup observations are synthetic; normal root construction,
Attempt and complete fixture loops remain unexecuted. All41 host tests, both
explicit ignored helper invocations, strict Clippy and ARM64 compilation pass.
The earlier ARM40 selection does not qualify the new test. See
[parent process report](native-auth-parent-process.md) and
[proof](evidence/native-auth-parent-process-provenance.json).
The current PKSCOPE4 source adds opaque one-time local closed classifications.
An actual event7 refusal RED first preserves31 earlier passes, peer natural
wait0/EOF/dead pidfd/proc absence, sticky Protocol quarantine and test abort6
(Cargo101), without a complete suite summary. The final host suite passes59
with3 explicit ignored helpers invoked by their parents. Actual ScopeChild waits
for the live peer's exit/EOF before issuance; already-issued tokens are refused
after Stop or original deadline without rewriting terminal snapshots. Five
malformed-stream controls require inner actual closure before occupied Drop
abort, then outer wait6/EOF. Timeout fallback supplies no independent inner wait
receipt and cannot pass; the owned container remains enclosed until closure.
Worker extraction and terminal serialization are wired/model-tested/compiled,
not actual Attempt/PAM/cgroup runtime evidence. Strict Clippy and separate ARM64
compilation pass; the earlier ARM40 snapshot cannot qualify these changes. No
actor result/receipt/unlock ingress is added. See [closed outcomes](native-auth-closed-outcome.md)
and [proof](evidence/native-auth-closed-outcome-provenance.json).
The unchanged59 library tests now separately pass under an actual Arch ARM64
kernel as UID/GID1000, groups[]/CapEff0. All62 host/ARM names and3 explicit ignored
helpers match, with no source/deadline changes. Root rehashes132 unique bindings;
actual test/QEMU/CLI/container waits0 and fresh both-CID absence close. Parent
nested wait assertions remain source/ELF-bound, not an independent raw journal;
no actual Attempt/Terminal/PAM/cgroup/actor/unlock executes. See [ARM59](native-auth-closed-outcome-arm.md)
and [proof](evidence/native-auth-closed-outcome-arm-provenance.json).
A later bounded host selection passes59 library +4 exact Terminal writer tests.
New tests first fail the missing private-adapter compile prerequisite, with0
bodies; a separate zero-test strict-Clippy failure remains preserved. The final
writer tests use actual pipe EAGAIN before outcome and after outcome/before
CLOSED, then pending revocation/original model clock expiry. One decision and
identical terminal snapshots persist; only the classification source is modeled.
The private Attempt adapter compiles, but actual Attempt and full fixture loops
remain unexecuted. ARM fixture compilation is separate; earlier ARM59 does not
qualify these new binary tests. See [writer report](native-auth-terminal-writer.md)
and [proof](evidence/native-auth-terminal-writer-provenance.json).
Next: runtime qualification of actual parent/Attempt forwarding and original PAM+scope, including retained
faillock and remaining fixed alias/filter routes, then broker/actor one-shot
synced restoration and a normal Messages reopen. Keep Source100ms/no-revival and all hardware gates.

The native Rust PAM worker now passes actual wrong/matching password controls
through the installed public ABI on Arch ARM7.1.6. The closed-worker missing-prompt
RED and first correct-denied failure remain preserved. Direct original-helper
controls establish the narrow audit-only bounding requirement while worker active
capability sets stay0. Root independently verifies all156 passing-run inputs,
actual waits/deaths, final ECHILD and exact-container absence. Helper-tree ownership
and a separate NNP0 broker remain in progress; the Messages actor keeps NNP1.
No actor unlock receipt or durable restoration is enabled. See
[worker boundary](native-pam-worker.md) and [proof](evidence/native-pam-worker-provenance.json).

The actor now independently refuses impossible exact-operation fact shapes and
changed generation/unlock records without a matching semantic revision. Three
negative RED groups execute18 mutations; four focused GREEN controls and all16
lifecycle components pass on host and actual Arch ARM. Cached exact samples and
sticky heartbeats remain accepted. This is a typed Provider/actor selection,
with no Source IPC, PAM or restoration execution. See
[actor provenance](evidence/native-session-protocol-actor-provenance.json).

The current Source now supplies an observed accepted lock-operation counter and
an exact-object protocol-unlock occurrence, retained in Rust internal snapshots.
Original producer and separate actual-packet REDs precede the repair. All55 host
packet cases pass. The joint original-compositor→Rust observer run passes eight
groups, including55 actual Arch ARM packet cases and12 lifecycle components. One live client establishes
successive generations; real exact unlock stays sticky, forced unlock plus a
later inert-object request supplies no positive cause, and late first attachment
stays generation-unknown. No authentication or restoration authority is added.
See [contract and bounded provenance](native-session-lock-generation.md).

The internal actor-attempt primitive now retains a single occupied slot with sticky
retirement, exact live locker/Source/actor binding, lock epoch and an original
BOOTTIME deadline. The missing-API RED is compile-only;33 host components and strict
Clippy pass. [Bounded proof](evidence/native-auth-actor-attempt-provenance.json).
That historical selection remains unwired and supplies no result or unlock API.
A later private capacity consumer matches exact actor incarnation, non-reused
attempt ID and original deadline, then removes only the slot. Its sole constructor
is cfg-test authored closure data; no production producer/listener exists. Seven
new controls pass among40 full UID0 lifecycle tests and22 separately selected
UID1000 actor tests (18 explicitly filtered there). Strict Clippy and AArch64
library-test compilation pass. The initial missing-API compile RED,33/7 failed
wrong-UID aggregate and zero-test root-output permission failure remain separate.
Release cannot change durable pending/grants/history, revive a Source/old handle
or arm unlock; a fresh explicit begin still validates its own context. Actual
closure after expired result authority needs an independent qualified broker
producer; do not infer it from this stamp. See [capacity report](native-auth-actor-capacity.md)
and [proof](evidence/native-auth-actor-capacity-provenance.json).

The unchanged22 actor capacity cases from8296587 now separately pass on an
[actual Arch ARM64 kernel](native-auth-actor-capacity-arm.md), UID/GID1000,
groups[]/CapEff0. All40 names are enumerated only;18 other authority/storage
tests are explicitly filtered and both ignored lists empty. The host-built ELF
and existing read-only VM image are reused without rebuild/mutation. Root
rehashes209 paths and verifies actual process/QEMU/CLI/container waits and fresh
guest/upstream CID absence. This remains typed Provider/pidfd component evidence,
with synthetic closure facts, no production producer, PAM/scope or actor unlock.

Next compose supervised native authentication with actor-owned attempts, then
one-shot exact-operation restoration, synced durable policy and a normal Messages
read. The separate [Original PAM prerequisite](native-original-pam-prerequisite.md)
passes typed wrong/correct responses but supplies no actor-owned receipt. Do not
use a mapped PID, observer revision, global unlock, process death, or a renewed
expired Source as restoration authority. Preserve the earlier
[owned-locker selections](native-session-owned-locker.md) separately.
The actor now terminally refuses locker changes without a corresponding sequence/
semantic revision and unlocked snapshots with a locker. Three actual typed-
Provider tests fail before the repair; all four focused controls and all12
lifecycle components pass afterward on host and actual Arch ARM kernels. Handles
and the replacement child are real; no Source IPC, PAM or unlock runs in this
selection. The earlier Original integration remains separately frozen. See
[actor consistency provenance](evidence/native-locker-actor-consistency-provenance.json).

The native Rust appearance owner now publishes schema3 exact RGBA16 and typed
style. An actual Original/Rust/private Qt alpha RED precedes the repair; fresh
GREEN passes17 phases/34 full captures with every592704-pixel plane equal. Root
rehashes223 bindings and independently decodes every image. Separate23 host/22
Arch ARM kernel groups and505 private Qt peer/QML cases pass. Prior failed
aggregates remain failed. Font resources, full parser/Border/normal-window
rendering and installed Source/lock/supervision remain open; the partial Qt app
migration stays private. See [alpha checkpoint](native-appearance-alpha.md) and
[proof](evidence/native-appearance-alpha-provenance.json).

Private font-resource attempts have not qualified a resource RED: an initial
setup failure precedes a reproducible Original/native baseline pixel mismatch.
A later getter-only observation run completes two capture pairs and owned cleanup
but keeps the pixel-equivalence aggregate failed. Font data, recorded geometry
and renderer API agree; application defaults differ and cause remains unproven.
The backend is absent in that authored component host; no normal Messages font
or full installed-theme result is claimed.
A subsequent single-default-alpha counterfactual compiles, but its GUI run fails
native positive readiness before a complete capture pair. The public broker is
available while the sampled native cache is unavailable. No rendering cause or
font-resource RED is established; the failed selection and cleanup are retained.

The historical preceding schema2 owner publishes exact typed style,
with generated/user overlays, all8font/6spacing tokens and full-payload revision/
queued-send fences. Nineteen host groups pass; actual user replacement/deletion,
unsafe inputs and absence recovery are covered after a genuine user-style RED.
A separate held-input RED exposes obsolete colors renewal across an atomic file
exchange with unchanged generation/lock. Retained inputs/chains/absence and final
content/path checks repair it; unchanged control observes null/withdrawal, fresh
collection and actual reaps. Every failed aggregate/setup label remains private.
See [style proof](evidence/native-appearance-style-provenance.json) and
[input proof](evidence/native-appearance-input-provenance.json).

The private schema2 Qt reader passes154 actual kernel-peer/QML controls after a
genuine schema2 baseline RED; malformed/revision controls execute only after
accepted baseline. Root independently reviews records and exact-CID absence.
New Rust static ARM and normal Qt6.12 compilation pass. Seven later original
ARM style groups pass: all15 observations match twice per phase after the real
size18 command and user overrides, in the same normal PID/window with private
HOME0700/EACCES. Root rehashes150 bindings and independently decodes all three
whole1080×2424 captures. Actual waits,569 observed native collector pidfd deaths,
exact-CID absence and input closure pass. The normal app stays uninstrumented.
Large-font buttons visibly clip; Original bar fonts/overlap remain incomplete.
See [style ARM proof](evidence/native-appearance-style-arm-provenance.json).

The repaired selection separately passes18 generic ARM kernel groups after a
fixture race: the first process aggregate remains12pass/1fail because SIGSTOP
finds the selected child gone before the intended publisher-kill assertion.
The fixture now excludes the initial post-lock PID, retains its successor's
pidfd/parent/start identity, requires a live original deadline and observes T.
All three ARM cases exclude actual nonzero initial PIDs. Production Rust and its
ELF are unchanged;19 host groups also pass. Root independently verifies59
bindings, all actual waits/no adopted children and exact-CID absence. See
[component proof](evidence/native-appearance-style-arm-kernel-provenance.json).
Concrete family/resources, alpha and original parser/overflow edges remain open;
do not promote the partial consumer and remove the public app's full theme path.
Historical17 host/two held-exec-FD and16 generic ARM kernel controls retain their
preceding palette-only source selection. External supervision, actual desktop/
current Source/lock and full smartphone gates remain. Source stays25ms/100ms.

A separate later ARM composition control passes five actual groups with genuine
Omarchy and directly launched normal UID11010 Qt: Tokyo→Latte→Tokyo follows the
Rust public palette in the same app/window while private HOME0700/EACCES remains.
Root independently decodes all three native1080×2424 captures and verifies87
bindings. Owned guest/broker waits, CLI0, exact-CID absence and input closure pass.
The backend is unavailable and no Source/actor/desktop ingress/per-app display
owner participates; the original installed RED remains open. Two reviewed clean
captures and [bounded evidence](evidence/native-appearance-palette-arm-provenance.json)
are public. Fonts/settings/alpha and normal QWindow DPR remain unqualified.
Next: full theme features and desktop/current Source/lock composition without
widening the100ms lease.

A later headless actual Arch ARM kernel selection passes sixteen groups:
eleven real process, four same-process kernel SEQPACKET and one confinement.
Root independently verifies57 bindings and full harness summaries, actual child
waits, no adopted descendants, CLI0, exact-CID absence and input closure. The first
label remains a zero-test fixture failure: existing root0555 violated the test's
excessive0755 assertion. Fresh successor records metadata before matching the
actual service ancestry rule; ELFs/source/deadlines remain unchanged. Google kernel
and installed capability profile are unqualified. See
[ARM kernel evidence](evidence/native-appearance-arm-kernel-provenance.json).

An independent pinned-source style contract identifies all8font/6spacing tokens,
user-overlay deletion, distinct Color/Border alpha behavior and separately
observed user/font consistency. A family name does not prove UID11010 access to
private font assets. Expanded Original oracle and actual Rust-socket/Qt-QObject
style RED now execute under generic ARM: baseline matches, then all15 font-scale,
font/spacing observations differ after original size18 and owned user overrides.
Palette/private HOME remain correct in the same normal PID/window. Compiler setup,
44 direct child waits, controlled Rust Stop/wait0, actual CLI0 and independent
exact-CID absence pass. Zero collector pidfd rows means no independent collector
reaping proof in this epoch. Both contract aggregates remain FAILED. The typed
Rust repair is published; full consumer migration remains open. See
[bounded RED](evidence/native-appearance-style-red-provenance.json). Contract:
`/tmp/pokelinux-appearance-style-contract-sol/contract.txt` (source review only).

The installed appearance boundary retains its genuine RED. Fresh `red-2`
launches unchanged UID11010 Messages against private UID1000/mode0700 HOME;
actual original Tokyo Night and its read-only oracle report `#1a1b26`, while
the normal app gets `EACCES` and renders `#101315` in its preselected native
interior patch. Root independently decodes the1080×2424 PNG and binds171 files;
owned guest cleanup, CLI0, independent exact-CID absence and input closure pass.
Aggregates remain failed. `red-1` remains a setup failure, not a palette RED.
The Source adapter is not loaded during this assertion. Typed consumer/installed
owner composition remains open, with no installed GREEN claimed.
See [appearance boundary](native-appearance-service.md). A pinned source-only
[Omarchy Mobile review](omarchy-mobile-reuse.md) identifies mobile shell,
keyboard and rotation reuse; its Pixel7/mainline backends do not qualify Pixel10
or change the matched-Google-kernel/native-Rust architecture.

The vertical phone tracer now qualifies eight actual ARM groups: authored SMS
commits to native storage while the window is closed, genuine Omarchy launches
unchanged UID11010 Qt Messages, actual keyboard submission precedes a normal history read, close
retires the app/context, a second arrival retains the same writer/incarnation,
and a fresh context reads both occurrences. Revoke/regrant passes. Both whole
1080×2424 SDL/X11 captures pass complete PNG/XImage RGB equality and current
Source health before/after; Root viewed one then two literal messages with the
original Tokyo Night app palette and Omarchy bar. Missing icon fonts remain a
fixture provisioning gap. Cleanup, actual CLI0, separate exact-container absence
and frozen input closure pass; Root independently binds169 selected files.

The service closure is compiled from `052c63c`, pre-lock core published at
`9804b02`; newer joint lock/camera runtime is outside this selection. Source
remains25ms/100ms with terminal failure and no revival. Epoch12's terminal
`NotConnected` is observed17.481079ms after the owned Stop request: ordering only,
not proven causality or vendor cleanup. All1–11 failed aggregates remain;11's
Source event has no Stop timestamp and remains unlocalized. Native Wayland buffer
equality is false and normal QWindow DPR is unobserved. Appearance is an immutable
fixture view, not a live production broker. Trusted controls, notifications/wake,
production modem/carrier and Pixel hardware remain open. See
[phone tracer](native-phone-tracer.md) and its bounded provenance.

Native lock entry now reaches the same Rust lifecycle actor through a fixed
UID1000 lock-only socket. The installed wrapper invokes `lifecycle-lock`, which
requires the actual root actor ACK within one original two-second BOOTTIME budget
before executing the exact retained Omarchy action. The action, QML, PAM and 500 ms
stabilization timer retain their original bytes. Initial ingress evidence remains
8 host groups, 6 original ARM groups and 8 components against its frozen selection.
It does not qualify secure frames, PAM, trusted unlocking or physical hardware.

A later genuine original ARM RED exposed a fresh unchanged normal Qt Messages
page after healthy actor Stop/wait and a changed compositor/display/Source epoch
using the same sealed policy directory while the original QML remained T-held.
Required `Durable.lock_pending` now revokes memory authority first, then requires
owned bounded storage file/rename/directory sync before a successful ACK. It is
restored before Ready; Source `locked:false` and root admin `locked:false` cannot
clear it. Monotonic `Gate::Lock` completion tolerates irrelevant focus revisions.
Fresh unlocked sessions require an explicit root-owned fresh storage lifecycle;
there is no unlock/reset API or old-schema compatibility. Later 6 host groups,
5 original ARM groups and 8 components pass separately; Root independently
verified 1,375 private bindings. Whole-actor SIGKILL establishes unclean-history
restart refusal, not successful crash recovery. Every failed build/runtime/component
epoch remains recorded. The latest format/Clippy and exact Cargo component ELF
pass; two dependent Cargo.lock graphs include the already selected native-camera
dependency, without camera plus new-lock merged runtime evidence. Source remains
`610aa` with its 100 ms lease, and same-Source continuation/old-compositor fresh-endpoint
reload, production install and trusted restoration remain unavailable. See
[lock admission](native-lock-admission.md), [public ingress proof](../tools/native-session-lock-admission/integration-proof.json)
and [public restart proof](../tools/native-session-lock-admission/restart-proof.json).

The native camera actor/process route now qualifies17 actual ARM Binder groups
after a genuine missing-route RED and an actual Supervisor-channel EAGAIN revoke
RED. Durable history is separate from the retained current stamp; every actual
submit/retry and final reply rechecks authority. Pre-dispatch cancellation stops
the ready worker, and local waits never imply remote cleanup. Protected markers
survive healthy restart; no implicit reopen follows. Root independently verifies
198 GREEN,80 EAGAIN RED and95 first-route bindings. This is public Rust/Rust Binder
with synthetic Root lifecycle controls and no Source/UI/vendor runtime. Installed
Source/Camera/media/containment/reset/device integration remains required. See
[service](native-camera-service.md) and [proof](evidence/native-camera-service-provenance.json).

The earlier native boot diagnostic checkpoint adds retained character1:11 kmsg
descriptors, nonblocking random execution epochs and consumed event ordinals,
exact bounded readback, and distinct post-readback stdout ACKs. A stronger actual
kernel correlation RED then unchanged-witness GREEN qualifies the startup pair;
separate serial healthy/missing-child runs and a real kmsg-disabled admission
refusal pass their intended controls. The quiet witness's child final exit/wait
remains intention-only, separate from ordinary serial child execution/wait.
The repaired1,462,272-byte unsigned candidate passes fresh two-build/original
unpack/Rust/three-frame-overlay/eight-refusal checks and executes its exact generic
LZ4 under Arch ARM7.1.6 with four ACKs and owned-CID closure. See
[current diagnostic proof](evidence/native-boot-diagnostics-provenance.json) and
[package](../native-boot-package/README.md). The simpler RAM runner establishes
only direct Docker CLI wait. A later separate exact-Google4K-kernel/QEMU run
executes that earlier generic LZ4 and its unchanged native ELFs: four ACKs, child
wait37 and QMP guest shutdown pass; Root rehashes52 bindings and checks owned
live identity plus fresh same-daemon exact-CID absence. Compiled protected-mode
arguments remain unchanged; synthetic EL2/PL011/DTB differs from Pixel hardware.
See [Google-kernel proof](evidence/native-boot-google-kernel-provenance.json).

A later [warm-reset experiment](native-boot-warm-reset.md) recovers the exact
162-byte PMSG canary and21,362-byte previous console under that unchanged kernel.
Two distinct boot/native epochs, the successful owned QMP reset, previous native
markers and second child wait37/shutdown qualify synthetic console/PMSG retention.
The first child is reset, not waited. Same-witness no-backend refusal and five
saved-output controls remain separate. Root rehashes90 positive/92 negative/78
failed-paused bindings and checks whole recovered bytes and fresh exact-CID absence.
This leaves Pixel retention/retrieval, pre-backend failure and panic records open.

Seventeen [native service prerequisites](native-google-kernel-services.md) now
pass on the same unchanged Google kernel:14 existing identity groups plus actual
distinct-UID admission/revocation, held-storage responsive Stop/reap and unclean
same-policy restart refusal. Root verifies287 bindings, seven SCM app-generation
checks, nine direct waits and six ECHILD observations. This is a RAM-only test
PID1 and standalone service, with same-boot tmpfs acknowledgement; Messages, UI,
persistent reboot recovery and trusted unlock/restoration remain separate.

Separate original AOSP AVB execution qualifies five internally self-consistent
signed metadata blocks, four expected child key/rollback-slot agreements and16
available descriptor checks;27 payload checks remain unavailable. The unsigned
compact candidate lacks parseable VBMeta and fails the stock init_boot descriptor.
Root independently reviews26 input identities,27 outer waits and42 nested
OpenSSL exit0/actual wait4 reaps. See [AVB admission](native-boot-avb.md). No
external device trust anchor, rollback state, bootloader/RAM selection, vendor
userspace, Pixel board, retained-log retrieval or usable hardware diagnostic
path is qualified. Effective phone console/RNG/kmsg policy and recovery remain
gates. A later [closed-loader static review](native-boot-loader-admission.md)
rehashes the complete factory archive and agrees with original FBPK metadata.
Its reviewed ordinary v4 path can request `init_boot` using a current-slot token
and prefer that qualifying generic ramdisk over downloaded generic data. A
separate peer checks766 actual instruction words and seven raw calls; no
contradiction is found in this conditional chain. Two tool prerequisite failures
remain failed, and runtime reachability/slot helpers/selectors remain open. The
whole vendor-boot span includes both fragments and projected helper padding; the
earlier chosen-three-frame overlay proves neither this layout nor loader selection.
See [bounded proof](evidence/native-boot-loader-provenance.json). Next: the exact
RAM request/context selecting authored init, usable phone diagnostics, and the
normal Messages service tracer on this kernel. A later
[full ramdisk result](native-boot-full-ramdisk.md) executes the complete padded
four-frame authored-generic counterfactual:512MiB guest/1GiB container reaches
native init, four ACKs, child wait37 and QMP shutdown. Its original256MiB run has
actual unpack write error/fallback ENOSPC, partial vendor init/module loading and
scheduler panic; it remains failed. The memory-only successor preserves all
input bytes/flags/CPU/PID/deadline limits. This is not a realized loader request.

The current normal Messages service now also passes three
[Google-kernel history groups](evidence/native-google-kernel-messages-history-provenance.json):
two equal locally seeded PDUs retain distinct exact IDs, project before the first
app launcher, read through current authority, deny after revocation with the
reader T-held, and reopen the same journal with fresh authority. Root checks226
bindings,33 response SCM generations, five direct waits and six worker
pidfd-dead/proc-absent observations. No independent native-worker wait trace,
live receiving, normal Qt or reboot/power-loss persistence is claimed. The
fixture adds actual random/urandom nodes after the retained UUID ENOENT failure;
full native init must provision them.

Three later [current-source Binder groups](native-google-kernel-services.md#later-current-source-binder-composition)
pass on the same kernel with the existing synthetic V3 receiver. Two equal
callbacks create distinct raw IDs and project before the first app launcher;
a third commits while the actual reader is T-held before its query. Revocation
denies that read; release projects all three through the same writer/source.
Clean same-generation reopen preserves IDs but stays history-only with fresh
authority and no new registration. The record separates 16 direct PID1 waits
(14 exit 0, two intentional SIGKILLs) from six service-worker pidfd-dead/proc-absent
observations; final wait-all returns ECHILD. No independent worker-wait trace is
claimed. The fixture lock adds only the current local camera dependency; 165
registry tuples stay unchanged. The prior locked-build and offline metadata
failures remain setup failures, with no VM. See [bounded proof](evidence/native-google-kernel-messages-binder-provenance.json).
Normal production incoming, Qt/session, persistent reboot recovery and physical
phone gates remain open.

The subsequent trusted lock→PAM
unlock→reopen transaction must preserve policy/history, establish current Source
and app authority, and never infer recovery from unlocked facts or deleted seals. No physical phone is accessed.

The original native early boot has a separate RAM-only checkpoint. A genuine
missing-`/init` ARM RED precedes the Rust root-PID1 probe. Native child actual wait
and controlled poweroff pass, and a missing-helper failure powers down separately.
The non-Pixel Arch ARM7.1.6 kernel is unchanged; no root disk, NIC, vendor service
or phone is attached. See [boot probe](native-phone-boot-probe.md) and
[proof](evidence/native-boot-probe-provenance.json). Full signal/stuck-child/device
IO, Pixel module/diagnostic/thermal/watchdog behavior and actual phone boot remain
unqualified. The first restricted Pixel candidate is independent of full IMS/UI.

Later unchanged-init ARM controls qualify held-child failure/powerdown, exit38
actual wait/rejection and a real `CAP_SYS_BOOT`-denied shutdown producing EPERM
and visible PID1 panic. Root verifies44 private bindings plus six unchanged
baseline sources. The failed shutdown consumer expectation is retained; corrected
classification uses the same frozen console. Held cleanup is source-bound
inference without independent syscall tracing, and uninterruptible tasks/full
signal policy remain open. These authored control ramdisks do not inherit Pixel
candidate evidence. No probe change was needed. See
[failure proof](evidence/native-boot-probe-failure-provenance.json).

The historical first candidate was built offline: a reproducible1,454,080-byte
unsigned v4 `init_boot` owns generic `/init` with the exact native ELF. Pinned AOSP
unpacking, Rust geometry, independent actual vendor-fragment overlay and eight
input/output refusal groups pass. Its exact extracted generic LZ4 also executes
native PID1/child wait37/poweroff under Arch ARM7.1.6 in1.987s. A hypothesis that
missing startup device nodes would abort init was falsified; the original
consumer's failed RED expectation/exit1 remain preserved. No runtime defect or
node repair is claimed. Public tooling and clean metadata are in
[native boot package](../native-boot-package/README.md). Candidate images and
original inputs remain private. That earlier epoch does not qualify the Google
kernel; later Google-kernel/QEMU evidence is recorded above. Pixel board startup,
vendor-overlay runtime, AVB, bootloader selection, temporary multi-partition boot
and diagnostic/watchdog/thermal behavior remain unqualified. Full deployment stays refused.

The full original frankelCP3A.261005.005 factory archive now matches Google's
published SHA256, with38 index reads rebound to the authenticated stream and14
selected boot/module inputs preserved privately. The new bounded Rust v4
inspector passes27 tests on each x86/ARM64 user-QEMU target and seven exact-byte
comparisons with pinned unchanged AOSP tools. Actual Google4K kernel/config,
separate16K inputs, generic `/init`, vendor ramdisk fragments and DTBs are static
metadata evidence. They do not qualify AVB, kernel/module KMI or hardware.
DEVTMPFS and PID namespaces are disabled in the selected kernels; tmpfs/proc/sysfs
are available. See [input evidence](../native-boot-inputs/README.md).
The live deployment profile now pins the authenticated4K kernel payload and
observed base release; complete module/firmware pins and all nine offline plus
one physical requirement remain unresolved. Full-device admission stays refused.

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
1080 ×2424 at422PPI with matched logical density420. The actual wrong-density RED
and a separate compiler prerequisite failure remain preserved. Fresh GREEN3 passes
three actual ARM groups: matching public physical-size API propagation, native
1080 ×2424 SHM/PNG pixel equality without resizing, and normal unload restoration.
Root independently verifies93 private bindings; five guest children and the
container are actually waited. The mapped measurement surface qualifies this
profile/capture boundary, not a phone-app/Source/PAM preview. Exact authored tools
and reusable private ELF pins are linked in the report. Do not resume historical
320/800 preview geometry or resize an image to qualify.

The native raw-response publication gate, absolute suspend-aware session expiry,
owned registration worker/death watch, and local process reclamation are implemented.
They are not native IMS semantics or a complete phone service.

The next original radio-runtime precursor now has public-only containment
evidence: a genuine unchanged-supervisor outside-root/Unix-IPC RED precedes six
actual Arch ARM7.1.6 groups under an isolated authored owner. Final credentials,
namespaces, restricted access, resource/task limits, second-exec fencing, late
readiness and parent death pass with actual child/thread waits and empty cgroups.
Maximum recorded status/stop is2.413108ms. Root independently verifies289
bindings; bootstrap and wrapper-closure failures remain failed. Production
radio code is unchanged. Original linker/Bionic/vendor loading, namespace/maps,
property behavior, registration and cleanup/reset remain separate boundaries.
See [tools](../tools/native-radio-load-only/README.md) and
[proof](evidence/native-radio-containment-provenance.json).

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

Continue the existing Messages lock→restore tracer from the qualified downstream
actor/storage path. Continue the remaining wrong-event/attempt-deadline and
uncertain-clear-expiry combinations plus full-actor unclean reopen; the current
six service cases now cover forced/expired Source, new-lock late completion and
held-clear Stop/storage reopen refusal. Real retained history now composes behind
the private test auth/Source; continue that path into the normal installed Qt app
and original compositor. Keep the real-history-specific stress and early-failure
gates separate. Keep authentication construction/ingress test-only until genuine
whole-owner completion provenance and trusted locker handoff are independently
qualified. Do not substitute models or ARM harness passes for authentication.
Preserve the queued ARM setup failure without relaxing its predicate. Complete
real authentication binding and normal Messages reopening before broadening the
smartphone matrix.

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
