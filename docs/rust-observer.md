# Rust background observation state

The target remains a daemon-owned observer, independent of the Phone window and
command admission. `runtime/src/observer.rs` now implements the state and lease
rules. `background::BackgroundObserver` connects these rules to an owned TLS
worker, a coalesced eventfd signal and an independent BOOTTIME timer. The daemon
entrypoint, D-Bus notifications, lock-screen presentation and Android runtime
integration remain unfinished.

The owner supplies CLOCK_BOOTTIME timestamps. `begin` returns an owned request
with a fresh identity and typed watch query. Initial/idle requests wait at most
25 seconds. An idle ringing reply cannot create a presentation lease: the next
query is a zero-wait confirmation. A confirmed ringing set uses one-second
watches, each under the separate five-second transport budget.

`complete` accepts only the current request; older replies are ignored. It rejects
expired replies, clocks moving backward and same-generation revision regression.
A short reply grants a lease only until its request start plus five seconds. A
new lease has a new local identity so an old expiry event cannot remove it. A
separate continuity identity survives only fresh renewals of the same source
generation and complete ringing call/episode set. Source loss, invalidation,
expiry or a changed set ends that continuity. Notification acceptance uses this
continuity while still checking both request and current lease deadlines.
Detached/unavailable/empty snapshots withdraw presentation. Generation changes
replace the complete set, rather than merging old call entries.

`failed` fences transport errors by request identity as well. `invalidate` is
reserved for external lifecycle loss such as shutdown or resume, and rejects
all pending replies. `lease(now)` never exposes expired data, even when an expiry
timer has not run. Expiry of a current lease schedules a short confirmation next.
These checks do not themselves remove an already-visible notification: the
future daemon must run an independent expiry path and reconcile its output.

Tests cover confirmation before presentation, request-start expiry, exact deadline
rejection, resume invalidation, stale replies/errors, renewal versus old timers,
source loss and revision regression. They use decoded protocol snapshots, not
Android callbacks. These state-machine tests do not start notifications or network work. The
separate integration tests described below exercise the background TLS worker.

Integration still required:

- Connect the tested BackgroundObserver to the daemon entrypoint and its
  presentation owner. The component already separates observation retries from
  command delivery and supports interruptible shutdown.
- An independent BOOTTIME expiry mechanism, resume invalidation and serialized
  state changes. Do not hold the observer lock during TLS or notification I/O.
- Reconcile generic silent notifications against D-Bus server ownership, user
  dismissal, lock state and source episodes. Fence delayed notification replies.
- Exercise actual Android watch replies through the Rust decoder, then daemon
  behavior while the UI is closed, commands are unresolved, Android restarts,
  enrollment expires and the notification server restarts.

## Cancellable watch transport

The typed TLS watch method now requires a WatchCancel token. The token owns at
most one active watch registration, remains reusable after a completed exchange,
and becomes permanently canceled on cancel(). A canceled token cannot start a
connection. Registration and cancellation share a short mutex; raw-socket
shutdown occurs after releasing it. Cancellation shuts down both directions of
the active watch socket, including while TLS handshake or reply reading blocks.
The registration is released on every return path. Commands do not accept this
token and retain their existing delivery/deadline behavior.

Before connect returns there is no socket to shut down. This watch-only connect
phase is therefore capped at one second, within the existing absolute five- or
30-second exchange budget. Once connected, cancellation is checked before
application output and around typed reply validation. The observer's request
identity fence remains necessary for a cancel-versus-completion race. This does
not prove prompt behavior across system suspend.

Linux tests cover pre-canceled requests opening no connection, cancellation of
a stalled handshake, and cancellation after a real mutual-TLS peer has read the
watch request. A separate ordinary exchange on the same client still succeeds.
No Android endpoint or physical phone is used by these transport tests.

## Independent lease timer

`lease_timer::LeaseTimer` now owns a nonblocking, close-on-exec Linux timerfd
using CLOCK_BOOTTIME and absolute one-shot deadlines. It can be included in the
service event loop independently of the blocking network worker. None disarms
it. An already-past deadline, including zero, expires rather than accidentally
disarming the timer. An out-of-range deadline is rejected before changing the
current timer. The event-loop owner must still compare current lease identity
and expiry after consuming readiness.

Real Linux kernel tests cover expiry, one-shot consumption, replacing a pending
expiry with a new deadline, disarming, and invalid-deadline rejection. They do
not suspend the host, request wake alarms or prove physical-phone behavior.
The timer is connected to BackgroundObserver; daemon entrypoint and
notification presentation integration remain outstanding.

## Background worker integration checkpoint

2026-10-12. BackgroundObserver owns one network worker, its permanent cancellation
token, a coalesced eventfd change signal and the lease timer. The caller polls
changes and timer readiness independently. Network operations run outside the
observer mutex. Successful cycles have a minimum 250 ms interval; failed cycles
use interruptible backoff of 1, 2, 4 and then 8 seconds. This limits immediate
reply loops; it is not a battery or wakeup measurement.

Shutdown cancels the active watch, invalidates observation state, disarms the
timer, wakes backoff and joins the worker. Worker exit also withdraws state.
The owner must still drive expiry and remove presentation; a lease getter alone
cannot remove an existing notification. A fresh-lifetime restart operation is now available; system sleep/resume event
wiring is not implemented.

Two real mutual-TLS Linux integration tests pass. One sends a long-watch ringing
reply, answers the required zero-wait confirmation, and stalls the following
watch. The independent timer expires the lease while the network request is
pending; cancellation and join then complete within the test's one-second bound.
The other verifies failed-connect observation and interruptible backoff shutdown.
These use a synthetic Rust peer, not the Android framework endpoint.

The selected Rust regression suite passed 83 tests, with 2 existing ignored
tests. Clippy with warnings denied and rustfmt checks passed. Evidence is in
`artifacts/rust-native-local/background-{green,regression,clippy,format}.log`;
source and log hashes are recorded in `background-result.json` beside them.
Tests ran in the local ARM64 Linux container, not Harbour.

Next: integrate service event handling with notification reconciliation and
resume invalidation. Exercise actual APK watch responses through this worker,
then validate service activation while the Phone UI is closed. No production
binary or package currently enables this worker.

## Restart after lifecycle loss

2026-10-12. `BackgroundObserver::restart` stops and joins its previous worker,
then creates fresh observation state, cancellation token, timer and change
signal using the same immutable enrolled TLS configuration. Old cursors and
presentation leases are not carried forward. If replacement creation fails,
the previous worker remains stopped. Enrollment replacement must create an
observer from the new enrollment instead of calling restart.

The event-loop owner must discard the old poll set and register the new file
descriptors. Call shutdown before sleep or on lifecycle authority loss, remove
presentation, and call restart only after authority is restored. Restart alone
does not subscribe to logind, inhibit suspend, or wake the device.

A real mutual-TLS integration test establishes a ringing lease and holds the
next request. It shuts down the worker, verifies lease withdrawal and timer
disarm, then restarts. The peer verifies that the new initial watch has no old
cursor. While the fresh confirmation reply is held, no ringing lease is
available. Only its successful reply produces a new lease identity.

The selected regression suite now passes 84 tests, with 2 existing ignored
cross-UID helper cases. Clippy and formatting pass. This is local ARM64 Linux
component evidence, not actual suspend/resume or Android endpoint evidence.
See `background-restart-{red,green,regression,clippy,format}.log` and
`background-restart-result.json` under `artifacts/rust-native-local`.

## Daemon-owned observation

omarchy-phoned now starts BackgroundObserver using the same immutable enrolled
TLS configuration as its command coordinator. Its poll set includes the
observer change signal and BOOTTIME timer alongside the local listener and
shutdown signal. It expires leases independently of held TLS requests and
passes confirmed ringing snapshots to the coordinator's durable presentation
policy. A presentation journal failure is reported once and does not close
the command service. Shutdown cancels and joins the observer before releasing
the service owner.

A native ARM64 integration test launches the actual daemon executable with a
private ephemeral loopback TLS peer. With no UI connection, the peer receives
the initial watch, zero-wait confirmation and subsequent renewal. The test
reads confirmed call state from the real journal, verifies local hello RPC
while the renewal is held, then sends SIGTERM. The daemon closes the held TLS
watch, exits successfully within two seconds and removes its local socket.

The native daemon/CLI suite's earlier ARM64 skips are removed. Its four tests
pass, and the selected regression suite passes 114 tests with three helper
skips. Clippy and formatting pass. Evidence:
`artifacts/rust-native-local/daemon-watch-result.json`.

This proves daemon-to-native-TLS-fixture behavior, not an Android framework
callback or physical incoming call. Desktop display remains disabled until
the notification generation owner and lock/suspend authority are connected.
The existing restart primitive is not yet driven by real logind events.
