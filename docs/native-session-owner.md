# Native Sounds session owner

The new `native-session-owner` Rust entry starts and owns the compositor, then
composes the existing Source Observer, lifecycle actor, per-app Wayland supervisor
and sole audio client. A fixed desktop request can launch the installed Sounds
profile, but cannot grant permission, choose an executable or restore a failed
Source. The initial host cohort passes32 cases; the later policy-directory
repair passes15 current preflight cases and two actual native worker groups.
The other24 original cases are reused, without a39-case current aggregate.
Host and ARM64 binaries compile; the combined original-compositor ARM session
remains the next runtime gate.

## Ownership and installation

SourceOwner retains the actual compositor Child before every fallible postspawn
operation. Its credentialed private socket crosses a fixed internal bootstrap;
root-parent credentials and pidfd are checked, supplementary groups are cleared,
UID/GID become1000, parent-death signaling is rearmed, and the same PID executes
the trusted compositor. Only the selected Source descriptor crosses that exec.
There is no raw descriptor launch CLI or automatic compositor replacement.

First valid Source readiness has one original BOOTTIME deadline, capped at120s.
Bootstrap receive has a separate5s cap; later Source samples keep their existing
100ms lease. The host startup test proves allowance admission, not cold Hyprland
performance. Terminal Source observations cannot recover by later samples.

The owner polls without blocking logging. Stop fences local facts and immediately
signals KILL; occupancy clears only after actual child wait. Every returning outer
setup path drains the same retained owner under one original cleanup cutoff.
An unresolved occupied owner aborts rather than detaching. Existing actor/storage
fail-stop paths are not graceful-return cleanup. This qualifies the direct leader
Child, without claiming native reaping of all compositor descendants.

The Sounds display Factory accepts its configured original Wayland path before
it is bound. An admitted launch connects nonblocking, checks the actual peer
against the retained compositor PID/UID/GID and pidfd, and starts the existing
owned SDK. Only a fresh per-app listener is given to UID11042. Listener withdrawal
precedes SDK termination even when withdrawal fails; occupancy lasts until reap.

Root-installed strict configuration binds one Sounds profile, exact six app
arguments, private UID1000/11042 HOME/runtime directories, protected installer
paths and a bounded environment. The `policy` path must be an existing root-owned
0700 directory, which the storage worker uses for its policy and owner seals.
Missing paths and filenames are refused before compositor spawn. Unknown
permission, unlock and Source fields are refused. The same binary dispatches existing app-entry, storage and display
workers. `--stop ABSOLUTE_RUNTIME` uses fixed typed bounded IPC directly, without
reading session configuration, profiles or HOME files.

The [crate instructions](../native-session-owner/README.md) describe building and
staging. Actual Make install stages two native binaries, the desktop file, unit
and two examples with exact hashes/modes. It activates no unit. Operational
accounts, policy, matching Source plugin/compositor configuration, app assets,
appearance and audio services still require provisioning. The unit's cgroup kill
policy is metadata, without an executed systemd or native descendant-cleanup claim.

## Executed boundaries

| Boundary | Behavioral RED before source repair | Final host cases |
| --- | --- | --- |
| Source Child/Observer | Two controls pass, four Unsupported failures; later one original-startup-window InvalidInput failure |7 |
| Fixed configuration | Six controls pass, valid selection Unsupported; later one missing display-environment-key failure |8 |
| Returning outer cleanup | Three retained-child/drain failures |3 |
| Sounds display Factory | Four controls pass, three missing Sounds-role failures; later actual cold-path ENOENT |8 |
| Native Stop | Typed IPC control passes, native Stop fails before connection when configuration is absent |6 |

Frozen behavioral cases precede their source-only repairs. Common extraction,
existing app-entry dispatch and install metadata do not receive manufactured RED
claims. The final display/Stop fixture constructors acquire Child custody before
fallible descriptor setup; their14 functional test bodies remain unchanged.
That partial-setup failure branch is source-reviewed, without forced execution.

Final host tests use real Linux credentials, pidfds, child waits, filesystem roles
and Unix sockets. The display peer receives actual SDK registry requests while
remaining Preparing; no invented Ready or original Hyprland result is claimed.
Stop executes the selected default native binary, including caller/reply UID
refusal, strict arguments and held-reply timeout. Observed timeout completion
slightly exceeds2s; the original admission deadline is not extended.

The first final runtime passes configuration8, Source7 and cleanup3, then all
eight display cases fail setup because `/run` is read-only; Stop is unreached.
That aggregate remains failed. With unchanged source/artifacts and a writable
private `/run`, only display8 and Stop6 run and pass. Thus32 cases pass across the
two runs, without a claim that the first aggregate passed or rerunning its18 passes.
All selected containers actually exit, are removed, and retain the same daemon;
CLI waits and applicable PIPE EOFs finish before their original cutoffs. Source
children use DEVNULL, so they have no child PIPE EOF claim. Display records nine
actual peer waits and Stop records eleven child waits with both PIPE EOFs.

Formatting, host all-target Clippy and strict ARM production Clippy pass. Host
helpers/tests/default owner/client and ARM64 default owner/client builds succeed.
The build aggregate then fails only because its Rust image lacks Make. Separate
actual Make staging in the existing Qt build image passes. Both setup failures
remain preserved, without behavioral RED or production repair claims.

The [hash-only evidence](evidence/native-session-owner-provenance.json) binds
final sources, artifacts, compiler messages, red/green qualifications, staged
files and actual runtime receipts. Private binaries and raw artifacts are not
published. Independent production-source review found no concrete blocker; it
adds no runtime evidence.

## Policy-directory correction

The first composed ARM attempt to reach the owner exits with `InvalidData` before
normal Qt or PCM. Its selected policy path ends in `state.json`, while existing
`storage::Disk::open` requires a directory. Two preceding attempts fail only in
the probe readiness and duplicate installer-write setup. All three remain failed;
actual QEMU/CLI exits and exact container absence are recorded independently.
There is no new ARM Source/window/audio qualification from these attempts.

The original preflight checked only the policy parent, and the published example
also selected a filename. Seven new fixed tests first produce two passing
controls and five invalid-selection acceptance failures. Only two production
lines change to reuse `private_directory(policy,0)`; all15 configuration cases then
pass. The original eight functional bodies remain unchanged, with their common
fixture corrected to a real root0700 directory before freezing the seven new
cases. Package instructions and the example now use that directory contract.
Storage semantics and unrelated Source/display/Stop code do not change.

Two actual default-native-worker groups independently exercise the existing
same-build `--storage-worker` entry over credentialed SEQPACKET. The directory
loads an authored locked policy, holds its actual exclusive flock and writes
matching live/clean epoch and policy-hash seals on Shutdown. A regular filename
returns Load failure/state-null with no false readiness or owner markers; its
Shutdown also fails as a request, while the worker itself normally exits0.
Both real children are waited and reach their merged PIPE EOF within original
cutoffs, followed by parent ECHILD. This is reuse integration, without a
manufactured feature RED or Source/full-session claim. The corrected final host
binary repeats these two cases; strict host/ARM64 builds and six-file Make staging
also pass. The other24 original host cases and prior Qt32 are not rerun.

[Correction evidence](evidence/native-session-policy-directory-provenance.json)
binds this new source, fixed tests, actual worker results, artifacts and failed
ARM selections separately. The corrected combined ARM runtime remains open.

## Cold-start Source failure

Two further generic ARM selections use the corrected production owner and
original Hyprland/Source. Both reach the owned UID1000 compositor but fail at
Pixel10 profile control before normal Qt or PCM. The second retains a dead
compositor pidfd and absent process before any cleanup Stop. Policy readiness and
the accepted Stop reply do not establish Source health. Actual owner wait0,
QEMU/CLI exit0 and exact container absence are process closure, independent of
these failed functional results. One guest audio-stop cleanup error is preserved.

A separate private owner adds a bounded summary only after Source drain and its
actual child wait. It records first terminal `TimedOut`, last returned sequence1
and sample age120,921,124ns against the unchanged100,000,000ns lease; actual child
exit is signal9. The selected owner fences and kills a failed Source. This
instrumented epoch remains failed and does not qualify production behavior or
retroactively diagnose the two preceding runs. Existing compositor logs end
during graphics initialization; the exact blocking startup operation is unknown.

The original adapter publishes synchronously during `PLUGIN_INIT`, then arms its
25ms Wayland timer. Focus/lock callbacks can also publish before first dispatch.
The [startup repair](native-source-timer-startup.md) now gates every publication
until an actual timer callback, while retaining conservative state observation.
A genuine early-publication RED precedes two unchanged real Wayland/socket ARM
GREEN cases and matching adapter SDK compilation. The fixture must complete heavy
display-profile setup before Source activation. The shared production publisher tests exercise actual Wayland dispatch and
its credentialed packet boundary, including permanent expiry after a real later
stall. Next replay original Hyprland with the unmodified production owner. The100ms freshness lease,
original120s first-sample allowance and terminal failure remain unchanged.

[Startup evidence](evidence/native-session-startup-provenance.json) binds the
failed selections and diagnostic separately. No new normal-window capture,
virtual PCM or physical phone qualification follows.

## Gated original-compositor replay

The first original Hyprland replay after the timer-gate component GREEN uses the
unmodified public Rust owner and published adapter/header. All three native
plugin builds pass. A private one-shot ready listener loads the fixed physical
profile before loop entry, with an exact literal-path permission rule. The
fixture's existing120s readiness wait additionally requires a bounded complete
ordered receipt before measurement. The preceding selected fixture without that
wait remains preserved and unexecuted; no feature RED is claimed for setup repair.

The Pixel10 metadata probe exits255 with empty output before Sounds or PCM. The
retained compositor pidfd is dead and its process absent before cleanup Stop;
the cause remains unestablished. Mandatory ordered/applied-receipt prerequisite
checks reached the probe, but those raw receipts were not copied to results
before failure. No ordering timestamps, native display qualification or current
Source health is inferred. The existing owned log has10,498 bytes and the exact
PID crash report is absent; its tail does not establish the blocking operation.

The run remains failed.34 actual guest direct waits and their applicable merged
PIPE EOFs meet original cutoffs, while the aggregate enforcing expected exits
fails on probe255. Two adopted subreaper waits retain their own statuses without
PIPE claims. Actual owner wait0, Stop0, QEMU/CLI0, exact container absence and input
closure qualify separately. They do not convert failed setup into success.
[Failed replay evidence](evidence/native-session-gated-startup-provenance.json)
records those distinctions; the two timer component GREENs retain their scope.

## Observed Source status

The existing root-admin Snapshot now reports bounded [Source observation history](native-source-observation.md):
state, last actually accepted sequence/sample BOOTTIME, and the first categorized
failure with actor-observed BOOTTIME. `observed` is historical acceptance, not
current health; `ready` remains policy/storage readiness. Static ErrorKind labels
include an explicit `Unclassified` fallback. No raw facts, PID, permission,
freshness or Source replacement behavior is added.

Four actual host admin-IPC cases and four local Authority/Provider cases pass
after eight missing-field RED failures with unchanged tests. The IPC cases use
real SourceOwner/Observer, an authored UID1000 producer and production actor/storage
code. Local rejection/missing/reconciliation/Stopping cases retain that narrower
scope. Actual waits, applicable PIPE EOFs and original cutoffs qualify separately.
The [provenance](evidence/native-source-observation-provenance.json) keeps focused
host evidence, matching strict host/ARM default-owner/client builds and the
current default library68-pass/20-ignored regression separate from composed ARM runtime.

On a matching replay, retain an authenticated Snapshot before cleanup Stop when
possible under the existing observation cutoff. The first record is not a causal
diagnosis, and the service may exit before any post-Stop inspection. The preceding
original-compositor failure remains failed and unexplained.

## Production Source observation ARM replay

A matching replay of the published observation feature still fails at the extra
Pixel-profile metadata window: Qt exits255 with empty merged output before
normal Sounds, all four boundary steps and PCM. Unlike the preceding unexplained
selection, this epoch retains actual authenticated pre-Stop Source status.

The initial root reply observes accepted sequence2 with no failure. The later
reply retains sequence37, sampled at147685962781ns, and the first
`provider_error / TimedOut` observed by the actor at147800206442ns. The computed
sample age is114243661ns against the unchanged100000000ns lease. The same actor
replies as root while `ready` is true and `locked` is true; policy readiness does
not imply Source availability. The compositor pidfd is already dead and its proc
entry absent before Stop. The snapshot fits the original shared2s observation
cutoff and exports no raw Source channel or packets.

The metadata probe's original30s budget begins, by derivation, at146872768233ns;
its actual wait255 is at148027297369ns. It creates an additional fullscreen raster
window. That overlap does not establish the cause of the publication gap, and
neither this observation nor the earlier private diagnostic identifies a blocked
operation. Do not retroactively diagnose the preceding selection.

All34 guest direct waits and applicable merged PIPE EOFs meet their original
cutoffs individually. Their expected-exit aggregate remains false on probe255.
Three guest subreaper statuses0/1/-13 are retained separately. Native Stop and
owner waits are0; the compositor Child's own wait/signal is not exported. Boot's
three waits/QEMU0, host's seven waits/both PIPE EOFs, DockerCLI0, same-daemon exact
container absence and complete input closure pass independently of functional
failure. No metadata qualification, new native capture or PCM succeeds.

The [hash-only replay evidence](evidence/native-session-source-observation-arm-provenance.json)
binds the current production owner/client, plugin builds, exact failed selection,
raw result hashes and independent qualification. Raw ordered/applied profile
receipts were again not retained before the later failure; prerequisite checks
must not be promoted to a final display qualification.

The following fixture revision removes the extra presenting measurement window.
It collects nonpresenting output metadata, then retains every original Pixel10
guard against actual normal Sounds frame/window metadata before captures or PCM.
It rejects output changes and preserves deadlines/the100ms lease. This is fixture
correction, without a production-feature RED or scheduling-repair claim.

## Non-presenting discovery replay

The next unchanged-production replay removes the extra raster measurement window.
The [derived discovery tool](../tools/native-phone-display-profile/qt-output-discovery.cpp)
keeps actual QGuiApplication/QScreen, Wayland output and DRM observation, creates
no QWindow, and compiles against the retained ARM SDK with strict warnings.
The fixture defers every original Pixel10/window guard to the actual Sounds frame
and current output before capture/PCM; the measurement consumer stays unchanged.
This is fixture correction, without a production-feature RED claim.

Discovery itself nevertheless exits255 with empty output before normal Sounds.
The first root snapshot accepts sequence3; before Stop the same actor reports
accepted sequence48 at183662962010ns and first provider TimedOut observed at
183776721161ns, a113759151ns sample age against100ms. The compositor is already
dead. The removed probe-window painting is therefore not a necessary trigger;
this does not exclude other rendering or identify the unavailable operation.

Early raw profile receipts are now retained: ready callback182146741813ns,
physical-load completion182256753645ns, and actual1080×2424/65×146mm/scale2.625
application. These receipts are setup evidence; final output/window measurement,
new capture, all four steps and PCM remain unqualified. The derived probe's
strict compile0 is separate from its failed runtime255.

All36 guest direct waits/applicable merged PIPE EOFs, one shutdown wait, three
boot waits/QEMU0 and seven host waits/both PIPE EOFs meet their original bounds.
Expected-exit aggregates remain false on discovery255. Stop/owner waits0 and
same-daemon exact container absence/input closure qualify separately; the
compositor Child's numeric wait/signal is not exported. Adopted0/1/-9 waits retain
their own scope. [Hash-only evidence](evidence/native-session-nonpresenting-discovery-provenance.json)
binds these failed facts, sources, retained receipts and independent reviews.

Stop fixture guesses here. Next obtain one bounded timing witness for the
original public configuration/property-refresh events, without another plugin
load, modified owner, Source packet access or lease change. Event landmarks can
order startup work against expiry; they do not by themselves prove exact callback
return, private refresh entry, an empty deferred queue or a cause. Choose a repair
only from the resulting evidence.

## Startup event witness replay

One diagnostic witness extends the existing read-only focus plugin, without an
extra plugin load, modified production owner, Source packet access or lease change.
It compiles against the actual matching guest SDK. A preallocated4096B tmpfs map
retains the earliest128 numeric records; complete records precede lock-free release
publication. Overflow, clock error and retirement are sticky. This is a cooperative
UID1000 fixture, not an authenticated audit channel.

The guest reads the map only after its retained compositor pidfd reports dead,
before Stop, within the original failure-observation2s and WORK. Both independent
readers decode the same13 committed records: armed, five preReload, five reloaded
and two scheduled props_refreshed landmarks. Flags are zero and the uncommitted
tail is empty. Raw memory stays private; [hash-only evidence](evidence/native-session-startup-event-witness-provenance.json)
binds the exact proposal, source review, selection, runtime and qualifications.

The last observed props_refreshed event is183439178490ns. Source later accepts
sequence50 at184793650204ns,1354471714ns after that event, then first reports
provider TimedOut at184912831352ns:119181148ns sample age against100ms. The
nonpresenting discovery again exits255/empty before Sounds. No final display,
new capture, functional step or PCM succeeds. Completion landmarks do not prove
exact callback return, private refresh entry, an empty deferred queue, in-flight
work or the cause of Source expiry. These observations do not justify delaying
activation until those events as a production repair.

All four compiles return0. All36 guest waits/applicable merged PIPE EOFs, one
shutdown wait, three boot waits/QEMU0 and seven host waits/both PIPE EOFs meet
their original cutoffs. Expected-exit aggregates remain false on discovery255.
Stop/owner waits0, same-daemon exact container absence and input closure qualify
separately. The compositor Child's numeric wait/signal remains unexported; adopted
0/1/-9 statuses keep their own scope.

The retained untimestamped log identifies virgl/LLVMPIPE software rendering; its
last line does not identify the later failing call. Preserve this ARM failure and
prepare matching-source native CPU-speed composition on Harbour rather than
another speculative startup change. Keep Pixel geometry, actual window/focus,
independent grants, all PCM assertions and100ms freshness. A full host pass would
qualify host composition only; ARM availability, TCG causality, the matched Google
kernel and physical phone gates remain open.

## Native CPU runtime checkpoint

The [native x86 KVM runtime](native-session-x86-runtime.md) now passes one actual
offline stock Arch kernel/bootstrap/poweroff selection. Authenticated same-QEMU
QMP confirms KVM enabled/present and guest shutdown, then socket EOF and actual
QEMU0. Serial identity/hash, all owned cutoffs and exact container/input closure
qualify. Matching-source Rust payload builds pass separately; no Qt, compositor,
Source or PCM executes in this checkpoint. A later [offline guest install](native-session-x86-runtime.md#offline-installed-guest)
now qualifies367 signed packages, exact actual Hyprland binary/SDK commit and
audio tools/SPA files in a fresh closed layer. Native Qt build and the full
functional run remain next.

## Remaining integration

Complete native CPU-speed execution of the same-source production owner and
original Hyprland/Source, fixed desktop client, normal UID11042 Sounds,
independent permission and real native focus, then return to ARM availability. Keep
per-app Wayland and independent virtual graph/PCM. Qualify null-focus background
continuation and terminal Source loss without app/audio revival. Earlier ARM
Sounds evidence uses root-authored focus and a separate selected actor cohort.
Trusted lock/restoration, complete Omarchy shell/startup, the other smartphone
services and matched Pixel graphics/audio/modem/camera/suspend/carrier operation
remain separate gates. No physical phone has been accessed or qualified.
