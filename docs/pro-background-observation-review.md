# Background call observation decision

2026-10-12. This is the target for implementation, not deployed behavior.

Use one Rust daemon-owned long-poll observer, separate from the app and command
workers. Return bounded full current-state snapshots, not a history replay.
Keep Android as the sole ringtone owner. Linux presentation is silent and opens
Phone for fresh controls; notification clicks do not carry Answer/Decline
commands or captured command authority.

An event stream could reduce handshakes but adds persistent framing, heartbeat
and backpressure machinery. Periodic sampling creates gaps and repeated idle
wakeups. Long polling fits the current one-request TLS framing while waking on
callbacks. Idle resource cost still requires measurement; this decision does
not assert battery efficiency.

## Implementation contract

- A read-only `watch` request carries request identity, an optional
  generation/revision cursor and a bounded wait. A new/different cursor returns
  immediately; an unchanged cursor waits for publication or timeout. The reply
  is the full ringing set plus source availability, generation and revision.
  It never rotates command session/admission tokens or consumes a dial offer.
- Give authenticated watches separate bounded capacity. Do not park them in the
  command executor or Android main looper. Main-looper work only captures and
  publishes state. Network writes occur after releasing state locks. Stop and
  enrollment revocation must close watch connections promptly.
- Capture the internal wake cursor before requesting a main-looper snapshot.
  The wake primitive compares/registers its wait under the same monitor used by
  publication. Register Call.Callback before initial sampling; discard queued
  callbacks for removed calls or obsolete observation generations.
- Timeout responses must resample through the main looper. A responsive socket
  thread returning old data does not establish live Telecom observation.
- Startup/reconnection/source-generation changes replace the presentation set
  from a complete snapshot. Unknown or rebuilding source state is not idle.
  Normal idle InCallService detachment is distinct from gateway failure.
  Observations may reconnect with bounded backoff; commands are never replayed.

## Freshness and presentation

Target idle watch hold: at most 25 seconds, with a separate 30-second observation
deadline. Keep ordinary command deadlines unchanged. A ringing result from an
idle watch must be confirmed by a zero-wait short query before presentation.
While ringing, watch at most one second with a five-second total deadline.
Presentation expires at short-query local BOOTTIME start + five seconds, not
five seconds after a delayed reply. A separate expiry mechanism must withdraw
the banner even when the observer is blocked. Recheck after resume.

Use generic Incoming call / Open Phone content, finite expiration and the
notification protocol's `suppress-sound` and `transient` hints. These are
[specified hints](https://specifications.freedesktop.org/notification/latest/hints.html),
not proof that the selected Omarchy notification server implements them.
Reconcile notification IDs against the server's current D-Bus owner. A delayed
Notify response after call removal must cause its returned ID to be closed,
not resurrect presentation. Respect episode dismissal and suppression, and do
not replay suppressed banners after DND ends.

The critique recommends suppressing the ordinary desktop banner while lock
state is unknown or locked. That is a boundary for this desktop-notification
path, not a final smartphone policy: the goal still requires a separate trusted
lock-screen incoming-call surface. Locking the phone must not become a permanent
reason that users cannot see or answer calls. Implement that surface without
bypassing the lock boundary, and verify actual locker/notification behavior.

Android's [InCallService contract](https://developer.android.com/reference/android/telecom/InCallService)
retains ringtone responsibility when declared by the adapter and exposes
`onSilenceRinger`. The current adapter has no override. Audible ringing,
suppression, stop-on-call-exit and physical wake still require separate work.

## Evidence

[Independent Pro critique]((private review transcript)):
Chat selected; Pro 5 of 5 selected; one marked user turn and one completed
assistant response; Copy/Rate/Share controls present and Stop absent; marker
`PRO-BRIDGE-20261012-b378` matched. Zero context rounds. Owned tab closed after
extraction. Android InCallService and freedesktop hint claims were independently
checked against their primary sources. Other proposals remain engineering
decisions requiring tests, not platform guarantees.

The new `CallChanges` wake primitive passes host-JVM tests for pre-wait changes,
two parked waiters, coalescing, timeout, interruption and bounds. Existing Java
policy tests and Java 8 compilation pass. The CallService source publishes its
existing changes and bind transition to that signal. No watch endpoint, Rust
observer, Linux notification path, new APK or actual Android callback run is
proved by those tests. See `ringing-contract.md` and `call-changes-*` artifacts.

Next tests must cover long-wait command isolation, source reset/gap handling,
stalled main looper, stale banners, delayed notification replies, notifier
restart, app closure and real Android callback/lifecycle behavior. Harbour
reauthentication remains required for the existing emulator environment.

Rust schema and typed TLS watch support now pass local Linux tests. Long watches
have a separate deadline; short observations and commands retain a five-second
maximum. Concurrent delayed TLS peers exercise that separation. This does not
prove Android executor isolation. See `watch-protocol.md` and
`artifacts/rust-native-local/watch-transport-result.json`.
