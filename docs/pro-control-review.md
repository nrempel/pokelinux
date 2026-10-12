# Independent control review

Chat + Pro verified; marker PRO-BRIDGE-20261011-f84c; completed response actions and completion indicator observed; no context rounds. The terminal answer ended with FINAL_RESPONSE rather than placing it after the marker. Complete response was inspected and the owned tab closed.

Conversation: (private review transcript)

Verified findings and changes:
- Replay lookup must precede mutable topology eligibility. A focused failing regression reproduced this and now passes. Current authorization and generation still precede replay lookup.
- Local Android call actions must consume the same revision as remote commands. Buttons and notification actions now share localAction, which advances the clock before dispatch.
- Live Call details/topology must be reconciled before reading revision, in addition to callback increments. Revision means bridge observations; unseen Telecom events cannot be made transactional by this API.
- Extracted the atomic admission gate for deterministic cancellation, exact expiry/deadline and post-CAS invalidation tests. A normal Telecom API return means submitted, not completed. Network loss remains an unknown outcome; the client never retries automatically.
- Production APK must exclude the legacy identity path, including on upgrades with old credential files. A separate test-only fixture preserves malformed TLS certificate tests.

Still required: actual GTK/Hyprland end-to-end result, production transport route without ADB, renewal/lifecycle regressions, final artifact and full-suite verification. Hardware graphics, carrier IMS and audio remain outside emulator proof.
