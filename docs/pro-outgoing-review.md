# Outgoing call review

BRIDGE VERIFIED: Chat selected, Pro selected, completed response actions observed, marker `PRO-BRIDGE-20261011-outgoing-98ae` matched, zero context rounds. The owned tab was closed. [Conversation]((private review transcript)).

The review distinguishes command execution, observed call progress and permission to start another call. A normal `placeCall()` return is not acceptance. Pending work must survive timeout, gateway stop, generation change and process death. Use an opaque correlation marker in outgoing extras, keep it through the live call, and reject unrelated or old-session completion as proof. An explicit recovery action may accept uncertainty but cannot cancel an earlier dispatch or redial automatically.

Independently checked against primary sources:

- [TelecomManager implementation](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/telecomm/java/android/telecom/TelecomManager.java): `placeCall()` returns void and catches service errors.
- [InCallService teardown](https://android.googlesource.com/platform/frameworks/base/+/master/telecomm/java/android/telecom/InCallService.java): superclass unbind destroys local call objects and marks lingering calls disconnected. Invalidate the observation session before superclass teardown.
- [Outgoing extras extraction](https://android.googlesource.com/platform/packages/services/Telecomm/+/master/src/com/android/server/telecom/CallIntentProcessor.java): the inner outgoing extras become call metadata. Verify the marker round trip on the actual emulator adapter.
- [GSM number parsing](https://android.googlesource.com/platform/frameworks/opt/telephony/+/master/src/java/com/android/internal/telephony/gsm/GsmMmiCode.java): some one- and two-digit strings are service commands. The ordinary-number grammar now requires 3 to 32 ASCII digits, with an optional leading plus.

Implementation acceptance:

1. Separate outgoing ledger and eligibility from existing incoming controls. Outgoing disabled, pending, unknown or saturated must not consume incoming-control capacity or block Answer/Decline/End.
2. Bind each short-lived, server-issued readiness offer to enrollment, command generation, outgoing revision, resolved default tel account and elapsed deadline. Recheck after storage and immediately before API invocation.
3. Reserve before invoking. Persist only a random attempt marker; no destination or destination hash. Storage failure means no invocation. No automatic retry or generation-upgraded resend.
4. Correlate by random attempt marker, not number, timing, next callback or account. Invalidate callbacks from lost service sessions. Preserve pending across teardown and process death.
5. Default-account-only routing must not choose the first account or silently fall back after a default change. Verify discovery under the actual role/permissions; do not add privileged permissions based on documentation ambiguity.
6. Separate invocation receipt from observed call state. Timeout changes the displayed state to unknown; it is not proof of cancellation. Recovery requires an explicit user action and never initiates a call itself.
7. Define emergency behavior separately. Pro proposed excluding remote emergency dialing in the initial ordinary-call subset; that is a scope recommendation, not an Android requirement. Emergency support and its device validation remain unfinished phone work. Never place emergency calls during automated tests.
8. Verify two consecutive native outgoing calls from cold idle, callback marker round trip, permission/role loss, lost response, process death, teardown callbacks, late unrelated calls, account changes, parser boundaries, and incoming-control regression.

Current evidence: parser and pending-policy unit tests pass. The real Android journal passes across forced process stops under enforcing SELinux. Outgoing API, account discovery, wire operations, native dial UI and callback correlation are not integrated yet; the earlier end-to-end proof remains evidence for the previous incoming-control package only.
