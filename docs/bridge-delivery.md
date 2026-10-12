# Bridge delivery and activation

This deliverable is the offline-tested phone-control bridge. It is not a bootable Pixel operating system. The Android and native Arch applications have been tested together; the Pixel init/container layout, GPU/display integration, carrier registration and voice audio still need separate work.

## Artifacts

- `artifacts/client/omarchy-phone-0.1.0-1-any.pkg.tar`: native GTK app, launcher entry, palette support, pairing issuer and CLI. No credentials.
- `artifacts/android-controls/bridge-unsigned.apk`: production bridge executable, not installable until signed.
- `artifacts/android-controls/bridge-test-signed.apk`: the same APK payload under the disposable emulator test signer. This is test evidence, not a distribution identity.
- `artifacts/android-controls/release-payload.json`: byte hashes of all APK payload entries, compared with the emulator-tested APK. APK signing metadata is excluded from this comparison.
- `artifacts/android-controls/delivery-verification.json`: results matched to exact source and package hashes. Its hardware, carrier and voice-media fields remain false.

Keep the test fixture APKs, provisioner, enrollment probe and test keys out of a deployment. The test framework builds them separately to exercise permission and certificate failures.

## Build and check

On the prepared, isolated Linux build environment:

```sh
bash tools/build-release-apk.sh
python3 tools/build-client-package.py
python3 tools/verify-release-apk.py artifacts/android-controls/bridge-unsigned.apk
python3 tools/verify-release-payload.py \
  artifacts/android-controls/bridge-unsigned.apk \
  artifacts/android-controls/bridge-test-signed.apk
python3 tools/verify-phone-delivery.py
```

The production build command needs JDK, Python, zip, and the pinned Android SDK layout described by `tools/prepare-framework.py`. It does not download anything, create signing keys, build test packages or access a device. It refuses an existing output file; pass a new output path as its second argument for another build. The payload comparison proves equivalence to the tested APK, not byte-for-byte reproducibility of ZIP timestamps or signatures.

A distribution owner must sign the unsigned APK with their controlled Android app-signing identity before installation. Keep the same identity for upgrades. No distribution signing key was created for this project. Installing an APK signed with a different identity over an existing installation is not an upgrade path; do not erase enrollment data to bypass that check.

## Activation contract

1. Start a compatible Android environment with Telecom and the required vendor cellular services. This is a prerequisite, not something this APK supplies. The intended phone route is shared local loopback; the gateway never listens on LAN interfaces.
2. Install the signed production APK through that environment's approved application mechanism. Install the Arch package with pacman. No root privilege, signature permission bypass, host ADB service or privileged Android app status is required by the tested application path.
3. Open the Android bridge. Request the default-phone role through its visible control. Phone-state and notification permissions are separate controls. Declining a permission must leave the affected observation or feature unavailable.
4. Export the public pairing request through Android's document picker. On Arch, run `omarchy-phone-pair request.json /new/private/enrollment-directory`. This creates the desktop private credentials; the directory must remain private and outside release artifacts.
5. Import only `phone-enrollment.zip` on Android. Compare the displayed fingerprint over a trusted path and confirm. Select `client.json` in Omarchy Phone. Pairing alone starts no gateway and grants no call-control authorization.
6. Explicitly start the Android bridge and enable desktop call controls. Enable boot startup only if wanted. The tested boot path starts after Android boot and unlock; it is not pre-unlock direct-boot support.
7. Use the local Stop control to disable access. Stop persists across reboot. Replacing enrollment requires the bridge to be stopped; previous client credentials must fail after replacement.

The user can optionally start the native window at login as described in [Omarchy integration](omarchy-phone.md). Closing it stops the native client. Do not claim background Linux ringing, an emergency dialer or a complete phone UI from the tested answer/decline/end path.

## Target acceptance still required

Before phone deployment, prove the retained Android services can start under the selected Pixel init/container policy, that the paired client has the intended network route and file access, and that SELinux enforces the chosen boundaries. Then validate real carrier/IMS registration, bidirectional audio and routing, incoming calls during suspend, power use, modem recovery and the graphics/display path. None of those tests was performed on a phone.

Harbour provides temporary build and validation capacity only. The installed applications have no Harbour address, account, service dependency or inbound Internet requirement.
