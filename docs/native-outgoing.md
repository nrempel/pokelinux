# Native outgoing-call path

Client 0.6.0 adds `omarchy-phone-local dial` over the same private Unix socket as incoming controls. It requires the original daemon epoch, operation ID, controller session, Android generation, offer and canonical number. There is no automatic retry. The UI dial pad is not implemented by this change.

Dial and control share the admission mutex, pending-worker flag, durable operation ledger and uncertainty gate. Duplicate identity lookup precedes current observation checks; the original payload hash includes operation kind and every transmitted field. Both observation lanes are atomically invalidated on session claim, admission, worker completion and acknowledgment. A poll started before invalidation cannot restore authority.

The daemon preserves Android `invocation_attempted/returned` and `not_invoked/<reason>` as known receipts. It maps `outcome_unknown/dispatch_error` to the existing local `unknown/dispatch_error` recovery state. Transport or protocol failure produces a durable unknown without a result. Invocation attempted is not a connected-call claim. Acknowledgment cannot change the receipt or dispatch anything.

Red tests first failed because the dispatcher rejected dial and session takeover did not invalidate dial status. A separate real Linux CLI/socket test failed because the dial subcommand was absent. After implementation, 130 Linux tests pass on Harbour, including duplicate CLI submission, conflicting payload, stale session/generation, explicit non-invocation, unknown recovery and both directions of dial/control worker exclusion. The current Java policy suite also passes, including an explicit check that a returned Telecom invocation cannot issue a new offer before call callbacks.

The Android implementation already has a separate durable pending-attempt marker. `DialPolicy.offer` returns no offer while that marker exists. `AndroidDialer.observedTerminal` clears it only for a matching terminal call marker from a valid InCallService session. The new Linux acknowledgment does not clear Android's pending-attempt marker. Ambiguous outgoing calls therefore still need the separately defined Android recovery policy; the Linux ledger alone cannot declare Android ready to dial again.

The package is `artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar`; plugin 0.4.0 remains unchanged. The enrolled ARM64/systemd/CLI-to-Android test is recorded separately when completed. Earlier 0.5.0 Android and upgrade results do not establish this new outgoing path. Native dial-pad input, its asynchronous intent binding, full incoming/outgoing recovery UI, and ringtone behavior remain open.

## Independent review

BRIDGE VERIFIED: Chat selected, Pro 5 of 5 verified, marker PRO-BRIDGE-20261012-dial-d26f, one user turn, terminal FINAL_RESPONSE with response actions and no Stop. Zero context rounds; owned tab closed.

private review transcript

The review found the stated result mapping coherent, not source-level proof. It requested proof for post-invocation readiness, cross-operation admission and identity conflicts, duplicate lookup ordering, terminal-commit failure, and cross-lane stale polls. Independent source review confirmed the existing Android pending marker and duplicate-first ordering; new tests cover worker exclusion and explicit post-invocation offer suppression. Not all listed fault combinations are established by the passing count. Native UI and exact-path emulator evidence remain distinct requirements.


## Offer polling correction

The first installed Android run failed local admission before recording a dial. A deterministic regression reproduced the cause: a later `dial_status` poll supplies a different offer while the original Android offer can remain valid. Comparing the submitted token with only the latest snapshot made valid user actions fail during normal polling.

The dispatcher now requires fresh current dialing readiness and the same Android generation, but transmits the original user-selected offer unchanged. Android remains responsible for checking that exact offer's existence, account, revision, deadline and one-shot admission. The daemon never replaces it with a newly observed offer. The regression failed before this change and passed afterward. Original failed-run evidence is `artifacts/native-dial-android-red/`; the repeated emulator run is separate.


Current 0.6.0 emulator checkpoint passed: `artifacts/native-dial-android/`. Installed ARM64 CLI -> private Unix socket -> systemd user daemon -> enrolled Android TLS -> Telecom produced independently observed DIALING, returned the same receipt for duplicate dial ID, and ended to idle. The same full Omarchy run then completed incoming virtual-touch Answer -> independently confirmed ACTIVE -> End -> idle. The ledger contained four admissions: invocation_attempted, outgoing End submitted, incoming Answer submitted, incoming End submitted. SELinux remained Enforcing. Both native package hashes, installed QML, Python and Android source hashes match. All 131 Linux tests and six Java policy test programs pass. This establishes the native CLI outgoing path; it is not a native dial-pad touch test, carrier call, audio or Pixel hardware proof.
