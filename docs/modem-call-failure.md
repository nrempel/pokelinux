# Answer failure investigation

The preserved failure in `artifacts/qml-service/android-call-failure/` is below the UI observation boundary:

- 01:41:19.007: radio `GET_CURRENT_CALLS` reports one INCOMING call.
- 01:41:20.344: Android sends radio `ANSWER` request 0231.
- 01:41:20.345: the radio acknowledges ANSWER, then request 0232 returns an empty call list.
- 01:41:20.348: Telecom records MISSED / INCOMING_MISSED.

The same run's earlier Android UI test returned ACTIVE after Answer. Thus neither a successful command receipt nor one earlier passing call proves reliable connection. The background-service probe failed before the QML test was reached. The guest's uncaught assertion caused the host to report a request timeout, obscuring the actual failure.

`sim/guest/guest_response.py` now returns FAILED for probe exceptions while retaining their traceback in the guest log. Expected transport unavailability remains UNAVAILABLE. Two red-green regression tests cover an assertion followed by another successful request and the separate unavailable result. This is disposable test-harness behavior, not a production protocol change.

The emulator supports `modem` and `radio` debug tags, verified with its own `-help-debug-tags`. The canonical framework runner now enables those tags for the next actual integration run. There is no Answer retry and no forced simulated ACTIVE state.

Public upstream reference source was retrieved under `artifacts/modem-audit/` from:
https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-master-dev/android/emu/telephony/src/android/telephony/

The reference `handleAnswer` changes incoming to active; `handleListCurrentCalls` enumerates voice calls. This is reference source from a moving branch, not a verified source match for the installed emulator. It does not establish the failure's cause. Modem-level evidence and a matched-source investigation remain necessary.

The diagnostic rerun passed the background and installed-service probes, including the real QML sequence, then failed in a later direct Answer at `native_enrollment.py:97`. Evidence is preserved in `artifacts/qml-service/android-modem-run/`; the overall run remains failed.

The running emulator identifies itself as 37.2.12-16428233 and creates `modem_simulator/modem_nvram.json`. The enabled legacy `modem,radio` tags did not expose AT traces in the inspected output. Public qemu source also has a modern wrapper under `android/third_party/modem-simulator/`, using Cuttlefish `call_service.cpp`. Its `modem_main.cpp` diagnostic macro is compile-time disabled (`MODEM_DEBUG 0`). Those reference files are saved beside the legacy source; `call_service.cpp` came from Cuttlefish main. This narrows the next investigation to the modern simulator and its exact build source. The legacy source alone cannot diagnose this run.

## Independent stock Android reproduction

`bash tools/framework-suite.sh radio-baseline` boots a fresh isolated emulator and asserts that `org.omarchy.bridge` is absent. The default role holder was `com.google.android.dialer`. Ten injected calls were answered once each using Android KEYCODE_CALL, with live Telecom state checked after each. Nine reached ACTIVE; iteration 8 did not. No failed Answer was retried or forced active.

The failure matches the bridge-run signature: at 01:57:56.258 radio request 0276 sent ANSWER; request 0277 returned an empty GET_CURRENT_CALLS list at 01:57:56.274; Telecom recorded INCOMING_MISSED at 01:57:56.276. Evidence: `artifacts/radio-baseline/result.json`, `radio.log`, and `telecom-8.txt`.

This proves the observed failure can occur without our APK, daemon or QML. It does not prove an exact emulator source defect, establish the sampled failure rate statistically, or validate real Pixel calling. Treat this as an unresolved test-environment defect while continuing bridge integration.

The baseline also found that `adb emu gsm list` returns only OK during confirmed active calls on this image. It is not a valid state oracle here. The Android UI test's End check now reads the live `CallsManager.mCalls` collection instead. A red-green test uses the actual active and missed-call dumps to distinguish current calls from historic SET_ACTIVE entries and rejects a plain OK response.

The stronger UI test passed on the actual Android emulator: incoming display, Answer, active display, End with no live Telecom calls, and Decline. Evidence and the real APK incoming screenshot are in `artifacts/callui-live-state/`. All three host harness tests pass. The stock-dialer baseline remains failed; no retry logic was added to the product.

The complete bridge enrollment/framework sequence passed in a separate verbose run. Evidence: `artifacts/qml-service/android-verbose-pass/`; all 20 recorded module/QML/package hashes match local sources. This is one passing integration run, not a resolution of the independent baseline failure.
