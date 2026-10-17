# Native display brightness

The new Rust brightness component separates deployment-selected backlight I/O,
local observation/intent ownership and the lifecycle actor's app permission.
The production daemon now passes native-host process/socket tests against
authored SyntheticTree attributes. Its separate Rust client passes real local
socket tests with authored peers. Their joined connection, configured lifecycle
adapter, Settings window, installed service and physical display change remain
unqualified at this checkpoint.

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

## App permission and lifecycle

The lifecycle actor now understands DisplayBrightness for the fixed
org.omarchy.Settings principal, UID11050. This names a future installed app; it
does not install or launch one. Typed brightness_set and brightness_status requests
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

## Remaining integration and hardware gates

Next are the joined concrete client/daemon connection, a configured lifecycle
provider adapter, normal Rust Settings client and themed UI, explicit installation
and a synthetic VM end-to-end tracer. Linux driver execution and matched Pixel
panel/backlight discovery, brightness response, physical readback, suspend/resume,
lock behavior and power/drain measurements remain separate gates. No phone has
been accessed, and this component does not advance calls, IMS or SMS hardware
qualification.
