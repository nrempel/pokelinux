# Native Omarchy phone interface — in progress

Current candidate: client **0.6.0**, native plugin **0.4.0**. Install both packages together, then restart the user daemon and Omarchy shell. Mixed running protocol versions are not supported by the new mandatory-session contract. The coordinated current-version upgrade and remove/reinstall test passed in `artifacts/native-session-upgrade/` with a synthetic Android adapter; see `docs/service-package.md` for its exact scope.

Native recovery now survives a full shell restart. `artifacts/native-recovery/` verifies installed ARM64 Hyprland/Omarchy, explicit touch acknowledgment, Answer with a deliberately lost reply, restart retaining the same uncertain operation, a second acknowledgment, and End. Exactly two synthetic backend commands were sent. `artifacts/recovery-card/` contains current component tests and screenshots. See `docs/recovery-discovery.md` for the session and durable-ledger semantics. The chronological checkpoints below describe their original versions and must not be read as current missing-feature lists.


`qml/CallCard.qml` and `CallActionButton.qml` are the deterministic QML incoming/current-call components. They expose presentation properties and an actionRequested signal; they do not connect to Android, play audio or start a daemon. The service must own command admission and retained outcomes. No action is emitted automatically on state or visibility changes.

The component uses large labeled controls and explicit unavailable/pending/unknown states. The caller must supply fresh authorized state and set busy synchronously on a user command. The daemon rechecks authority independently. A successful local button test is not proof of an Android call.

Eight QtTest cases plus init/cleanup pass in the ARM64 Arch guest (Qt 6.12). Tests exercise real QML pointer and keyboard events, incorrect call state, missing permission, stale data, pending/unknown command suppression, and screenshots. Evidence: `artifacts/qml-card/`. Images use synthetic data and Qt offscreen/software rendering. They are not screenshots of the full Omarchy shell or the physical phone.

The source plugin now has a manifest, service and panel entry points, live Omarchy surface/text colors, and an incoming display that opens while the panel is closed. It has been loaded from the desktop user's plugin directory in the full Omarchy shell. The separate native plugin package passed its installed full-shell test with synthetic call data. A discoverable launcher/bar control, lifecycle recovery and DND/sound policy remain incomplete. The combined full-shell touch/Android test passed; see the checkpoint below.

The card now captures its contextKey when a button is pressed and cancels the action if that key changes before release. The red test failed before the property existed; the implementation now passes in the ARM64 guest. The future controller must bind this key to the exact service epoch, generation, call ID and revision, then submit the original observation. Component tests do not yet prove that controller-to-daemon invariant.

## Controller checkpoint

`qml/CallController.qml` now owns the displayed observation and pending command state. It rejects out-of-order snapshots, stale press contexts, wrong receipt identities and commands while another result is pending or unknown. Each command uses the exact daemon epoch, Android generation, revision, call ID and state that were displayed. A generation counter rejects observation processes started before submission or result handling. The daemon still independently validates authority and freshness.

The real card/controller binding caught a pending-state display defect: expiring authority also removed the displayed call. The controller now retains display data separately from authority, and the card gives pending/unknown status priority over unavailability. Controls remain disabled. Nineteen QtTest checks pass on ARM64, including actual pointer press/release through the binding, with current source hashes and screenshots in `artifacts/qml-controller/`.

The controller has a PhoneService/JsonCommand transport adapter and a plugin service entry point independent of the visible panel. Its pending state survives only while the object exists; full shell-process restart recovery still needs durable-ledger discovery. Do not call its reset method during a panel close/reopen. Separate real Android proof exists for the installed Python service and programmatic QML actions; the full-shell touchscreen test has also passed against the enrolled Android emulator.

When adding the Quickshell transport, use installed `omarchy-phone-local` with argument arrays and one-shot command dispatch. Read-only receipt lookup may be repeated; command dispatch must not be repeated by a process exit/reload callback. Quickshell owns and terminates its child processes, so the daemon remains in the separately installed systemd user service. Reference APIs: https://quickshell.org/docs/v0.3.2/types/Quickshell.Io/Process/ and https://quickshell.org/docs/v0.3.2/types/Quickshell.Io/StdioCollector/ .

See `docs/qml-service.md` for process-adapter behavior, current evidence and remaining integration work.

## Full shell checkpoint

`manifest.json` declares `nbrempel.phone` as a retained service and panel. `Service.qml` presents each fresh incoming call once, using daemon epoch, Android generation and call identity. Repeated observations cannot reopen a dismissed call. `Panel.qml` uses the existing call card, theme bindings and a read-only `status()` method available through Omarchy's plugin IPC. Closing the panel does not reset command state or send a command.

`artifacts/phone-plugin-state/` records the actual ARM64 Omarchy/Hyprland run: plugin discovery, saved enable setting, incoming presentation from closed state, close/reopen without commands, virtual touchscreen Answer -> visible active -> End -> visible idle, and exactly two daemon transmissions. All recorded plugin source hashes match. Graphics are virtual; Android call data is synthetic. Missing stock Omarchy helpers/icon fonts in the guest remain separate from this proof. This is not Pixel GPU, IMS, ringing, audio, wake or lock-screen proof.

## Installed native plugin

Build with `python3 tools/build-plugin-package.py`. Install `omarchy-phone-0.6.0-1-any.pkg.tar` and `omarchy-phone-plugin-0.4.0-1-any.pkg.tar` with pacman. As the desktop user, run `omarchy-phone-plugin register`, reload the Omarchy shell, and enable Phone through Omarchy plugin controls. Registration links `~/.config/omarchy/plugins/nbrempel.phone` to package-owned files in `/usr/share/omarchy-phone/plugin`. It does not start the daemon, enroll Android, or enable call controls. Existing user paths are never replaced. Disable Phone and run `omarchy-phone-plugin unregister` before removing the package; reload the shell to unload it.

The native add-on currently depends on the existing client package, which still includes the historical GTK interface. The native plugin uses the installed local CLI and daemon, not GTK. Client 0.3.0 makes the existing Phone desktop entry open the native plugin; historical GTK is available only through `omarchy-phone-legacy`. Actual app-menu activation, upgrade/restart verification and final distribution packaging remain open. No distribution license is asserted by these local test packages.

## Full shell with Android emulator

`artifacts/native-plugin-android-pass/` records full-shell virtual touchscreen Answer -> Android Telecom ACTIVE -> End -> idle, through the installed Arch daemon and enrolled Android 17 bridge. The panel opened from closed state on the incoming call. Android remained SELinux Enforcing, and installed/source hashes were checked. These are emulator results, not physical phone, audible ringing, carrier or GPU evidence. This checkpoint preceded package-based plugin registration; subsequent package tests are separate evidence.

`artifacts/plugin-package-shell/` proves pacman installation of the native add-on, no automatic user registration, explicit repeated register/unregister/register, exact installed source hashes, shell discovery and saved enable state, and virtual touch Answer/End through the local daemon with synthetic Android data. All 103 Linux transport tests passed. Combined packaged-plugin/Android verification also passed; evidence is in `artifacts/packaged-plugin-android/`. The installed native package, Python runtime and Android source hashes match. Android independently confirmed ACTIVE and idle.

The installed native launcher passed in the full ARM64 shell with synthetic call data (`artifacts/native-launcher-shell/`), including disabled-plugin rejection and opening without call commands. Both packages must be upgraded together. Restart the user daemon and Omarchy shell after an update; mixed running versions are not verified. The current test invokes the executable under a prepared session environment; graphical launcher activation is still pending.

Client 0.3.1 now waits for the panel's read-only opened status after summon is accepted. The real-shell broken-QML fixture first returned false success; after the fix it returns failure with a diagnostic. Evidence: `artifacts/native-broken-panel-red/` and `artifacts/native-broken-panel-green/`.

`artifacts/native-lifecycle-ready/` proves fresh synthetic incoming state after a complete shell process-group restart, preserved plugin enable state, installed desktop-entry activation via `gio launch`, two launches leaving one panel, zero automatic call commands, and touch Answer/End. This is not an app-menu click, package upgrade, pending-operation recovery, Android modem, ringtone or hardware test. All 107 Linux tests passed.

Current client 0.3.1/plugin 0.3.2 also passed the enrolled Android 17 end-to-end test (`artifacts/native-ready-android/`): installed native launcher, incoming presentation, touch Answer, independent Android Telecom ACTIVE, touch End and idle. Installed Python/QML/Android sources and package hashes match; SELinux remained Enforcing. The separate upgrade/remove/reinstall proof is in `artifacts/native-upgrade/`. These results do not establish pending-operation recovery, real call audio/ringing, IMS/carrier behavior or Pixel hardware.

Client 0.6.0 now provides outgoing calls through the native CLI and shared daemon; the actual ARM64/systemd/Android path passed in `artifacts/native-dial-android/`. Native dial-pad controls remain unfinished. The coordinated upgrade checkpoint is specifically 0.5.0; see `docs/native-outgoing.md`.
