# Incoming-call presentation ownership review

Review checkpoint: 12 October 2026. The goal remains the complete Omarchy
smartphone scope in `smartphone-goal.md`. No phone access occurred.

BRIDGE VERIFIED: Chat selected, Pro 5 of 5 verified, completed response with
Copy/Rate/Share controls and no Stop control, marker
PRO-BRIDGE-20261012-owner-7b9e matched. One context round supplied the full brief
after an initial marker-only submission. Owned tab closed after extraction.
[Review conversation]((private review transcript)).
Used the [chatgpt-pro-bridge skill](chatgpt-pro-bridge skill (local tooling)).
The review is architectural advice, not implementation or device evidence.

## Accepted direction

Use one presentation owner loop in the existing Rust service. Keep the live
Phone and shell call surface independent of desktop notification delivery.
Publish complete current state with a daemon identity and an absolute BOOTTIME
expiry. Consumers must expire cached state even if the daemon is frozen.
Renew live state with confirmed observations, not by extending an old banner.
A lost observation means unconfirmed state, not an ended or missed call.

Attempt a generic banner once per ringing episode during a daemon process.
Keep this attempt set above bus generations, in memory. A fresh confirmed
undismissed episode after process restart can receive a new banner; a duplicate
under this failure condition is an accepted cosmetic limitation. Do not add a
durable delivery-attempt journal or pretend SQLite can commit atomically with
the notification service. Persist explicit episode dismissals as already
implemented. Banner dismissal does not hide a live call or silence Android.

Keep one local Notify operation outstanding. Preserve late cleanup while its
connection can return a reply. A terminal connection loss retires the local
operation as ambiguous; it must not consume capacity forever. Reconnect and
cleanup must stay bounded and independent of the continuing call surface.
All cleanup uses the original connection and unique owner. Connection abort
also invalidates subscriptions and owner discovery on that connection.

Open Phone is navigation to a fixed installed app, not call authority. Preserve
an accepted click if the banner closes immediately afterward. The app reads
current state; it must not restore old call-control context. Activation tokens,
when supported, are opaque transient data, never shell commands.

## Independently checked protocol boundaries

The [notification protocol](https://specifications.freedesktop.org/notification/latest/protocol.html)
starts its timeout at display time, offers server-issued IDs rather than a
client idempotency key, and defines action and close signals separately.
Thus local expiry withdraws local eligibility and requests closure of known
IDs; it cannot retract an unknown delayed remote display. The real-bus tests
in this checkpoint independently demonstrate and fix action-before-close
ordering in both visible and delayed-reply paths.

The [systemd source manual](https://raw.githubusercontent.com/systemd/systemd/main/man/org.freedesktop.login1.xml)
defines LockedHint as desktop-supplied state. Its PrepareForSleep signals
support coordination; preparing properties do not emit property-change signals.
Use the actual intended graphical session, not an SSH or lingering service's
assumed session. Hints can guide generic-banner suppression but are not an
unlock authorization. The [Wayland lock protocol source](https://raw.githubusercontent.com/wayland-mirror/wayland-protocols/main/staging/ext-session-lock/ext-session-lock-v1.xml)
assigns rendering and input isolation to the compositor and requires it to
remain locked when the locker dies. Privileged shell surfaces need a separate
privacy review.

The existing `hardware-boundary-decision.md` requires one power authority with
Android wake-lock participation. Retained Android suspend cannot be assumed
to pass through logind. BOOTTIME limits old leases, but a short suspend needs
an actual power transition signal to force a new confirmation. Physical wake
and radio availability remain unverified.

## Advice not adopted as a scope reduction

Pro proposed requiring unlock before answering in the first version. This is
not the completion target: the smartphone goal still includes an appropriate
trusted incoming-call lock surface. An ordinary app can require unlock while
that integration is unfinished, but it does not complete the lock-screen row.

Pro also recommended immediate in-memory dismissal plus asynchronous durable
writes. Current coordinator code commits before exposing policy and latches
storage failure. Moving persistence off the owner loop needs a real ordering,
queue and shutdown test; this review does not claim that change is implemented.
The new actual-daemon SQLite fault test proves rollback, one generic error,
hello RPC availability and clean held-watch shutdown, not storage-stall bounds.

## Next integration evidence

1. Assemble owner discovery, pending/visible transitions, attempted episodes,
   bounded cleanup/reconnect and shutdown; keep command/observation paths live.
2. Wire installed Phone activation and continuing shell state. Test 30–60s
   ringing, banner expiry, closed-app launch, app-already-open and daemon freeze.
3. Bind lock/activity policy to the real graphical session. Exercise actual
   notifier, compositor and locker, including false/missing hints and locker
   failure. Do not bypass lock to bring Phone forward.
4. Test bus replacement, unknown delivery, episode changes, coalesced revocation,
   lost observation, all close reasons, persistence stalls and restart.
5. Integrate the actual suspend authority; test held replies across short and
   long suspension. Keep later physical wake/IMS/audio proof separate.

Current code checkpoint: `artifacts/rust-native-local/notification-action-close-result.json`.
147 tests pass, including 32 isolated real-bus cases and five daemon/CLI cases;
three helpers are ignored. Notification display/activation owner integration,
installed Omarchy validation and the expanded smartphone features remain open.
Harbour was rechecked and still requires Tailscale SSH authentication; no login
approval was bypassed. Native ARM64 Linux tests ran locally instead.
