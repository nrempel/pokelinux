# Native display brightness

The current [ARM battery/brightness observation pair](native-installed-power-observation.md)
now passes actual missing-unit RED, production seven-file staging and a distinct
fresh automatic cold boot. The same brightness Main follows real virtual-GPIO
presence→absence→recovery through the typed Rust client; battery completes an
honest unavailable observation. Both PID1 services Stop normally with empty
cgroups. This readonly headless pair does not inherit the earlier kernel Set or
x86 graphical results and supplies no Pixel hardware qualification.

The new Rust brightness component separates deployment-selected backlight I/O,
local observation/intent ownership and the lifecycle actor's app permission.
The production daemon and its concrete Rust client now pass joined native-host
process/socket tests against authored SyntheticTree attributes. A lifecycle
Provider adapter and explicit installed configuration have separate qualified
host evidence. A first prepared normal Settings frame now reads through the actual
actor and installed service. The current normal client now passes a same-app five-state
trace against authored attributes after the Rust actor reservation/readiness fixes.
Earlier failed traces and the older successful client retain their own cohorts. These GUI cohorts stay separate from the new generic
ARM kernel/PID1 qualification of the actual LinuxBacklight driver interface.
Cold startup, all-click reliability and physical display behavior remain open.

## Attribute backend

A deployment explicitly chooses LinuxBacklight under `/sys/class/backlight` or
SyntheticTree for authored test files. Linux selection requires the fixed class
path and sysfs filesystem; it never falls back to a synthetic tree. Applications
cannot supply a device name or filesystem path. The component tests never open
host sysfs or a physical phone.

The backend reads bounded canonical decimal attributes: positive max_brightness,
brightness in range and optional actual_brightness in range. Missing optional
actual is distinct from malformed input. The Linux ABI distinguishes the requested
brightness attribute from actual_brightness; even the latter may be the driver's
stored value when hardware readback is absent. Neither is a luminance measurement.
See the [upstream backlight documentation](https://docs.kernel.org/gpu/backlight.html).

An explicit percent is converted with bounded integer half-up rounding. Set
validates the input attributes, opens the existing brightness attribute, makes
one write attempt and reads attributes independently afterward. A short or failed
write is never resent. A changed maximum or mismatched requested readback refuses
confirmation without rescaling/retrying. Attribute reads are sampled, not atomic
with external driver changes; the local expected revision is not sysfs CAS.
The caller must retain this potentially blocking operation in a child. The
separate real-worker tests below qualify custody; these backend tests alone do
not establish it.

## Local ownership and freshness

One ticket occupies the owner until supplied retirement. Set invalidates the
previous observation/revision on admission. Cancellation or original BOOTTIME
deadline expiry withdraws publication immediately, while retaining occupancy;
a late successful result cannot confirm the operation. Only a matching ticket
can release its slot. A fresh readonly observation after retirement may establish
current attributes, while preserving the earlier Set's uncertainty. A new write
requires a new explicit intent and a current positive readback revision.

The controlled-clock tests use one-second collection deadlines and observations
expiring two seconds from collection start. Calling retired in these tests is a
policy input: it proves no process wait, pipe EOF or kernel suspend behavior.

## Stable value revisions across readonly refreshes

A revision now versions usable observed attributes rather than the number of
reads. A successful identical readonly refresh preserves the revision only while
freshness is uninterrupted, and renews collection expiry separately. Changed raw
attributes, Set admission, expiry, unavailable state and cancellation still
invalidate the previous version. Read worker occupancy remains busy; preserving
a version does not admit concurrent writes or extend a caller's original deadline.

The frozen real Rust Client/normal Release daemon regression first fails at an
actual remote decision: two unchanged completed reads advance revision2→4→6,
then reject one explicit Set using the still-fresh cached revision2. The authored
requested attribute stays200. After the model-only repair, the same test observes
revision1→1→1 and one Completed Set75%, with independent raw300/max400 and
unchanged optional actual120. Raw200→201 still invalidates the version despite
both values rounding to50%. Actual expiry, missing-attribute and cancelled-write
gaps retain new revisions, uncertainty and no replay.

One full Release suite passes52 tests:32 backend/owner/deadline/protocol/client,
six composed Client/Main, eight service and six worker groups. Format, strict
all-target Clippy and normal host/ARM Release builds pass. The host service is
executed; ARM is crossbuilt only. The composed/service/credential controls close
16 retained processes with wait0 and32 PIPE EOFs; the outer runner closes10 CLI
waits/20 EOFs, unchanged inputs and exact container absence. No GUI cause is
inferred from this independent defect. See the
[readonly-version provenance](evidence/native-brightness-readonly-version-provenance.json).

## App permission and lifecycle

The lifecycle actor now understands DisplayBrightness for the fixed
org.omarchy.Settings principal, UID11050. The session configuration now validates
this fixed profile when its brightness owner is selected; this does not itself
install, launch or grant the app. Typed brightness_set and brightness_status requests
carry a percent, expected readback revision and bounded lifetime, without device
identifiers or app-supplied capabilities. The configured provider entry explicitly
supplies the backend; unconfigured entries return Unavailable.

Existing registered-principal, permission, Source/session, foreground, lock,
background and suspend checks stay unchanged. Dispatch rechecks the current
readback revision and occupancy. Publication rechecks the original authority and
readback deadlines on every retry. Provider inspection may establish a fresh
readonly connection; a failed Set submission is terminal and never automatically
reconnected, queued or resent. Pending backend custody remains distinct from
whether an app reply is still publishable. Reaping cannot undo a property write.
The separate native-suspend authority and readonly battery service are unchanged.

## Actual RED and GREEN

All14 backend/policy groups compile against the stub:12 assertions fail and two
negative rejection groups already pass. The same frozen tests then pass: seven
actual synthetic regular-file/class-symlink I/O groups and seven controlled-clock
ownership groups. Literal rounding vectors include u32::MAX. A deliberately
hardlinked synthetic maximum/brightness topology changes the maximum during the
single write; actual post-readback refusal and final75 establish no rescaled
retry. That topology is authored filesystem evidence, not a kernel driver.
Format, strict all-target Clippy and optimized host/ARM64 library builds pass.
ARM64 is a crossbuild only, not an executed service. Each component RED/GREEN
container closes10 actual host CLI waits and20 PIPE EOFs with original bounds,
container absence and selected inputs unchanged. See the
[component provenance](evidence/native-brightness-component-provenance.json).

The lifecycle first preserves API-absence compiler errors separately, then runs
a valid compiled consumer: existing Query/grant controls pass, while the new Set
and grant assertion fails at runtime. An initial nine-group implementation passes.
A further compiled regression then exposes two failures: stale unsent revision
and expired queued readback. A brightness-session-only repair makes both controls
pass. The final ten focused groups, frozen consumers, formatting, strict Clippy
and host Release build pass. Eleven adjacent media and four Source groups pass
on the first candidate; their sources and common predicates are unchanged by the
later brightness-only correction. Controlled authority/provider inputs and actual
local SEQPACKET EOF, extra-packet, EAGAIN and credential boundaries stay distinct.
Across all five lifecycle epochs,50 host CLI waits/100 PIPE EOFs close. These
checks do not execute the full actor with a configured backlight. See the
[lifecycle provenance](evidence/native-brightness-lifecycle-provenance.json).

## Retained worker and exact caller deadlines

The worker now owns one actual exec child and an atomically nonblocking/CLOEXEC
pipe. Safe child entry installs PDEATHSIG KILL, checks the exact parent and the
original BOOTTIME deadline before backend I/O, then checks again before output.
Main-side cancellation withdraws the result before kill. Wait and EOF are tracked
separately; neither alone allows a result. The fresh result getter independently
checks deadline expiry. Unreaped Drop kills and aborts rather than detaching;
service callers must retain custody through actual retirement. Trusted local
executable setup and default SIGCHLD remain deployment assumptions. Permanently
blocked exec setup or an uninterruptible kernel operation is not qualified.

Six compiled real-process groups first fail against the worker stub, then pass
unchanged. They exercise actual FIFO blocking, responsive poll/fence, independent
wait/EOF controls, bounded canonical child output, nonzero exits, no-child failed
admission and a successful zombie whose buffered Set result expires before it
can publish. All21 spawned children are actually waited/drained with ECHILD and
process absence. Child PDEATHSIG/parent setup is observed, but actual parent death
and unreaped-Drop abort are source-reviewed guards, not induced test results.
The initial test-API compilation failure and a later wrapper exit-expectation
failure remain separate from the qualified runtime RED. Format and strict worker
Clippy pass.

A separate caller-deadline regression compiles against an ordinary-one-second
forwarding stub: three deadline assertions fail and the ordinary budget control
passes. Six packet-policy groups add five positive failures and one already
passing strict-serde control. The repaired owner now uses the exact earlier
caller deadline, while observation expiry stays anchored to collection start.
Strict private packet types have no device/path fields, require canonical decimal
BOOT deadlines and all nullable fields, and distinguish requested from optional
actual attributes. All24 selected backend/ownership/deadline/packet groups then
pass with frozen tests, format checks and strict all-target Clippy. This phase
validates authored bytes and clocks, without executing a socket daemon or worker
harness. The generic RED wrapper's erroneous compile/setup label is preserved;
a separate qualification records the actual successful compilation and assertion
failures without upgrading that wrapper receipt.

The final worker's formatting-only successor is selected in a fresh whole-crate
format check and optimized host/ARM64 library build. Worker runtime and24-group
policy execution retain their separate exact source cohorts. Neither library is
a daemon executable, and ARM64 is crossbuild evidence only. See the
[worker/deadline provenance](evidence/native-brightness-worker-provenance.json).

## Readonly reconciliation while custody is retained

Integration review exposed a scheduling stall: cached provider pending=true could
suppress every later readonly Inspect after the actor retired its local intent.
A compiled controlled-provider regression first records zero reads while remote
custody is pending; its existing idle control passes. Removing that one cached
pending guard lets the adapter attempt readonly inspection. Actual transport
occupancy still returns false without queuing; actor pending, Stop,250ms cadence
and the original one-second inspection cutoff remain unchanged. The same test
then passes with11 focused brightness groups, strict Rust checks and host Release.
Cached pending remains true until a new observation; no process retirement or
cleanup is inferred. This is scheduling evidence, separate from daemon transport.

## Actual daemon and root-actor transport

The normal Rust Main owns a private root:root0700 runtime directory and0600
SEQPACKET endpoint. Its fixed protocol accepts no app-selected device, path or
backend. Deployment supplies an explicit backend/class/device selection. Root
SO_PEERCRED is verified at accept; every packet must carry matching PID/UID/GID
SCM credentials. Truncated frames, extra descriptors, invalid JSON and pipelining
on an occupied request close the connection. A verified persistent peer can first
Inspect, then submit one explicit Set with the observed revision. Up to16 bounded
peers can read unavailable/pending status while one global worker stays retained.
Peer loss, original deadline and SIGTERM fence publication before worker polling;
no successor is admitted until actual wait and pipe EOF. There is no queued reply
or Set resend after partial/backpressured transport. Stop drains custody and
removes the socket. Killing/reaping is not property rollback.

Six service cases first compile and fail against an Unsupported Main, then pass
unchanged. Review then found a distinct write-boundary defect: with Main stopped,
a child could finish a blocked preflight read after its250ms deadline, write300,
and only afterward reject its result. The actual regression records300 versus
required200 while retaining and reaping the child. Its unchanged successor now
records200: the production setter checks the immutable BOOTTIME cutoff after
preflight, immediately before the potentially mutating open, immediately before
its single write, and around readback. No retry is introduced. Opening synthetic
regular files with truncate is itself a mutation, so the pre-open check matters.
The clock check and syscall are not atomic; an already admitted kernel write may
finish after expiry, and cancellation cannot guarantee no later device effect.

The joined backend/owner/deadline/protocol/worker/service epoch passes38 groups,
including the original six service cases, this late-preflight regression and an
actual inherited-root-socket writer at UID11010. The inherited descriptor grants
no per-packet write authority, even while normal private socket modes stay intact.
All native service cases use temporary files and actual child processes, not a
Linux backlight driver. Format and strict Clippy pass. Optimized host/ARM64 service
ELFs are built and inspected; those exported Release files are not executed. The
actual integration cases run the same-source normal Cargo test-profile binary.
The cohort deliberately retains an unexecuted client stub, so it does not qualify
the subsequently implemented client. Earlier mode/setup/compiler failures remain
private recorded attempts, distinct from genuine runtime RED and GREEN. See the
[service provenance](evidence/native-brightness-service-provenance.json).

## Native Rust owner client

The concrete client verifies the root peer and exact per-packet credentials,
accepts bounded strict replies and samples BOOTTIME after I/O and on every view.
Only explicit readonly Inspect may establish or replace a connection. Set requires
that existing connection, a fresh observation, matching revision and no occupied
request; it sends once. Poll never sends, reconnects or flushes an unsent command.
Actual EAGAIN leaves no command queue. Unknown outcome, expiry, malformed/extra
reply or EOF withdraws state. Known or unresolved custody stays pending across
revoke until a fresh readonly reply establishes retirement. Reconciliation may
report current attributes while preserving the old Set's uncertainty; it cannot
acknowledge that earlier effect. A new explicit intent can act on the fresh state.

Eight actual root SEQPACKET authored-peer groups first compile and fail before
implementation. The first implemented run passes six, while two no-packet test
observers incorrectly reject ECONNRESET produced by closing an unread socket;
their preceding deadline/refusal/disconnection assertions already pass. One
observer branch is corrected to accept that terminal reset while still rejecting
any received packet or other error. The production client is unchanged. With the
same successor consumer, the inert stub fails eight and the implemented client
passes eight; this later stub control is not relabelled as the preimplementation
RED. Strict client Clippy and formatting pass. Tests include actual backpressure,
rights rejection, independent optional-actual readback, original deadline expiry,
unsolicited packets, and cancel→readonly reconnect→busy status→reconciliation→a
new explicit Set. These authored peers do not qualify a daemon connection,
lifecycle grant, backend write, ARM runtime or hardware. See the
[client provenance](evidence/native-brightness-client-provenance.json).

## Composed client, lifecycle adapter and deployment selection

Three additional original-code groups execute the real Rust client against the
normal daemon and retained worker. They cover persistent Inspect→25→75 with
independent optional actual attributes; a FIFO-held Set cancelled into retained
pending state followed by readonly reconciliation without replay; and a worker
that succeeds while Main is stopped, whose queued result the client rejects after
the original deadline. A later readonly request observes current300/max400 while
preserving uncertainty and never acknowledging the expired Set. All three Main
processes exit0 and their stdout/stderr reach EOF. The two held worker lifetimes
are separately observed; their internal wait remains the daemon's responsibility.

The first composed attempt passed two cases, then failed its immediate fresh-view
observer in the third. Its snapshot and cause remain unknown. The production
code stayed unchanged. The successor logs snapshots and permits only explicit
pending replies to trigger another readonly Inspect within one fixed800ms cutoff.
It passes all three; the final observer needs one Inspect, so the repeat branch
is not exercised. Whole-crate format, strict all-target Clippy and normal host/ARM
Release builds pass. Those exported Release ELFs are build-only in this cohort;
the composed cases execute the same-source normal test-profile daemon.

The concrete lifecycle Owner delegates transport to that Rust client. It converts
raw attributes with bounded half-up integer rounding, keeps requested and optional
actual readback distinct, and clears expired values on every getter. Only readonly
inspection connects or refreshes the incarnation. Set forwards once; poll never
sends, reconnects or retries. Five actual root-peer adapter groups first compile
against an inert adapter, with four assertion failures and one passing control.
The same frozen five then pass with the11 existing brightness policy groups,
format, strict Clippy and a host build. These authored peers do not establish a
full actor grant or real backend operation.

The mandatory session `brightness` field accepts explicit disabled mode or an
owner socket. Owner mode requires the fixed UID11050 Settings profile and a
private root-owned endpoint parent, distinct from other configured sockets.
It accepts no app grant, backend or device selection. Four new configuration
assertions fail before implementation and then pass with15 existing config and
eight Messages configuration groups. Existing fixtures now explicitly select
disabled mode; omission has no compatibility default. A first aggregate retains
its separate missing runtime-helper failure before spawn. The successor excludes
that Source/runtime case explicitly; it remains open. Config tests retain their
pre-adapter runtime snapshot and cannot qualify the later runtime composition.
See the [composition provenance](evidence/native-brightness-composition-provenance.json)
for the separate source cohorts and preserved failures.

The normal session entry now constructs the selected concrete Owner and passes
it into the lifecycle brightness service entry. A compiled constructor regression
first passes disabled mode and fails the explicit Owner assertion before any
service lookup or spawn. After repair, the same two tests pass. The positive
case uses the exact optimized host service ELF: Inspect→one explicit75% Set→raw
300/max400 and observed75%, while optional actual120 remains30%. Normal SIGTERM
ends Main0 with both pipe EOFs and same-parent ECHILD. This executes the actual
constructor and typed Provider; it does not invoke the full session actor or
Source. Format, strict Clippy and a current owner Release build pass separately.
An offline scan covers17 tracked lifecycle-dependent lock closures;16 gain only
the brightness path package and dependency edge, with no registry version or
checksum changes. Settings' separate app lock remains outside that selection.

## Normal Rust Settings client and Qt build

The new [Settings application](../native-settings-ui/README.md) runs as UID11050
and uses only the lifecycle actor's public credentialed endpoint. Its Rust engine
owns parsing, transport, fresh BOOTTIME state, a single explicit queued click and
one joined worker thread. A click retains its revision and original local deadline;
a revision change cancels it. Readonly status may reconnect, while an uncertain
Set is never replayed. The transmitted relative TTL is bounded by remaining local
time; actor admission has its own authority/deadline checks. No remote absolute
click-deadline equivalence is claimed.

Qt forwards explicit25%/75% choices and renders the current observed attribute as
“Brightness”, separate from the optional actual attribute and from request success.
A false can_set gives unavailable controls; only an explicit Denied reply gives
access denied. The normal six-argument app loads the shared installed Omarchy
appearance QML and Rust appearance client. It cannot choose a backend or grant
itself permission.

Seven real root-peer/UID11050 socket/C ABI groups first fail at runtime against
the inert client, then pass unchanged. They cover observed74 after explicit75,
malformed and ancillary packets, fresh getter expiry, held-request uncertainty,
readonly recovery without replay, stale queued revisions and actual Stop/join.
Formatting, strict Clippy and the Rust Release archive pass. A disposable x86 KVM
VM then builds/installs the normal Qt6.12 target, checks both exact Rust archives,
strong C exports and the loader, and executes the real adapters/shared QML as
UID11050 with unavailable endpoints. A wording-only successor is rebuilt; both
qualified builds remain distinct. No component window is shown. Full actor/service
composition and visible native Pixel-profile captures remain open. See [app provenance](../native-settings-ui/evidence/qualification.json).

## Fixed Settings desktop launch

The normal fixed `settings-launch` client now sends only `launch_settings` to the
lifecycle actor's dedicated endpoint. Desktop UID1000 can request the installed
Settings profile through the existing Apps lifecycle. The packet accepts no app,
executable, permission, backend or device field. This endpoint exists independently
of brightness availability so Settings can display unavailable state; an absent
profile returns Unavailable. Session configuration still fixes UID11050 and six
absolute arguments. Child ownership, duplicate-launch refusal and Stop use the
existing Apps implementation; endpoint cleanup checks its inode.

Six actual socket/actor groups first fail at the missing endpoint. Two fixed-CLI
functional groups fail against a compiled Unsupported stub while the invalid-argv
control passes. Three frozen GNU Make install assertions then fail against the old
stager. Its first aggregate's missing-make setup errors are retained separately.
After repair, all six route groups, three CLI groups, three install groups and
six unchanged Sounds controls pass. Selected formatting, strict Clippy and normal
host/ARM Release providers pass; these exported ELFs are built and inspected,
not executed in this cohort. A formatting-only intermediate failure is preserved.

The examples-only core install now requires an explicit SETTINGS_LAUNCH_BIN and
stages ten files, adding the fixed native client and original Settings desktop
entry. It does not activate policy or services. The component app is an authored
six-argument UID11050 process, not the normal Qt window; no Source, compositor,
brightness operation or phone is selected. See the
[desktop provenance](evidence/native-settings-desktop-provenance.json).

## First normal Settings frame

The prepared x86 KVM session now starts the normal fixed desktop client and maps
the actual UID11050 Settings window. It reads25% through the lifecycle actor and
installed PID1 Rust service against explicit SyntheticTree attributes100/max400.
The shared Omarchy Tokyo theme is active. Root inspected the original unedited
frame at1080×2424; external compositor/window/output/capture metadata qualifies
the Pixel10 profile with2.625 scale and65×146mm nominal output size. The normal
QWindow DPR is not measured directly. The compositor debug banner and pointer
remain in the capture; this is a prepared prototype session, not full-shell or
phone evidence.

![First normal Settings frame](assets/settings-brightness-first-frame.png)

That earlier full attempt is a failure: its original20-second/eight-capture image observer
expects a button fill different from the background. This theme renders equal
colors with thin borders. The final frame visibly says “Brightness ready”, but
that observer never admits a click. No change/unavailable/recovery steps pass.
The earlier import-setup failure is also retained separately. A detector-only
successor reproduces the old failure on all eight unchanged PNGs and detects
the actual paired button borders while still rejecting the first pending frame.
A fresh VM then admits the initial state and sends exactly one actual75% touch.
The requested attribute fails to reach300 within its original five seconds;
remaining steps do not run. That failure originally exported no post-touch raw
trace, so its cause remains unknown. A separately frozen diagnostic successor
retains bounded raw/inotify samples and original frames without retrying input,
changing production code or extending the functional deadline. The stable-revision
repair above is independently qualified. Its later UI qualification follows below;
the original failed taps' precise cause was not observed.

Installed brightness start and Stop are separate measured evidence: normalMain0,
retained pidfd dead, process/socket absent and empty exact cgroup with no restart.
This does not establish fresh cold autostart or a fixture parent wait for PID1's
child. The actor/audio/appearance owners and VM/container close under their
original bounds despite the failed visual aggregate. See the
[first-frame provenance](evidence/native-settings-first-frame-provenance.json).

## Settings touch, unavailable state and recovery

Replacing only the installed brightness service with the stable-revision build
makes the unchanged prepared five-state tracer pass. The normal UID11050 Settings
app, fixed launcher, lifecycle actor, Source rules, shared Tokyo theme, input
sequence and original deadlines stay unchanged. Root viewed all five original
native captures at1080×2424; no capture was resized, cropped or repainted.

| State | Observed behavior | Original screenshot |
| --- | --- | --- |
| Initial | Current25%, ready; authored requested100/max400 | [25%](assets/settings-brightness-initial25.png) |
| One explicit75% touch | Requested100→300; current75%, ready | [75%](assets/settings-brightness-explicit75.png) |
| Backend unavailable | Remove authored maximum, let prior freshness expire; no displayed percent, muted controls; two actual touches leave requested300 unchanged and no MODIFY/CLOSE_WRITE events | [Unavailable](assets/settings-brightness-unavailable.png) |
| Readonly recovery | Restore maximum; current75% and usable controls return without restarting or sending a fixture Inspect/Set | [Recovered75%](assets/settings-brightness-recovered75.png) |
| New explicit25% touch | Requested300→100; current25%, ready in the same app and service | [25% again](assets/settings-brightness-second25.png) |

![Native Settings after one explicit75% touch](assets/settings-brightness-explicit75.png)

The two successful transitions each have independently sampled raw attributes
and actual MODIFY/CLOSE_WRITE events within the original five-second functional
bounds. An inotify event count is not a write-syscall count. All four inputs are
retained: one75%, two while unavailable, and a new25%; none is retried implicitly.
Optional actual_brightness stays120, distinct from the requested attribute and
from physical luminance. Current readonly recovery does not acknowledge an
uncertain earlier request. The GUI trace does not directly observe the Set ACK;
that protocol boundary has separate actual-client tests.

External compositor, mapped window, registry and native-buffer metadata qualify
each capture's Pixel10 profile:1080×2424,2.625 scale,65×146mm nominal size/about422PPI.
Normal QWindow DPR and direct Qt.enabled getters remain unmeasured. Root observes
muted labels during unavailability, alongside the two no-mutation inputs. The
debug banner and pointer remain visible. This is a prepared session with a warmup,
not fresh cold graphics or the full original Omarchy shell. Test root explicitly
grants DisplayBrightness; no app self-grant or setup-permission flow is inferred.

The installed PID1 brightness service stays in one invocation with no restart,
then stops normally with ExecMainStatus0, dead retained pidfd, removed socket and
empty cgroup. Actor/audio/appearance owners, guest, QEMU and container close under
their original bounds. Root's semantic review is separate from the automated
capture result. The initial missing-admission-field setup failure launched no
container and is preserved separately. Earlier failed taps remain recorded; this
success does not identify which boundary stopped them or prove all input races
resolved. A separate client correction now preserves an accepted unsent choice
through fresh same-revision readonly pending work, under its original one-second
deadline. Four new real-socket cases and the seven existing groups pass after
compiled RED, with explicit earlier fixture-error evidence. That repaired Rust
archive has a separate native Qt qualification and failed current touch attempt
below; these five published baseline images retain the earlier engine. Actor busy
refusal and all post-first-send uncertainty/no-replay rules stay unchanged. See the
[client correction](evidence/native-settings-pending-choice-provenance.json) and
[five-state provenance](evidence/native-settings-brightness-tracer-provenance.json).

## Updated native Qt builds and current touch gate

The accepted-choice client now links into normal Qt6.12 Settings builds on both
native x86 and an actual ARM7.1.6 guest kernel. Native loader/install checks and
UID11050 C ABI plus real adapter/QML construction pass with the unchanged
assertions and200ms owned-thread Stop budget. No ARM window is shown. ARM adds
only the exact eight previously qualified signed build tools to353 packages,
preserving every existing version and all50 selected Qt libraries/plugins.
The two architecture results remain separate from crossbuilds and GUI evidence.

The fresh x86 normal app then fails the first75% change: one touch,116 raw samples
at requested100/max400/actual120, no MODIFY/CLOSE_WRITE events. Root views initial
25 ready, immediate unavailable/pending and final25 ready. The session and app
remain healthy, but the exact Qt click/first wire Set/reply boundary is unobserved.
Only the initial state passes; the earlier five-state baseline is not inherited.
Actor/service/guest/QEMU/container custody closes despite the failed trace.

That client let a fresh readonly response replace the displayed outcome of a tap.
The current Rust correction keeps the last choice outcome separate from current
attributes and controls: known unsent, sent but unconfirmed, or observed actor
Unavailable after send. Only a new accepted explicit choice or matching positive
completion clears it. Denied/Stop retain display priority and clear protected
values. A refused second choice never replaces active intent. The final intent
check, single nonblocking send and sent marker share a short model lock, released
before poll/wait, so concurrent expiry cannot label a full send as unsent. No
write retry, Source policy, cadence or deadline changes. The earlier native Qt
builds and failed trace retain their historical engine. See the
[platform and failed-trace provenance](evidence/native-settings-qt-platform-provenance.json).

Two compiled assertion REDs precede this outcome correction. The final fixture
intentionally changes the earlier accepted-unsent-expiry status0 to not-sent7;
all other prior case bodies stay unchanged. All15 distinct Rust socket/C ABI
requirements pass with actual waits/EOF and original Stop/deadline bounds. The
same aggregate subsequently fails import formatting; an exact import-order-only
successor passes format, strict Clippy and host/ARM normal release archives without
repeating functional tests. Thin Qt text is reviewed, not executed in this run.
The send/expiry lock invariant is reviewed without claiming a deterministic
interleaving reproduction. See
[outcome provenance](evidence/native-settings-choice-outcome-provenance.json).

## Earlier outcome trace: four states, final change fails

A new normal x86 Qt6.12 build/install/offscreen component passes with the persistent
outcome archive. Its fresh actual-touch run reaches initial25→explicit75 with
requested raw100→300→unavailable with no mutation from two disabled inputs→readonly
recovery75. Root views those four original1080×2424 group captures plus the immediate
pending frame. The same app/window and brightness Main remain in place.

The later new25 tap fails its unchanged five seconds:116 raw samples remain300,
with no requested-attribute mutation events and no retry. No final failure frame
exists under the original first-tap-only diagnostic fixture. Its tap outcome,
Qt click/first wire Set/reply boundary and precise cause remain unobserved. This
is a failed whole trace, with four successful current-state groups. Current raw
readback and displayed status do not establish a Set acknowledgement.

All100 direct waits/107 PIPE EOFs close, alongside a separate adopted wait-6 which
is not relabelled as normal app Main exit. Brightness PID1 Stop reports Main0 and
success with dead retained pidfd, absent socket and empty cgroup. QEMU/container
and input closure pass. Earlier clients, failures and the older full five-state
success retain their own cohorts. See
[current native provenance](evidence/native-settings-outcome-native-provenance.json).

![Current normal Settings at75%, from the four-state partial trace](assets/settings-brightness-current75.png)

## First-send reservation behind the actor's own readonly Inspect

The concrete owner now distinguishes its own in-flight Inspect from reported
remote custody. With a fresh retained observation, a new explicit choice can
reserve its first send while that readonly request retires. The actor suppresses
successor periodic Inspects and sends no Set until the reply establishes fresh,
idle state at the original revision. The same connection, authority stamp and
original deadline remain required. Remote pending, changed revision, expiry or
loss terminate the reservation; there is no reconnect, deadline renewal or write
retry. Historical uncertainty from an earlier Set remains distinct from the
new choice and is never turned into an acknowledgement.

One frozen actual Rust test uses the production Session/Owner and a real
root-credentialed SEQPACKET peer under controlled Authority. Before the change,
all four admissions fail with Unavailable. After the change, the unchanged reply
permits exactly one original Set and matching completion; reported remote busy,
changed revision and original expiry each prevent dispatch. Each case closes its
socket, pidfd and storage owners before the final assertion. This reproduces an
actor boundary, not an attribution of the preceding graphical failure. The test
does not exercise an actual Settings UID, app ingress, Source, daemon worker,
backend or normal window.

The current source also passes all52 brightness cases across two epochs,16
existing lifecycle brightness cases, six Settings and six Sounds desktop controls,
and two normal owner-constructor cases. The already executed focused reservation
test is not repeated. Formatting, strict all-target Clippy and normal host/ARM
owner, launcher and brightness releases pass. The first aggregate stops after24
passing cases because its observer expects libtest output from a custom harness;
that harness itself reports six passing groups and exits0. The corrected narrowed
epoch runs only the remaining28 brightness cases and remaining checks. Its
successful closure does not erase the earlier aggregate failure or the interrupted
ARM controller, whose nested compilers are not individually claimed as waited.

The new brightness ELFs differ from the preceding kernel-tested cohort. Neither
the old ARM kernel execution nor a fresh build qualifies these new binaries for
kernel runtime. The graphical experiment retains its frozen brightness daemon
and changes only the qualified actor/launcher plus failure diagnostics; it is not
yet qualified. See [reservation provenance](evidence/native-brightness-readonly-reservation-provenance.json).

## Published readiness follows effect custody

A second actual Session/Owner/socket boundary exposes two public-state errors:
own fresh readonly polling reports pending although it can reserve a first send;
after that read retires, an accepted effect reports idle before first dispatch.
The same frozen append-only test first fails both assertions, while still
executing the refusal and one-Set continuations. The known-remote-busy control
passes. Each case closes its socket, pidfd and storage owners before the final
assertion. The earlier four-case fixture remains an exact byte prefix.

The Rust projection now keeps accepted effects pending and lets an idle control
remain actionable during its own fresh readonly Inspect. Private transport
occupancy remains true; dispatch still waits for a fresh idle reply at the original
revision and deadline. Remote pending remains unavailable, and an old revision
cannot bind a new explicit choice. The correction changes no Qt policy, permission,
wire field, Source rule or cadence. It is not a causal diagnosis of a previous tap.

Both original four and new three cases pass, alongside16 lifecycle brightness
and six Settings/six Sounds desktop cases. Format, strict Clippy and normal
host/ARM owner/launcher builds pass; unchanged daemon/client, constructor, Qt and
Source suites are not repeated or relabelled. The new normal owner/window join
remains unexecuted at this checkpoint. See
[readiness provenance](evidence/native-brightness-readiness-provenance.json).

## Earlier reservation owner join: first tap fails

The published first-send reservation repair now runs in a fresh prepared normal
Settings session with the matching launcher. Qt, the client archive, frozen
brightness daemon and all policy/resource bounds remain unchanged. One explicit
75% input fails the original five seconds:116 independent raw samples stay100/400,
with optional actual120 and no inotify event. Only initial25 completes; the second
tap and its new failure diagnostics are unreached.

Root views the initial, immediate and failure originals at1080×2424. All show25%
and “Brightness ready”; no pending or retained outcome is visible in those frames.
Qt handler delivery, the first full Set, its reply and the cause remain unknown.
Two authenticated Source snapshots observe no recorded failure, which does not
establish continuous health. No input is retried. All100 direct waits/107 PIPE
EOFs close within their original cutoffs; a separate adopted compositor descendant
exit1 is not the normal Settings exit. Brightness PID1 Stop, QEMU/container and
185+773 input closure pass. The aggregate remains failed. See
[the new failed-join provenance](evidence/native-settings-reservation-native-provenance.json).

## Current readiness owner: five-state normal Settings trace passes

The current persistent-outcome Qt app, Rust actor and installed PID1 brightness
service now complete one fresh actual-touch trace:25→explicit75→unavailable with
muted controls→readonly recovery75→new explicit25. The same normal UID11050 app,
window and service Main stay running. Exactly four authored touch inputs are used:
one75, two while unavailable, then one new25. Inputs are never retried.

Independent requested attributes follow100→300→300→300→100 at max400. With max
withdrawn, both disabled inputs produce no requested-attribute mutation events;
restoring it recovers readonly75 before a new explicit choice. Optional actual
stays120 throughout. Neither raw readback nor rendered status establishes a Set
acknowledgement or a physical luminance change. The fixture sends no private
Inspect/Set to the daemon.

The only three selected changes from the preceding failed join are the normal
readiness owner, matching Settings launcher and their route proof. Qt/app/archive,
the frozen daemon, all23 Python helpers, QML, feature contract, Source100ms, input
sequence and original deadlines stay unchanged. The pass establishes this finite
trace; it does not prove a cause for either earlier failure or universal click
reliability. The new owner/launcher have separate actual component RED→GREEN and
normal host/ARM release-build evidence; the GUI executes their x86 providers only.

Root views and fully decodes the five original1080×2424 group captures, verifying
CRC, opacity and native RGBA/BGRA hashes. External output/window metadata qualify
scale2.625 and65×146mm, approximately422PPI. No capture is resized, cropped or
repainted; the debugging banner and pointer remain. Normal QWindow DPR, numeric
app exit, cold full-shell startup, ARM graphical execution and physical backlight
remain open. All103 direct waits/110 PIPE EOFs close within their original cutoffs. The
separate adopted compositor descendant exit1 is not a normal app Main status.
PID1 brightness reports Main0/success with socket removed and cgroup empty;
owner/audio/appearance Stop, QEMU/container and185+1002 input closure pass.
The authored VM backend stays separate from the ARM GPIO kernel
qualification below. See [current trace provenance](evidence/native-settings-readiness-native-provenance.json).

| Initial25 | Explicit75 | Unavailable | Readonly recovery75 | New explicit25 |
| --- | --- | --- | --- | --- |
| ![Initial25](assets/settings-readiness-initial25.png) | ![Explicit75](assets/settings-readiness-explicit75.png) | ![Unavailable](assets/settings-readiness-unavailable.png) | ![Readonly recovery75](assets/settings-readiness-recovered75.png) | ![New explicit25](assets/settings-readiness-second25.png) |

## Actual ARM kernel LinuxBacklight integration

The retained normal ARM daemon and real Rust Client now run through PID1 and the
actual `/sys/class/backlight` filesystem in a generic7.1.6 ARM guest. The existing
matching `gpio_backlight` module binds one authored QEMU PL061 device; no new
kernel or module is built. This is a virtual GPIO backlight with max1, separate
from the regular-file SyntheticTree and graphical traces above.

The same frozen positive probe first reaches a credentialed daemon Inspect with
no device: connected, not pending, no observation, then the exact positive
assertion fails101. Only the virtual device-tree deployment changes for GREEN.
Four fresh Client invocations pass: initial raw0, explicit75→raw1/current100,
new25→raw0/current0, then Unavailable after unbinding only the authored device.
Each invocation retains one original one-second BOOTTIME budget across Inspect
and its optional Set on the same verified connection. There is no write retry.
Independent sysfs reads and the same Main/Invocation are checked throughout.

`actual_brightness` is present but follows the kernel core's requested-value
fallback for this driver. It does not prove independent hardware readback,
optional-attribute absence, gradual panel control or physical luminance. Normal
PID1 Stop reports exit0/success, dead retained pidfd, removed socket and empty
cgroup. All36 GREEN direct command waits/PIPE EOFs close before original cutoffs,
with QEMU exit0, container removal and unchanged readonly inputs. The baseline's
27 waits/EOFs remain separate. No fixture parent wait for PID1's child is claimed.

The initial unpacked baseline never reaches guest output and remains a failed
setup, not functional RED. A representation-only upstream FDT pack preserves all
old nodes/raw property bytes before the completed baseline and GPIO runs. The
exact cause in the original booting binary remains unproved; the fixture observes
the [ARM64 DT size requirement](https://docs.kernel.org/arch/arm64/booting.html#setup-the-device-tree).
Tool release-key trust remains unqualified. No Source, normal ARM app/window,
cold autostart, Pixel kernel/panel or phone execution follows from this test.

See [hash-only kernel provenance](evidence/native-brightness-arm-kernel-provenance.json)
and the [verbatim original probe](../tools/native-brightness-kernel/README.md).

## Remaining integration and hardware gates

The finite Settings touch/change/unavailable/recovery path is qualified only for
the earlier explicit SyntheticTree backend cohort above. The updated normal app's
touch path, persistent outcome/actor refusal UX, cold startup and full-shell
composition retain their own gates. The separate generic ARM driver proof does
not qualify matched Pixel panel/backlight discovery, brightness response, physical readback, suspend/resume,
lock behavior and power/drain measurements remain separate gates. No phone has
been accessed, and this component does not advance calls, IMS or SMS hardware
qualification.
