# Android framework integration environment

The environment runs Google's Android 17 / API 37 x86_64 emulator on Harbour with KVM. It tests Android service activation, Binder authorization, TelephonyCallback permissions, Telecom call delivery, and a reference IMS lifecycle. It does not emulate Pixel hardware or establish carrier service.

The tested image is `system-images;android-37.0;google_apis;x86_64`, revision 6: `google/sdk_gphone64_x86_64/emu64xa:17/CE2A.260420.019/15611780:userdebug/dev-keys`, security patch `2026-05-05`. This is an API 37 contract environment, not the Pixel factory build `CP3A.261005.005`. Debug-build behavior is not treated as proof of production admission policy.

## Reproduce

From the package root on Harbour:

```sh
bash tools/framework-suite.sh prepare
bash tools/framework-suite.sh build
bash tools/framework-suite.sh test
```

The build command reuses the graphics/audit images when present and builds them from this package's Dockerfiles when absent. Preparation downloads pinned official Google SDK archives and Android 17 system API stubs. Build uses Java 21, javac targeting Java 8, d8, aapt2, zipalign, and apksigner. No Gradle service or AOSP checkout is needed for this test scope. The cached SDK and emulator disks are under `android-framework/` on Harbour; evidence and APKs are copied to `artifacts/android-framework/` locally. Test signing keys stay in the disposable build workspace and are not deployment keys.

The test command cold-boots fresh emulator user data. It uses a private **internal-only** Docker network, no published ports, 4 CPUs, a 10 GiB memory cap, a non-root container user, dropped capabilities, and only `/dev/kvm` as a mapped host device. Android SELinux remains enforcing. Every adb invocation pins `emulator-5554` inside that container. No USB device, host adb server, physical phone, carrier, or host service is involved. The runner removes its container and private network on exit. Harbour remains temporary validation capacity; the build recipe is portable.

Loopback-only `--network none` failed because the emulator's modem socket address lookup uses address-family availability checks that reject an interface set with only loopback. An internal Docker interface plus container-local IPv4 selection fixed the virtual modem connection. No outside network route or published port is needed.

## What is tested

| Boundary | Evidence |
|---|---|
| Provider/client identity | Separate application UIDs; matching bridge/trusted-client certificates; a different certificate for the untrusted client. APK SHA-256 and signer digests are recorded. |
| Binder access | Same signer plus permission request succeeds. Different signer fails. Same signer without the permission request fails. |
| Forwarded Binder handle | A trusted client passes the acquired handle to an untrusted app. Per-transaction enforcement rejects that app's call. |
| Protocol | Wrong interface token, unsupported version, and unknown operation fail. |
| Framework phone permission | Actual `TelephonyManager.registerTelephonyCallback` rejects denied and revoked `READ_PHONE_STATE`; a granted bridge obtains a framework observation. No adopted shell identity is used by the clients. |
| Service availability | Explicit rebind after force-stop works; package disable causes a distinct missing-service result; enable restores binding. This does not assert automatic recovery from every force-stop. |
| Telecom | Emulator radio call → Telecom → our `InCallService` → protected Binder → separate trusted client. Ringing, active, removed, and role-removal states are checked. |
| Call-service protection | Ordinary same-signer client is denied direct binding by `BIND_INCALL_SERVICE`. |
| Reference IMS | Real resolver selection, query, MMTEL creation, readiness callback, framework service binding, and ordinary-app bind denial. Uses Android 17 system API stubs and the emulator's test override. |
| Wrong IMS permission | Resolver explicitly rejects a fixture protected by the wrong binding permission. No `override_bind_check` metadata or enforcement bypass is used. |

The bridge has a small versioned read-only Binder contract. It does not expose phone numbers, contacts, arbitrary intents, shell commands, or call-control operations. Its in-call snapshot is explicitly scoped to callbacks in the current process; an empty collection is not proof of carrier availability. The client APK and instrumentation runner are **test tools**, and the IMS implementation is explicitly a **reference fixture**. Its READY feature state is synthetic test input; it never reports carrier registration.

The dialer role is assigned through the emulator shell for integration testing. The placeholder Activity is a qualification fixture, not a complete dialer or a tested user-facing role-consent flow. Production call UI, direct boot, multi-SIM, and process-death reconciliation remain separate work.

## Stock Pixel package results

Factory APKs remain unmodified and retain their signatures. Each is checked against the earlier factory extraction manifest before installation.

| Stock package | Observed result |
|---|---|
| `com.shannon.imsservice` | Installs as a data app; resolver selects it; `com.android.phone` binds; query and MMTEL feature-creation calls execute. Hidden API checks reject methods including `TelephonyManager.getSimStateForSlotIndex`, `SubscriptionManager.isValidSlotIndex`, and an `ImsReasonInfo` constructor. A crash and an invalid feature container are recorded. A later READY notification does not establish usable IMS. |
| `com.android.telephony.imsmedia` | Installation rejected with `INSTALL_FAILED_SHARED_USER_INCOMPATIBLE`: original Pixel signer cannot join the emulator's `android.uid.phone` shared UID. Its ARM64 native dependency path is therefore not reached by this test. |
| `com.samsung.slsi.telephony.oemril` | Package installation succeeds. Installation is the only credited stage; its non-exported primary service and vendor dependencies are not activated by this test. |

This supports retaining the original matched Android framework, package placement, signing relationships, privileged permission rules, and vendor services for stock phone support. It does **not** establish the exact minimum Android subset. Moving APKs into a directory or re-signing them is not demonstrated as a solution. The test never disables hidden API enforcement, changes platform keys, grants signature-only permissions, or disables SELinux.

The emulator shell requires an explicit `-f 1` for this MMTEL override path. A successful command without an explicit feature was not sufficient evidence of selection; the runner verifies the queried selection, process-side callbacks, and framework binding.

## Additional framework defect found

On this emulator build, a wrong-permission IMS service with **no service metadata bundle** caused a `NullPointerException` in `ImsResolver.searchForImsServices` and restarted `com.android.phone`. The original log is retained as `results/ims-null-metadata-crash.log`. A normal, inert `org.omarchy.fixture` metadata item lets the separate wrong-permission test reach the intended resolver rejection. This does not disable the permission check. No upstream framework binary was changed; this is an observed emulator defect, not a Pixel-device result.

## Remaining integration boundaries

- An Arch process has no Android APK signing identity. The new [native status transport](arch-android-transport.md) uses mutual TLS and has passed 38 end-to-end ARM64 Arch/Android emulator checks without active instrumentation. Its final retained-Pixel namespace, enrollment, and production user interface remain unverified.
- Stock ARM64 native service execution, Samsung radioExternal, modem firmware, calibration and device nodes require a different hardware-dependent test tier.
- Carrier registration, VoLTE, Wi-Fi calling, SIM/eSIM provisioning, emergency calling, audio routing and audio quality remain unverified.
- PowerVR/DRM buffer allocation, synchronization, scanout and Hyprland integration remain independent of these telephony results.
- Existing ARM64 Arch/Hyprland/touch simulation and native graphics loader tests remain separate evidence. This x86 Android image is not a bootable Pixel image.

The next stock activation environment must preserve the factory package trust and framework contracts while providing the vendor interfaces. A generic x86 emulator cannot establish that by itself. The native status gateway now runs against these reference contracts, but must be checked again against that retained environment.

Official references: [IMS framework contracts](https://source.android.com/docs/core/connect/ims), [default dialer requirements](https://developer.android.com/develop/connectivity/telecom/dialer-app), [bound service lifecycle](https://developer.android.com/develop/background-work/services/bound-services), [emulator console](https://developer.android.com/studio/run/emulator-console). The Pro review is recorded in `artifacts/android-framework/pro-review.md`; executable evidence comes from the harness, not the review.

## Current Rust framework rerun (12 October 2026)

After restoring Harbour access, the current APK was rebuilt from the local
source. The isolated emulator passed AndroidKeyStore enrollment and the visible
Storage Access Framework export/import/confirmation/start/stop flow. The current
Rust suite passed four tests on native x86-64 and ARM64 through QEMU user mode:
TLS observation and rejected-command handling, receipt recovery after UI
connection loss, local RPC observations, and typed current-state watch.

Evidence is in `artifacts/harbour-current/framework-watch-pass/`, including both
architecture logs, source hashes and SELinux state. The suite's older result
scope string predates the watch test; use the test logs for exact coverage.
The first run required missing locked Cargo dependencies to be fetched. The
first typed-watch run failed because the fixture lacked the Android dialer role;
assigning that role in the owned emulator made the test pass. Production role
checks were retained. Desktop command switches remained disabled.

This does not prove accepted call actions, native UI integration with Android,
carrier IMS, physical audio, Pixel graphics or power behavior. An expanded
emulator radio test is separate from this checkpoint.

### Emulator radio through the Rust watch API

`artifacts/harbour-current/radio-watch-pass/` records a clean complete suite
exit. On both native x86-64 and ARM64 through QEMU user mode, the emulator
console injected one incoming call; the current Android InCallService exposed
it through authenticated TLS and Rust decoded an observed ringing call with a
valid ID. After emulator cancellation, the same API reported no ringing calls
with an available source. The four earlier framework tests also passed on both
architectures. SELinux remained enforcing.

This checkpoint tests radio simulation and framework callbacks, not carrier
IMS, accepted call controls, audio, notifications or the native Phone window.
The later background-worker assertion is not part of this checkpoint's source
hashes and needs its own completed run.

### Background observer on real emulator snapshots

`artifacts/harbour-current/background-radio-pass/` adds the daemon's production
BackgroundObserver to the incoming-radio test. With no native Phone UI, it
obtains a fresh one-call lease from the Android gateway, stops within two
seconds, and exposes no lease after shutdown. Both architecture runs and the
full preceding suite passed. This is worker-level proof; the notification
presentation owner is still not wired into the production daemon, so this does
not prove a closed-window banner, ringtone or lock-screen wake.

### Accepted Rust controls

`artifacts/harbour-current/accepted-controls-pass/` records Answer and End
through the Rust coordinator, local RPC dispatch, durable receipt path and
current Android gateway on both architectures. The test enables desktop call
controls through the visible Android switch, injects a radio-simulator call,
requires a submitted receipt for Answer, then verifies Telecom reports ACTIVE.
End must return a submitted receipt and the subsequent call list must be empty.
The switch is disabled after each architecture's test. The complete suite
exited 0. These are emulator call-state transitions, not audio or carrier proof.
A native Qt touch integration target is being built separately; this checkpoint
does not claim UI touch execution.
