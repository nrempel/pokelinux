# Notification integration review

2026-10-12. [Independent Pro review]((private review transcript)).
BRIDGE VERIFIED: Chat selected, Pro 5 of 5, one marked user turn and one completed
assistant turn, Copy/Rate/Share present and Stop absent. Marker
PRO-BRIDGE-20261012-notify-e4c9 matched; zero context rounds. The exact owned tab
was closed after extraction. Review input was a narrow architecture summary,
not credentials, call data or a complete source diff.

## Verified findings and corrections

The pinned zbus source independently confirms that method_timeout starts after
call_method_raw sends the request, and dropping the pending call unregisters its
reply serial. The current blocking adapter therefore does not prove bounded
setup/write/shutdown, nor late cleanup after a method timeout. Source hashes
are in `artifacts/rust-native-local/zbus-timeout-audit.json`.

The exact-lease-UUID acceptance guard rejected a valid pending notification on
an uninterrupted renewal. A failing test reproduced this. Lease now retains a
separate continuity identity across fresh observations of the same generation
and complete ringing episode set. Each expiry lease still receives its own ID.
Continuity changes after expiry, source loss, explicit invalidation or a changed
call/episode/generation. Notification adoption compares continuity while still
checking the original request deadline and the current lease deadline. This
does not extend the lifetime of an old pending request indefinitely.

Notifications.invalidate revokes pending adoption without freeing its capacity.
The owning loop must call it on lock/permission loss, bus-owner transitions,
suspend and shutdown. Restored permissions or an A-to-B-to-A owner sequence do
not make that pending request eligible again. Event subscriptions are still
needed to deliver those transitions; the method alone is not integration proof.

## Selected next integration

Use one dedicated notification event loop with async zbus. Keep the independent
TLS observer and synchronous command service. Do not retain two production
notification runtimes. Replace the blocking adapter once the async boundary
passes the same real-bus cases plus its lifecycle cases.

- Keep one coalesced desired-state slot that preserves revocation/continuity.
- Permit one unresolved Notify, including a request abandoned for presentation
  but still retained for a late reply. Timeout is uncertain delivery, not proof
  that no popup exists. Do not free capacity and retry on each observation.
- Continue handling timerfd readiness, state changes, owner/connection changes,
  notification signals and shutdown while Notify is pending.
- Cleanup needs independent bounded work. A late valid ID is cleanup-only and
  stays attached to its original connection and unique owner.
- Subscribe before owner discovery and before Notify. Fence stale discovery,
  signal sender/path/interface, owner transitions and full bus restarts. Handle
  dismissal/action signals received before the method reply.
- Latch user dismissal by ringing episode. Open Phone launches the fixed app;
  it never dispatches a cached Telecom command.
- Bound connection setup, authentication, send, cleanup and join explicitly.
  A caller timeout or unconditional graceful shutdown does not cancel a worker.

Required tests: delayed/never-returned IDs without duplicate creation; renewal
while Notify takes 1.2-1.4 seconds; dismissal before reply; stalled setup/write/
close; owner A-B-A and full bus restart; completion versus BOOTTIME expiry order;
actual notifier expiry, sound suppression, history, hover, DND, shell reload,
lock transitions and app activation.

The positive desktop timeout is relative to display. It cannot guarantee that
a delayed server will not show stale content. A strict absolute presentation
expiry needs renderer cooperation. Lock-screen incoming calls still require a
trusted surface. Generic desktop suppression while locked is only a boundary
for this notification path, not fulfillment of the smartphone goal.

## Verification checkpoint

The selected regression suite passes 92 tests (2 existing ignored cross-UID
helper cases), and the real-bus suite passes 5 tests. The latter includes an
actual 1.2-second Notify response across a one-second observation renewal.
The response remains eligible, and no close is sent before explicit test cleanup.
State tests additionally cover an invalidation followed by reconfirmation,
expiry without an empty sample, generation and episode changes, and a revoked
request whose permission/owner later returns. Clippy and formatting pass.
Evidence and hashes: `artifacts/rust-native-local/notification-renewal-result.json`.
The async notification loop and unknown-delivery cleanup are not implemented.
