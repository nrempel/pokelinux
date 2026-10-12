# Command recovery and acknowledgment

Client 0.4.0 adds `omarchy-phone-local recovery`. The authenticated local Unix socket returns the current daemon epoch plus up to 64 retained uncertain operations. Each item contains only its original epoch, operation ID, state and bounded outcome/reason. Raw commands, numbers and command digests are not returned.

Pending admissions and unknown results are included. A finished record whose outcome is `unknown` is also included. Confirmed submitted/rejected results are omitted. `more: true` means additional uncertainty exists; a consumer must not treat the returned list as complete. Reading the query neither writes the ledger nor executes or retries any command. Restart converts interrupted pending admissions to unknown using the existing ledger recovery rule.

The CLI validates envelope identity, field sets, IDs, states, duplicate identities, result consistency, record count and overflow. A stale daemon without this API returns an unavailable result; it is not interpreted as an empty recovery list. Consumers must match the returned daemon epoch to their current observation before combining state.

Evidence: `artifacts/recovery-package/`. Red-green Linux tests cover persistence, immutable readback, absence of raw numbers, explicit overflow, malformed responses, and the production daemon/CLI socket after two restarts. All 110 Linux tests passed. The real ARM64 package test seeded an interrupted admission, started the installed systemd service, and read the original operation as unknown through the installed CLI. Package and installed source hashes match.

Historical 0.4.0 checkpoint (superseded below): this was recovery discovery, not completed interface recovery. QML does not consume the query yet. There is no acknowledgment, automatic replay, new dispatch gate, or result-resolution policy in this change. The native interface must integrate it before shell-restart handling of pending actions is complete. Android and Pixel hardware claims are unchanged.

## Session and acknowledgment implementation

The source now provides `omarchy-phone-local session` and `acknowledge --epoch CURRENT --session TOKEN --record-epoch ORIGINAL --record-id ID`. A session claim fences subsequent admission from previous controller sessions and invalidates earlier control observations. The descriptor contains the daemon epoch, session token and bounded recovery page. Control supports `--session TOKEN`; the token never enters the Android command payload.

Acknowledgment writes a separate durable table. It does not alter execution state, outcome, digest or replay protection. Pending records cannot be acknowledged. Missing or expired session authority rejects the request. Acknowledged uncertainty leaves the recovery attention list but remains available by original ID. Known outcomes need no acknowledgment. Repeating an acknowledgment does not dispatch. After acknowledgment the daemon invalidates observations, including requests that started before acknowledgment and complete afterward. A new read with unchanged Android revision is sufficient to refresh authority.

New command admission checks the ledger's unresolved attention list and blocks until recovery is complete. This includes records beyond the first response page. The existing admitted worker must finish before acknowledgment is accepted. No worker is marked terminal while it can still start a first dispatch.

Evidence: `artifacts/background-service/session-green.log`, `ack-red.log`, and `ack-green.log`. The final full Linux suite on Harbour passes 121 tests. Real Unix sockets and CLI subprocesses cover session replacement, stale and missing session rejection after a claim, acknowledgment without Android dispatch, immutable unknown results, persistence, legacy database upgrade, hidden-page blockers and stale-observation ordering. Local macOS broad socket tests encountered sandbox bind restrictions; Linux is the authoritative socket result.

Historical checkpoint before consumer migration: QML integration and package verification were still pending; the sessionless compatibility path described in that checkpoint has now been removed. The ledger's 1024-entry capacity remains a production lifetime issue to resolve without evicting live replay guards. No hardware claims change.


## Native consumer migration checkpoint

Base package 0.5.0 and plugin 0.4.0 implement mandatory session authority and explicit native recovery controls. QML starts with controls disabled, claims a session before clearing local pending state, and requires a fresh observation after acknowledgment. A lost command reply fences delayed old-session helpers before recovery proceeds. Acknowledgment accepts uncertainty; it never asserts that the original action failed and never retries it.

Harbour's full Linux suite passed 122 tests (`artifacts/background-service/required-session-tests.log` on Harbour). The ARM64 installed-base/QML test passed in `sim/results/qml-service-20261012T034420Z`: seeded unknown acknowledgment, Answer applied with a lost reply, explicit second acknowledgment, then End; exactly two backend transmissions. The adapter is synthetic, not Android or Pixel hardware.

The ARM64 component suite passed; locally retained evidence is `artifacts/recovery-card/`. Its tests cover acknowledgment identity changing while pressed, hidden call controls during recovery, and touch target size. `recovery.png` is an actual Qt software-rendered component with synthetic state.

The full installed Omarchy shell restart/touch test passed in `artifacts/native-recovery/` (Harbour `phone-recovery-20261012T035140Z`). The test applied Answer and lost its reply, stopped the entire shell process group, restarted the shell, verified the same unresolved operation identity, acknowledged by virtual touchscreen, and ended the call by a separate touch. Backend transmissions were exactly Answer and Disconnect. Both package hashes and all eight installed plugin runtime source hashes match the local artifacts. The adapter was synthetic; Hyprland, Omarchy, QML, CLI and the Unix daemon were real ARM64 software. The package upgrade protocol, real Android regression for these versions, and the 1024-operation lifetime limit remain open. These results do not prove modem, IMS, audio, GPU, camera, or wake behavior on the Pixel.


Current-version Android regression also passed: `artifacts/native-session-android/`. Client 0.5.0/plugin 0.4.0 installed on ARM64, enrolled AndroidKeyStore TLS, production enrollment UI, native touch Answer, independent Android Telecom ACTIVE, touch End and Android idle. SELinux was Enforcing. Installed plugin, Python and Android source hashes and both native package hashes match local sources. This normal-call emulator test is separate from synthetic lost-reply/restart injection; neither proves carrier IMS, physical call audio, or Pixel graphics.


Stronger abrupt-termination test passed: `artifacts/native-recovery-crash/` (Harbour `phone-recovery-20261012T040000Z`). After the synthetic backend received Answer, it held the daemon worker active. The harness SIGKILLed the complete shell group. The restarted panel discovered the durable pending operation, kept controls busy and acknowledgment disabled, then showed that same identity as unknown after the backend was released with a lost reply. Explicit touch acknowledgment followed by End produced exactly one Answer and one Disconnect. Current source hashes match. This establishes shell-crash recovery with a live daemon and synthetic remote work; daemon-death plus late Android execution remains a distinct fault-injection scope.

The full Linux transport suite now passes 123 tests (`artifacts/background-service/recovery-commit-tests.log`). The added test forces actual SQLite acknowledgment COMMIT failure through a deferred constraint, verifies rollback leaves uncertainty and the original operation intact, then verifies a later successful acknowledgment. It also checks the writer uses DELETE journal mode and FULL synchronization. No production behavior was changed for this test.


Coordinated native package upgrade passed: `artifacts/native-session-upgrade/`. Actual pacman upgraded base 0.2.0/plugin 0.3.0 to base 0.5.0/plugin 0.4.0. Both shell and synthetic-adapter daemon stopped before replacement; the new daemon epoch differed and the seeded durable historical record was byte-for-byte unchanged at the row level. New shell discovery, GIO desktop-entry activation, touch Answer/End, disable/unregister/remove/reinstall/register and persisted disable all passed. No automatic call commands occurred. This fixture uses installed daemon classes with a synthetic adapter, not a systemd-managed Android-backed upgrade. See `service-package.md`.
