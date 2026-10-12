# Watch endpoint review and shutdown fix

2026-10-12. [Independent Pro review]((private review transcript)).
Chat selected, Pro 5 of 5 verified, one marked user turn and one completed
assistant response, Copy/Rate/Share present and Stop absent. Marker
`PRO-BRIDGE-20261012-d37e` matched; zero context rounds. Owned tab closed after
extraction. The review examined the pre-fix code shape, not the final diff.

## Implemented and checked

The original gateway closed an SSLSocket while holding the monitor used by
main-thread observation checks. The single timer and Service.onDestroy also
called TLS close directly. Android 17 ConscryptEngineSocket attempts outbound
TLS shutdown before closing its underlying socket. This is a blocking dependency;
a specific Android hang was not reproduced. The primary-source hash and URL are
recorded in `artifacts/rust-native-local/conscrypt-close-audit.json`.

The gateway now accepts a plain loopback TCP socket, retains ownership, and wraps
it with its enrolled TLS context inside the authentication worker. The wrapper
explicitly uses server mode, TLS 1.3 and required client authentication. There is
no plaintext protocol path. Watch and command authentication checks are retained.

Connection cancellation marks terminal state and cancels its timer under a short
lock. It releases that lock before aborting the raw socket and publishing the
wake. Timers and service shutdown never call TLS close. Owning workers clean up
TLS only after raw-socket abort. An accept that resumes after stop is canceled.
Timer callbacks still check the current deadline atomically before cancellation.

Host tests use a deliberately blocked TLS-cleanup substitute to show that abort
and state inspection remain independent. A separate real JSSE mutual-TLS test
checks server-mode wrapping, authenticated exchange, and interruption of a blocked
read through raw close. Test certificates are temporary and removed by the test
script. All production Java sources compile against the pinned Android 17 SDK.
These are not Android provider, scheduler or lifecycle tests.

## Follow-up findings to implement or verify

- Reserve zero-wait confirmation capacity separately from parked positive-wait
  watches. Abandoned watches are not promptly detected by local isClosed checks.
  Consider authenticated replacement of the single observer's previous watch;
  replacement must never affect command ownership.
- Clamp hold time against the remaining connection budget, leaving room for the
  final main-thread sample and write. Bound FutureTask.get by remaining budget.
  The parser already enforces 0..25000 and Rust uses an absolute BOOTTIME budget;
  the review's uncertainty about those two points is resolved by source/tests.
- Anchor credential lifetime into the elapsed-time cutoff and test expiry while
  waiting/writing plus wall-clock movement. Existing guards reject expired
  observations but do not schedule a credential-expiry cancellation.
- Ordinary Java timed waits exclude deep sleep. Elapsed-time guards reject stale
  results after returning, but prompt resume cleanup needs a suitable mechanism
  and runtime evidence. Do not add battery-costly periodic polling without a
  justified design and measurements.
- Test queued RINGING -> ACTIVE -> RINGING callbacks. Current onStateChanged
  ignores its state argument and rereads current details, so it can miss an
  intermediate transition and keep an episode identity. Fix this before relying
  on episode dismissal semantics.
- Exercise accept/registration/stop, timer replacement rejection, early watch
  completion, stalled or rejected main-thread posting, and real provider abort.
  cancel(false), checked Handler.post, bounded queues and remove-on-cancel timer
  policy already exist; their runtime interactions remain unproved.
- Shared authentication capacity remains susceptible to permitted local clients
  occupying it. Parked authenticated watches have separate workers; that is not
  proof of command availability against hostile pre-authentication traffic.
- The future Rust observer must fence late confirmations, use the confirmed
  generation/episode set, bind expiry to its lease and reject replies after
  short-query-start + five seconds. None of that presentation work is complete.

Evidence: `transport-abort-{red,green,tls,compile,parser}.log` and
`transport-abort-result.json` under `artifacts/rust-native-local`.

## Capacity and deadline follow-up

Implemented: WatchWorkers has one positive-wait lane and one zero-wait lane,
each a single worker with SynchronousQueue and no task queue. Thus a parked
watch cannot consume the confirmation slot. Host tests hold both lanes, verify
that excess requests are rejected rather than queued, and verify shutdown
interrupts both workers. This does not solve abandoned positive-wait ownership
or pre-authentication saturation; those remain separate checks.

Implemented: CallWatch clamps its hold deadline to the connection deadline
minus 2500 milliseconds. The final main-looper FutureTask wait is bounded by
min(2000 milliseconds, remaining connection budget). When the remaining budget
is too short to park, the request samples and returns immediately. The timing
margin is not a guarantee of scheduler latency or successful delivery. A host
test uses a late-start clock and a bounded Future to verify that no wait occurs.
The existing expiry guard still rejects samples completed after the deadline.

Host policy/parser tests and Android SDK compilation pass. See
`watch-capacity-budget-result.json`, `watch-workers-{red,green,compile}.log` and
`watch-budget-{red,green,compile,parser}.log` in `artifacts/rust-native-local`.
Credential expiry anchoring, queued callback episode transitions, prompt resume
cleanup and Android runtime integration remain open.

## Credential lifetime follow-up

Implemented: after successful certificate/pin checks, CredentialExpiry converts
min(enrollment expiry, peer certificate notAfter) into an elapsed-time limit. It
anchors this once per exchange, before reading the request body. The anchor is
capped at the maximum 30-second exchange duration and checked for arithmetic
overflow. The existing socket timer is rescheduled to the earlier credential or
exchange deadline. Extending an authenticated watch cannot exceed that anchor.
Command admission receives the earlier credential/command elapsed deadline and
the earlier certificate/enrollment wall expiry.

Host tests cover exact expiry, backward and forward wall-clock changes, expired
credentials, distant dates, arithmetic overflow and actual Admission rejection
after a wall rollback. All production Java sources compile against Android 17.
Evidence: `credential-expiry-{red,green,compile}.log` and
`credential-expiry-result.json` under `artifacts/rust-native-local`.

This only anchors an existing exchange. It does not establish trustworthy wall
time across process restarts, alter credentials, or prove prompt cleanup during
suspend. Forward wall changes fail the next guard; they do not themselves wake
a parked worker. The existing elapsed-time timer and resume limitations still
need Android runtime evidence. No phone was accessed.

## Queued callback episode follow-up

CallService now uses onStateChanged's delivered state to track transitions out
of ringing. Before resampling current details, it retires the old episode via
RingingRegistry.leftRinging. Therefore a queued ACTIVE event followed by current
RINGING details cannot preserve the previous episode ID. Per-call last-callback
ringing state avoids repeated resets for consecutive non-ringing transitions;
it is initialized after callback registration and removed on call removal,
unbind and destruction. Existing session/membership checks run before mutation.

The host registry test exercises departure while the sampled ringing set remains
identical, verifies the affected episode changes and the other call's episode
stays stable, and rejects obsolete-generation/unknown-ID retirement. Policy tests
pass and all production Java sources compile against Android 17. This does not
exercise actual Android Call objects or callback delivery order. The emulator
regression must still queue RINGING -> ACTIVE -> HOLDING -> RINGING, verify one
new episode, and deliver obsolete callbacks after removal/unbind. Evidence:
`ringing-callback-{red,green,compile}.log` and `ringing-callback-result.json` in
`artifacts/rust-native-local`.
