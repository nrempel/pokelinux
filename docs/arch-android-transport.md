# Arch-to-Android phone bridge

> Historical Python/GTK implementation and framework evidence. The current
> target is the Rust service with a standalone Qt/QML app. See
> [current transport decision](transport-decision.md) and
> [native app evidence](../native-app/README.md). The package and runtime claims
> below describe that earlier checkpoint, not the current target release.

Android owns the calls. The tested native Arch GTK client observes Android Telecom and can answer, decline or end one eligible call after explicit local authorization. It can also submit an explicitly entered outgoing number after separate outgoing authorization. The protocol does not expose shell commands, arbitrary intents, Binder transaction codes, incoming caller numbers, contacts, subscriber IDs or voice media. Audio controls remain unimplemented. The QML replacement and independent Linux service are in progress; the tested GTK package remains the baseline.

See [architecture and limits](phone-architecture.md) for the proposed phone structure. No Waydroid or physical Pixel is used in the current tests.

## Enrollment and local controls

The release code accepts only an explicitly selected AndroidKeyStore enrollment. The app creates a non-exportable P-256 signing key and exports a public request through Android's document picker. The offline desktop tool issues a dedicated CA and server/client certificates. Only the public response ZIP goes back to Android. The user compares SHA-256 fingerprints and explicitly confirms the pairing. The active selection has one AtomicFile commit point; failed validation leaves the previous enrollment unchanged.

```sh
omarchy-phone-pair request.json /new/protected/enrollment-directory
```

The issuer refuses an existing destination and limits validity to 90 days. Protect the desktop directory: it contains private CA and client keys. Do not put it in a repository or artifact bundle. Import `phone-enrollment.zip` on Android and select the resulting `client.json` in the Linux app. Both files display fingerprints for comparison over a trusted path. Pairing does not enable the bridge or call controls.

Android settings separately control gateway activation, startup after reboot and unlock, and desktop call controls. The default-phone role is needed for detailed Telecom observations and controls. READ_PHONE_STATE is separate and enables only the coarse default-subscription status source. The local Android call screen works while the network gateway is off.

The APK has a local dialing UI and call-specific incoming/ongoing notifications. It is not a complete daily-use dialer: mute, DTMF, hold/merge, audio-route selection and broader phone features remain incomplete. Voice media is not carried over this bridge.

## Protocol and authorization

- IPv4 loopback only: `127.0.0.1:37179`.
- TLS 1.3 with mutual CA validation and exact leaf-certificate pins. The client also checks the server SAN.
- Four-byte big-endian length and canonical compact JSON. One request per connection, at most 4,096 request bytes, 65,536 response bytes and 64 observed calls.
- Operations: `status`, `controls`, `control`, `dial_status`, `dial`. Control carries a random operation ID, generation, opaque call ID, observed revision/state and one of `answer`, `reject`, `disconnect`. Dial uses a separate fresh offer, generation and explicit canonical number; a durable Android journal records its admission before Telecom submission.
- Four workers, four queued connections, a five-second absolute socket close, a 4.5-second admission deadline and a two-second main-thread dispatch wait.
- Admission rechecks authorization, live call eligibility, observed revision, elapsed deadline and enrollment expiry. A per-request atomic gate separates cancellation from commitment.
- Local actions and remote submissions consume a revision before invoking Telecom. Live details/topology are reconciled when reading the revision, in addition to callbacks. This covers bridge observations, not unseen modem events.
- A bounded 1,024-entry ledger never evicts within its control generation. Exact duplicates return the recorded result; conflicting reuse is rejected. Permission changes, service lifecycle, enrollment and registry changes invalidate old commands.
- A normal API return means `submitted`, never “call completed.” An ambiguous dispatch or connection failure remains unknown. The client does not automatically retry commands.
- Configuration and private-key permissions are checked. There is no network enrollment, default production identity, plaintext fallback or hot trust reload.

The production APK has no PKCS12 file-identity loader. A separately built `bridge-legacy-fixture.apk` exists only to retain malformed-certificate regression cases. It must never be used as a deployment APK. `verify-release-apk.py` rejects legacy file markers, embedded credentials and test instrumentation in the release artifact.

## Observations and GUI behavior

`status.phone_state` registers a fresh TelephonyCallback for the default subscription. Missing permission or missing observation produces unavailable/null. Detailed calls come from the default dialer's InCallService. Missing role, lost binding or empty callback storage produces unavailable with no calls; that is not proof of global idle.

The GTK app accepts actions only from a fresh, authorized snapshot, permits one pending command and invalidates polls started before command completion. An unchanged request correlation ID does not replace visible buttons. Disconnection removes actionable state. Android and Linux monotonic timestamps are not compared.

Install the native package using pacman, then launch `omarchy-phone`. It includes `omarchy-phone-pair` and `omarchy-phone-status`; no credentials or test fixtures are included. The package is architecture-independent Python, with GTK 3, PyGObject and OpenSSL dependencies.

## Offline verification

```sh
python3 -m unittest discover -s transport/tests -v
bash tools/test-control-policy.sh
python3 tools/test-enrollment.py
bash tools/framework-suite.sh prepare
bash tools/framework-suite.sh build
bash tools/framework-suite.sh enrollment
bash tools/framework-suite.sh gui
bash tools/prepare-direct-runtime.sh
bash tools/framework-suite.sh direct
bash tools/framework-suite.sh test
```

The GUI suite requires the provisioned ARM64 graphics guest. The direct suite additionally requires the test-only Arch glibc runtime built with `build-support/Dockerfile.arch-runtime`. Prepared SDK/images and guest disks remain on the validation host; runtime tests use isolated containers and no physical device access.

Verified in the saved `artifacts/android-controls/` run: real AndroidKeyStore TLS, production SAF enrollment, independent local Android call UI, and native ARM64 Arch answer/decline/end with subsequent Telecom state. Replay, stale revision, revoked controls, changed authorization generation, wrong certificate pin and Stop checks passed. The package and source must be matched to final evidence before a release claim.

The initial installed GTK/Hyprland touch suite, direct-loopback suite, legacy-upgrade/renewal/boot additions and full framework regression passed. The final installed package also passed desktop-entry reuse, live palette changes and touch controls. `tools/verify-phone-delivery.py` matches the saved evidence to the delivered package, executed sources and test-signed APK. Earlier 38-test transport records in `artifacts/android-transport/` describe the earlier fixture-based status bridge and remain historical evidence.

The cross-VM route uses ADB forwarding solely as test plumbing. The proposed phone route uses shared loopback. A direct same-kernel test is separate from proof of the final Pixel container, SELinux, init, audio and device configuration. No carrier, IMS registration, PowerVR or physical voice-media success is claimed.

## Sources

- [Android default dialer responsibilities](https://developer.android.com/develop/connectivity/telecom/dialer-app)
- [Android foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [TelephonyCallback](https://developer.android.com/reference/android/telephony/TelephonyCallback.CallStateListener)
- [Python TLS validation](https://docs.python.org/3/library/ssl.html)
- [Official Arch runtime source](https://archlinux.org/download/)
