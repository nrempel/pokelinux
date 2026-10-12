# Current completion audit

**Expanded completion scope:** [Omarchy smartphone goal](smartphone-goal.md) is the authoritative product checklist: calls, messaging, camera, power, connectivity, sensors, background/lock-screen behavior and live Omarchy theme integration. The checkpoints below are historical evidence, not a complete feature inventory. Passing the call bridge alone does not complete this goal.

**Current target change — Rust, no backwards compatibility:** The clean Rust target has typed local requests, digests, a Linux socket reader, a durable SQLite journal and pinned mutual-TLS transport. Twenty-seven native Linux component tests and twenty-six ARM64 QEMU user-mode component tests pass on Harbour; formatting and Clippy pass. Separate real Android emulator tests pass from both architectures for status, controls and dial availability, disabled-command rejection and strict request parsing. This found and fixed the Java endpoint's JSON field-order dependency using `WireRequest`. All six Java call-policy suites pass. The unsigned Android APK has the same executable payload as the emulator-tested APK. Retrieved source inventories match current Rust and Android sources. Evidence: `artifacts/rust-core/` and `artifacts/rust-android/`. Typed control/dial observations, command encoding and command receipts now pass through the exact production contract code in the real emulator checks. The Rust daemon, production enrollment loader, coarse phone-status validator, executed call integration and installed Arch package are still incomplete. Earlier Python/Java call results do not prove those Rust paths. See `runtime/README.md`.

The offline bridge is built and tested. The complete native phone experience and Pixel deployment are not complete. Do not treat `offline_bridge: PASS` as completion of the full phone project.

| Requirement | Current evidence | Verdict |
|---|---|---|
| Explicit, secure enrollment | AndroidKeyStore TLS, SAF export/import, fingerprint confirmation, invalid-request rejection, replacement and old-client rejection in `artifacts/android-controls/` | Verified in emulator |
| Lifecycle and user controls | Opt-in boot, three Stop/Start cycles, Stop across reboot, role/permission regression and authorization generation tests | Verified in emulator |
| Native Arch integration | Installed ARM64 pacman package, GTK under Hyprland, installed desktop-entry activation, one window, live theme and actual uinput touch | Verified in VM |
| Call-control boundary | Answer/decline/end observed through Android Telecom, stale/replay/revocation rejection, unknown outcomes retained | Verified with synthetic calls |
| Production build artifact | Separate unsigned APK build, no signer creation or test APK build; all executable entries equal the emulator-tested artifact | Verified on Harbour |
| Deployment components | APK, Arch package and activation runbook present; shared-loopback Arch glibc client runs under enforcing Android emulator | Component proof only; no Pixel platform image |
| Independent build/test capacity | Owned Docker images, framework/VM runners, bounded resources and recorded results on temporary Harbour | Verified; runtime does not depend on Harbour |
| Native phone functionality | Incoming controls implemented; outgoing API passes two real emulator calls from Arch glibc; native GTK dial UI verified with two outgoing touch calls; QML plugin integration not yet implemented; background Linux call presentation/ringing absent | Incomplete |
| Pixel kernel, retained Android environment and graphics | Stock PowerVR libraries load; GPU initialization cannot complete without devices; exact Pixel init/container/SELinux/display arrangement not implemented and verified | Incomplete |
| Cellular/IMS and voice | Reference IMS lifecycle and negative permission tests pass; stock dependencies audited; real modem/carrier/audio path absent from emulator | Hardware-dependent proof missing |

Outgoing API and native client code now compile. The direct shared-kernel Arch glibc test passed two outgoing synthetic calls through mTLS and public Telecom, duplicate receipt preservation, changed-number rejection, correlated terminal journal clearing, and subsequent incoming answer/end/decline. Evidence is in `artifacts/outgoing/direct-transport.json`. No ADB data forwarding was used; Android SELinux remained enforcing. This is emulator calling, not carrier or voice-audio proof. The native GTK outgoing layout and optional keypad are implemented; graphical touch validation passed for two outgoing calls and the incoming controls. Python tests: 53 pass; Java admission, control, journal and outgoing policy suites pass.

The separate lifecycle regression also passed: opt-in reboot, three Stop/Start cycles, and Stop across reboot. Graphical touch validation passed. The saved APK, installed ARM64 package, complete GUI source inventory and direct-client source inventory match current files; see `artifacts/outgoing/source-verification.json`. Existing delivery artifacts describe the earlier incoming-control build. The older full delivery verifier still covers the earlier release and must not be treated as proof of the expanded release.

Next software work should address the released Omarchy QML plugin interface, incoming-call presentation when the panel is closed, and explicit recovery of uncertain outgoing requests, with explicit authorization and real framework tests. These features must not weaken the existing command replay, freshness, role, enrollment or transport boundaries. Pixel-specific boot and service deployment still require a separate target integration; generic emulator success cannot prove them.

QML integration checkpoint: the bounded local JSON codec now has eight real Unix-socket tests; the full local Python suite has 61 passing tests. See `docs/pro-qml-review.md` for the verified independent review and remaining lifecycle/admission requirements. No QML plugin or background daemon is integrated yet. The new transport module is outside the earlier tested package; the outgoing source-verification report is a historical checkpoint, not current full-inventory proof.

Durable service-ledger checkpoint: `transport/phone_ledger.py` commits reservations before granting dispatch, serializes concurrent admissions, rejects changed-payload duplicates, rotates service identity on restart, and converts unfinished records to unknown. Nine real SQLite tests cover these rules, exclusive ownership, bounded capacity, immutable results, failed writes and abrupt subprocess exit. The complete Python suite passes 70 tests locally and on Harbour. These are component tests, not QML/daemon or Android end-to-end proof. The ledger is not yet packaged or used by the active client.

The background service still needs validated request admission, observation freshness, recovery enumeration with separate incoming/outgoing scopes, and a safe retirement policy for historical ledger entries. The current ledger intentionally rejects new work at its total capacity rather than evicting replay guards. It must not be presented as the finished long-running service until these requirements and the actual socket/framework integration tests pass.

Background observation integration passed on ARM64 Arch against the real enrolled Android emulator, including local-client disconnect/reconnect while an incoming call remained observable. All 77 Python tests pass on Harbour; the exact Arch runtime image also passes its seven service tests. Source hashes were checked after the run. The same filesystem socket bind was denied in the Android shell domain; that deployment finding remains separate and unresolved. Native ringing, automatic incoming-call presentation, suspend/wake delivery, daemon commands, QML integration and Pixel hardware behavior remain unverified. Incoming-call handling must work while the panel is closed; it is a required phone behavior, not an optional enhancement.

QML runtime checkpoint: the preserved ARM64 guest was extended with Quickshell 0.3.2 and Qt 6.12. A real guest check as the desktop user passed all 13 requested module imports and event-loop exit with software/offscreen rendering. See `artifacts/qml-runtime/`. The native observation client passes through the service socket; all 83 Python tests pass on Harbour. The visible QML phone plugin, incoming ringtone, automatic panel presentation and suspend/wake behavior remain incomplete.

Background command checkpoint: all 93 transport tests pass on Harbour. Real ARM64 Arch service integration now passes Answer after local-client disconnect, read-only receipt recovery, duplicate suppression, observed ACTIVE, End to idle and Decline to idle. Exact current transport/Android source hashes match `artifacts/background-service/commands/`. Guest UID 0 and ADB cross-VM forwarding limit this proof. One preceding Android UI Answer attempt became MISSED; an instrumented rerun passed, but the intermittent radio/call failure remains unresolved. Native QML UI, sound ownership/DND, ringtone playback, user-service lifecycle and wake remain incomplete. Design concepts are in `design/incoming-directions.png`; they are not implementation screenshots.

Native QML component checkpoint: `omarchy-plugin/qml/CallCard.qml` renders incoming, answering, active and unknown states on ARM64. Seven interaction/render test cases (nine QtTest passes including init/cleanup) pass. Source hashes match `artifacts/qml-card/result.json`. Screenshots use synthetic data and software/offscreen Qt; there is no live shell, Android, ringtone or wake proof from this component check. The initial test fixture omitted visible=true; it was corrected before claiming pointer coverage. The component remains separate from a complete installable plugin.

### Native shell checkpoint — 2026-10-12 00:50 UTC

- ARM64 call-card tests: 10 pass, including cancellation if call context changes between press and release. Current source hashes verified against `artifacts/qml-card/result.json`.
- Full pinned Omarchy shell renders its bar after the upstream-equivalent Qt 6.12 Color namespace fix. Current archive, patch and 54 QML hashes match `artifacts/omarchy-shell/` evidence. Screenshot reviewed; missing icon font and desktop helper dependencies remain explicit.
- Independent Pro review completed and cleaned up; recorded in `docs/pro-shell-compatibility-review.md`.
- No physical phone access. No claim of hardware graphics, ring audio, suspend wake, modem/IMS, complete plugin packaging or complete end-to-end delivery.
- Next: finish the native plugin controller and service lifecycle, wire exact observed call identity into commands, validate through the real plugin loader and Android emulator, then package and test installation/recovery.

### Native command and daemon entrypoint checkpoint — 2026-10-12 UTC

Implemented and verified production `phone_local.py` control/result subprocess commands and production daemon Ledger/Dispatcher startup with ordered shutdown. Four CLI socket tests and one real daemon process lifecycle test were added red-green. All 98 Linux Python tests pass. The updated ARM64/Android suite passes and its source hashes match current code (`artifacts/background-service/daemon-cli/`).

A preceding run passed the new native CLI scenarios but lost a later direct-client answered call. That failed result is preserved separately in `artifacts/background-service/cli-integration/`. Instrumentation was added; the new run passed. Intermittent call loss is still open, not classified as fixed.

Next implementation: native QML service/controller with immutable observed command identity, actual Omarchy plugin loading and incoming presentation while closed, systemd and package delivery, then equivalent touch-driven framework tests. Preserve recovery/uncertainty through shell and daemon restart. Ringing/audio/wake and hardware boundaries remain explicit.

### Packaged desktop-user service checkpoint — 2026-10-12 UTC

- Package 0.2.0 now contains the production daemon, dispatcher, ledger, local CLI and systemd user unit alongside the existing GTK app. Installation does not enable/start it. The historical 0.1.0 artifact remains separate.
- Red-green package test added; all 99 Linux Python tests pass.
- Actual ARM64 pacman installation and systemd user-manager checks pass as UID 1000: source equality, disabled after install, private permissions, repeated start, crash restart, fresh epochs and explicit stop. Evidence: `artifacts/service-package/`.
- Actual installed-package call integration passes as UID 1000 against enrolled Android TLS: Answer -> active, saved result, duplicate receipt, End -> idle. Source/package hashes match; SELinux Enforcing. Evidence: `artifacts/service-package/android/`.
- QML service/controller, plugin installation and incoming presentation remain unfinished. Next connect them to these installed entrypoints and repeat actual touch/framework tests. Intermittent call loss, ringing policy/audio, wake and Pixel hardware limits remain open.

### QML controller checkpoint — 2026-10-12 UTC

Added CallController with exact displayed identity, stale/out-of-order snapshot rejection, command/result observation barriers, matching receipt identities and retained uncertainty without automatic retries. First tests failed because the controller did not exist. Core tests then passed. Binding the actual CallCard exposed a second red test: pending Answer showed Phone unavailable. Display data is now retained separately from action authority, and pending/unknown presentation takes priority. All 19 QML checks pass on ARM64 with current source hashes; actual rendered screenshots are in `artifacts/qml-controller/` and were shown to the user.

This is card/controller integration with simulated replies, not native plugin/Android integration. Still required: Quickshell process adapter, service/manifest/panel entrypoints, durable pending discovery after full shell restart, closed-panel incoming presentation, native outgoing UI and touch-driven framework tests. No phone access.

### QML service and Android checkpoint — 2026-10-12 UTC

PhoneService and JsonCommand now bridge the QML controller to the installed CLI. Process failure/timeout tests and the 19 card/controller checks pass. The installed Unix service fixture passes, and the real enrolled Android emulator now passes the programmatic QML Answer -> active -> Disconnect -> idle sequence as UID 1000. Package and all recorded QML hashes match. Evidence: `artifacts/qml-service/android-modem-run/source-verification.json` and the nested `qml_user_service` result.

The complete framework run is still red: a later direct Answer was accepted but the call disappeared. Prior failure logs show radio ANSWER followed by empty GET_CURRENT_CALLS and Telecom MISSED. No cause or reliability fix is proven. The guest harness now reports assertions as FAILED rather than losing them behind a timeout; two red-green tests pass. See `docs/modem-call-failure.md`.

Still incomplete: installable native plugin/panel/bar, full shell restart recovery, closed-panel incoming presentation, native outgoing UI, touch-driven framework tests, ringing/audio/wake, deployment policy and Pixel-specific graphics/modem/IMS hardware proof. No physical phone access.

### Independent modem baseline and full integration — 2026-10-12 UTC

The full bridge enrollment suite now has a passing run including real QML/installed service, revocation and replay protection. Twenty recorded source/package hashes match; `artifacts/qml-service/android-verbose-pass/` preserves evidence.

A clean stock Android baseline, with bridge APK absent and Google Dialer holding the role, reproduced the same ANSWER -> empty radio call list -> INCOMING_MISSED failure in one of ten attempts. Nine attempts reached ACTIVE. This isolates a failure path independent of our bridge but does not identify the precise emulator/radio defect or prove Pixel reliability. Evidence: `artifacts/radio-baseline/`.

Fixed a weak Android UI End assertion that used `gsm list`, which this emulator returns empty even during active calls. It now checks Telecom's live call collection. Red-green fixture tests and the actual Android UI run pass; screenshot/evidence: `artifacts/callui-live-state/`. Product call semantics are unchanged. Native plugin and hardware requirements above remain incomplete.

### Full Omarchy plugin and touchscreen checkpoint — 2026-10-12 UTC

Added `nbrempel.phone` manifest, retained service and panel entry points. The first real-shell test failed because the plugin was absent. The implementation then passed registration, closed-panel incoming display, dismissal staying closed, manual reopen and no automatic call commands. A stronger persistence test exposed root-owned fixture directories; corrected desktop ownership now permits saving the enabled-plugin setting.

Actual virtual touchscreen events now travel through Hyprland, the full pinned Omarchy shell, the native panel, installed CLI and Unix daemon. Answer reaches visible active state and End reaches visible idle state, with exact original state/revision and two unique operation IDs. The first screenshot caught a transient stale display; the test now waits for the panel's own status and verifies its active/idle titles before accepting the result. Fresh idle status is displayed as No current call. Evidence and current source hashes: `artifacts/phone-plugin-state/`.

Android is synthetic in this full-shell test. The separate programmatic QML/real Android result remains distinct. Next combine full-shell touch with real Android, then complete launcher/bar discovery, native outgoing calls, package delivery, full shell restart recovery and ringing policy. Pixel graphics/modem/audio/wake and shared-kernel deployment limits remain unverified. No phone was accessed.

### Native Omarchy touch to Android emulator checkpoint

The full native plugin test passed with the installed Arch user service and enrolled Android 17 bridge. Starting with the panel closed, an injected emulator incoming call opened it. Dismissal stayed dismissed, manual reopening worked, and no call command was admitted before touch. Virtual evdev touch answered the call; independent Android Telecom inspection confirmed ACTIVE before the test released the End action. Touching End returned the visible panel and Android to idle. Two ledger admissions finished as accepted.

Evidence: `artifacts/native-plugin-android-pass/guest/result.json`, incoming and active PNGs, and `evidence/plugin-bridge.json` / `plugin-provenance.json`. All recorded plugin, installed Python, and Android source hashes match local sources. Android SELinux remained Enforcing.

An earlier run reproduced the documented emulator radio answer failure; its evidence remains in `artifacts/native-plugin-android-first/`. A subsequent enrollment failure exposed a one-second emulator clock lag behind test certificate validity. The test harness now waits, with a bounded timeout, for Android time to reach certificate notBefore; it changes neither clocks nor TLS validation. Red-green tests cover readiness and timeout.

This is virtual graphics and simulated cellular input. It does not establish audible ringing, DND behavior, lock-screen wake, real audio, IMS, carrier service, Pixel PowerVR, or a bootable Pixel OS. Native plugin packaging, launcher integration, and other open completion items remain. No physical phone was accessed.

### Native plugin package checkpoint

Built reproducible `omarchy-phone-plugin-0.3.0-1-any.pkg.tar` with package-owned runtime QML and an explicit per-user registration command. Registration never replaces existing user paths; unregistration only removes the exact package link. No package install hook starts services or edits user configuration. Four new red-green tests plus the existing Linux suite passed (103 total).

`artifacts/plugin-package-shell/result.json` proves real ARM64 pacman installation, register/unregister/re-register, exact installed source and package hashes, native full-shell discovery/enable and incoming/answer/end touch flow with synthetic Android data. This add-on still depends on the existing client package containing the historical GTK interface. Native default launcher, recovery, sound/DND and release licensing remain open. A combined packaged native plugin and Android emulator run is in progress; do not infer its result from this checkpoint.

The combined package/Android run subsequently passed: `artifacts/packaged-plugin-android/`. Both Arch packages installed in the ARM64 guest, native registration lifecycle passed, closed-panel incoming presentation worked, virtual touch answered, Android independently confirmed ACTIVE, and touch End returned Android and the panel to idle. Installed plugin/Python/Android source hashes and native package hash match; SELinux remained Enforcing. This supersedes the pending-run statement above, without expanding emulator evidence to physical hardware or audible ringing.

### Native launcher checkpoint

Client 0.3.0 switches the existing desktop entry and `omarchy-phone` command to the native Omarchy panel, with no duplicate application entry. The legacy GTK command is explicitly `omarchy-phone-legacy`; GTK dependencies are optional. Native plugin package 0.3.1 requires client >=0.3.0. The launcher checks discovery/enabled state, requests summon once, bounds both IPC calls and error notification, and never starts services, enables permissions, or dispatches a call command.

106 Linux tests passed. `artifacts/native-launcher-shell/` verifies actual ARM64 package installation, rejection while disabled, installed launcher reopening the panel, and synthetic call touch Answer/End. Both package hashes and installed QML match. An initial package installation failed because libnotify was absent from the network-isolated guest; the normal provisioner now installs it, and a rerun passed. Previous guest disk is preserved on Harbour in `sim/before-shell-20261012T024613Z`.

This run invokes the installed executable with the guest session environment; it does not yet prove an actual app-menu selection or cold launch after package upgrade/shell restart. Android-backed proof for the prior package remains separate.

Independent review: chatgpt-pro-bridge, Chat selected, Pro selected, marker PRO-BRIDGE-20261012-0245-a71f, completed FINAL_RESPONSE with response controls, zero context rounds. Conversation private review transcript . Follow-ups: actual desktop activation after upgrade/restart, changed call state while shell absent, duplicate launch behavior, enabled-but-broken QML negative fixture, dangling registration unlink/reinstall, and disable persistence. These are unverified test gaps rather than demonstrated defects. Both IPC and notification timeouts are already implemented.

### Shell lifecycle and launcher readiness checkpoint

A full-shell test now stops the shell process group, verifies no live old Quickshell remains, changes synthetic call state while it is stopped, restarts it, and checks fresh incoming state. It activates the installed desktop entry with GIO twice, verifies exactly one visible phone panel and no automatic call commands, then exercises touchscreen Answer/End. `artifacts/native-lifecycle-ready/` passed with client 0.3.1 and plugin 0.3.2; both package hashes match. An earlier failed shutdown assertion was a test-wrapper/process-group issue; the test now stops the entire group and distinguishes zombies from live processes.

A separate real-shell negative fixture demonstrated false launcher success when enabled QML could not load. The launcher now polls only the read-only panel opened status for a bounded interval after a single summon. It returns failure rather than treating asynchronous acceptance as visibility. Red and green VM evidence: `artifacts/native-broken-panel-red/`, `artifacts/native-broken-panel-green/`. 107 Linux regression tests passed, including delayed readiness and timeout.

Still unverified: actual graphical menu click and its environment, update/reinstall/disable persistence, durable pending-command recovery, latest package through real Android, audible ringing/DND/lock-screen behavior and physical hardware. This checkpoint does not establish those requirements.

### Update and removal lifecycle checkpoint

`artifacts/native-upgrade/` proves a real pacman upgrade from client 0.2.0/plugin 0.3.0 to client 0.3.1/plugin 0.3.2 while the synthetic-adapter daemon remains running with the same epoch. After the shell stops, the call state changes; the restarted interface reads the fresh state, opens via the desktop entry, and completes touch Answer/End without automatic call commands. The candidate installed launcher matches source and both package hashes match current local artifacts. This verifies that exact version pair; it is not a promise that future incompatible runtime upgrades need no daemon restart.

The test then explicitly disables and unregisters Phone, removes the native plugin with pacman, verifies no plugin runtime files or registration command remain, reinstalls/registers, and restarts the shell. The saved disabled state remains effective, the launcher refuses to open it, and no additional call commands occur. A first removal assertion incorrectly required deletion of an empty untracked directory; the corrected test verifies removal of every plugin file and command.

Current-package Android end-to-end verification is running separately. Pending command recovery, outgoing native UI, ringtone/DND/lock-screen behavior and hardware-dependent work remain open.

Current client 0.3.1/plugin 0.3.2 also passed the enrolled Android 17 end-to-end test (`artifacts/native-ready-android/`): installed native launcher, incoming presentation, touch Answer, independent Android Telecom ACTIVE, touch End and idle. Installed Python/QML/Android sources and package hashes match; SELinux remained Enforcing. The separate upgrade/remove/reinstall proof is in `artifacts/native-upgrade/`. These results do not establish pending-operation recovery, real call audio/ringing, IMS/carrier behavior or Pixel hardware.

### Durable recovery discovery checkpoint

Client 0.4.0 adds the read-only `recovery` CLI/socket query for retained uncertain operation IDs. It includes pending, unknown and terminal-unknown results, limits a response to 64 entries and explicitly flags overflow. It exposes no raw command/number and never mutates or dispatches. Strict client validation rejects malformed/duplicate/inconsistent records.

110 Linux tests passed, including the actual daemon and CLI across restarts. `artifacts/recovery-package/` proves installed ARM64 systemd/CLI discovery of an interrupted operation as unknown, plus crash/stop/restart behavior and exact package/source hashes. The native interface integration, resolution/acknowledgment behavior, and no-duplicate behavior after losing pending UI state remain incomplete. See `docs/recovery-discovery.md`.


### Native uncertainty recovery checkpoint

Base 0.5.0/plugin 0.4.0 now require controller sessions and expose explicit recovery acknowledgment. `artifacts/native-recovery/` verifies both installed packages in actual ARM64 Hyprland/Omarchy: touch acknowledgment, Answer with deliberately lost backend reply, whole-shell restart with the same unresolved identity, second explicit touch acknowledgment, and End. Exactly two backend commands were transmitted; acknowledgments and shell restart sent none. All eight installed plugin source hashes and both package hashes match the local candidate. The source package also matches all eleven Python runtime files. `artifacts/recovery-card/` retains component tests and real Qt screenshots.

This run uses a synthetic Android adapter. Current-version real Android regression is separate and not established by this checkpoint. Required remaining work includes compatible upgrade/restart behavior, ledger lifetime capacity, native outgoing calls, audible ringing/DND/wake behavior, deployment topology, and the stated hardware limits. See `recovery-discovery.md` for prior lower-level socket and QML evidence.


Current-version Android regression also passed: `artifacts/native-session-android/`. Client 0.5.0/plugin 0.4.0 installed on ARM64, enrolled AndroidKeyStore TLS, production enrollment UI, native touch Answer, independent Android Telecom ACTIVE, touch End and Android idle. SELinux was Enforcing. Installed plugin, Python and Android source hashes and both native package hashes match local sources. This normal-call emulator test is separate from synthetic lost-reply/restart injection; neither proves carrier IMS, physical call audio, or Pixel graphics.


Stronger abrupt-termination test passed: `artifacts/native-recovery-crash/` (Harbour `phone-recovery-20261012T040000Z`). After the synthetic backend received Answer, it held the daemon worker active. The harness SIGKILLed the complete shell group. The restarted panel discovered the durable pending operation, kept controls busy and acknowledgment disabled, then showed that same identity as unknown after the backend was released with a lost reply. Explicit touch acknowledgment followed by End produced exactly one Answer and one Disconnect. Current source hashes match. This establishes shell-crash recovery with a live daemon and synthetic remote work; daemon-death plus late Android execution remains a distinct fault-injection scope.

The full Linux transport suite now passes 123 tests (`artifacts/background-service/recovery-commit-tests.log`). The added test forces actual SQLite acknowledgment COMMIT failure through a deferred constraint, verifies rollback leaves uncertainty and the original operation intact, then verifies a later successful acknowledgment. It also checks the writer uses DELETE journal mode and FULL synchronization. No production behavior was changed for this test.


Coordinated native package upgrade passed: `artifacts/native-session-upgrade/`. Actual pacman upgraded base 0.2.0/plugin 0.3.0 to base 0.5.0/plugin 0.4.0. Both shell and synthetic-adapter daemon stopped before replacement; the new daemon epoch differed and the seeded durable historical record was byte-for-byte unchanged at the row level. New shell discovery, GIO desktop-entry activation, touch Answer/End, disable/unregister/remove/reinstall/register and persisted disable all passed. No automatic call commands occurred. This fixture uses installed daemon classes with a synthetic adapter, not a systemd-managed Android-backed upgrade. See `service-package.md`.


Current 0.6.0 emulator checkpoint passed: `artifacts/native-dial-android/`. Installed ARM64 CLI -> private Unix socket -> systemd user daemon -> enrolled Android TLS -> Telecom produced independently observed DIALING, returned the same receipt for duplicate dial ID, and ended to idle. The same full Omarchy run then completed incoming virtual-touch Answer -> independently confirmed ACTIVE -> End -> idle. The ledger contained four admissions: invocation_attempted, outgoing End submitted, incoming Answer submitted, incoming End submitted. SELinux remained Enforcing. Both native package hashes, installed QML, Python and Android source hashes match. All 131 Linux tests and six Java policy test programs pass. This establishes the native CLI outgoing path; it is not a native dial-pad touch test, carrier call, audio or Pixel hardware proof.
