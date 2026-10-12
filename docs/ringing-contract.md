# Incoming ringing contract — incomplete

Incoming calls must remain observable and alert the user with the panel closed. Respect explicit silent/DND settings. Answer and Decline require fresh call authority; reconnect must never repeat a command.

## Current ownership finding

The bridge manifest declares `android.telecom.IN_CALL_SERVICE_RINGING=true`. Android documents this as the InCallService taking responsibility for ringtone playback: https://developer.android.com/reference/android/telecom/InCallService . `CallService.notifyCall` creates an incoming notification channel with the default ringtone URI and USAGE_NOTIFICATION_RINGTONE, then posts a CallStyle foreground notification. It does not establish Linux audio output, loop duration, mute synchronization, screen wake or delivery while Linux is suspended.

The failed emulator UI run recorded `SKIP_RINGING (Dialer handles)` in Telecom. This confirms that Android Telecom delegated ringing for that call. It does not prove audible playback or explain the later missed-call transition. Do not add an independent Linux ringtone without choosing one sound owner; otherwise the retained Android notification and Linux sound can both alert.

The intended architecture keeps vendor call audio on Android initially. Before choosing Linux versus Android ringtone playback, test the retained Android audio service and notification behavior, suppression/DND, stop-on-answer/decline/end, and service recovery. The native QML card can be independent of the sound owner. Any Android-to-Linux policy bridge must report effective suppression state rather than infer it from call state alone.

## Required evidence

- Actual audio activity and stop events in the emulator; simulated device audio is not physical Pixel speaker proof.
- Panel closed and shell restart do not stop observation or cause duplicate command dispatch.
- Silent/DND and per-call silence remain effective; suppressed calls do not unexpectedly ring after reconnect.
- Caller hang-up, answer elsewhere, answer, decline and backend loss stop the relevant sound.
- Lock-screen presentation must preserve the lock boundary. Display wake and modem wake from physical suspend require later authorized hardware tests.

Status: contract and source audit only. Native ringing and wake are not implemented or verified.

## Native replacement checkpoint — 2026-10-12

The current Rust daemon serves local requests but has no independent observation
worker. `PhoneObservations.qml` only reads while the app is active. The historical
Python background-service results do not establish background observation for
the Rust replacement. This remains a required integration change.

The Android callback edge now has a small `CallChanges` wake signal. Existing
call-change and registry-invalidation paths publish to it; binding also publishes
the availability change. A reader takes an opaque cursor before obtaining its
main-looper snapshot, then waits only if that cursor is still current. A callback
between snapshot acquisition and waiter registration cannot be lost. The signal
coalesces changes; it does not retain call data, queue events, assign wire IDs,
grant command authority or perform network work on the main looper.

Host-JVM tests cover a change before waiting, two observers, 10,000 coalesced
changes, timeout, interruption and invalid timeout bounds. The test first failed
on a lost pre-wait change. All existing Java policy tests pass with the new
primitive. Evidence: `artifacts/rust-native-local/call-changes-{red,green}.log`.
These tests do not exercise Telecom callbacks. No new APK or emulator run has
verified the CallService wiring; the local Android SDK is absent and Harbour
still requires Tailscale reauthentication.

The authenticated observation endpoint, Rust worker, snapshot freshness lease,
silent notification delivery, lock-screen policy and Android sound behavior are
not implemented by this primitive. It is the callback wake boundary needed by
that work, not a claim that closed-app incoming calls already work.

The current Android adapter also lacks an `onSilenceRinger` override. Android's
[InCallService contract](https://developer.android.com/reference/android/telecom/InCallService)
explicitly exposes that callback. The ringtone-owner work must handle it and
prove stop behavior without re-alerting on unrelated call detail updates.
