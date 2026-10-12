# Native QML service boundary

`omarchy-plugin/qml/PhoneService.qml` now connects the call controller to the installed `omarchy-phone-local` command. It runs read-only observation independently of any visible card, captures the observation barrier before each process, and only submits commands from explicit `requestAction` calls. A separate short Python process generates a UUID after the gesture; the original press context is rechecked before dispatch, so a call change during identity creation cannot retarget the action.

`JsonCommand.qml` gives each child process its own lifetime and context. It rejects overlapping jobs, passes literal argument arrays without a shell, waits for both process exit and stdout completion, bounds accepted JSON, and terminates a timed-out child. Old completion callbacks cannot complete a newer job. Quickshell owns these short client processes; the independently installed systemd service owns admitted commands and their durable results.

Timers only request snapshots and saved receipts. They never repeat a command. A failed command process makes the result unknown; later matching receipt reads can resolve it. This does not yet discover pending operations after a full shell-process restart, and it is not an installable Omarchy plugin until its manifest/panel/bar entry points and delivery are complete.

## Verified checks

- Actual ARM64 Quickshell tested literal arguments, overlap rejection, malformed JSON, nonzero exit, timeout, missing executable and a successful job after failures. Evidence: `artifacts/qml-process/`.
- QML -> installed CLI -> actual private Unix socket -> installed daemon/dispatcher/ledger -> synthetic Android adapter passed as desktop user 1000. Exactly Answer then Disconnect were dispatched, with original expected revisions 3 and 4 and unique operation IDs. Evidence: `artifacts/qml-service/`.
- The direct installed CLI returned valid snapshots while the first QML integration rejected them. Normalizing the signal-delivered response before controller schema/array checks fixed the integration. This boundary was not exercised by the earlier tests that called the controller with JavaScript object literals.
- The card/controller regression suite passed all 19 checks after that fix. Current evidence: `artifacts/qml-regression/`.
- The first real Android/QML attempt failed while writing its log into the guest's read-only source mount. The harness now writes to `/results/qml-service.log`. Its rerun failed earlier, in the existing background-service Answer probe: the command receipt was accepted, but no active call was observed. The guest assertion then appeared as an outer request timeout. This rerun did not reach QML. Evidence: `artifacts/qml-service/android-call-failure/`. Real Android/QML success and the intermittent answered-call failure remain unresolved.

The simulator uses software graphics. No phone, PowerVR, carrier IMS, voice audio or hardware wake was tested. The adapter tests are not touch-input tests. Closed-panel incoming presentation and full shell restart recovery still need actual plugin integration.

## Real Android checkpoint

The subsequent diagnostic run completed `qml_user_service`: the actual QML service ran as UID 1000, through installed package 0.2.0 and the enrolled Android emulator, observed Answer -> active and Disconnect -> idle. `qml-service.log` contains `PHONE_SERVICE_CHECK_PASSED`. All nine recorded QML/test source hashes and the installed package hash match the local sources. Evidence: `artifacts/qml-service/android-modem-run/`.

The overall suite still failed later in a separate direct Answer test, which returned no active call. Keep its top-level `passed: false`; the nested QML success is a narrower milestone. Input was programmatic QML method calls, not touch, and the full Omarchy plugin was not loaded.

The next verbose run passed the complete enrollment/framework sequence, including QML, permission revocation, stale/replay rejection, wrong-pin rejection and stopping the bridge. All 20 recorded native module/QML/package hashes match current local files. Evidence: `artifacts/qml-service/android-verbose-pass/`. This passing run does not resolve the previously recorded intermittent radio failure.

Reference APIs: https://quickshell.org/docs/v0.3.2/types/Quickshell.Io/Process/ and https://quickshell.org/docs/v0.3.2/types/Quickshell.Io/StdioCollector/ .
