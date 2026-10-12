# Native phone background service

`transport/phone_service.py` implements background observation and a local snapshot endpoint, plus optional call-control dispatch used by integration tests. The CLI still starts observation only. It is not yet the complete call-control service and is not installed or enabled by the Arch package.

The service has two independently polled Android observations: call controls and outgoing-call availability. It reads through the production mTLS client. Closing a local connection does not stop either poll. Each snapshot contains a service epoch, increasing sequence and an explicit freshness flag for each observation. A failed or delayed observation does not become an empty/idle call state; the last value remains marked stale. Time uses Linux BOOTTIME where available, so suspend does not extend authority.

The Linux endpoint is a private pathname Unix socket with same-UID peer verification, eight concurrent handlers, bounded JSON input, and read/write timeouts. Exactly one request is accepted per connection. Snapshot requests are always available; when constructed with a Dispatcher, the endpoint also accepts control and read-only result requests. Dial is not connected. Same-UID access does not isolate the plugin from other applications owned by that user.

Control admission checks the original Android generation, revision, call state and allowed action against a fresh observation. It durably reserves the operation ID and payload digest before dispatch, then preserves that ID through the Android request. Identical duplicates return the ledger record without dispatch; changed arguments are rejected. A lost Android response becomes unknown. Invalidating an observation also discards polls that began before that invalidation. Closing the local socket does not cancel an admitted command or remove its result. These command paths pass Linux socket tests and a real ARM64 Arch-to-Android framework integration run, described below.

A private runtime-directory lock gives one owner. A new owner can remove a stale socket only after obtaining the lock and checking the socket type and owner. The process handles SIGTERM/SIGINT and closes its socket and polling workers. This module must run independently of the shell process; the user-service unit and package integration remain to be added.

Example for the emulator/test environment (paths must refer to that environment):

```sh
python3 transport/phone_service.py --config /private/path/client.json --runtime /private/runtime/omarchy-phone
```

The local request is `{"v":1,"id":"<32 lowercase hex digits>","op":"snapshot"}` followed by a newline. The response echoes the ID and contains the snapshot. The future QML client must treat transport loss and stale observations as unavailable, and must discard responses from superseded connections. A heartbeat or a fresh local response does not by itself prove Android freshness.

## Verification

- Latest local Python suite: 93 tests discovered, 88 passed and five Linux-only socket service tests skipped on macOS.
- Latest Harbour Python suite: all 93 passed, including command completion after local disconnect, duplicate suppression, conflicting-payload rejection, read-only result lookup and delayed-poll invalidation.
- `framework/tests/service_probe.py` adds a production-client integration to the native ARM64 Arch enrollment suite. It checks a synthetic incoming call through the local service socket before and after client disconnection. This run uses the existing emulator test transport (ADB forwarding inside the owned test container), not the direct shared-kernel route. The run passed. `artifacts/background-service/native-enrollment.json` records incoming state 2 before/after local client disconnection; the service ran as guest UID 0. `enrollment-provenance.json` and the probe source hashes describe that earlier observation-only checkpoint; subsequent dispatcher changes are not covered by that run. This is service observation proof, not desktop-user lifecycle, ringtone, wake, or QML presentation proof.
- The same service socket tests pass in the exact x86_64 Arch runtime image. A separate attempt to run the filesystem socket service as Android shell UID 2000 failed at bind with EACCES. Evidence: `artifacts/background-service/android-shell-bind-error.txt`. This identifies a deployment permission boundary; it does not by itself identify which SELinux rule caused the denial. Android enforcing mode was preserved. The direct-client regression retains its original scope; this denied-bind attempt is not reported as passing.

## Remaining integration

Connect outgoing dispatch, preserve separate incoming/outgoing uncertainty, implement recovery and bounded historical-record retirement, add the independent user-service lifecycle and QML presentation, then repeat real framework and touch tests. Native incoming ringing, automatic Answer/Decline presentation, silent/DND behavior and suspend/wake remain unfinished. The earlier APK/package evidence does not cover these new modules. Pixel graphics, modem/IMS, audio and device boot remain separate hardware limits.

## Real command integration checkpoint

`artifacts/background-service/commands/native-enrollment.json` and `enrollment-provenance.json` cover the current production transport and Android bridge source hashes, checked locally after retrieval. The ARM64 Arch guest runs the production local client, service, dispatcher, SQLite ledger and mTLS adapter against the enrolled Android emulator. Answer completes after the submitting local socket closes; read-only result lookup recovers the receipt; repeating the same command returns the receipt without another counted invocation of the real Android adapter. A subsequent fresh observation shows ACTIVE. End returns the call to observed idle. A separate call is declined through the service and observed idle.

This run passed with Android SELinux Enforcing and no physical phone access. It uses the established cross-VM ADB test forwarding and guest UID 0; it does not prove the desktop user unit, shared-kernel socket policy, UI, audible ringing or suspend/wake. No task-owned emulator container remains running after the suite cleanup.

The expected red run failed because the old probe only observed calls. The first implementation run failed earlier in the existing Android call-UI test: Telecom recorded REQUEST_ACCEPT/SET_ANSWERED followed by MISSED, without a crash. The instrumented rerun passed, with radio ANSWER followed by an ACTIVE current-call record. The intermittent failure is not explained or fixed by that pass. Its preserved evidence is on Harbour under `artifacts/background-service/ui-failure/`; all three run logs are saved with the command evidence. Retain this as an open reliability issue, not a timing fix or proof of flawless calls.

## Native CLI and production startup work — 2026-10-12 UTC

`phone_local.py` now exposes `control` and read-only `result` subcommands while preserving the existing default snapshot command. Arguments are passed as a process argument array, with no shell evaluation. The caller supplies the original daemon epoch, operation identity, Android generation, revision, call ID and state. No failed command is retried automatically. A transport failure reports unavailable with the original identity, not a fabricated rejection; an absent receipt remains distinct from a terminal outcome.

Four new subprocess tests use real Unix sockets to verify the exact transmitted command, read-only result lookup, explicit absence, invalid argument rejection and no reconnect after a lost response. The complete Linux transport suite passed 97 tests before the next daemon-startup change.

The real ARM64/Android service probe now calls the production CLI for saved-result recovery, duplicate submission and End. This integration run is in progress; do not treat the changed probe as proof until its result is recorded.

A separate red test exposed that the normal daemon entrypoint only enabled observation. Production startup is being connected to the existing durable Ledger and Dispatcher, with an explicit `--state` directory or the standard XDG state location. The lifecycle test starts the actual daemon process, checks recovery of an unresolved preexisting operation without replay, checks private database permissions, sends SIGTERM, and verifies socket cleanup and a fresh service epoch on restart. This startup implementation remains unverified at this checkpoint.

### Updated evidence

The normal daemon startup lifecycle test now passes on Linux: the real process opens the durable ledger, exposes read-only recovery of a preexisting unresolved operation as unknown, closes its socket on SIGTERM, and starts again with a distinct service epoch. Its database is mode 0600. The implementation closes request workers before their ledger.

Run from source (not yet packaged):

```sh
python3 transport/phone_service.py --config /path/to/private/pairing.json \
  --runtime /path/to/private/runtime --state /path/to/private/state
python3 transport/phone_local.py --socket /path/to/private/runtime/phone.sock snapshot
python3 transport/phone_local.py --socket /path/to/private/runtime/phone.sock result \
  --epoch PREVIOUS_SERVICE_EPOCH --id ORIGINAL_OPERATION_ID
```

The first updated Android run proved both native CLI service scenarios: Answer after the local client disconnected, receipt lookup and duplicate suppression through the CLI, active state, CLI End and idle; then Decline after local disconnection and idle. The later direct-client Answer scenario failed to observe active state and instead observed no call. **The full suite failed.** Evidence is preserved in `artifacts/background-service/cli-integration/`; this does not resolve the earlier intermittent Android call failure. The next run adds continuous radio logging and captures Telecom and general logs before cleanup.

### Final checkpoint for this change

The current source now passes **98 Linux Python tests**, including the real daemon startup/shutdown/restart test. The instrumented ARM64/Android enrollment suite also passed with SELinux Enforcing. Native CLI receipt lookup, duplicate suppression, End, and both background-service incoming scenarios passed. All recorded transport and Android source hashes were compared with the local files and match. Evidence: `artifacts/background-service/daemon-cli/`, including `source-verification.json` and the radio/Telecom diagnostics.

The previous call loss remains unexplained. A subsequent successful run is not a fix, and no automatic command retry was added. QML controller integration, systemd user-unit delivery, package installation and service recovery UI remain unfinished. The package hash in the framework provenance still identifies the earlier GTK package; it does not claim these new modules are packaged. No phone, real modem/IMS, speaker audio or physical wake was tested.
