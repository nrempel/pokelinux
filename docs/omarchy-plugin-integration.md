# Omarchy phone plugin integration

The expanded smartphone UI boundary is defined in [UI framework decision](ui-framework-decision.md): keep shell controls in the plugin and run full applications separately. The historical checkpoints below prove specific builds; they do not require every future smartphone app to run inside Quickshell. See [runtime language decision](runtime-language-decision.md) for the proposed Rust service target and compatibility gates.

## Verified baseline

Target the released Omarchy **v4.0.4**, commit `c668141e9c42b13c80c9ca4ea108e11708c5e8a5`, before adding support for development branches. The release tag resolves directly to this commit. This is a source/API baseline, not evidence that the release runs on the Pixel or in the existing ARM64 VM.

Primary references:

- [Released plugin manual](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/manual/32-shell-plugins.md)
- [Released shell contract](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/shell/README.md)
- [Released media manifest](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/shell/plugins/services/media/manifest.json)

The shell runs Quickshell/QML. Plugins declare schemaVersion 1, kinds and entryPoints in manifest.json. Third-party plugins use their own namespace: `omarchy.*` is reserved. The shell injects capability-scoped interfaces; built-in firstPartyServiceFor calls must not be copied into our plugin. A shell service and a panel can have separate lifetimes. keepLoaded preserves a service during reload and therefore also delays service-code changes until shell restart. Plugins are not a security sandbox.

## Implementation direction, not yet verified

Use QML for the phone panel and bar indicator, with the same compact number field and optional keypad already selected in the design record. Preserve the transport and call-policy boundary. Keep long-lived connection handling outside the visible panel so closing it does not stop incoming-call observation. Select and test the local IPC contract before exposing call actions. UI reload, reconnection and repeated events must never repeat a call command.

The current GTK implementation remains the verified integration baseline until the QML replacement passes equivalent real touch and Android framework tests. Do not remove that evidence or label GTK execution as QML proof.

## Required next checks

1. Inspect released third-party service lookup and lifecycle code and validate a plugin through the actual Omarchy validator.
2. Establish a pinned Quickshell/Qt runtime in the owned ARM64 test environment; record render backend and module versions. No host-wide installations.
3. Test backend observation, freshness, request identity and uncertain outcomes independently of panel lifetime. Preserve incoming controls during outgoing requests.
4. Run the real plugin under the released shell in Hyprland, including panel close/reopen, theme changes, incoming presentation, and actual touch-driven outgoing and incoming controls against the Android emulator.
5. Test suspend/reconnect, shell/service restart and authorization changes. No automatic retry of commands.
6. Keep Pixel PowerVR, carrier IMS, voice audio and device boot as separate unverified hardware gates.

## Evidence checkpoint

`artifacts/outgoing/source-verification.json` records a current-source match for the saved outgoing tests: two ARM64 GTK touch calls, two direct Arch glibc calls, replay checks, lifecycle checks and equality of unsigned/test-signed APK executable payloads. It explicitly does not establish full delivery, QML or Pixel hardware completion. All 53 Python tests also passed during this checkpoint.

## Runtime checkpoint

The ARM64 guest now has Quickshell 0.3.2-1, qt6-base 6.12.0-2, qt6-declarative 6.12.0-1 and qt6-wayland 6.12.0-1. Exact outputs are in `artifacts/qml-runtime/`. The previous guest disk is preserved on Harbour under `sim/before-shell-20261011T235522Z`. The pinned Omarchy shell subset contains 196 files verified against the release Git tree, stored under `sim/omarchy-source/` with a source manifest.

The first actual ARM64 module-import check timed out and did not prove module compatibility. Its harness now delays Qt shutdown until the event loop and captures timeout output. Before the second run, all task-owned Docker test images disappeared from Harbour; the VM disk remained. The builder and simulator images were rebuilt successfully. The corrected check passed on ARM64 with exit 0, loading all 13 listed QML modules as the alarm desktop user, including Quickshell Networking, Wayland, Notifications, Pipewire and Polkit. Evidence is in `artifacts/qml-runtime/result.json` and `qml-output.txt`. This verifies imports and event-loop startup/shutdown with Qt offscreen/software rendering, not the full Omarchy shell, the phone plugin display, GPU rendering, audio or Pixel behavior.

`transport/phone_local.py` is a one-shot observation client for the future QML service. It validates response identity, exact schema, epoch, sequence and independent freshness fields. All 83 Python tests pass on Harbour, including the production local client through the real Linux service socket. It is not yet packaged or wired to a QML plugin. The Android integration proof predates this additional client module and must be refreshed after final integration.

## Full shell and call-card checkpoint — 2026-10-12 UTC

The pinned shell now visibly renders its bar on ARM64 Hyprland with the Qt 6.12 palette fix. The initial blank-bar run was a real failure even though IPC and plugin registration worked. Four identity probes isolated the cause: unqualified `Color` resolves to QtQuick.Color; qualified imports resolve Omarchy's palette. The generated namespace-only backport uses the same `Commons.Color` approach as upstream PR #14553. All 54 patched QML hashes and the source archive/patch hashes match the passing guest run. Original files retain their verified Git blob hashes and executable modes.

Evidence: `artifacts/omarchy-shell/`, including `shell.png`, full logs and `source-verification.json`. Manual visual review confirms the bar; no undefined-property or TypeError messages remain. The screenshot also reveals missing icon fonts. Several stock helper commands, BlueZ and UPower are absent in this limited installation. This is a working shell rendering checkpoint, not a complete Omarchy installation or phone-plugin proof. The virtual GPU remains unrelated to Pixel PowerVR.

The native call card passes eight tests plus init/cleanup (10 total) on ARM64 Qt 6.12. A new red-green test verifies that replacing call identity during a press cancels the action. Pointer and keyboard input remain covered. All three QML source hashes match the saved evidence in `artifacts/qml-card/`. These are synthetic component tests; service/controller wiring and true touch-based Android call integration remain required.

The background command proof is newer than the earlier 83-test entry: 93 Python tests passed, and the real Android framework test exercised an ARM64 service command whose UI client disconnected before the result. The durable receipt prevented duplicate dispatch. Evidence and remaining intermittent emulator-answer failure are in `docs/background-service.md`. The command dispatcher is not yet packaged or connected to the QML component.
