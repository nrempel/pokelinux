# IMS replacement checkpoint

11 October 2026. No physical phone accessed. This is a feasibility comparison, not a completed replacement implementation.

## Decision

Do not treat retaining Android as a permanent requirement. Current evidence favours the public Telecom bridge for the first working phone, but does not prove that a smaller native replacement is impossible. The next alternative to evaluate is an existing Linux telephony stack plus Pixel-specific vendor support, rather than a new complete telephony stack.

Our bridge APK is original Java source under `framework/bridge/`. It calls public Telecom APIs and receives InCallService callbacks. There is no need to reverse engineer it. Removing this helper does not remove the vendor IMS or modem requirements.

The stock Shannon APK is a different component. Replacing it would require establishing the division of work between its Android code, vendor radio services, modem firmware and separate media service. The existing factory audit does not establish that division completely.

## Evidence checked

- Existing factory evidence: ShannonIms.apk SHA-256 `8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879`. Its method table includes Samsung radioExternal `sendRequestRaw`, `sendRequestRawSegment`, callback setup and raw indications. This is evidence of a vendor protocol surface, not a decoded message contract or proof of runtime invocation.
- Its type table includes Android IMS media audio configuration, media sessions, carrier configuration and connectivity types. The APK contains no bundled native libraries in the audit; that does not mean it has no external native dependencies.
- The previous emulator activation experiment reached Shannon framework binding but encountered hidden API failures. Pixel IMS media installation failed the shared-UID signer check. These demonstrate integration constraints, not that replacement is impossible.
- AOSP documents that IMS may run on the application processor, modem, or both, and that modem offload uses vendor HAL extensions. Standard radio AIDL alone is not a complete IMS implementation.
- Sailfish's `ofono-binder-plugin` is an existing Linux-to-Android radio adaptation. Its current `binder_plugin.c` explicitly selects AIDL or HIDL, so this is not merely an obsolete HIDL-only lead. Exact Pixel AIDL version compatibility remains untested. Its IMS registration source uses `BinderExtIms`, `BinderExtCall` and `BinderExtSms` in addition to radio interfaces. No matching Pixel 10 extension has been established by this review.

## Smallest useful comparison

1. Pin the open-source oFono adaptation and compare its supported AIDL versions, service names and callback types with the factory VINTF declarations.
2. Map ownership of IMS registration, call setup, IMS data bearer, SIM authentication and RTP/audio. Mark unknown ownership explicitly; do not infer it from class names.
3. Identify whether a documented vendor interface can replace Shannon at a narrow boundary. A raw request transport alone is insufficient without message formats, sequencing, errors and recovery.
4. Compile a candidate Linux adapter against a synthetic Binder service on Harbour. Verify serialization, asynchronous indications, disconnect/reconnect and failures using independently specified fixtures. This proves local interface behaviour only.
5. Before selecting it as the main implementation, require later authorized hardware evidence for real IMS registration, outgoing/incoming calls, bidirectional audio, suspend/wake and recovery. Carrier provisioning, Wi-Fi calling and emergency behaviour remain separate validation areas.

A mock that repeats assumptions from our implementation cannot resolve undocumented modem semantics. No offline result can establish carrier acceptance or physical audio routing.

## Packaging distinction

Our original helper and Linux UI can use a redistribution-friendly licence; a project-level licence still needs to be selected and added. Upstream components retain their own obligations. Google's factory-image terms restrict redistribution and reverse engineering except where device terms specifically permit it. A replacement written independently does not itself grant redistribution rights for retained firmware, GPU drivers or radio libraries. This checkpoint is not a legal determination of applicable interoperability rights.

## Primary references

- [Android IMS implementation contract](https://source.android.com/docs/core/connect/ims)
- [oFono Binder adaptation](https://github.com/mer-hybris/ofono-binder-plugin)
- [oFono AIDL selection](https://raw.githubusercontent.com/mer-hybris/ofono-binder-plugin/master/src/binder_plugin.c)
- [oFono IMS registration source](https://github.com/mer-hybris/ofono-binder-plugin/blob/master/src/binder_ims_reg.c)
- [Google factory image terms](https://developers.google.com/android/images)
- Local evidence: [cellular audit](cellular-ims-audit.md), [framework experiments](android-framework-tests.md), and `artifacts/telephony-audit/results/evidence.json`.

## Independent review

BRIDGE VERIFIED: Chat selected; composer Power showed Pro (5 of 5); marker `PRO-BRIDGE-20261011-ims-replace-c83f` matched; final response actions appeared and Stop was absent. One context round corrected an initial keyboard-split prompt. Review conversation (private transcript). Owned tab closed.

The review agreed that the retained Android route is better supported by current evidence, while a narrow modem/media interface could change that decision. It specifically cautioned that neither static references nor an emulator mock prove which responsibilities can be removed. These are reasoning conclusions; only the local artifacts and primary documentation supply implementation evidence.
