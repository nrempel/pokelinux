# Independent review — enrollment and call controls

Consultation: [Review bridge enrollment lifecycles]((private review transcript)), marker `PRO-BRIDGE-20261011-e49b`. Chat mode and Pro 5/5 verified before sending. One prompt; completed response observed with response actions and no Stop control; owned tab closed after extraction.

Recommendations accepted for implementation:

- Use AndroidKeyStore EC keys, public-key export, offline issuance, and explicit fingerprint confirmation. TLS signing needs real adapter tests; authorize NONE and SHA256 digests. Select the active alias explicitly. Our key-manager returns the validated certificate chain directly, avoiding mutation of the key's generated certificate.
- One AtomicFile commit for the selected identity and public trust bundle; copy a bounded SAF candidate once and confirm those exact bytes. Do not silently fall back to prior trust after a corrupt committed configuration.
- Enrollment does not enable networking or call control. Separate explicit gateway, boot-start, and control settings. Android local calling must remain independent.
- Add call revisions to prevent active/holding/active ABA. Consume revision before dispatch; bound duplicate ledger without eviction. Rotate control generation on registry loss/authorization change. A timed-out queued action needs an atomic cancellation/dispatch boundary.
- Preserve submitted/rejected/unknown distinction. Do not retry call actions automatically. Start with answer/decline/hang-up on unambiguous single calls.
- Call-specific immutable notification intents must not retarget later calls. Implement normal Android dialer contract and call UI; do not infer hardware audio or emergency routing success from API invocation.
- Emulator ADB forwarding is not deployment connectivity. Specify a separate namespace/relay arrangement with end-to-end mTLS.

Still to validate after initial enrollment adapter test: production SAF flow, crash/renewal/expiry cases, old and pending key selection, rejected import preservation, controls races, Android fallback UI, native Arch UI and deployment without ADB. These are requirements, not passed results.

Primary documentation checked or referenced by review: Android KeyGenParameterSpec.Builder, Android Keystore system, AtomicFile, Storage Access Framework, InCallService, default dialer guide, CallStyle notifications, PendingIntent, Android 17 background audio restrictions, and OpenSSL x509. Review claims remain subject to implementation tests.
