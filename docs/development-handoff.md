# Development handoff

## Immediate integration order

Finish the updated Sounds/Permissions joined VM acceptance when its unchanged
resource admission passes. The held-Status Play change has component and normal
host-build evidence; it has not yet passed the joined graphical/PCM run.

Keep the current boot diagnostic frozen. The selected future development-partition
experiment does not require a temporary RAM override. Its artifact format and
restoration inputs have offline evidence; actual slot policy, loader acceptance
and fragment selection need later authorized device evidence. Production native
rootfs/service supervision is still a separate implementation gap, not proven by
the diagnostic PID1. See [deployment scope](native-deployment.md) and
[development bundle gates](native-boot-development-bundle.md).

The next service integration is persistent wireless ownership in the existing
connectivity worker. It currently constructs only `VirtualLink`; the separate
association and `ExistingIpv4Profile` operations do not yet own one combined
lifetime. A controlled backend should retain that lifetime across app closure,
then remove its profile before explicit Disconnect, preserving original deadlines
and terminal uncertainty. Reuse the existing Main/actor boundary and hwsim
consumer. Open association and the fixed test IPv4 profile are tracer inputs;
WPA credentials, DHCP/DNS, matched Pixel driver startup and hardware remain open.
See [address/packet scope](native-wireless-address-packets.md) and the
[unexecuted persistent-service acceptance contract](../tools/native-wireless-service/README.md).
The current veth Fact requires a witnessed lifetime; existing wireless identity
must be represented separately. One-shot association closes its event sockets,
so a persistent backend must retain radio observation rather than renew readiness
from IPv4 alone. Read-only observation helpers now reuse the existing kernel validators; the
process-driving consumer and real wireless service RED remain unimplemented.

## Goal and constraints

Current permission work: [explicit installed app roles](native-permission-controller.md)
pass the same five protected-profile RED→GREEN consumers. This is configuration
only. The subsequent owner selection passes protected configuration and actual
per-app display-helper ownership/cleanup tests. Scoped credentialed requests now use
the existing durable writer, with unsent cancellation and retained submitted-write
ownership. The 16 targeted credentialed/service-boundary tests pass, followed by
138 lifecycle, 35 owner configuration and 20 display checks plus strict Clippy.
These use authored Source/media facts and do not run the full actor loop. See
[ingress evidence](evidence/native-permission-ingress.json), including preserved
fixture failures and the corrected intermediate RED classifier. Desktop launch,
the Rust request/state client and Allow/Revoke UI are now implemented. Separate
actor launch tests, 12 credentialed client scenarios and real unshown QML button
clicks pass; the full genuine Omarchy flow remains open. See
[Permissions UI evidence](native-permissions-ui.md). The first joined VM attempt
reaches the correct menu item but observes only a brief UID11045 actor child,
without a captured normal executable or window. No consent/PCM test runs; cleanup
passes. A separate exact-ELF guest diagnostic now reproduces exit127 before main:
Qt metadata copy relocations are rejected by the guest library. The two expected
64/65 refusal controls fail with identical loader output; complete cleanup passes.
Explicit PIC compilation now fixes this: the compiled-ELF contract goes RED→GREEN,
normal build/QML clicks/staging pass, and the identical guest consumer returns64/65
with complete shutdown and input preservation. Retry the normal Omarchy flow with
fixed ELF3b73a5e5…. The second actual run now observes its normal window and real
Source focus, but the first unavailable frame trips an exact-RGB text observer.
No Set/PCM run follows; complete cleanup passes. The corrected observer now passes
independent real-QML enabled/disabled rendering controls. The third normal VM run,
using maximized Permissions to preserve bar access, observes one actual Allow,
the persisted Sounds/media_route grant and confirmed controls. The bar click
reopens the menu over Permissions, but menu-image recognition exhausts its eight
frames before Apps/Sounds selection. No PCM, Revoke or normal app-close acceptance
runs; guest/QEMU/container cleanup and unchanged inputs pass. Preserve this partial
result separately. A bordered-card observer correction now passes retained-frame
RED→GREEN and six refusing controls. The fourth normal VM run repeats Allow and
completes bar→Apps→Sounds launch/focus, then fails visible-control readiness:
normal desktop tiling splits the two windows vertically and clips Sounds Play/Stop.
No PCM/Revoke runs; full guest/QEMU/container cleanup and unchanged inputs pass.
The fifth VM selects the scoped phone-window profile and passes the guest
Allow→Play→Revoke→quiet→reallow/refocus→quiet sequence with all original audio
bounds. Both app classes retain actual maximized internal/client mode1 and
369×855 logical windows below the bar across focus changes and close-return.
Both apps close normally; guest/boot results pass, QEMU0 and exact container
absence pass, inputs remain unchanged. The external viewer completes16 actions
but fails its final image read with unexpected server message; the outer result
therefore remains failed. The viewer now reports the numeric unsupported message
type while retaining refusal and leaving its payload unread; three actual socket
controls pass RED→GREEN and the original nine parser controls pass. The old VM
message type remains unknown. A sixth integrated run repeats guest/PCM success
and now records actual type3 before the final frame; the outer result stays failed.
The viewer now discards bounded ServerCutText (max8 messages,4096 bytes each)
without applying/logging contents, with eight socket RED→GREEN cases and the
original parser/refusal checks passing. An actual x11vnc/Xvfb authored-cutbuffer
pair now passes RED→GREEN, full raw frame/EOF and exact owned cleanup. The original
VM payload/origin remain unknown. The seventh integrated run stops earlier at
measure-tone: the observer exhausts its original 64 graph observations without
observing a playback producer. Both recorded outputs are silent; no app wire
request or refusal cause was captured. The actual Play touch does not establish
that Play was submitted. Guest/boot cleanup and input preservation pass. This
run never reaches the repaired final-frame boundary. The separate held-Status interaction now passes unchanged-consumer C ABI and
actual periodic Qt socket RED→GREEN. Rust accepts one copied Play intent, then
requires final fresh eligibility before its first send; 18 preview groups and
35 socket cases pass. This is a deliberate UX contract change, not a proven
cause of the VM failure. The normal host Qt6.8.2 app build and argument64/UID65 refusals now pass.
Build it against guest Qt6.12 and qualify its joined run next; see [held-Status evidence](evidence/native-sounds-held-status.json). Do not relabel earlier runs as
fully passing. New QWindow density evidence,
public screenshots, ARM and physical-phone qualification remain open. Ordinary current profiles require `role.kind=application`; frozen
previous cohorts retain their matching old binary/profile pairs.

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

The [genuine app lifecycle](native-omarchy-app-lifecycle.md) now passes cold menu
launch → Super+W close → fresh menu reopen → normal manager shutdown. The only
configuration change is the standard close binding; the matched consumer has a
real missing-binding RED. Both actual app windows pass Pixel 10/focus and 48,000
silent PCM frames each with permission withheld. The earlier host-readability
and post-registry Source-readiness failures remain separate. This is guest uinput,
not viewer input. A separate paused host-only Unix RFB/frame/closure probe passes.
The later [full viewer continuation](native-omarchy-viewer.md) now passes all seven
RFB actions, actual close/reopen, both native Pixel/quiet-PCM checks, final raw
frame receipt and normal shutdown. An owned initial host-window setup corrects
the no-WM SDL geometry; the decoder accepts RFC-permitted padding with all bounds
retained. Earlier failed runs remain separate. That frozen cohort retains the
capture wrapper; human/SSH connection and consent remain open.
The later [normal-entry external-client checkpoint](native-omarchy-human-preview.md)
now passes direct native Sounds launch with the corrected Rust pair, frame receipt,
client disconnect and complete VM shutdown. The two real viewer timeout/disconnect
controls also pass separately. The [operator launch command](try-harbour-preview.md)
now passes fresh preparation, readiness notification and a full VM/client run.
Human SSH/VNC access remains unverified. A separate connected-client expiry check
passes at a requested two seconds; five-minute endurance remains untested. No fresh QWindow
metadata or screenshot qualification is inherited from the capture-wrapper cohort.

The earlier [menu integration](native-omarchy-menu-launch.md) closes actual
bar→Apps→Sounds RED→GREEN in genuine Omarchy. The new actor-owned UID 11042 app
passes native Pixel/window/focus and 48,000 no-grant silent PCM frames. The later
audio continuation still fails before tone.json; no preview diagnostic lines
identify dispatch or startup, so do not attribute this failure to deadline clamping.
The 255-file candidate uses an explicit three-row menu overlay and retains the
original consumer deadlines/criteria. The full run and composite manager
Stop-after-Ready remain failed; independent custody and guest closure pass.

Prioritize a reproducible interactive VM preview with the now-qualified
menu/close/reopen and normal shutdown, clearly labeling playback experimental. Keep
the matched corrected audio Main/session-owner pair in new previews, preserving
the frozen viewer cohort. After the operator workflow, implement the visible
Sounds Allow/Revoke page. The private
controlled Qt 6.8.2 startup fixture now qualifies an unpaused control and a held-Source
RED against unchanged production audio code. Main expires Starting at captured D
while Source remains fresh and the original app budget remains valid; the intended
final native assertion fails after prerequisites, with full original cleanup and
unchanged inputs. Earlier setup/observer failures and a separate outer-runner cleanup
failure remain preserved, distinct from that qualified RED. The narrow Rust [D/A separation](native-audio-startup-deadline.md) now passes the
same held-Source positive oracle and existing Source-loss/app-expiry controls, with
full closure and no replay. Formatting, strict Clippy and selected host component
checks pass: 56 audio, 117 lifecycle and 34 owner cases. Matched normal release
binaries are built; graphical deployment and playback remain separate gates.
This establishes the controlled startup failure mechanism, not the cause of earlier
unrelated graphical audio failures. Preserve the existing background promotion
policy described below. Do not expand this into
another unbounded fixture campaign or treat the VM as phone qualification.

The persistent connectivity worker now [checks readiness before starting its
receive deadline](native-connectivity-idle-receive.md). An actual empty-channel
delay reproduces worker retirement before the change and preserves the same live
worker afterward. The identical consumed-command and parent-death controls still
reject late work and close the owned processes. The qualified host pair keeps
the same Main and consumer and changes only the worker binary. Thirty active
component tests, formatting, strict Clippy and normal host/ARM builds pass; four
unrelated isolated tests remain ignored. Earlier failed fixture cohorts remain
failed. Main's receive path is unchanged; persistent ARM execution and Settings
integration still require their own evidence. The prior one-shot ARM packet
trace does not qualify this persistent idle path.

The current [Source timing recovery](native-session-timing-recovery.md) replaces
terminal timing expiry with immediate authority withdrawal and fresh-pair
reestablishment in the same owned Source. The 100 ms budget is unchanged;
identity, wire, protocol and death failures remain terminal. Old reads,
authentication attempts and finite previews stay retired, and durable lock and
permission policy survives. The existing late durable-write/lifecycle mismatch
still permanently invalidates the actor.

Actor checks pass 117 cases with 26 unrelated isolated cases ignored; the two
original timing REDs also pass unchanged. Receiver checks pass the original RED,
nine new recovery controls and the process/establishment targets, whose shared
cases are counted separately. SourceOwner passes 34 process/status/startup/Stop
cases and a normal feature-free x86 release build. Formatting and strict Clippy
pass for those packages. Failed setup and test-migration cohorts remain recorded.
A subsequent genuine Omarchy x86 KVM run qualifies pre-Play recovery: after
one controlled compositor pause, fresh Source state returns 52.257 ms after
resume with the same retained compositor, owner, Sounds process and mapped window.
The first 1080 × 2424 / DPR 2.625 boundary and quiet PCM check pass. That whole
run nevertheless fails during later Play: only a 1,024-frame authored PCM prefix
appears before Starting-state retirement. Its exact guard and cause remain
unidentified; Source continuity around that retirement is unestablished.

A fresh diagnostic-only repetition now passes recovery, the unchanged 50% PCM
and operator-revoke silence checks, and full manager-driven closure. The same
window survives; fresh Source returns 54.852 ms after resume. Actual attributed
logs confirm the app requests about one second, but the captured Source cutoff
leaves only 55.015 ms at Main startup admission. This passing run identifies the
clamp, not the cause of the earlier failure or a behavioral repair. Normal
diagnostic owner/audio builds pass 151 and 56 checks, formatting and strict
Clippy. See [startup diagnostics](native-preview-startup-diagnostics.md).

Next establish a controlled real-boundary RED with continuously fresh Source and
briefly held virtual audio startup. Preserve the existing successful Starting-reply
publication → approved background lease policy; do not require foreground until
Playing. A prospective split must retain the original Source cutoff for dispatch
and publication, and the original app/TTL ceiling for admitted startup, without
renewal or replay. ARM/phone recovery and interactive use remain open; the separate
menu/app boundary above now passes.

The shared process owner now [retains stdout through actual EOF after child wait](native-process-custody.md#later-stdout-eof-custody-repair). A real held-writer RED passes after the repair, alongside nine existing Supervisor controls and strict Clippy. Local output closure remains separate from remote cleanup and successor authority. Earlier consumer cohorts are historical and were not rerun.

The [registered app connectivity path](native-connectivity-actor.md) now
passes a durable permission grant through the existing lifecycle actor to the
normal Rust Main and real packets. The connection survives actual app exit; a
fresh generation observes it and explicitly Disconnects with independent address
removal and silence. A genuine refusing-actor RED precedes the implementation;
114 default lifecycle tests, six normal binaries and strict Clippy pass. This
uses an actual UID11050 Rust process fixture. That earlier cohort does not
execute the normal Settings engine/window; installed selection, ARM,
Wi-Fi/Internet and all phone gates remain open.
Earlier [private Main](native-connectivity-private-ingress.md) and
[persistent-worker](native-connectivity-worker.md) cohorts remain separate.

The [normal Settings window](native-settings-connectivity-window.md) now
passes real touch Enable, close/relaunch with the connection preserved, and
explicit Disable through the Rust actor/Main/native worker. Two independent
packet round trips, address removal and quiet pass in a generic x86 VM. Original
1080×2424 screenshots show the ready control and the reopened enabled state.
This uses authored session authority and the normal appearance fallback; full
installed runtime, ARM, Wi-Fi/Internet and every physical-phone gate remain open.

The normal window joins the existing per-app display owner and source-built
upstream compositor to the actor/Main packet oracle. Eight preceding staged or
failed attempts retain their exact classifications; Sol9 passes after bounded
visible-control readiness. First app retirement preserves the connection, and a
fresh generation explicitly disables it. No production repair is attributed to
the earlier unlocalized touch failure. The installed Config component remains a
separate pass. Current native Rust Main/worker, normal app helper and the
13-API Settings archive now cross-compile for ARM64; the exact test-only 32s
observation bridge compiles
separately with unchanged production outputs. Native Qt6.12 CMake/install,
current loaders and the exact three production app files now pass in a separate
generic ARM preparation. The first missing-bootentry setup failure is preserved;
the corrected cohort has 63 direct waits/stdout EOFs and exports its layer after
actual QEMU closure; no normal app/compositor/backend executes during preparation.
Six later ARM window attempts remain failed. The first two stop at an existing
appearance endpoint and a late denial child. The third passes the minimal denial
probe under its original 1s bound but fails the retained-worker guard. Its mapped
window is sampled after cancellation; mapping-versus-guard ordering and the
worker-loss cause are unproven. Its later unavailable observation is not a
pre-cancel cause. The second's custody-timeliness failure remains false despite
eventual cleanup.

The fourth diagnostic attempt reaches one actual Enable touch, matching Completed,
independent `192.0.2.1/30` and one nonce packet before guard failure. Its original
Enable-ready ARM frame is published as a partial failed trace. Before cancel,
Main is live, its child vector is empty and the retained worker pidfd is ready;
final actual worker exit is 1 with stderr `TimedOut`, exact operation unknown.
The first app wait 0 is observed after cancel, so background persistence, fresh
reopen and Disable remain unqualified. Literal controller `SETUP_FAILURE` and
functional desired=false remain; controller custody qualifies while guest-case
closure does not, despite eventual ECHILD/one adopted exit 0. All 51 scope waits/
PIPE EOFs, QEMU/Xvfb 0, container 1/removal and unchanged inputs are recorded
separately. No production RED/GREEN or new live theme-provider claim follows.
See the [ARM window failures](native-settings-connectivity-window.md#arm-window-attempts-remain-failed).

A focused [worker error-context checkpoint](native-connectivity-worker-error-context.md)
now passes a genuine normal-CLI missing-context RED→GREEN with the same consumer.
Actual UID1000/cap0/NNP1 capability reduction returns original EPERM and natural
worker exit1; GREEN retains its fixed `ReduceCapabilities` stage, no call cutoff
yet, and an error-only BOOTTIME sample inside the actual spawn/wait span. Both
controls close one actual wait/PIPE EOF under the original1s cutoff. Baseline QA
passes35 active host tests; candidate QA reruns10 connectivity tests with3 ignored,
while unchanged shared QA remains baseline evidence. Four normal host/ARM Release
ELFs build without executing during either build. No wire, guard, cutoff, retry
or retirement changes. A separate fifth actual ARM attempt now observes
`ReceiveCommand`/`TimedOut`: detection is 26.972719 ms past the recorded cutoff,
with a derived 76.972719 ms envelope-origin span. This supplies no syscall-duration,
pre/post-guard or CPU-throttling cause, and no stage attribution for attempts
three/four. Whole-old-image verification and one current-worker install/loader
pass before the same partial Enable/Completed/IP/first-packet behavior. The final
worker wait is signal 9/code null; first app wait 0 follows cancel. Main 0/driver
101, controller custody true and guest closure false remain distinct, despite
eventual ECHILD/one adopted exit 0. All 53 scope waits/PIPE EOFs, QEMU/Xvfb 0 and
container 1/removal/absence are recorded with unchanged inputs. Full background,
reopen and Disable remain failed. Executed resources are guest 6 CPUs/6144 MiB,
outer 8 CPUs/10 GiB.

The separate sixth 16-guest/16-outer-CPU cohort closes as another failed case,
with guest 6144 MiB/outer 10 GiB and unchanged binaries, deadlines and guards.
It fails before Enable-ready, capture, touch or packet qualification; mapping is
sampled after cancel. `ReceiveCommand`/`TimedOut` records original cutoff
`42391806186` and detection `42436693683` BOOTTIME ns: 44.887497 ms late and a
derived 94.887497 ms span from the 50 ms envelope origin. No syscall, pre/post
guard, scheduling or CPU-throttling cause is qualified. The non-atomic pre-cancel
sample has an empty child vector, ready worker pidfd and Main live. Final actual
waits record worker natural exit 1, Main 0/driver 101 and first app signal 9/code
null after cancel. Controller custody remains true and guest closure false,
despite eventual ECHILD/one adopted exit 0. All 52 waits/PIPE EOFs comprise
13 host, four boot, 29 guest and six controller observations. QEMU/Xvfb 0,
container 1/removal/absence on the same daemon and independently rehashed
unchanged inputs remain separate. No production behavior RED/GREEN, earlier
cause or CPU repair is claimed. Repeated graphical ARM resource campaigns are
parked with the production 50 ms guard unchanged. See the
[sixth failed attempt](native-settings-connectivity-window.md#sixth-attempt-cpu16-receivecommand-failure).

A separate [disabled connectivity bundle](../native-connectivity-control/packaging/README.md)
now passes actual Make/installer staging of exactly two current ARM provider
files while preserving the existing Disabled config. All eight filesystem
installer tests pass. This executes no provider and adds no installed activation;
trusted namespace provisioning and runtime selection remain open. A new
behavioral tracer now passes native Rust nl80211 discovery of the exact fresh
hostapd AP in a generic ARM hwsim guest after genuine refusing RED. The operation
takes 47ms, with actual worker wait/EOF at 102ms under the unchanged 5s cutoff;
full AP/endpoint/radio/module/process cleanup passes. 13 host tests, strict QA and
normal host/ARM builds pass. The exact same completing ELF also passes a separate fresh-guest loopback
unavailable control within 156ms; cfg80211/rfkill inventory additions are
recorded, without a zero-effects claim.
See [wireless discovery](native-wireless-scan.md). Subsequent explicit association
and Disconnect pass separately in [the ARM connection tracer](native-wireless-association.md).
The [fixed-address packet tracer](native-wireless-address-packets.md) now adds
normal Rust Up, two independent nonce ACKs, Down and quiet in separate AP/STA
namespaces, with complete closure. Internet, normal Settings Wi-Fi and matched
Pixel wireless remain unqualified.
The [retained Pixel WLAN inventory](native-pixel-wireless-boundary.md) now binds
existing config/index/catalog facts to the authenticated selected profile:
external cfg80211 and bcmdhd4383/bcmdhd4390 candidates. Exact hardware selection,
producing source/KMI, firmware and initialization remain open. No new extraction,
module loading or phone execution occurs; the generic scan is not promoted.
Keep Source/authentication/full-session Main
and broader capture campaigns parked; authored session=None/admin facts remain
explicit. Worker reaping is separate from remote cleanup; no implicit mutation
retry or same-namespace recovery is allowed.

The [normal Settings Rust connectivity SDK](native-settings-connectivity-sdk.md)
now has genuine actual-UID refusing-setter RED through this same actor/Main/packet
boundary. Its effect implementation retains a readonly-established transport,
original absolute deadline and one send, with permanent queued retirement and
sticky uncertainty. The unchanged consumer now passes actual packets, app exit/relaunch and
explicit Disconnect; six real-socket protocol groups and the original seven/five
pending brightness groups pass too. The green epoch's outer field-name error
remains failed; offline schema-corrected review validates its original product,
QA and complete custody without rerunning. That SDK cohort supplies no Qt control
or installed selection. Preserve the published actor checkpoint separately.

The [connectivity C ABI](native-settings-connectivity-cabi.md) now passes all six
unchanged socket groups after three qualified refusing-setter behavioral REDs.
The setter delegates the untouched absolute deadline and explicit boolean to the
Rust engine. Formatting, strict Clippy and the normal Release archive pass; a
separate UID11050 C++ consumer uses the real header and exact archive to qualify
unavailable/refusal and actual Stop/Join/Free. The earlier format-only setup
failure remains recorded. This is separate from the published safe SDK
checkpoint141c622. The later [Qt controls](native-settings-connectivity-qt.md) now
pass actual enabled-button input, one exact-deadline Set, fresh Ready/Enabled
state and asynchronous owned close through the same Release archive. The genuine
missing-Set RED precedes effect wiring; three setup failures and one unexecuted
predecessor remain separate. Normal CMake builds/loads four selected targets and
two existing unavailable components execute. This is native unshown Qt6.8.2,
not the older Qt6.12 or ARM/window cohorts. Next reuse the existing packet oracle
for ordinary app ownership and explicit installed session-owner/Main selection;
normal Main runtime, window and deployment remain open.

The later [installed connectivity selection](native-connectivity-installed-selection.md)
requires explicit Disabled/Owner and exactly one UID11050 Settings role if either
brightness or connectivity selects an owner. A real filesystem RED rejects all
five explicit selections and accepts omission before the repair. All four new
groups and27 existing filesystem tests now pass, as do formatting, strict
all-target Clippy and the feature-empty normal Release owner build. The preserved
first GREEN attempt fails only on two test lints; those corrections change no
assertion. The actual Source/projector case is filtered. This qualifies Config
and compilation, not provider execution, installed runtime, ARM or phone behavior.

The [native local-network status path](native-network-status.md) now passes both
host and native ARM service-to-Rust-client-to-Settings integration. In the ARM VM,
PID1 starts the installed service automatically; the same unprivileged QML
instance shows0/0, clears stale counts after Stop and recovers through a fresh
service instance. Both Stops finish normally. The native Qt6.12 build installs
Settings with its three matching Rust engines. Counts establish no Wi-Fi,
Internet or modem connectivity. The [combined headless development image](native-arm-development-image.md)
is qualified separately; normal app main/window and physical-phone gates remain
open. No new screenshot is claimed.

The [current native ARM Settings/Messages build](native-current-arm-apps.md#current-settings-and-messages-with-battery)
passes production CMake/install and offscreen components for its frozen
pre-network-card Settings600 and three-engine Messages frontend. All64 frozen sources and four
archives match; new normal ELFs59fb0ef1/c7bb89e2 remain unexecuted. The56 Scope
waits/merged EOFs and seven nested waits are distinct from the legacy shutdown
request, which has no direct QEMU/QMP receipt. No window or default ARM Cargo is qualified by that build; the later headless
combined image is qualified separately. Older6f9/c52 app results remain separate.

The [prepared three-app navigation branch](native-three-app-navigation.md) is
parked. The first epoch fails before a functional group; the second and third
complete three groups, then fail Sounds metadata/capture. The third's actual
class check passes but capture exits6; the exact capture cause is unknown. No
Sounds frame, complete five-group join or return-to-Settings75% result follows.
Preserve all failed aggregates and the third's theme-host255 cleanup error.
Source100ms, warmup, authority and cutoffs are unchanged. Do not retry this branch
or resume Source/authentication/capture investigations.

The [combined ARM development image](native-arm-development-image.md) now
cold-boots seven installed services together: battery, brightness, network status
and the four-unit audio stack, with Settings and Sounds installed. One fresh boot
passes network and power recovery, then real QML input and measured half-volume,
Stop and muted audio preview. Unrelated services retain their identities through
local Stops; all seven finish with normal shutdown. The393 package versions,
50 selected Qt files, site configs and unrelated data stay unchanged. This is a
headless generic ARM checkpoint; normal app windows, the complete phone session
and physical-phone gates remain open. The earlier three-service reinstall and
two-boot evidence remains separate.
Preparation uses the existing production Python installer for fourteen rows,
reusing the independently qualified normal Sounds build from a failed earlier
preparation. Its58 waits/merged EOFs are separate from the fresh cold boot's187
direct waits and201 PIPE EOFs. Audio Stop actually observes a nonterminal sample
then terminal state under its one original cutoff; the outer checks reuse that
cutoff. Failed setup epochs remain failed. No new normal Main/window, Source,
graphical capture, Google-kernel or physical-phone result follows.

The earlier full-level [finite Play/Stop Sounds preview](native-sounds-preview.md)
passes one registered/granted host actor Play through the normal Rust-owned producer to
original private PipeWire: 24,000 frames per sink contain 440 Hz speaker content
and exactly zero earpiece samples. Earlier zero-PCM runs and the fixed
`unsupported_channel_layout` diagnostic remain preserved; creation-time producer
port initialization is the sole behavior fix. Host Rust and frontend C ABI/socket/
unshown-QML components pass separately. A separate same-session actor case now
passes explicit Stop, actual producer retirement and graph absence, full paired
quiet, then one new explicit Play with a distinct producer. All 144,000 measured
frames and actual owned closure pass without a production change. Separate cases
now pass natural completion (96,000 measured frames, same-FD `completed`, a new
original eight-second case cutoff) and permission revocation/regrant/one explicit
Reconnect/new Play (192,000 frames under one original six-second cutoff).
Regrant/reconnect alone stays quiet. A separate native app-close/relaunch case now
passes 192,000 measured frames under one original six-second cutoff: old app and
producer retire, a distinct same-profile app retains permission but starts
unavailable, and one explicit Reconnect/full quiet precedes a new Play. Numeric
app ExitStatus and literal actor generation remain unmeasured. Backend-worker
death/producer retirement and full quiet also pass: one privileged reconcile
restores availability with preview `failed`/`applied:null`; only a new Play starts
sound on the same app FD. A later unshown real QML consumer now passes four real
route/Play/Stop/new-Play mouse pairs, public QML Playing→Retiring→Idle state and
144,000 measured frames across three paired windows under one original six-second
cutoff. Current Rust client/actor/normal audio providers remain unchanged; only
test C++ discovery/layout preparation changes. The specified host preview gates
now pass. After the separate current Rust ARM crossbuild, generic Arch ARM
preparation now passes all five Rust loaders and native Qt6.12 consumer build/
link/loader checks (15 Sounds and 11 appearance APIs). The old active/available
PID1 daemon fails the required-field assertion with exit 101. All four old units
then Stop successfully before explicit four-file installation and export after
QEMU closure. The first cold ARM aggregate fails whole-service terminal
verification despite schema/QML/PCM passes; its final assertion never runs. A
second cold boot now passes all four fresh PID1 invocations, schema, four real
QML clicks and 144,000 PCM frames under PipeWire 1.6.9. One target Stop has two
independent qualified terminal records under the same original cutoff. Each
records one observation, so the new repeat-poll branch is unexercised. The final
compiled QML assertion exits 0 after cleanup, then QEMU exit 0/closure pass.
No production change or causal repair is claimed. The [repository test entry](../tools/native-audio-preview/)
now passes on the host with all three fresh outputs of its passing public
Cargo/CMake builds: four real clicks and 144,000 PCM frames under one original
six-second cutoff, final assertion 0 after cleanup. The earlier prebuilt entry
retains its own mapping. The added C++ belongs to the test fixture; production
policy remains Rust. Those full-level host and installed ARM cohorts retain
their original providers.

The qualified next-preview level cohort is Rust-owned, local and atomic; default 100%/unmuted,
no choice IPC, and each Play freezes its pair and original deadline. Actor and
Main require typed level/mute fields and scale only that preview's fixed PCM.
The frozen valid50 request progresses from actual denied/48,000 quiet-frame RED
to measured half-amplitude GREEN. Current native QA passes 46 audio tests (one
existing ignored), 11 authority and seven input tests, fmt and strict Clippy.
The 16-API/5616-byte-View frontend passes Rust checks and all seven Qt targets,
including 31 Qt behaviors and unshown level/mute bindings. The current public
entry passes six real mouse pairs and 144,000 PCM frames under one original six-
second cutoff: half-level tone, actual Stop/retirement and full quiet, then muted
Play with a distinct live attributed producer and active link. Its final native
assertion exits 0 after cleanup. Failed check/build preparations remain in
[hash-only level provenance](evidence/native-preview-level-provenance.json).
The current Rust AArch64 crossbuild now passes all five ELFs and the 452-member/
16-API Sounds archive. Installed-layer preparation also passes all five loaders,
native Qt6.12 consumer build/link/load and four normal PID1 Stops before update,
with the 393 package versions and 50 selected Qt files unchanged. QEMU closes before
export. One fresh cold installed level/mute tracer now passes four new PID1
invocations, six real QML clicks and 144,000 PCM frames: half-level tone, Stop/
retirement/full quiet, then a distinct live muted producer/active link. Its upfront
ARM 12-second aggregate and six-second windows remain separate from host bounds.
Final assertion 0 follows controller/parent cleanup; normal Stop and QEMU closure
pass, as does independent review. Both terminal verification records need one
observation, leaving repeat polling unexercised. This preview-level campaign is
complete; do not expand host/ARM combinations or resume a window/capture campaign.
Normal-window, Pixel integration, capture and physical-audio gates remain open.

[Live preview mute](native-live-preview-mute.md) is now implemented across the Rust
feeder, owner, actor and app engine. Five real QML clicks now produce one Play,
Mute and Unmute with the same live producer: tone→full quiet→tone, 144,000 measured
frames within 2.322333488 seconds of the original four-second asset. A real failure
from buffered tone precedes the bound on queued PCM; clocks and oracles stay fixed.
55 audio tests, 31 focused lifecycle tests, the 17-API frontend and 32 Qt behaviors pass.
This is a host unshown-component result; ARM live control, normal windows, physical
audio and all phone gates remain open. No phone access is authorized.

The earlier private read-provider compiled assertion RED remains historical.
The current [private Messages owner](native-messages-private-provider.md) now
passes a separate actual normal-daemon RED→GREEN and focused host controls.
One background Projector serves the session's Remote adapter; the session actor
retains permission/Source checks and cancels only its own reads. Session shutdown
closes that local connection without stopping the daemon. The
[current owner integration](native-installed-messages-history.md#current-owner-integration)
now passes one new headless ARM/PID1 cold boot and actual typed Stop. A second
new boot, normal ARM windows, cold graphical startup, normal Source/UI and all
matched-device gates remain open.

The [native Rust brightness service](native-brightness.md) now joins its real Rust client,
owned worker and authored attribute backend in six process/socket cases. Unchanged
fresh readonly observations now preserve the state revision, so background reads
do not reject an otherwise valid explicit choice. A compiled real-client RED
passes after the model repair; all52 brightness tests pass in one Release suite.
Raw changes and expiry/unavailable/cancellation gaps still invalidate old intent.
Separate lifecycle adapter, installed configuration and constructor qualifications
remain distinct. A generic ARM kernel/PID1 run now drives the real LinuxBacklight
class through a virtual GPIO driver: raw0→1→0, then Unavailable after device removal,
with normal Stop. The matched Pixel panel and physical display gates remain open.

The current [ARM battery/brightness observation pair](native-installed-power-observation.md)
now passes actual missing-unit RED, production seven-file staging and a distinct
fresh automatic cold boot. The same brightness Main follows real virtual-GPIO
presence→absence→recovery through the typed Rust client; battery completes an
honest unavailable observation. Both PID1 services Stop normally with empty
cgroups. This readonly headless pair does not inherit the earlier kernel Set or
x86 graphical results and supplies no Pixel hardware qualification.

The [normal Settings app](../native-settings-ui/README.md) now passes seven frozen Rust socket/C ABI
groups after compiled assertion RED, plus a native Qt6.12 build/install and real
unprivileged adapter/QML construction. Rust owns transport, state, deadlines and
thread shutdown; Qt renders the shared Omarchy theme. The component window is
never shown. Fixed desktop launch now passes six actor groups, three CLI and
three staging groups plus six unchanged Sounds controls, with separate host/ARM
builds. Earlier successful and failed prepared touch traces remain preserved with
their own client/owner cohorts. The current persistent-outcome Rust client now
completes a fresh five-state
normal-window trace after the actor reservation and readiness fixes:25→explicit75
→unavailable with two disabled inputs and no raw mutation→readonly recovery75
→new explicit25. The same normal app/window and installed service remain running.
Five original1080×2424 captures show the genuine Omarchy theme and qualify the
external Pixel10 profile. Earlier failed traces remain separate and their causes
are unknown; this pass does not establish all-input reliability, cold full-shell
startup, ARM graphical execution or physical display behavior. See the brightness
report for the exact current cohort and screenshots.

The actor reserves a first send behind its own fresh readonly Inspect, preserves
the original revision/deadline, and publishes accepted effects as pending.
Frozen actual Session/Owner/socket RED→GREEN tests retain remote-busy, changed-
revision and expiry controls. Focused lifecycle/launch checks and host/ARM releases
pass. The new graphical pass changes only the owner, matching launcher and route
proof from the preceding failed join; app, daemon, Source100ms, observer, inputs
and deadlines remain unchanged. Readback is not a Set acknowledgement.

The [audio SIGTERM repair](native-audio.md#sigterm-enters-owned-stop) now routes
the shared Rust termination latch into existing idempotent Stop. The unchanged
actual-process RED (signal15) is GREEN (Main0, backend reaped, partial frontend
drained).25 active audio tests/one existing ignored listener, fmt, strict Clippy
and host/ARM releases pass. All18 audio-dependent locks close through actual
offline Cargo; existing camera/Messages fixture staleness is recorded separately.
The new ARM audio provider is distinct from the earlier frozen GUI provider.
[Production audio/appearance units](native-installed-services.md) now separately
pass installed ARM start/IPC/Stop and a fresh PID1 autostart with explicit
unavailable theme/backend states, Main0 and empty cgroups. Physical audio and a
healthy phone session remain open.
See [provenance](evidence/native-audio-signal-stop-provenance.json).

The later [installed private audio backend](native-installed-audio-backend.md)
passes actual ARM preparation and a separate fresh cold boot. The original
completed-unavailable RED is retained. Explicit VM units select private D-Bus,
PipeWire and WirePlumber; a new Rust nonblocking socket-ready helper passes three
frozen real-socket tests after compiled assertion RED, plus all28 active audio
tests and host/ARM releases. The exact signed32-package addition retains all353
versions and Qt50. Both real graphs/whole-target Stops pass with four Main0,
empty cgroups and original execution closure. The first host stage's file-mode
refusal remains separate. No Source, grant, playback/PCM or phone evidence is
inferred. Keep the blocked authentication and expired-Source branches parked;
next work should advance a bounded native phone-service path.

The earlier [production Messages history service](native-installed-messages-history.md)
passes missing-deployment RED→explicit installation and two fresh ARM boots.
Fresh normal-feature current service/Stop/launcher providers are separate from
the isolated seed. Preparation creates only two authored unqualified raw records
and never starts Messages; cold1 reconciles exact selected IDs/provenance into
Inbox, and cold2 retains them without reseed or manual starts. The same frozen
probe checks actual writer/readonly DB ownership, Main owning neither DB,
locked/zero-principal policy and no receiving/counters. Both PID1 Main/ExecStop0
and all three VM closures pass;353/Qt50 and original owners remain unchanged.
This is local history, not incoming SMS, notifications, app reads or a complete
phone session. The earlier Qt app remains an unlaunched dependency.

The current [private Messages owner](native-messages-private-provider.md) separates
one background Projector from the session's Remote read adapter. A real normal
daemon missing-socket RED and unchanged-consumer GREEN close typed Stop and
actual waits. Separate closed QA scopes pass formatting/strict Clippy; Messages
unit 2, projection 9, raw consumer 1, Remote 5 and direct-worker 3; Config 15; and
later Messages/brightness/connectivity configuration 10/4/4. The completing cohort builds 12
normal host/ARM Release ELFs with optimization 3 and no optional features.
Missing-provider setup cleanup preserves NotFound, Source ECHILD and no session
databases, with actual helper exit 0; it is no successful normal startup.
The first GREEN compile failure, restart parser fault and four failed broad-QA
aggregates remain preserved. A separate new ARM/PID1 cold boot now qualifies the
three-argument daemon, sole writer/readonly pair, exact private reads of both
authored IDs/text `hello`, and typed Stop with helper/Main 0, empty cgroup and
dead retained pidfds. Preparation preserves the raw seed, original 353/Qt50/pair,
keeps the old daemon masked with no Main/Inbox, and stages only service/Stop/unit.
Both phases close QEMU 0, owned waits/EOFs and exact container removal on the same
daemon. This is one current cold boot; the old two-boot cohort stays historical.
No second new reboot, normal session Source/UI, incoming SMS or hardware follows.

The [current language boundary](runtime-language-decision.md#current-language-boundary)
retains upstream Hyprland/Aquamarine/Qt C++ scene, protocol and native operations.
Rust remains responsible for shared phone services and ownership/recovery policy.
The graphics integration includes substantial C++; it must not be reported as
progress porting phone services to Rust or as an all-Rust stack.

The [Sounds client engine](native-sounds-rust-client.md) now owns direct and
queued requests, packets, BOOTTIME deadlines, reply validation, pagination and
the accepted UI model in Rust. Qt retains the credentialed nonblocking socket
edge and QObject/QML presentation; permission/provider authority stays in the
Rust owner. Six frozen C ABI groups, unchanged30 Qt behaviors plus setup/cleanup,
29 appearance cases, strict Rust checks, default offline Cargo/CMake/CTest and
full host Sounds linking pass. The obsolete public queue ABI is removed.

A fresh native Qt6.12 build of this whole engine now passes all four prepared
GUI/PCM groups with192,000 measured frames and Pixel10 captures, independently
of the [earlier queue-only result](native-sounds-rust-queue.md). The first new
native run failed by latching an early Qt size before normal configure; preserve
it. A stronger same-PID Qt/native-size predicate passes within the original30s
frame cutoff, retaining both stability assertions and every original audio group.
No production client change or old-result inheritance supplies that GREEN.
Cold installed startup and ARM integration remain open.

The [Messages engine](native-messages-rust-client.md) now owns its full request,
parser/model, pagination, stale generations, scheduling, FIFO/ACK and shutdown
verdict in Rust. Nine frozen C ABI groups, seven original worker controls, six
current-Qt modes/40 rows, five real-thread ordering groups,13 permanent CTest
cases, all29 appearance and UID11010 installed-QML/no-window controls pass.
Strict Rust checks, optimized host/ARM64 archives and native default
Cargo/CMake/CTest/full-app linking pass. Actual pre-entry false-wait RED and
whole-client numeric-overflow RED precede Rust repairs; retain both failures.
The final seven phases close91 host and113 inside waits/PIPE EOFs, with45
separate regular stderr closes; that host epoch does not qualify ARM execution.
A later manual generic ARM kernel build links the normal Qt6.12 app and executes
nine frozen C ABI groups, seven typed worker controls and five Qt ordering cases.
All49 distinct guest waits/18 PIPE EOFs close; compiler regular logs remain
separate. See [manual ARM provenance](evidence/native-messages-arm-client-provenance.json).
A later native ARM production CMake Release build and actual guest installation
now pass with the explicit matching archive and frozen nine-group consumer.
Eight signed additive tools preserve all353 original package versions; the50 Qt
libraries/plugins remain identical.24 guest command waits/PIPE EOFs and one
separate native child wait close within original bounds. Normal ARM main/window,
default ARM Cargo and the full installed cohort remain open.
See [CMake/install provenance](evidence/native-messages-arm-cmake-provenance.json).
The [fresh prepared x86 Messages tracer](native-messages-prepared-tracer.md) now
passes denial, granted authored history, original Omarchy live dark/light themes
and revoked-list clearing in one normal app/window, with four Root-reviewed
native Pixel10 captures and actual owner Stop/closure. Both initial setup and
pixel-oracle failures remain preserved; no production repair is claimed.
The [shared appearance client](native-appearance-rust-client.md) now keeps schema,
cache, revision, generation, readiness and deadline policy in Rust. One actual
counter regression RED precedes its repair; strict Rust checks, host/ARM archive
builds, eight frozen C ABI groups and all29 unchanged native Qt cases pass. Fresh
normal Sounds/Messages explicit and default-Cargo host builds, dual-archive
links, protected installation and four windowless controls pass. The earlier
ARM and visible captures retain their pre-port appearance client. A [fresh normal
Messages Qt6.12 session](native-messages-appearance-rust-tracer.md) now passes the
same four groups with both Rust engines, one PID/window, four original native
captures and native Stop/owned closure. The closed review counts102 primary
waits/109 PIPE EOFs plus one separate C ABI child wait. Numeric app exit and
direct QWindow/DPR stay unmeasured. The required historical Sounds profile is
inactive; no current Sounds GUI or full shell is qualified. Both current normal
apps now separately pass [native ARM production CMake/install](native-current-arm-apps.md)
with matching app/shared archives,23/24 normal APIs, unchanged Qt50, frozen9/6
consumers and five protected files.36 guest waits/36 merged PIPE EOFs close;
outer CLI, inspections and nested child wait stay separate. Current Sounds-engine
and five production service/launcher ARM crossbuilds plus the separately featured
seed are closed, without service target execution. Retain the failed archive
`nm` timeout and archive-only10→30s correction; Root review was postlaunch.
Both normal ARM mains/windows, ARM default Cargo and the complete running cohort
remain next.

The later full-shell x86 attempt is parked after two setup failures and zero
functional groups. A copied database-signature policy failure precedes the sole
`DatabaseOptional` correction, with trusted local package signatures still
mandatory. The successor passes package/current app component checks but its
owner exits1 with `Kind(PermissionDenied)` before compositor/Source/menu execution;
the exact cause remains unknown. Real user-manager start/Stop stays separate
from a successful owner or shell. Keep both aggregate and guest-case-closure flags false; successful recorded
waits remain separate facts. See the [compact failure record](evidence/native-full-shell-parked-provenance.json).
No further x86 retry or admission investigation is selected. Next is a normal
ARM app/window under the unchanged passing compositor configuration; native menu
execution remains unqualified.

Retain Qt presentation. Extra conservative parser-error
classification refinements are parked; no exhaustive Qt equivalence is claimed.

The [later finite fd metadata epoch](native-callback-spans.md#later-finite-file-descriptor-metadata-samples)
still fails cold startup. Two existing-watch proc samples complete before Source1;
fd25 is a blocking monotonic timerfd in the second,158.458194ms before Source1.
The ring again ends with an unmatched AQ fd25 entry,25.949us after Source1;
cached expiry is113.574824ms. This does not prove fd continuity or a blocked read.
AQ idle dispatch has a readiness guard; a separate authenticated Hyprutils0.14.2-1
package/source chain identifies zero-timeout poll, without new native execution.
The outer span does not isolate the read or pending callback work. All315 pins
and149 waits/156 PIPE EOFs close. Keep this original
metadata component/native diagnostic separate from the passing prepared Sounds
whole-engine run. No C++ source rebuild or production repair was added here.

The [retained Rust process channel](native-process-channel.md) now implements
one nonblocking1..8192-byte opaque message over the existing authenticated
READY/GO pair. Original control and genuine missing-API RED precede five actual
host groups;13 child waits/11 PIPE EOFs,9 shared and68 radio tests/3 ignored,
strict production checks and Release builds pass separately. Startup deadlines
are not reused for application operations. No retry or remote-cleanup authority
is added. Keep fixture prerequisite failures distinct from functional results.

The original installed ARM Main/worker/status path now has its own operator
baseline RED: service-account03 unsupported, root refused before parsing,
independent zero105, native worker9/PID1 Main0 and unchanged-journal restart78.
All25 direct guest waits/13 PIPE EOFs and owned outer closure pass. The later
[installed Rust operator](native-radio-operator.md) now qualifies root admission,
copied startup config, a prewarmed intent writer, authenticated ACK→Finish→actual
wait0 before the sole permit, fresh original gates, independent request history
and Stop for both roles.83 radio/nine process tests, five existing termination
groups and six ARM Release builds pass. All six fresh generic ARM Binder cases
pass: accepted, writer-held, expired-before-admission, permit-expires,
stop-before-consumption and separate actual manager Stop with the writer held.
The214 guest waits/118 PIPE EOFs,12 native waits, six PID1 Main0/unit-success and
six unchanged-marker restart78 outcomes close within original bounds.
Two earlier direct-Stop runs reached native waits/Main0 but failed duplicate
ExecStop; the repair retains the shared SIGTERM guard and uses KillMode=mixed
without ExecStop. Fixture/property-observer and audit-format failures remain
failed, not functional evidence. The explicit harness3→harness5
observer-only bridge preserves actual Release ELFs and compiled source. No vendor
cleanup, app permission, production modem transport or phone capability follows.

The [finite main-thread epoch](native-callback-spans.md#later-finite-main-thread-metadata)
now records one authenticated native R/running sample156.067174ms after the
unmatched AQ entry and36.926877ms after cached expiry validation. It identifies
no kernel wait, continuous execution or busy-loop cause. The second attempt finds
the pidfd dead and reads no proc bytes. Cold startup remains failed at partial
Quickshell argv observation; all334 pins/148 waits/155 PIPE EOFs and existing
closure qualify separately. The helper and frozen consumer passed genuine host
control/RED/GREEN before this run. No C++ source/build or production repair changed;
raw proc/addresses remain private. Preserve this result alongside the earlier
finite descriptor observations, rather than promoting either to a blocked-read cause.

## Current stopping point

The [graphics critical-path decision](runtime-language-decision.md#graphics-critical-path)
now selects normal upstream Hyprland under the Rust session owner for the next
Sounds/Source/permissions/PCM/theme/Stop integration. Stop extending the custom
CPU/native renderer reconstruction as the app-launch prerequisite; preserve it
and its failed runtime evidence as component research. Source freshness and
render-stall availability, uncertain native operations and genuine display
cleanup stay open. No physical phone is authorized.

Two fresh [normal upstream startup attempts](native-normal-upstream-tracer.md)
retain terminal Source failures, including the unsuccessful private initial-present
candidate. Do not merge it or add another speculative activation predicate.
After actual missing-logical-output RED, a one-entry upstream allowlist patch now
passes the same real registry consumer and the full prepared four-group Sounds
session: denied silence, independent grant plus real touch/virtual speaker PCM,
null-focus restriction with retained approved media, and terminal Source loss
with quiet PCM and later launch refusal. Eight windows retain192,000 measured
frames. Actual screen/window/native capture qualifies the Pixel10 profile without
resizing. Full matching SDK and plugins, current32 audio/29 appearance cases,
Stop/owner/guest/VM/container/input closure pass separately within that epoch.

The captures are from a prepared native x86 session with a visible debug banner.
This is a completed app/service tracer, not a cold-start repair, live theme-change
or full-shell qualification, render-stall proof, ARM selection or phone boot.
Two later whole-compositor SIGSTOP epochs now independently qualify expiry with
pre-Stop silent PCM/refusal, and native Stop while the compositor remains stopped.
The latter observes Ok0.952184ms and actual ownerwait0 at69.934551ms after request
start, with its previous healthy sample only11.315965ms old at send. Each passes
six PCM windows/144,000 measured frames and original owned/input closure. These
are existing-behavior x86 qualifications, not fresh Source during rendering-only
stalls or native display cleanup. No production repair is claimed.

Next assemble one installed ARM cohort through production CMake/package paths,
with current Messages/Sounds, real Omarchy shell/menu and live themes. Include
the bounded shared-appearance Rust policy port with existing29 cases and the
visible theme tracer. Then repair cold startup from a concrete causal hypothesis;
avoid another instrumentation ladder. Resolve the finite matched-Pixel deployment
dependency closure before further PowerVR work. Keep the diagnostic boot bundle
frozen and the custom renderer parked. Keep Pixel targets fixed.

The explicit normal-session bundle installer now passes its frozen missing-target
RED→GREEN consumer:205 selected files plus eight core files, including196 locked
Omarchy source blobs. This is host staging evidence only; no unit or cold startup
is qualified. The [graphics boundary review](native-pixel-graphics-boundary.md)
retains the original EGL initialization failure and leaves actual PowerVR devices,
module membership/loading/KMI and separate display scanout unqualified.

Two actual cold installed runs now fail before Sounds: the genuine shell's
selected Wayland connection is absent and the shell exits255 while Source still
awaits its first sample. The second frozen observer qualifies process/VM/input
closure while runtime and unit health remain failed. Native owner main exit0 is
separate from a later failing unit ExecStop. Native Rust/systemd
[startup readiness](native-session-startup-readiness.md) now passes actual actor
and socket/process boundaries, with notify unit metadata. Run the installed
PID1/cold-shell selection and complete the separate Stop lifecycle correction;
keep the original100ms Source and120s startup bounds. See the [cold result](native-normal-upstream-tracer.md#cold-installed-startup-fails-before-sounds).

The [Rust SIGTERM owner repair](native-session-signal-stop.md) now passes its
unchanged actual production-owner consumer before policy/Source readiness, with
Main exit0/no adopted children/ECHILD;110 composed regressions and default Release
pass. The selected unit now removes ExecStop and uses KillMode=mixed. Two later
[cold PID1 epochs](native-session-manager-stop.md), with identical corrected
observers, reach notify readiness and then fail terminal Source expiry before
shell connection. Actual manager Stop after that loss records Main0/Resultsuccess
and empty cgroup for both baseline and candidate; both functional aggregates stay
false. Retained unit references preserve matching invocation exit evidence.
The earlier queued-job observer failure remains failed, without Source inference.
Later passive diagnostics now retain the activating owner's compositor identity
and13 completed startup events. One real sequence2 is rejected for a116.863834ms
sample gap. Configuration completion precedes sequence1; the later blocking
operation remains unidentified. Default file logging is disabled, so absence of
a captured log supplies no cause. Keep100ms unchanged and identify the gap from
concrete runtime evidence, without inventing another activation predicate.

A later explicit container8→16CPU diagnostic keeps guest6CPU, profile, providers,
functional consumer,100ms Source and original deadlines unchanged. All60 bounded
cgroup samples report zero own-bandwidth throttling; the last precedes QMP guest
shutdown by585.796ms, without an invented guest/host clock alignment. Source1
still expires at103.241220ms, shell exits255 and no app/PCM group qualifies.
Actual manager cleanup and153wait/160PIPE plus QMP/unit-reference/input/CID closure
pass separately. An authenticated existing log is retained but stops before the
gap. The next logging-only profile is admitted separately from the original205-file
installer proof; actual guest205-file installation is checked. It retains100,868
log bytes and another115.427033ms Source gap, with seq2 received/validated52.269us
later. It identifies no blocking operation or logger buffering behavior. All146
waits/153PIPE plus original closure pass, while full startup stays false. The
[bounded callback diagnostic](native-callback-spans.md) now passes frozen
structural RED→native component GREEN and its matching full GNU16 compositor/
498-file SDK build. Its1024-record BOOTTIME/main-thread CPU ring brackets only
three existing callback sites. All16 component and31 build waits/merged EOFs
close separately. The fresh cold diagnostic now executes real deferred/AQ
callbacks with retained inode/main pidfd and post-exit reading. It keeps31 commits,
14 complete deferred spans, one complete AQ span and one unmatched AQ fd25 entry
27.131us after Source1; the cached sample expires at109.601368ms. Writer death is
already observed before explicit manager Stop, so neither a permanently blocked
call, CPU duration, internal cause nor shutdown trigger is established. The case
retains its shortened-shell-argv assertion and later shell255; all148 waits/155
PIPE plus original closure pass. The next focused observation is bounded fd
metadata in the existing watch, with reuse uncertainty. Full cold startup,
new-client/GUI/PCM and production repair remain unqualified.

The Rust [one-shot add-PDN response observer](native-radio-add-pdn-response.md)
now retains immutable request/session/route/error/byte history after frozen
component RED→GREEN and a separate actual ARM Binder differential check. All79
active host tests, formatting, strict Clippy and Release pass; nine Binder cases
remain ignored on the host. A callback queued before dispatch permission cannot
bind later, and a duplicate cannot replace the first response. OutcomeUnknown,
occupied capacity and no implicit retry remain. Installed request admission,
APN/IMS semantics, vendor cleanup and phone qualification remain open.

The [experimental executor ingress wake fix](native-render-ingress-wake.md)
passes its exact frozen RED→GREEN test, a separate expired-retry refusal and all66
active Rust tests, with two ignored child modes, fmt, strict Clippy and release.
The new archive is not C++/normal-main/ARM qualified or selected into those older
test epochs. This component result does not qualify the integrated app slice.

The [native graphics resource operations](native-render-exact-buffer.md) pass their matching normal AQ15 build, ten existing CPU groups and four API checks after missing-interface RED. Two real-driver runs observe all five resource groups passing, but both aggregates fail a control-session check; the diagnostic identifies revocation with a queued CHANGE. Production exact-buffer preparation/execution, normal frames, Source, ARM and phone gates remain open.

The [Rust format planner](native-render-format-policy.md) now chooses exact implicit or explicit requests from copied native observations. Its six-group C++/Rust boundary test, all64 existing Rust tests, five previous C++ programs, formatting, strict Clippy and Release build pass after genuine missing-interface RED. The obsolete planning API is removed. The shared native startup/CHANGE collector removes both old C++ format choices and passes its matching full compositor/SDK build. The unchanged normal startup/CHANGE test also passes on the real generic x86 KVM virtio driver: Rust accepts the fresh sequence4 configuration after bootstrap3, with one output and no Stop. Client frames, native retirement and the integrated normal app session remain open.

The [real normal Wayland client boundary](native-render-format-policy.md#actual-normal-wayland-client-boundary) now executes after that accepted Rust metadata. Its authenticated registry round trip reports none of the three required app globals; a later same-main snapshot stays healthy Ready. This is a genuine functional RED. All38 waits/45 PIPE EOFs close; frames, native Stop and the integrated app session remain open.

The earlier [Rust session-reconciliation component](native-render-session-reconciliation.md) passes64 Rust tests, six C++/Rust integration groups and its matching main/SDK build. Its later native format-negotiation refusal is resolved by the V2 planner integration above. The recorded failed and diagnostic epochs remain distinct; real frames, Source and phone gates stay open.

The [Rust first-Stop observer](native-render-first-stop.md) now identifies a real startup trigger: a direct Aquamarine device-change revocation requests full Stop before startup completes. The unchanged native wire test passes; the first record survives a later actual Stop and two snapshots. The matching normal main/SDK,64 Rust tests and compiled interface checks pass separately. At that checkpoint, recovery and normal readiness remained open; the later planner integration above qualifies backend metadata. Real client frames and phone gates remain open.

The earlier [normal render profile](native-render-normal-profile.md) now excludes the original test-only probe after actual source-configuration RED and clean syntax checks in both profiles. Matching normal AQ and compositor/SDK builds pass, with23 Rust exports, five strict API checks,15 existing CPU groups and the five-group texture program in the complete main selection. The earlier fixture failure remains preserved. That earlier normal startup reports STOPPING at its0.284s observation, before the20s deadline, without recording its cause. Its126 replies stay responsive, and all37 waits/44 PIPE EOFs close, without backend readiness or client launch. Normal startup, real client frames, Source7 and phone gates remain open.

The [immutable CPU texture checkpoint](native-render-cpu-texture.md) now passes two strict signature checks and five real C++/Rust integration groups after the original missing-API structural RED. Owned pixels survive producer disposal; immutable replacement and two joined-thread refusals pass even with actual C++ thread-ID reuse. The CPU adapter has no GPU identity. Core Wayland capture, normal frames, Source7 and all phone gates remain open.

The [retained CPU surface-content checkpoint](native-render-surface-content.md) now passes unchanged-consumer five-group C++/Rust integration after actual geometry RED. Source extent uses retained Rust pixels after producer overwrite/unmap/close; no-attach identity, transforms, detach/replacement and independent clones pass. The two-file patch changes CPU geometry only; core GL capture/upload, normal Wayland frames, Source7 and phone gates remain open.

The [initial broker batch peek](native-render-initial-batch.md) now passes unchanged-consumer real seatd/udev/virtio capture after a qualified missing-API compile RED and matching AQ15 source/SDK build. Three initial events reach Rust before queue admission; two pending duplicates close while original service leases remain open. Ten existing CPU groups also pass. The later main/control-to-Rust-worker failure boundary now executes below; normal startup, Source7 and all phone gates remain open.

The [owned startup checkpoint](native-render-owned-startup.md) now passes the same two failed-startup child-retention cases after qualified native RED. A production AQ dependency patch retains the selected child and all preexisting resource identities through both injected failures; matching AQ/main SDK builds pass. Each case records zero child destructors and valid weak metadata. Startup still fails, Ready stays false and join stays Busy and native shutdown stays Pending. Earlier failures remain preserved; nested factories, normal readiness, retirement, Source and phone gates stay open.

The current [Messages session assembly](native-messages-session-composition.md)
selects explicit local history alongside Sounds, shares the actor and per-app
Wayland Factory, and selects the typed public appearance client/QML. Current
host components36 and Source-status12 pass in the earlier selection, whose later
`sounds-launch` build fails on an omitted fixture input and loses its tmpfs
artifacts. That aggregate remains failed. A separate completed556-input selection
preserves all551 prior files and retains all three release opt3, feature-empty
owner/launch-client ELFs, with strict Clippy and three exact retained-owner
history passes. The36/12 cases are reused, not rerun or combined into a new
aggregate. Separate actual GNU Make4.4.1 staging now passes all eight current
files with exact hashes, root:root ownership and0755/0644 modes; no activation or
test rerun follows.
Historical owner RED→GREEN and native Qt6.8.2 Release/nonpresenting appearance
checks remain separately pinned. The successful normal Messages main/launcher,
current actor, per-app Wayland, real Source and grant→read/revoke/source-loss GUI
path remains the next integration gate; ARM and all phone gates remain open.

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

The new [native session owner](native-session-owner.md) now implements that
assembly and retains/actually waits its compositor Child.32 host cases pass
across the unchanged-source18+14 runs; failed `/run` setup remains failed.
Host/ARM64 compilation and actual six-file Make staging pass. Independent source
review found no blocker. No combined original-compositor or ARM execution follows
from those host results; the unit remains staged metadata.

The first actual combined ARM attempts remain failed: probe readiness setup,
duplicate live-binary install, then native owner `InvalidData` before a usable
compositor. That last selection passes a policy filename to an existing directory
contract. The [policy-directory repair](native-session-owner.md#policy-directory-correction)
now has a genuine2-pass/5-failure RED and15 preflight GREENs, plus two actual
native storage-worker groups, strict final host/ARM builds and Make staging.
The example and validator require an existing root0700 policy directory.
Original24 unrelated host cases are reused; no39-case current aggregate exists.

The corrected production owner reaches original Hyprland in two further ARM
selections, but both fail during display-profile setup before Qt/PCM. A separate
private owner diagnostic records first terminal `TimedOut`, sequence1 age120.9ms
against the100ms lease, and actual compositor child SIGKILL9. That diagnostic is
not a production GREEN and does not identify the blocking startup operation.
The [producer startup repair](native-source-timer-startup.md) now gates all
publication until actual timer dispatch. A genuine early-publication RED precedes
two unchanged real-Wayland/credentialed-socket ARM GREEN cases; matching adapter
SDK compilation also passes. Rust SourceOwner/Observer and the100ms lease are
unchanged. Complete heavy fixture
display setup before Source activation. Preserve the original lease and terminal
failure, and restore the unmodified owner for functional qualification. See
[startup evidence](native-session-owner.md#cold-start-source-failure).

One [public-gate original replay](native-session-owner.md#gated-original-compositor-replay)
now passes all three plugin builds and reaches the display metadata probe, which
exits255. The compositor is dead before Stop, but its first terminal reason is
not exposed. No boundary step, normal Sounds or PCM passes. Overall failure is
preserved separately from actual waits/EOF and QEMU/CLI/container closure.
The private ordered/applied receipts were not retained before the later failure;
only the mandatory executed prerequisite checks are source-bound.

The [root-admin Source observation](native-source-observation.md) now retains
first observed failure and last actually accepted sample without policy,
freshness, privilege or retry changes. Four actual host IPC and four local cases
pass after eight missing-field RED failures; observed state is historical, not a
health or cause claim. Matching default-owner/client host/ARM builds and the
current default library68-pass/20-ignored regression qualify separately; the first
regression runner's missing-directory failure remains preserved. A matching
[production observation ARM replay](native-session-owner.md#production-source-observation-arm-replay)
still fails at metadata Qt255 before Sounds/PCM. The initial root reply accepts
sequence2; the pre-Stop reply retains sequence37 and first provider TimedOut at
114.24ms sample age against100ms. Cause remains unknown; exact owned closure
qualifies separately. The [non-presenting discovery replay](native-session-owner.md#non-presenting-discovery-replay)
then compiles the derived probe but also fails before Sounds/PCM. Source accepts
sequence48 before provider TimedOut at113.76ms; early actual profile receipts
are retained, but final measurement is unreached. Removed probe-window painting
is not a necessary trigger; the gap's cause remains unknown. The bounded
[startup-event witness](native-session-owner.md#startup-event-witness-replay) now
compiles against the matching SDK and retains13 committed landmarks, armed with
zero flags, read after retained compositor death within the original2s/WORK. Its
last observed property-refresh completion precedes the last accepted Source sample
by1.354s; sequence50 is followed by provider TimedOut at119.18ms sample age. No
Sounds, final display, functional step or PCM follows. Both independent readers
agree; all owned waits/EOF and exact container closure qualify separately from
functional failure. This does not justify another startup gate or establish a
blocked call, callback return or empty deferred queue. The witness is cooperative,
not an authenticated audit channel.

The [native x86 runtime checkpoint](native-session-x86-runtime.md) now passes
one actual offline KVM stock-kernel bootstrap and normal poweroff, with QMP,
serial, owned waits/EOF and exact container/input closure. Matching-source native
Rust payload builds pass separately. This runs no compositor, Qt app or PCM.
The later [offline installed guest](native-session-x86-runtime.md#offline-installed-guest)
now qualifies the signed367-package cohort, complete actual stock upgrade, exact
installed Hyprland binary/SDK commit, seven audio tools and four SPA files. Two
failed guest setups remain failed; the third fresh layer closes in36.11s with
all31 direct waits/38 PIPE EOFs. Build native Qt and run the full composition
next. No prior Qt tests are relabeled as current.

The actual [native composition attempt](native-session-x86-runtime.md#actual-native-qt-and-composition-attempt)
now builds/installs the current Qt app and tools, passes32 audio checks including
setup/cleanup (30 behaviors), and passes29 appearance cases. All three plugins
and the discovery helper compile. The original compositor then dies during cold
startup: root-admin Source retains sequence1 and first provider `InvalidData`.
That error also covers EOF and multiple frame-validation failures; it establishes
no rejected frame, sample gap or scheduling cause. No normal Sounds, final Pixel
profile, functional group, capture or PCM executes. The60.82s selection remains
failed;97 direct waits/104 PIPE EOFs and normal QMP shutdown/QEMU0/exact container
closure qualify separately.

[Bounded first Observer rejection](native-source-observation.md#first-observer-rejection-evidence)
now preserves typed receive/drain/frame/chain/freshness decisions before cleanup
and exposes them through the existing root snapshot. Eight actual missing-record
REDs precede12 admin,57 process and four lifecycle GREENs; strict formatting and
Clippy pass. The immutable record retains no packet, source token or peer identity.
The100ms lease, predicate order, ErrorKinds and terminal refusal stay unchanged.
The compiled default owner/client use release opt3, replacing the previous debug
opt0 artifacts in the next original replay; this adds no timing causal claim.
The following [original native replay](native-session-x86-runtime.md#typed-rejection-replay)
now retains validation/sample_gap: sequence1 at55,903,193,306ns, then sequence2
at56,014,017,571ns. The110,824,265ns gap exceeds100ms; the successor is only
5,576,433ns old at failure observation. The freshness predicate is not reached
because chain rejection comes first. This is a producer sample gap, without an
identified compositor operation or retrospective diagnosis of earlier failures.
No normal app, final profile, functional group or PCM runs. Actual current
Qt32/appearance29 and97 waits/104 PIPE EOFs qualify separately; the62.77s native
selection remains failed. A later [render-witness replay](native-session-x86-runtime.md#render-witness-replay)
now compiles the matching plugin and retains31 committed numeric records with
zero flags. Source sequence1→2 has a113,028,790ns sample gap; adjacent
LAST_MOMENT5/POST6 records are112,844,181ns apart inside that gap. Records carry
no frame/monitor identity, and establish no particular blocked call, callback
pair, presentation or cause. Current Qt32/appearance29 and97 waits/104 EOFs
pass separately; the61.30s full selection still fails before normal Sounds,
final Pixel profile, all four groups, capture or PCM. The focused
[endRender observation](native-endrender-observation.md) now passes13 component
cases and a separate offline native source build with matching SDK. After distinct
readonly-staging and SDK-probe setup failures, a fresh replay builds all plugins
and passes current Qt32/appearance29. Its57 valid records bind116.618ms around
fence creation in invocation2 inside a116.918ms Source gap. Full composition still
fails at policy readiness before Sounds/PCM. Narrow further diagnosis to that
measured span; no exclusive CPU/driver or earlier-failure cause follows. Preserve
both setup failures and original Source/functional guards; no broad startup
profiler, Source gate or lease change is justified. A later inner-fence component
adds paired eglCreateSyncKHR/glFlush/native-FD-dup marks and passes24 frozen-v2
cases after eight semantic REDs. Its prior13 controls revise the formerly invalid
phase24 record to30 before RED2; the old13 proof is not relabeled unchanged.
Source build2 completes GNU C++26/O3 with all four affected CPP object lines and
a verified499-file matching SDK, without compositor execution in that build. The
new matching replay retains69 zero-flag records and113.338ms around eglCreateSyncKHR
in invocation2 inside a113.644ms Source gap; flush is4,439ns and FD duplication
9,227ns. All plugins and current Qt32/appearance29 pass, but full composition still
fails at the same live-compositor assertion before Sounds/PCM. Pre-Stop retained
pidfd death does not include process disappearance; final external closure has
both. Independent full-prefix decoding agrees;101 direct waits/108 PIPE EOFs and
original cutoffs close the failed60.587s selection. Root8 plans bounded
owned-compositor/all-worker entry/exit observations for header-selected EXECBUFFER,
WAIT, TRANSFER_TO_HOST and TRANSFER_FROM_HOST; full observer-runtime qualification
remains open. Same-clock request/FD identity, result/loss and after-closure evidence
are required. A matched long selected interval chooses the next narrow boundary.
Short or absent selected requests leave the delay unresolved: this non-exhaustive
set excludes no other ioctl, sync_wait, futex or scheduling wait and establishes
no userspace-only cause or offload. A separate native kernel capability probe
qualifies private tracefs control write/readbacks, exact format availability and
selected metadata only, without observer activation, DRM ioctl or timing evidence.
That capability probe alone does not qualify inheritance, BTF/probe/device
identity or loss/pairing/cleanup behavior. A later frozen observer-absent baseline
executes14 tests:6pass,8 intended missing-observer failures (02–09),0 errors and
no setup failure. Outer0 acknowledges expected RED, not a production pass. A
private implementation follows RED. Its first native attempt passes13/14 with
only the printed-timestamp bracket failing. Source-supported nearest-microsecond
decoding then passes unchanged14/14, limited to EXECBUFFER/VERSION helpers. A
separate authored host7-case decoder RED4→GREEN0 remains separate. Actual full
Root8 compiles all four actual header requests and completes BTF setup, but its
full replay fails at discovery SIGABRT6 after policy readiness, before final Pixel/
Sounds/groups/capture/PCM. Source expires its cached first sample at102.579357ms.
The diagnostic gate conflates original expected-exit success with physical closure
and skips collection:0 retained pairs supply no qualified timing or absence.
A source-only correction separates physical custody from the unchanged functional
exit contract. Root9's corrected bounded capture qualifies11 main-TID selected
pairs (7 EXECBUFFER/4 WAIT) and one successful116.925–116.927ms EXECBUFFER interval
inside the116.959ms EGL bracket. Full runtime still fails at policy readiness
before Sounds/PCM. The15 observed TIDs form a lineage, without same-TGID proof.
A bounded source review finds no concrete avoidable wait or incorrect operation
and selects no driver patch. That discrimination investigation is closed. A later
[prepared rendering-stall baseline](native-render-authority.md) now qualifies actual
RED: the three no-stall focus/death/lock controls and authority-stall negative pass;
the same three operations under entered native EGL holds all time out with positive
state false. All seven cases pass setup; five earlier attempts remain setup failures.
The later [connected Rust SHM prerequisite](native-render-import.md) compiles and
links into full native Hyprland with a501-file matching SDK; five actual C++/Rust
CPU lifetime groups pass in the x86 KVM guest. A separate actual compositor
no-attach case now qualifies RED: a retained unassigned surface's import identity
changes7→8 despite unchanged16,384-byte content. Earlier old-SDK/borrow-compile
setup failures and the component-RED/failed outer join remain preserved. A one-file
repair gates recapture on the new state's buffer-update bit; its matching native
rebuild and five FFI groups pass. The unchanged actual no-attach oracle then
passes: identity stays7→7 with the same bytes, real client/clone facts and closure.
This is CPU import identity GREEN only. A separate502-file SDK baseline build
and actual padded ARGB Cairo-consumer RED now close: real pointer-role/cache
controls pass, but stride128 consumption of stride144 bytes produces992 wrong
pixels out of1,024. Two preceding focus/motion setup failures remain separate.
The one-helper stride repair passes its matching native rebuild/five FFI groups
and unchanged actual oracle: both live Cairo comparisons check1,024 pixels with
zero mismatches. Separate post-client-exit checks retain raw Rust bytes, invoking
no helper. This is CPU-consumer GREEN only. GL ownership, the seven-case Source RED
and original cold full-composition failure remain unchanged.
A separate original Rust commit-permit component now passes eight Linux x86
thread/clock/admission cases after a real control-owner-drop RED and repair.
It orders grant-time intent/Stop and same-epoch revocation; actual native binding,
worker/FD custody, AQ and output completion remain open.
The [qualified31 source checkpoint](native-render-import.md#qualified-scene31-checkpoint)
now builds all15 affected C++ units and505 matching SDK files after a separate
29-path compile failure. A fresh unchanged Source7 replay still yields four
controls and three rendering failures, with original windows and full closure.
The new scene producer/GL primitives compile only; no owner starts, and this ELF
selects old snapshot-only Rust69 rather than the separate permit module.
The later [frame CPU custody checkpoint](native-render-frame.md) passes14
postimplementation Rust cases and a historical FULL33/private-header build with
507 SDK files, five snapshot FFI and four real codec CPU groups. The new public
frame header contains only implemented capture/release/visitor functions and
passes separate Rust QA. Its fresh matching native33 build/507-file SDK and the
same five FFI/four codec CPU groups also pass. Authored plan values qualify CPU custody only; live producer,
GL primitives and a production worker have not executed in these epochs.
The [actual native GL baseline](native-render-gl-baseline.md) separately passes
real worker CLEAR/RECT pixel comparisons and retained96-byte CPU custody after
fixing an inherited test result socket in Xwayland. Fresh live Source before/after,
actual EOF/unload/join and bounded KVM closure qualify this prepared component.
It selects the earlier Frame archive3cf0a142, without the new Rust executor;
That baseline contains no SHM/fence behavior or production Rust executor.

The later [native owner resource checkpoint](native-render-owner-resources.md)
qualifies two actual native KVM RED/GREEN pairs: missing fence export, then a
nondefault sampler override producing eight wrong bytes. Final eight-case GREEN
checks padded AR24/XR24, owner/context/allocation refusals and six real sync-file
facts before exact readbacks, followed by EOF/unload/join. Its four native sources
compile into a test plugin against unchanged Frame507/9184/Rust3cf inputs. The
production Rust executor, AQ output/presentation, Source7 and phone remain open.

The later [owned bootstrap/worker checkpoint](native-render-bootstrap.md) passes62
active Rust cases:8 permit/28 worker/14 Frame/12 descriptor bootstrap, plus two
owned-child modes invoked by parent tests. A retained deferred holder takes the
move-only initial session/FD pack before native work. Copied discovery must match
that input, frame admission matches actual discovered values and final grants
match the selected device tuple. An original BOOTTIME timerfd exposes expiry
while bootstrap is held; control accepts a copied candidate before Ready.
Stop/expiry never revive a late result or release pending native custody. The
bootstrap archive1dcce61e was CPU-QA only; the earlier native33/SDK result still
selects3cf0a142 without this worker. The later
[current worker C ABI](native-render-executor-ffi.md) now passes actual C++
lifecycle and five authored-owner groups, all62 Rust cases, strict Clippy and
release compilation using archive246a6680. Copied per-executor tables/options,
exact refusal retention, held Stop/BOOTTIME expiry and worker-side destruction/
actual join qualify at that authored native boundary. Structural missing-ABI RED
and failed fixture selections retain separate classifications. The separate
matching AQ15/main/SDK build now passes CPU/loader checks, but actual runtime
startup remains failed before Source/actor. Next integrate the legacy startup
with the genuine broker/render owner, then real AQ/GL, later session ingress/OOB
revocation, actual output/routing and distinct Stop-retirement authority. The unchanged Source7/full-runtime gates remain open.
The [executor GL lifecycle checkpoint](native-render-executor-gl.md) now qualifies an actual
missing-handshake RED after all eight GL cases, 42 packets and six sync-file
facts. Stop returns `unknown request`; no finalize, unload or Rust join/release
occurs. A separate [AQ15/current-ABI native build](native-render-executor-gl.md#separate-aq15current-abi-build-and-startup-failure)
now passes CPU/loader checks, but the matched runtime aborts during backend
startup before Source/actor; its exact cause remains unresolved. Source GREEN
and all phone gates remain open.

Production control/render ownership separation and GREEN remain open. Source100ms,
terminal authority expiry, real facts and fence/buffer lifetime remain required;
no exclusive-CPU or earlier-failure cause follows. Strict matching ARM64 owner/client
compilation also passes separately,
without ARM execution.
Messages history composition was excluded from that render-witness replay. The
current integration described above preserves its Source-rejection changes;
Messages component results do not qualify the normal Messages GUI.

Preserve the ARM failure and stop speculative fixture changes. Continue
matching-source native CPU-speed composition on Harbour, with the same Pixel
geometry, real focus/grants, app/PCM guards and100ms lease. Existing TCG/virgl
software rendering is an execution constraint, not a proven cause. A host pass
would qualify only host composition; original ARM availability stays open. Then
qualify null-focus background lease continuation and terminal Source loss without
app/audio revival. Preserve the earlier ARM selection and its authored-focus
scope. Do not reopen auth/PAM or a broad static/diagnostic campaign. Hardware
remains untested.

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
exits 0 naturally. Strict Clippy and separate ARM64 compilation pass. Local
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

Current product priority: deliver a reproducible VM preview with a documented
launch path at the Pixel 10 display profile. Six ARM Settings connectivity epochs
remain failed; the sixth 16-guest/16-outer-CPU run closes with ReceiveCommand
context before Enable-ready, capture, touch or packets. Repeated graphical ARM
resource campaigns are parked with the production 50 ms guard unchanged. The
fifth epoch's partial Enable/IP/first-packet result and the sixth epoch's natural
worker exit 1 remain separate, with no CPU cause or repair qualified.
Headless native Rust open association and separately requested Disconnect now pass
the frozen ARM virtual-radio consumer with independent STA/AP kernel checks.
See [association evidence](native-wireless-association.md). The subsequent
[fixed IPv4/packet case](native-wireless-address-packets.md) passes normal Rust
Up, two packet round trips, Down, quiet and separate Disconnect with full
closure. WPA, DHCP/DNS, persistent Settings integration and Pixel qualification
remain open. The x86 window/packet/capture result remains separate
from full Source/auth/session runtime, Wi-Fi/Internet and Pixel hardware. Keep
the broader Source/authentication and capture branches parked.

The current single Messages owner now passes one installed ARM/PID1 cold boot.
Its session-app join in the x86 preview remains blocked. The latest
[backend dispatch diagnostic](native-session-x86-runtime.md#bounded-backend-dispatch-diagnostic)
fails at `cached_sample/sample_expired` with only sequence 1 accepted, age
110.114129 ms; earlier `sample_gap` failures remain separate. The ring records a
completed fd 22 call and unmatched fd 25 entry; the same-epoch log maps fd 25 to
idle; neither a hang nor a cause is established. The new
[bounded Source establishment](native-source-establishment.md) withholds authority
until a fresh consecutive pair, preserves protocol history through provisional
timing gaps, and keeps established failure terminal. Its frozen kernel RED→GREEN
and 96 scoped host cases pass, with a normal optimized owner built. The original cold shell/Sounds consumer now reaches genuine shell IPC ping and
background/bar surfaces with Source sequences 14 then 76, ready and no failure.
It fails at unavailable appearance before app launch. No new screenshot, final
display-density qualification or positive manager Stop case is claimed; owned
cleanup closes. Genuine [theme provisioning](native-omarchy-theme-provisioning.md) now passes
the unchanged fresh-home RED→GREEN consumer against the real appearance service,
plus preservation and partial-selection refusal controls. Integrate its normal
oneshot/dependency rows, verify actual PID1 failure propagation, and resolve the
separate shell namespace/environment gaps before the next full cold qualification.
The [presentation helper](native-presentation-environment.md) now passes eleven
actual process controls, including unchanged-consumer exec RED→GREEN. It validates
as UID1000 and execs fixed Quickshell in the same PID; no EnvironmentFile is used.
Root has prepared the 54-file namespace correction separately. Neither component
qualifies the pending combined PID1/Qt cold run. The Messages
app join remains subsequent work.
Source's 100 ms limit, all consumer criteria
and the separate 50 ms connectivity guard remain unchanged. Historical
six-argument harnesses require their recorded sources/ELFs; a second current
ARM boot/reboot and normal Source/UI remain open.

The following Messages lock→restore work remains backlog from the qualified
downstream actor/storage path. Continue the remaining wrong-event/attempt-deadline and
uncertain-clear-expiry combinations plus full-actor unclean reopen; the current
six service cases now cover forced/expired Source, new-lock late completion and
held-clear Stop/storage reopen refusal. Real retained history now composes behind
the private test auth/Source; continue that path into the normal installed Qt app
and original compositor. Keep the real-history-specific stress and early-failure
gates separate. Keep authentication construction/ingress test-only until genuine
whole-owner completion provenance and trusted locker handoff are independently
qualified. Do not substitute models or ARM harness passes for authentication.
Preserve the queued ARM setup failure without relaxing its predicate. Real
authentication binding and normal Messages reopening remain separate completion
gates; no private-auth fixture establishes them.

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

## Native battery kernel and client checkpoint

The standalone `native-battery` service now passes seven component groups and
host/ARM64 release builds. A real kernel `test_power` integration exposed repeated
identical TYPE fields in actual uevent data. A new compiled regression failed,
then the parser was narrowly repaired to allow identical TYPE while refusing
conflicting TYPE and repeated capacity/status. Original collection/publication
deadlines, held-reader shutdown and late-success fencing remain unchanged.

An actual three-file deployment now passes installed PID1 startup and a fresh
automatic cold boot in offline native x86 KVM guests. Both observe73% charging,
51% discharging and kernel PRESENT0/service unavailable in the same Main. Stop
records normal Main exit0, empty cgroup and dead Main/reader pidfds. Cold startup
has a fresh InvocationID without install/reload/enable/start. An intermediate
fixture used numeric presence input where the driver expects true/false; that
failed case and its unrelated bytecode input mutation remain recorded separately.
See `native-battery.md` and the kernel hash-only provenance.

The shared battery client now owns readonly socket I/O, peer validation, schema,
BOOTTIME request deadlines and expiry in Rust. Eleven real Unix/C ABI groups pass
after actual initial and corrective RED. A native Qt6.12 build links all three
Rust engines, compiles the actual thin battery adapter, passes the nine original
Messages C ABI groups and verifies protected installation/loader dependencies.
The app header uses existing Omarchy font/color tokens. See `native-battery-ui.md`.
Existing QML composition tests now pass all three original groups after an
actual creation assertion RED and wiring the real battery adapter into the
fixture. The first graphical battery attempt failed its setup executable-hash
check before any app/window/capture; its guest closure is false due to a driver
unload cutoff and custody error. Outer QEMU/container shutdown does not upgrade
that result. The next fixture qualifies readiness/Stop/closure and maps the normal app,
then fails before capture at a dictionary binding shadowing identity(user). The
following run fixes only those bindings and passes all three original capture
states in one app/window/service. Root visually verifies the native original
labels and byte-copies approved1080×2424/~422PPI/scale2.625 images. Source and
production deadlines remain unchanged; current Tokyo Night uses genuine Omarchy
values, with older qualified session/Source providers explicit. See
`native-battery-ui.md` and graphical provenance. Cold graphics, full current
cohort/theme accessibility and all physical gates remain open.

The public owned-process helper is separately repaired after actual timeout RED.
Five real-child cases now preserve failed original deadlines while obtaining
actual wait/PIPE EOF under the original cleanup budget; regular stderr retains
identity/hash and closes after wait. An earlier finish deadline also controls
its timeliness flag. Incomplete custody retains handles. Historical kernel
proofs still name helper350577, not this successor. See the current helper
provenance and focused test beside `owned.py`.

Next product slice after the battery tracer: display brightness with shared
Rust ownership, an explicit device backend, readback and unavailable behavior,
while preserving the existing suspend authority. Keep the parked auth/Source
investigation parked. ARM graphical startup remains unqualified.

Battery observation supplies no suspend or charging authority. The earlier auth/
Source investigation stays parked; no permission freshness rule changes.

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

## Current cold graphical checkpoint

One current [cold Omarchy/Sounds trace](native-cold-omarchy-sounds.md) now passes
on x86 KVM with genuine shell, real Source/focus and Pixel 10 capture. Actual
touches on Speaker, 50% and Play produce measured 440 Hz on speaker and quiet
earpiece; operator revoke retires the same Main-owned producer and yields paired silence.
The new 96,000 measured frames and initial 48,000 silent frames pass. Source
remains healthy, and normal Stop and child, VM and container closure pass. The
public screenshot shows 50% selected before Play, with Requested/Applying output and the development warning visible.

Two earlier Play attempts lost the producer before a full tone window and remain
unexplained. The passing run retains the same `a499d675…` normal diagnostic ELF,
functional consumer and oracle as the immediately preceding failed diagnostic.
Only postcleanup journal collection changes; this is no functional repair or
repeatability claim. Next: identify the intermittent retirement under the same
budgets, then complete the genuine menu/close/reopen and interactive preview path.
User-facing consent, live theme changes, authentication and physical gates remain
open; earlier failed evidence stays preserved.

The next launched repeat stops at actor metadata after Source rejects a
102,441,987 ns sample interval; it reaches no boundary step, Play or PCM. Closure
and unchanged-input/exact-container checks pass. This adds a Source prerequisite
failure, not an audio result or scheduling diagnosis. A preceding file-mode
preparation failure starts no owned host helper, container or VM. Source freshness
architecture is under review; no code repair has been selected.
