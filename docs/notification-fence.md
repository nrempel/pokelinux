# Incoming-call notification integration

Current source, 2026-10-12. The async Rust D-Bus adapter and request guard are
component-tested. They are not yet enabled by the daemon. Android remains the
ringtone owner; this desktop path requests silent generic presentation only.

## Request and freshness contract

`notification::Notifications` permits one unresolved Notify. A request captures
the unique notification-server owner, confirmed ringing continuity, and its
original BOOTTIME deadline. `Observer` preserves continuity only across fresh
renewals of the same source generation and complete call/episode set. Expiry,
source loss, invalidation or a changed set starts new continuity. Per-query
lease IDs remain separate for timer fencing.

A reply may be adopted only when its request is current and eligible, the owner
and continuity still match, presentation is allowed, and both the request and
current lease deadlines remain valid. Otherwise its returned ID is cleanup-only.
`invalidate()` permanently revokes pending adoption without releasing capacity.
The owner must call it on lock/permission loss, owner transitions, suspend and
shutdown. Restored permission or an A-B-A owner sequence must not restore an
old request. `not_sent()` frees capacity only with proof of no send. Timeout
or a transport error after sending begins is uncertain delivery.

## Async D-Bus boundary

`notification_bus::NotificationBus` uses async zbus with no terminal Notify
reply timeout inside zbus. The pending loop retains the unresolved future for
late-ID cleanup until 1.5 seconds after the original BOOTTIME lease deadline.
It then aborts the whole connection and reports unknown delivery. Neither fresh
inputs nor revoked/restored permission extend that cleanup deadline. It must not
send another Notify while that slot is unresolved. Setup races the entire
connection build, including authentication and Hello, against a 1500 ms async
timer. This is an awake-time budget, not a physical suspend/wake guarantee.

Notify targets the unique owner and sends Phone / Incoming call / Open Phone,
an empty body, and typed true suppress-sound/transient hints. It rereads
CLOCK_BOOTTIME immediately before sending, recalculates remaining milliseconds,
and rejects expired or sub-millisecond requests rather than emitting zero.
Returned zero IDs or replies from an unexpected sender are rejected.

A `Delivery` retains its exact connection, unique owner and ID. Close never
resolves the well-known name again or uses a replacement connection. Cleanup
must remain possible after presentation is revoked. Canceling a partial send
requires discarding the entire connection and its deliveries; do not reuse
that byte stream. Write cancellation and bounded owner-loop shutdown still
need integration tests. There is no blocking production fallback.

## Notification signals

`subscribe()` installs one ordered signal stream with capacity 32 before owner
discovery or Notify. Its match covers the `/org/freedesktop` path namespace;
the decoder then filters exact sender, path, interface, member and relevant name.
It accepts notifier ownership changes only from org.freedesktop.DBus. It accepts
NotificationClosed and the default Open Phone action only from the current
unique notification owner. Other actions, paths and senders are ignored.

The consumer must match returned IDs to its own deliveries, preserve dismissal
by episode, reconcile queued owner changes with pending discovery, and handle
signals arriving before a Notify reply. Reception itself neither launches an
app nor grants call-control authority. Bus disconnection ends the stream with
an error. Reconnection requires a new connection generation and subscriptions;
old IDs must never be carried into it. The owner must keep draining the bounded
stream while method calls are pending.

## Evidence and limits

The real-bus tests create disposable dbus-daemon processes and a synthetic
notification service. They cover content/hints, late-ID cleanup after adoption
is revoked, never-reply responsiveness and local cancellation, stalled setup
with socket release, pre-send expiry, owner transfer, renewal during a 1.2-second
reply, spoofed sender/action/path rejection, owner events before discovery,
dismissal before the initial reply, and bus loss. Evidence and source hashes:
`artifacts/rust-native-local/notification-signals-result.json`.

The [desktop protocol](https://specifications.freedesktop.org/notification/latest/protocol.html)
starts expiration at display time. A finite timeout cannot prove absolute
BOOTTIME removal if the server delays processing. A server that never returns
an ID can leave an unknown popup that the client cannot address. Hints and
successful replies do not prove actual silence, visible expiry or absence from
history. Those require the pinned Omarchy renderer and, for strict deadline
or lock enforcement, a cooperating trusted surface.

Remaining: the full generation owner loop, lock/suspend policy, saturated-write
fault injection and integrated shutdown, Open Phone activation, daemon display
wiring, installed-package tests and actual Omarchy rendering. Discovery ordering,
per-episode policy, visible handling and bounded pending cleanup have component
coverage in the later checkpoints below. The trusted phone lock-screen surface remains required;
suppressing an ordinary desktop banner while locked does not fulfill it.

The [Pro review](pro-notification-review.md) records the integration design and
independently reproduced findings. The old blocking adapter's timeout problem
is recorded in `artifacts/rust-native-local/zbus-timeout-audit.json`; it is not
the current runtime implementation. No phone access or new installed package
is part of this checkpoint.

## Latest-state coordinator input

`notification_inputs::presentation_channel` supplies one publisher and one
consumer for the notification loop. Publication uses async-channel's capacity-one
force_send, so it never waits for D-Bus and retains only the latest complete
lease/policy snapshot. The publisher is deliberately not Clone; one producer
owns update order. The consumer must still enforce BOOTTIME freshness.

The publisher changes an epoch on every eligibility or ringing-continuity
transition. Thus allowed -> disallowed -> allowed, or a missing lease followed
by ringing, cannot erase revocation even when both updates replace the queued
value before the consumer runs. Healthy renewals keep the epoch. The consumer
invalidates pending notification adoption before exposing a changed epoch.
This channel does not free the unresolved Notify slot or replay commands.

Producer closure revokes pending adoption and rejects any still-buffered state.
Consumer closure makes publication fail immediately. Tests cover 10,000
coalesced healthy updates, permission loss and return, and closure of either
endpoint. A real-bus test holds Notify, publishes loss and restoration before
consumption, then verifies that the late ID is closed with only one Notify.

This is the coordinator's input boundary, not the complete service loop.
Bus/policy sources, continuous polling alongside signals and pending methods,
expiry, known-delivery cleanup, activation and daemon integration remain to be
connected. Evidence: `artifacts/rust-native-local/notification-inputs-result.json`.

## Async BOOTTIME checkpoint

LeaseTimer can now transfer ownership of its existing Linux timerfd to an
async-io reactor. The wrapper uses read_with and maps an empty nonblocking read
to WouldBlock; it does not replace BOOTTIME with an ordinary async timeout.
Cancelling an expiry wait leaves the timer armed. The single consumer still
has to compare current lease identity and deadline after each readiness event.

A real D-Bus test sends Notify with a short remaining lease, processes timerfd
expiry while the reply is pending, retains the unresolved slot, and closes the
late returned ID. Timer tests cover cancelled waits, rearming, disarming and
one-shot consumption. Selected regression tests and real-bus tests total 111
passes, with two existing cross-UID helpers ignored; clippy and fmt pass.
Evidence and source hashes: `artifacts/rust-native-local/async-lease-result.json`.

This is component integration on local ARM64 Linux, not the complete daemon
event loop, physical suspend/wake proof, or a displayed-notification guarantee.
Harbour SSH was rechecked and still requires Tailscale reauthentication.

## Pending-reply event pump

`notification_pending::wait_pending` now drives the production async Notify
future concurrently with the capacity-one presentation input, BOOTTIME timer,
and ordered notification signal stream. It does not cancel the reply when
presentation is revoked. It returns the original Delivery for the outer loop
to close, together with the latest input and observed owner.

Early signals are bounded to 32 candidate IDs and are filtered against the
original request's unique owner even after the well-known name moves. Only
signals matching the returned delivery ID affect its dismissal or activation
result. User dismissal is reported separately from automatic expiry/closure so
the outer loop can latch the original ringing episode. An unrelated ID or a
replacement notifier reusing the same numeric ID cannot dismiss this request.
Matching default actions are only reported for an otherwise eligible result;
this module does not launch an app or issue call commands.

Input loss, timer/bus errors, and early-ID overflow revoke adoption and return
an error. The caller must discard that entire bus generation on error because
cancellation during an incomplete write cannot safely reuse its stream. A
never-returned ID remains unknown remote delivery, not verified cleanup.

Real-bus tests exercise coalesced permission loss, expiry without further
inputs, early own/unrelated dismissal, expiry versus user-dismissal reasons,
never-reply shutdown, and original/replacement owner ID collisions. Evidence:
`artifacts/rust-native-local/pending-loop-result.json`.

Still required before enabling the daemon: owner-discovery ordering,
continuous visible-delivery handling, bounded close/send
and generation teardown, episode dismissal persistence, policy sources,
activation, and daemon wiring. The reply/signal ordering gap at this checkpoint
is addressed below; completion is still not final display authority.

## Bounded cleanup and socket cancellation

Delivery.close now consumes its handle and races the complete close operation
(including send) against a 1.5-second budget. Success requires the original
unique owner's reply. Timeout, cancellation or failure shuts down the raw Unix
socket without waiting for zbus's write lock. This affects every connection
clone and stops reuse of a potentially partial message stream. It does not
prove that the desktop server removed the notification. Cleanup is not retried.

The adapter retains a clone of the local session-bus socket before handing it
to zbus. Setup, owner discovery, subscription and Notify also have cancellation
guards. Normal NameHasNoOwner from the bus is exempt: absence during startup
keeps the subscription alive for the notifier's later appearance. The setup
budget still covers connection, authentication and Hello. File and abstract
Unix addresses work, and an address GUID must match the authenticated bus.
Other transports are rejected. This is the desktop notification connection;
it does not change the Arch-to-Android control transport.

The real-bus suite proves timeout on a server that never answers Close,
connection-wide abort after cleanup timeout and cancelled Notify, preservation
of subscription when no notifier exists, abstract socket connection and GUID
rejection. Source review confirms the raw shutdown does not take the zbus
write lock. A deliberately saturated socket/partial-write test and physical
suspend test remain outstanding; the 1.5-second async budget is not claimed as
a suspend-inclusive bound. Evidence: `artifacts/rust-native-local/notification-abort-result.json`.

## Notify reply ordering

Delivery now retains zbus's connection-local receive sequence. Before completing
the pending request, the event pump drains NotificationSignals through that
reply position. It uses ordered-stream's poll_next_before, whose pinned zbus
implementation guarantees that an empty signal queue cannot later produce a
signal older than an already received message on that same connection. This
uses the existing single socket reader; no timed grace period or extra round
trip is needed.

The signal adapter retains at most one message read past the boundary, and
next() consumes it normally. The barrier verifies socket identity, so a receipt
from another connection cannot supply an ordering token. Both read paths use
the same sender/path/interface/body decoder. Ignored signals yield to the async
executor instead of looping indefinitely in one poll.

Real-bus tests deliberately complete Notify without polling its signal stream,
then drain a prior dismissal while retaining a later action. They also reject
a foreign-connection receipt. A production-pump test receives dismissal emitted
immediately before the method reply and returns cleanup-only with the dismissal
flag set. Evidence: `artifacts/rust-native-local/notification-order-result.json`.

This only fences events before the Notify reply. Initial owner-discovery
ordering and checks of newer policy/events before activation remain required,
as does continuous monitoring of visible notifications. No daemon, emulator,
desktop renderer, or phone behavior is claimed from these component tests.

## Initial owner discovery

NotificationBus.discover now requires the already subscribed signal stream
from the same connection. It validates the bus reply sender and unique owner,
then consumes initial signal history through that reply's receive sequence.
Older transitions are superseded by the snapshot; later transitions remain
queued. NameHasNoOwner produces None while preserving the connection. The raw
production owner query was removed, leaving one discovery implementation.

The complete query and history drain share a 1.5-second async timeout and raw
socket cancellation guard. A wrong-connection stream is rejected before any
request. This is initial discovery, not a means to erase revocation history
for active deliveries; any later resynchronization must first revoke those
deliveries. The timeout is not a physical suspend-inclusive claim.

Real-bus validation creates and removes an old notifier, installs its
replacement, discovers the replacement, and successfully accepts a new Notify
through the production pending loop. It then observes a subsequent owner loss.
Another case starts with no notifier and receives its later appearance.
Selected regression and real-bus suites pass 122 tests with two existing
cross-UID helpers ignored; clippy and formatting pass. Evidence:
`artifacts/rust-native-local/notification-discovery-result.json`.

Remaining integration includes continuous visible-delivery handling, persisted
episode dismissal policy, current lock/suspend policy, activation, bounded
generation lifecycle, and daemon wiring. No physical phone was accessed.

## Visible-delivery lifecycle

VisibleNotification adopts only a Visible PendingReply, returning any rejected
reply intact for cleanup. It owns the original delivery and handle while
polling BOOTTIME expiry, presentation updates and notification signals. Input
epoch comparison is shared with the pending phase and advances once per
consumed update. Coalesced permission loss/return therefore withdraws a visible
delivery too. Terminal state remains terminal even if permission returns.

The effective deadline is the earlier of the original notification deadline
and current observation expiry. Healthy updates cannot extend an old desktop
timeout. At most one OpenPhone intent is emitted per delivery; it neither
launches an application nor executes a call command. The coordinator must
check current activation policy before using the intent.

Matching user dismissal is separate from server expiry/closure. Unrelated IDs
do not affect this handle. Owner changes withdraw presentation while retaining
the original Delivery for cleanup. Errors withdraw authority and preserve that
handle too. A signal stream from another connection is rejected immediately;
its connection-local owner names and IDs cannot be used for this delivery.

Real-bus tests cover permission loss/return, unrelated dismissal, duplicate
Open Phone actions followed by independent expiry, user dismissal versus
server expiry, owner loss, input loss and foreign signal streams. The selected
regression suite plus 30 real-bus tests passes 128 tests, with two existing
cross-UID helpers ignored; clippy and formatting pass. Evidence:
`artifacts/rust-native-local/notification-visible-result.json`.

This completes the individual visible wait phase, not the service coordinator.
Generation orchestration, call-episode dismissal persistence, lock/suspend
policy, app activation, daemon wiring, packaging and actual desktop renderer
validation remain. Harbour SSH was rechecked and still requires Tailscale
reauthentication. All these tests ran locally on ARM64 Linux without phone
access.

## Call-episode dismissal policy

Notification handles now retain the original validated ringing snapshot. This
is the call set represented when Notify started, even if its reply is late and
the current ringing set has changed. VisibleNotification exposes that snapshot
for dismissal processing. A late dismissal must never apply to all current
calls merely because the generic banner had the same text.

Dismissals retains Android registry generation, numeric call ID and ringing
episode identity. It suppresses only represented episodes still present in the
last observed registry. A newer call, changed episode or changed generation is
eligible independently. Detachment, unavailability and transport gaps do not
retire existing dismissal state. A complete observed registry can retire it.
The active and dismissed sets are each bounded by the validated 64-call limit.

The policy is owned above notifier connection generations. A real-bus test
dismisses a visible notification, replaces the notifier, confirms that the
same ringing episode does not create another notification, then confirms that
a new episode does create one. Unit tests cover concurrent additional calls,
late old-generation dismissals, observation gaps and observed retirement.
This policy issues no Android commands and does not silence Android's ringtone.

Selected regressions and real-bus tests pass 133 tests with two existing
cross-UID helpers ignored; clippy and formatting pass. Evidence:
`artifacts/rust-native-local/notification-policy-result.json`.

This policy currently survives notifier restarts through its service-owned
in-memory state. Durable restoration across a phone-service restart, full
coordinator wiring, notification renewal, lock policy and activation remain
unfinished. This checkpoint does not claim those broader lifecycle guarantees.

## Durable dismissal journal

The existing private service journal now stores one Android registry generation
and at most 64 active call/episode identities with their dismissal flags. No
phone numbers, contact text, notification IDs, command payloads or application
activation requests are stored. Retaining the active identities also permits
correct dismissal handling during an observation gap after restoration.

Store replaces the complete state in one existing SQLite transaction. Failure
rolls back both deletes and inserts without changing prior dismissal state or
the command admission journal. Restoration validates generation/episode UUIDs,
call-ID bounds, duplicate identities, flags and the 64-row limit. It reads at
most 65 rows and rejects malformed or orphaned state without silently resetting
it. The journal's existing private-file, ownership lock and durability settings
apply.

This is journal schema 3. Earlier development schemas are rejected with no
migration or compatibility path; old packaged artifacts do not contain this
format. Production packaging must use the matched component set.

Tests cover journal close/reopen, a new episode after restoration, failed-write
rollback, malformed identity, oversized/orphaned rows and rejection of schema
2 without modification. A real-bus lifecycle test saves dismissal, destroys the
in-memory policy, reopens the journal, replaces the notifier, and proves that
the old episode stays suppressed while a new episode notifies. Selected
regressions and real-bus tests pass 137 tests (two existing cross-UID helpers
ignored); clippy and formatting pass. Evidence:
`artifacts/rust-native-local/dismissal-journal-result.json`.

These are actual SQLite and D-Bus boundaries, but not a restarted production
daemon. Startup loading, commit-before-publish sequencing, bounded storage
worker integration, and full daemon lifecycle tests remain to be connected.

## Native ARM64 process-crash check

The journal process-death test now runs on Linux ARM64 as well as x86-64. Its
owned child calls production command admission and dismissal storage, then
signals readiness. Before killing that exact child, the parent verifies the
exclusive ownership lock and reads both committed values through a separate
SQLite connection. It verifies SIGKILL termination, reopens the production
journal, and confirms that dismissal survives, a new episode remains eligible,
and the unresolved command is classified unknown without replay.

The focused journal/policy suite passes 16 tests on native aarch64 Linux. The
one ignored crash-child helper is explicitly launched by the parent test; it
is not an untested crash case. Clippy and formatting pass. Evidence:
`artifacts/rust-native-local/dismissal-crash-result.json`.

This strengthens recovery evidence beyond graceful close/reopen. It does not
simulate physical power loss, kill during an in-progress commit, or restart
the complete omarchy-phoned service. Those limits remain explicit.

## Coordinator restoration and commit ordering

Coordinator construction now restores dismissal state from its already owned
journal before returning. Its constructor is fallible; omarchy-phoned propagates
that error before binding the service socket. All target callers use this one
constructor contract. Corrupt presentation state is reported rather than reset.

Service-owned observe_presentation and dismiss_presentation compute a candidate
policy, commit any changed persistent state, and only then replace the in-memory
policy and return dismissal eligibility. Identical observations skip writes.
A failed save latches presentation failure for that coordinator lifetime; later
observations cannot silently restore eligibility. Existing command-session and
admission state remain separate. These methods do not perform network calls or
grant presentation authority without the independent freshness/lock/bus checks.

The presentation mutex precedes the journal mutex. Command paths take only the
journal mutex, and no network operation occurs under either lock. These are
synchronous owner methods; integrating their calls with the notification worker
and observer is still outstanding.

Tests exercise coordinator destruction/reconstruction with durable dismissal,
new-call eligibility, unchanged-observation write avoidance, rollback and
latched failure, continued command-session access, and corrupt-state startup.
The selected regression suite and 31 real-bus cases pass 141 tests with three
test helpers ignored (including the crash child explicitly spawned by its
parent); clippy and formatting pass. Evidence:
`artifacts/rust-native-local/presentation-owner-result.json`.

Startup restoration is now in the daemon construction path. The background
observer, complete notification generation loop, lock policy and activation
are not yet connected to the running daemon. No installed-service or hardware
success is claimed by this checkpoint.

## Daemon observation wiring

The background observer is now owned by omarchy-phoned and its update/expiry
descriptors are in the daemon poll loop. Confirmed ringing snapshots reach
the coordinator's durable policy with no open UI. Actual daemon process and
local TLS peer tests pass, including held-watch cancellation on SIGTERM and
concurrent hello RPC. See `rust-observer.md` and
`artifacts/rust-native-local/daemon-watch-result.json` for exact scope.

Notification display is still disabled: the notification generation owner,
lock/suspend policy, activation and renderer integration remain unfinished.

## Action followed by banner closure

Real-bus regression tests reproduced two lost-click cases: an adopted banner
and a Notify whose reply is delayed. In both, a default action followed by
NotificationClosed erased the Open Phone intent. The runtime now preserves
the earlier action while keeping the display terminal. An action received
after closure remains ignored. Expiry, policy loss and owner fencing still
apply; an intent is not permission to activate or control a call.

For a delayed reply, PendingReply.open_requested can now be true alongside
Completion::Close. The lifecycle owner must handle the valid intent separately
from display cleanup and recheck current activation policy. For an adopted
banner, the first next() returns OpenPhone and the next returns the latched
closure. This change does not itself launch an application.

The actual daemon test also injects a presentation-only SQLite insert failure.
It verifies rollback, exactly one generic diagnostic, concurrent hello RPC,
and clean cancellation of a held TLS watch on shutdown. This proves local RPC
availability after this fault, not completion of an Android call command.

The final checkpoint passes 147 tests (115 selected Linux regressions and 32
real-bus tests), with three test helpers ignored. Lint and formatting pass.
Both action-order bugs have failing-before and passing-after logs in
`artifacts/rust-native-local/notification-action-close-result.json`.
No new package, emulator integration or phone validation is claimed.

## Service-lifetime banner attempts

BannerAttempts now implements the reviewed one-attempt-per-episode policy.
It retains attempts across observation gaps and notification connections, and
retires them only against a complete observed registry or a new generation.
The vector is bounded by the snapshot's 64-call limit. It is not durable:
process restart may offer a freshly confirmed undismissed episode again.
Durable user dismissals still apply after restart.

Eligibility intersects unattempted and undismissed episodes individually.
An old undismissed call plus a new dismissed call must not produce another
banner merely because each aggregate set is nonempty. The regression tests
cover this case, call waiting, gap recovery, call-ID reuse and generation change.

Coordinator owns the attempt state beside restored dismissals and exposes a
service-only claim_presentation operation. It does not perform network or
journal writes. The future notification loop must call it only after checking
freshness, current permission, owner and capacity, just before dispatch. The
operation is not a local RPC and does not authorize display or call commands.
A latched presentation storage error also refuses new claims.

An isolated real-bus test has a server accept Notify and withhold its ID. It
then cancels the client connection, reconnects, replaces the notification
owner, and verifies no repeated attempt for that episode. A new call episode
can notify the replacement server. This is actual D-Bus adapter execution,
not evidence that an Omarchy notification was rendered.

This policy is built and component/integration-tested; the complete runtime
notification owner loop, activation and lock/suspend policy remain to be wired.
No package or phone changes were made. Evidence is in
`artifacts/rust-native-local/banner-attempt-result.json`.


## Bounded unresolved Notify lifetime

The pending loop now retains late replies only until the original request's
BOOTTIME deadline plus 1500 ms. At that deadline it returns TimedOut with
unknown delivery and drops the Notify future; its existing raw-socket abort
invalidates all clones of that connection. The old guard remains unusable.
The generation owner must replace the connection, subscriptions and guard,
while keeping Coordinator's episode-attempt state to avoid repeated banners.

The grace is a local cleanup policy, not a remote display-removal guarantee or
permission to wake a suspended phone. The absolute clock includes suspend, but
userspace can act only when scheduled. An ID returned within the cleanup window
still reaches the caller for closure against its original owner. Existing tests
cover this late-ID path and continue to pass.

A new real-bus test reproduces a server that accepts Notify but never replies.
It verifies bounded termination both with no further input and with continuous
fresh inputs, TimedOut/unknown status, no guessed CloseNotification ID, and an
unusable original connection. The disconnect assertion uses owner discovery,
which performs a real bus request: duplicate subscriptions can use a cached
match rule and are not evidence that the socket is live. The initial cached
probe failure is retained separately from the final passing test output.

See `artifacts/rust-native-local/notification-pending-bound-result.json` for
source hashes and test output. Complete runtime orchestration and installed
Omarchy integration remain unfinished; no phone was accessed.

## Integrated connection-generation loop

`notification_generation::run_generation` now joins subscription and initial
owner discovery, coalesced input consumption, episode claiming, pending reply
handling, visible notification handling and bounded cleanup. It waits for a
notification owner if none exists. It keeps one request/delivery active at a
time and returns on connection or input failure. It does not reconnect itself;
the service supervisor must create the next connection with the same episode
policy. No old delivery handle crosses that boundary.

The loop emits OpenPhone navigation intents and Dismissed events containing the
original banner snapshot. These are service events, not call commands. The
service must recheck current activation policy and apply dismissal against its
latest observed state. Claim and emit callbacks must be short and nonblocking;
service event queue, durable persistence and activation wiring are not yet
implemented by this loop. Do not put synchronous disk work in these callbacks.
In particular, coordinator mutex contention must be addressed when connecting
its claim method to the async loop; this checkpoint does not prove that boundary.

PendingReply now explicitly records a matching server-side closure, and visible
handling exposes its latched closure inside the crate. Known closed IDs are not
closed again, including when an OpenPhone or dismissal event fails to enqueue.
Other failures with a known live delivery attempt bounded close before return.
A caller should close the input producer and await normal shutdown cleanup;
arbitrary cancellation while idle with a visible banner is not equivalent to
that completed cleanup sequence.

Four new real-bus tests execute the complete generation loop: click and producer
loss cleanup; notifier arriving after startup and user dismissal; event-queue
failure after closure; and banner expiry followed by same-episode suppression
and a new call. A failing-before test exposed and then verified the duplicate
close fix. The tests use an actual isolated broker and synthetic notifier, not
the installed Omarchy renderer. See
`artifacts/rust-native-local/notification-generation-result.json`.

Still open: service supervision/reconnect, the nonblocking service event path,
lock/suspend sources, installed Phone activation and persistent shell state,
daemon enablement, package installation and full target integration. No Android
emulator or phone execution is part of this checkpoint.

## Nonblocking service event boundary

`notification_events` now provides a 16-entry typed notice queue and a Linux
nonblocking CLOEXEC eventfd for the daemon poll loop. OpenPhone and original
Dismissed snapshots retain FIFO order; only wakeups are coalesced. A full queue
returns WouldBlock immediately, and consumer loss returns BrokenPipe. The
notification generation treats either as an event-delivery failure and performs
its bounded cleanup. There is no disk write or app launch in the sender.

Producer destruction disconnects the channel before waking the consumer, so a
receiver cannot drain a premature wake and then miss terminal state. Each drain
is bounded to 16 notices. A full drain rearms readiness, preserving notification
of further events or producer closure. Consumer tests exercise empty readiness,
FIFO delivery, full-queue refusal, closure with a full batch, consumer loss and
2,000 concurrent deliveries without a lost wakeup. The generation click test
now uses this actual channel through async eventfd readiness with the real
isolated D-Bus broker.

Coordinator::claim_presentation now uses try_lock through its shared claim
implementation. Policy-lock contention returns WouldBlock without consuming an
attempt. A cross-thread test holds that lock and requires an immediate busy
result, then verifies one successful claim after release. Existing public
coordinator tests still exercise the production method with its real journal.
This removes the blocking claim issue noted in the preceding checkpoint.

The queue is ready for daemon poll-set integration, but that integration and
worker supervision are not enabled yet. The receiver must process notices using
current service policy, preserve error diagnostics and perform persistence away
from the notification loop. This checkpoint does not establish disk-stall
bounds, installed app activation, physical wake behavior or Android call control.
Evidence: `artifacts/rust-native-local/notification-events-result.json`.

## Notification connection supervision

`notification_supervisor::run_notifications` now owns serial bus generations.
It keeps the caller's per-episode claim state across reconnects and aborts the
old socket before starting a replacement. Transport failures and temporary
claim contention have a one-second retry delay. Permanent claim errors and
service-event delivery errors terminate the worker rather than retrying a
local failure indefinitely. Dropping the presentation publisher requests
shutdown; known-delivery cleanup is awaited within the bus adapter's bound.

Four real-bus/Unix-peer tests cover broker replacement after unknown delivery,
setup-failure backoff, permanent claim refusal, and full service-event queue
cleanup. Broker replacement uses an explicitly GUID-less abstract address;
this does not bypass a configured bus GUID. The same call does not notify
again after reconnect, while a fresh call can notify. A failed generation
waits for a queued or subsequent full presentation update; it does not retain
a consumed lease for reuse. The service publisher must continue observations.

This is a component checkpoint. Daemon worker and event-poll integration,
lock/suspend authority, installed activation and current package verification
remain open. There was no Android emulator or physical phone execution.
Evidence: `artifacts/rust-native-local/notification-supervisor-result.json`.

## Service-owned worker lifecycle

`NotificationWorker` owns the publication channel, notice receiver and a named
thread running the reconnect supervisor. The receiver exposes its eventfd for
the daemon poll set. Explicit shutdown drops the publisher before joining and
returns the terminal worker error once; the receiver remains available to drain
queued events and closure. Destruction also joins rather than detaching work.
The claim callback must remain short and nonblocking.

Three new real-bus tests cover explicit shutdown of a visible banner, terminal
policy failure without Notify, and destruction with an active notification.
All 45 notification bus tests pass on native ARM64 Linux. Clippy and formatting
checks pass. This checkpoint does not enable the worker in the daemon or add
lock/suspend inputs, activation, installed package or Android emulator proof.
Evidence: `artifacts/rust-native-local/notification-worker-result.json`.

## Installed native app D-Bus startup

The CMake install now supplies `share/dbus-1/services/org.omarchy.Phone.service`,
configured with the install prefix's absolute binary directory. The Arch
package builder renders the same template with `/usr/bin`. The descriptor
contains the fixed Phone executable and no call/number arguments. It uses the
existing `org.omarchy.Phone.Activate` method; it does not declare support for
the separate `org.freedesktop.Application` desktop activation interface.

A real isolated session broker with the standard session policy now starts the
installed native ARM64 Qt app in response to Activate. A second Activate keeps
the same unique owner and PID. The test terminates that exact fixture process.
The existing direct second-process activation and missing-runtime refusal tests
also pass. These are three application behavior tests (five Qt entries with
setup/cleanup), plus four package tests. Rendering is Qt offscreen/software on
Debian Qt 6.8.2, not an installed Arch/Hyprland session. Initial custom broker
policy in the test rejected connections; the final test uses the distribution
session policy rather than inventing its own authentication rules.

Inspection of the pinned Omarchy locker confirms that its Wayland session-lock
state is distinct from logind hints. No unlock permission or call authority is
derived from those hints here. Daemon event-to-activation wiring, current
session/lock eligibility and power authority remain unfinished. No complete
new Arch package or Android emulator execution is claimed by this checkpoint.
Evidence: `artifacts/rust-native-local/activation-result.json`.

## Rust-to-native activation boundary

`notification_bus::activate_phone` now opens a dedicated local Unix session-bus
connection and calls only `org.omarchy.Phone.Activate` with no arguments. A
false response is refusal. Setup and method exchange each have a 1500 ms bound;
there is no automatic retry after an uncertain result. The dedicated socket
is aborted on success, failure or cancellation, without affecting the ongoing
notification connection. This function supplies no unlock or call authority;
its service caller still needs current presentation/session eligibility.

The real-bus suite now has 47 passing tests, including accepted/refused startup
and a held reply that times out without retry or notification-connection loss.
The installed native test now invokes the exact Rust function through the
`activate_phone` integration harness, rather than making its first request in
Qt. The isolated broker launches the CMake-installed app; a second Rust request
keeps the same owner and PID. Direct app activation and missing-runtime refusal
also pass: three native behavior tests plus Qt setup/cleanup. Rust lint and
format checks pass. The test still uses Debian ARM64 Qt 6.8.2 offscreen rendering.

The event dispatcher, daemon wiring and real graphical-session/lock/power
policy remain incomplete. No installed Arch/Hyprland, Android emulator, new
release package or physical-device execution is established here.
Evidence: `artifacts/rust-native-local/phone-activation-result.json`.

## Pollable activation worker

`ActivationSlot` admits one app activation thread and no queued requests. A
second start returns WouldBlock until the service explicitly collects the old
result. The worker owns the sole peer of a Unix socket pair; completion or
unwind closes that peer and wakes poll. `finish` checks readiness without
waiting for D-Bus, then joins and returns the result once. Destruction joins
outstanding bounded activation rather than detaching it. The caller still
checks current session policy before each start.

All 50 real-bus tests pass, including three new worker tests: overlapping start
refusal and completion wake, a new explicit request after collecting refusal,
and destruction during a held reply. Lint and formatting pass. This worker is
ready for the daemon poll set but is not yet enabled there. The test does not
establish installed session/lock/suspend policy or physical wake. Harbour was
rechecked and still requires Tailscale SSH authentication; the waiting SSH
connection was interrupted. No physical phone was accessed.
Evidence: `artifacts/rust-native-local/activation-slot-result.json`.

## Presentation owner assembly

`PresentationOwner` assembles the notification worker, service-owned coordinator
and bounded activation slot. It publishes complete current observations and
external host eligibility, exposes notification/activation completion sockets
for polling, processes bounded notice batches and joins presentation work at
shutdown. It rechecks current eligibility before starting app navigation.
Overlapping activation is discarded without queueing. The event-loop caller
must collect activation results through `finish_activation`.

Dismissals use the actual coordinator and durable journal. A failing-before
integration test exposed lost queued dismissals when observations disappear.
The fix retains the last accepted snapshot only as dismissal context; no old
lease is republished and no call authority is derived from that context. New
confirmed snapshots replace it, while the coordinator intersects original and
current episodes. Persistence remains synchronous on the owning service thread,
not the notification thread; arbitrary disk-stall responsiveness is not proven.

Three assembly tests exercise click-to-activation plus durable dismissal and
journal reopen, queued-click rejection after policy loss, and queued dismissal
across observation loss. All 53 real-bus tests pass, with the relevant daemon,
coordinator, journal and input/policy regression suite also passing.
The host session/lock/power adapter and daemon enablement remain incomplete.
No new package, installed Arch/Hyprland, Android emulator or physical-phone
proof is established by this local ARM64 Linux checkpoint.
Evidence: `artifacts/rust-native-local/presentation-owner-result.json`.
