# Omarchy smartphone goal

This scope supersedes the earlier call-bridge-only completion target. It records Nicholas's expanded instruction: build a complete Omarchy smartphone system for Pixel 10, including SMS, camera, power and all phone functions that need special treatment, with genuine Omarchy plugin and theme integration.

Build and test everything possible offline on temporary Harbour capacity. Do not access or modify the physical phone. Harbour must not be a runtime dependency. The active goal remains unfinished and now records this full smartphone scope.

**No backwards compatibility:** build one target implementation and matching component set. Do not add legacy adapters, dual production runtimes, old-format migrations or downgrade support for earlier iterations. Test required behavior and restart recovery within the target version. Prior code and artifacts provide evidence, not a requirement to preserve old formats.

## Product and architecture

Nicholas updated the target on 12 October 2026: implement the required phone
userspace services natively in Rust, including cellular/IMS, calls, SMS, camera,
power, audio, connectivity and sensors. Retaining Android framework userspace is
not the intended finished architecture. Existing Android bridges and emulator
components are test infrastructure and reference implementations, not a second
production runtime to maintain.

Use available open-source implementations and documented interfaces first. For
closed-source components, use REA to recover behaviour and interfaces. Compare
Rust against original-code execution where possible; mark static deductions,
simulation and hardware evidence separately. Recover actual contracts rather
than inventing protocol behaviour to satisfy a mock. Track unresolved closed
firmware and driver dependencies explicitly.

Retain Google's matched Pixel kernel, kernel modules and hardware firmware
initially. This userspace replacement does not require rewriting the kernel or
modem firmware. Arch Linux ARM, Hyprland and native Omarchy apps remain the user
interface. Preserve Omarchy theme/plugin integration; QML presentation and
necessary ABI glue do not establish permission to retain Android framework
services in the final design. Any unavoidable retained userspace component must
be documented as an unresolved exception, not silently declared complete.

Phone, Messages, Camera and phone settings form an installed smartphone bundle. Shared services own hardware state and background work independently of open app windows. Each mutable resource has one owner. Preserve the subsystem decisions and unresolved deployment questions in `hardware-boundary-decision.md`.

## Completion matrix

Every row requires permission boundaries, error and recovery behavior, service restart tests, and relevant offline integration tests. Device-dependent rows additionally require later authorized Pixel verification. An unavailable feature must be explicit in the UI and release report.

| Area | Required scope | Completion evidence |
|---|---|---|
| Omarchy integration | Native app discovery and launch, plugin lifecycle, shared shell controls, touch access, live theme switching | Installed packages in the real Omarchy shell; reload, upgrade and uninstall tests; screenshots of actual UI in multiple themes |
| Calls | Incoming and outgoing calls, ringing with UI closed, answer/decline/end, uncertain-result recovery, history, contacts, DTMF, mute and audio routes | Native touch actions through the Rust call service and native radio/IMS backend; original-code comparison and Android Telecom reference tests; restart/replay tests; later carrier and two-way audio proof |
| Messaging | SMS send/receive, conversations, notifications, multipart and Unicode messages, delivery state, duplicate and retry handling; audit MMS and carrier dependencies | Native UI through Rust messaging and native radio/IMS adapters; original-code comparison where available; background and restart tests; later modem/carrier proof. Do not label an API acknowledgement as delivery |
| Camera | Preview, photo and video capture, camera selection, orientation, flash controls where supported, privacy and access controls, media storage | Adapter and UI tests for formats, buffer ownership, fences, cancellation and service death; later sensor, image and recording proof |
| Power | Battery and charging state, display brightness, screen off/on, idle, suspend/resume, wake sources, thermal and charging policy | One native suspend authority accounting for retained kernel/vendor wake sources; recover required vendor power contracts; offline policy and failure tests; later charging, thermal, wake and battery-drain measurements |
| Cellular and subscriptions | Registration, signal, mobile data, SIM/PIN, subscription selection and airplane mode; audit eSIM provisioning, roaming, IMS and emergency-call dependencies | Explicit service ownership and permission tests; later actual SIM, modem, carrier and authorized emergency-call validation. No live emergency test calls without a controlled plan |
| Audio | Ringtone, DND/silent behavior, volume, vibration, media, microphone, speaker, wired/USB and Bluetooth routes | One route owner; focus, interruption, reconnect and call/media interaction tests; later physical route and microphone proof |
| Connectivity | Wi-Fi, Bluetooth, tethering and USB modes as supported | Audit native versus vendor interfaces; connect/disconnect, permission and recovery tests; later radio/coexistence proof |
| Sensors and location | Rotation, proximity, ambient light, GNSS/location, haptics and physical buttons | Define ownership and access rules per interface; synthetic events and adapter tests; later sensor and location proof |
| Background and lock screen | Incoming calls/messages while windows are closed, locked or suspended; notifications, wake and privacy rules | Closed-UI, shell restart, service restart and lock-state tests; later hardware suspend/wake delivery |
| Setup and updates | Setup, permissions, revocation, diagnostics, package upgrades, matched kernel/vendor interfaces and recovery; enrollment only where a separately authenticated boundary remains necessary | Installed-path tests with durable data preservation, bounded logs and no secret exposure; reproducible package/source inventories |
| Graphics and platform | PowerVR rendering, display presentation, input, boot/init, SELinux and shared-kernel ownership | Offline ABI and service integration evidence followed by later actual Pixel rendering, display, boot and stability proof |

Contacts, media storage and notifications are shared facilities, not private copies in each app. Investigate MMS, eSIM and advanced camera features before promising their exact support; record implementation and hardware limitations separately. Retaining the stock camera provider does not establish Pixel Camera feature parity.

## Theme contract

- Use Omarchy's installed theme values and supported plugin APIs rather than an independent theme picker or copied palette.
- Update visible apps, panels, notifications and controls when the Omarchy theme changes, without losing active calls, draft messages or camera state.
- Apply consistent typography, spacing, focus, pressed, disabled and error states; preserve readable contrast and accessible labels in every supported theme.
- Use familiar phone controls with adequate touch targets. Keep desktop keyboard access and responsive layout.
- Test installed integration in at least two materially different themes, including a live switch during active UI state. Save real screenshots and identify synthetic/emulator data.

## Execution and honest completion

Prioritise native replacement contracts and original-code comparison tests across phone services. Preserve the existing bridge tests as reference evidence while implementing the native target. Extend the same installed end-to-end test environment to each subsystem; do not substitute mocks for a real boundary that can be exercised offline.

Track each feature as designed, built, component-tested, installed-integration-tested or hardware-verified. Existing call emulator proof does not prove SMS, camera, GPU, carrier IMS, physical audio or power behavior. The project is not complete merely because the bridge or an emulator suite passes. If all offline work is complete, report the exact remaining physical-device gates without touching the phone.

## Native messaging checkpoint

The SMS payload encoder and multipart planner are built and component-tested,
with 27 byte comparisons against actual AOSP encoder execution and Linux/ARM64
user-QEMU validation. The durable single-owner outbox also passes twelve actual SQLite tests and
SIGKILL/restart under x86 and an actual ARM64 kernel. These are components, not
installed messaging integration. Send/receive, transport receipt qualification, delivery, native app and
background/lock/suspend behavior remain open. See [native messaging](native-messaging.md).

Incoming SMS text/metadata parsing also matches33 actual AOSP decoder outputs with bounded malformed-input tests on x86 and ARM64 user-QEMU. Durable incoming ownership, vendor acknowledgement and installed Messages integration remain open.

The first native power-request arbiter is built and component-tested, including18 outcomes from actual public source. Installed permission/lifecycle, native suspend authority and hardware policy remain open; see [native power](native-power.md).

The native audio authority is built and boundary-tested against original
PipeWire/WirePlumber in a private virtual mono graph:14 stages cover call/media
priority, authenticated capacity, capture-link revocation, backend recovery and
crash-owned links. Five policy tests also pass on x86 and ARM64 user-QEMU.
Installed responsive lifecycle, per-app streams, physical routes and mute,
Pixel AoC/carrier audio, DND/volume/haptics and idle power remain open. See
[native audio](native-audio.md).
