# Native display brightness

The new Rust brightness component separates deployment-selected backlight I/O,
local observation/intent ownership and the lifecycle actor's app permission.
The production daemon and its concrete Rust client now pass joined native-host
process/socket tests against authored SyntheticTree attributes. A lifecycle
Provider adapter and explicit installed configuration have separate qualified
host evidence. A first prepared normal Settings frame now reads through the actual
actor and installed service. A later same-app touch/change/unavailable/recovery
sequence now passes against authored attributes with five original screenshots.
Cold startup, other input races and physical display behavior remain open.

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

## Remaining integration and hardware gates

The finite Settings touch/change/unavailable/recovery path is qualified only for
the earlier explicit SyntheticTree backend cohort above. The updated normal app's
touch path, persistent outcome/actor refusal UX, cold startup and full-shell
composition retain their own gates. Linux driver execution and matched Pixel
panel/backlight discovery, brightness response, physical readback, suspend/resume,
lock behavior and power/drain measurements remain separate gates. No phone has
been accessed, and this component does not advance calls, IMS or SMS hardware
qualification.
