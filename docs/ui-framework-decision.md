# Smartphone UI framework decision

Research date: 2026-10-12 UTC. Scope: Omarchy on Pixel 10, including calls, messages, camera and phone settings. Existing bridge code is not a reason to select a toolkit; the transport, permissions and durable command service have independent interfaces.

## Evidence and recommendation

Use QML with Qt Quick Controls for new native smartphone interfaces, with separate application processes for full Phone, Messages, Camera and Settings applications. Keep a small Quickshell plugin for Omarchy shell surfaces: indicators, incoming-call presentation and quick controls. Background services must survive both app and shell restarts.

This is an architecture recommendation supported by source and current VM evidence, not a claim that the complete mobile experience is verified. Do not expand the current shell panel into an all-in-one phone application host.

Omarchy v4.0.4 is pinned at `c668141e9c42b13c80c9ca4ea108e11708c5e8a5`. Its [shell contract](https://github.com/omacom/omarchy/blob/c668141e9c42b13c80c9ca4ea108e11708c5e8a5/shell/README.md) describes a shared Quickshell process. Local `sim/omarchy-source/source-manifest.json` records the source inventory. Current development [shell documentation](https://github.com/omacom/omarchy/blob/quattro/shell/README.md) retains that model. Development documentation is not a substitute for the pinned release contract.

[Quickshell](https://quickshell.org/about/) explicitly targets desktop-shell components and adds integrations for Wayland, Hyprland, audio, Bluetooth and battery monitoring. QML is its UI language; Qt Quick Controls can also run in independent applications without Quickshell. A shared process creates a shared native-crash and event-loop failure domain. Keeping camera decoding, application databases and large app models outside that process reduces the affected scope of a failure. Separate processes alone do not provide a security sandbox.

| Candidate | Relevant strengths | Cost or unresolved issue | Decision |
|---|---|---|---|
| Qt Quick Controls apps + thin Quickshell plugin | Shared UI language with Omarchy; styleable standard controls; native rendering and C++ integration; app process isolation | Need a common live Omarchy style adapter, mobile input validation and Pixel rendering proof | Preferred default for new apps |
| Everything inside Quickshell | Direct shell theme bindings; one runtime and shared objects | Application and native-media faults can affect the shell; shell lifecycle becomes app lifecycle | Restrict to shell surfaces |
| GTK4/libadwaita | Strong adaptive layouts, standard controls, accessibility and existing Linux phone applications | Needs an Omarchy theme adapter; cannot be loaded as a QML shell plugin; existing modem backends still need adaptation | Valid alternative, especially if app reuse saves substantial work |
| Kirigami | QML framework designed for mobile/desktop navigation; existing phone apps | Additional framework and style integration; verify actual components and dependency cost before adopting | Evaluate for substantial app reuse or complex navigation; not a replacement for Omarchy's shell |
| Flutter | Native Linux desktop support and a mobile-oriented UI toolkit | Adds a separate engine, style and platform-integration path; no identified reduction in the Android hardware adapter work | No demonstrated advantage for this project today |

These cost assessments are engineering judgments, not comparative performance measurements. No battery, startup, memory or Pixel GPU benchmark currently establishes a winner.

## Mobile controls and themes

[Qt accessibility guidance](https://doc.qt.io/qt-6/accessible-qtquick.html) recommends existing controls and explicit accessibility metadata. Use Qt Quick Controls for buttons, fields and navigation; style them instead of replacing their behavior with raw rectangles and pointer handlers. Validate text selection, focus, touch cancellation, input-method composition, keyboard occlusion, rotation, font scaling and assistive technology in the installed Wayland environment.

[Qt's control customization API](https://doc.qt.io/qt-6/qtquickcontrols-customize.html) permits application styles. Omarchy's local `shell/commons/Color.qml` exposes palette and surface roles, and loads theme overrides. Bind shell components to those roles. Standalone apps need their own shared style adapter driven by the same theme data and activation events; importing QML does not automatically make an app follow Omarchy.

Current [Omarchy theming documentation](https://github.com/omacom/omarchy/blob/quattro/docs/theming.md) describes generated theme files and activation hooks. Use the installed version's supported mechanism, including typography and surface overrides. Do not copy private shell runtime internals into each app, execute arbitrary theme data or restart a call/message app merely to recolor it. Verify atomic theme replacement, legacy keys, malformed input and light/dark switching without losing user state.

[Libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/adaptive-layouts.html) supports adaptive mobile layouts. Its [appearance API](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/styles-and-appearance.html) includes light/dark, accent, CSS and high-contrast behavior. It is incorrect to reject GTK as incapable of mobile UI or theme integration. [Kirigami](https://develop.kde.org/frameworks/kirigami/) also supplies adaptive QML components; adopting it does not require replacing Hyprland with Plasma.

Qt Virtual Keyboard is not automatic proof of a working Hyprland-wide keyboard. Its [deployment guide](https://doc.qt.io/qt-6.12/qtvirtualkeyboard-deployment-guide.html) distinguishes application and desktop integration and demonstrates a Qt Wayland compositor. Test the actual selected input-method path across apps. Review module-specific packaging terms if that module is selected.

## Camera and existing apps

No UI toolkit solves Pixel's camera HAL boundary. [Qt Multimedia on Linux](https://doc.qt.io/qt-6/qtmultimedia-linux.html) requires platform drivers for accelerated media. Its [GStreamer backend](https://doc.qt.io/qt-6/qtmultimedia-gstreamer.html) documents custom video sources, conversion and DMA-BUF considerations. These are possible renderer interfaces, not evidence that Qt Camera accepts Google's Android camera provider directly.

Keep native camera acquisition, buffer ownership, fences, permission handling and recording outside the shell. Select the renderer interface after testing it. Camera frames must not pass through the Python JSON control protocol. A copied-frame fixture can validate UI behavior but cannot establish zero-copy or Pixel performance.

[Spacebar's source](https://github.com/KDE/spacebar) documents a Kirigami frontend and a separate D-Bus daemon that stores messages and handles background notifications. Its modem backend is ModemManager. [GNOME Calls](https://github.com/GNOME/calls) has loadable call providers including ModemManager and SIP. These are concrete reuse candidates, not drop-in clients for our Android bridge. Compare an actual adapter with a new frontend before rebuilding mature messaging/contact behavior. Preserve Android Telecom/IMS ownership during that evaluation.

## Required acceptance tests

1. Kill the Camera or Messages app during active work; the shell, phone service and unrelated app must remain responsive. Restart the shell; background call/message state must remain intact and commands must not replay.
2. Exercise installed Qt Quick Controls with real VM touch input, a Wayland input method, Unicode/IME text, focus traversal, text scaling and accessibility actions. A numeric keypad alone does not prove messaging input.
3. Switch real Omarchy themes while a call is active and a message draft is open. Verify palette, surface roles, typography and retained state in separate apps and shell components.
4. Run native app actions through the existing real Android emulator adapter. Retain source and package hashes. UI replacement must not weaken enrollment, permissions, freshness or durable receipt behavior.
5. Test camera preview with a native synthetic source, bounded buffering, service death, cancellation and explicit frame ownership. Keep hardware buffer-import and encoder tests separate.
6. Measure idle CPU, wakeups, memory and startup in the VM. Later authorized device tests must measure actual GPU behavior, frame pacing, battery use and suspend/wake. Do not substitute software-rendered VM results for these.

The current native call panel remains a valid shell component. Full apps and a shared standalone style are not implemented yet. The native dial-controller regression tests are intentionally red: four missing `requestDial` cases, with the existing 22 QtTest cases passing. Evidence: `artifacts/dial-controller-red/`. Implementation was held when the user requested this framework review.

Splitting Phone from the shell also requires an explicit command-owner design. The current service session claim replaces the prior session. The shell and Phone app must not independently claim and invalidate each other's authority. Retain one command coordinator, or implement and test an explicit ownership handoff before both surfaces can issue commands. Independent windows alone are not an integrated solution. Process separation also cannot contain a shared compositor or GPU-driver failure.

## Independent review

The Pro review (private transcript) independently recommends standalone apps plus a thin Quickshell plugin, treats Qt Quick Controls as conditional on mobile input/accessibility tests, and identifies existing app reuse as the strongest alternative. It also identified competing command-session claims as an integration issue, now included above. Its exact pinned-file retrieval failed; the local pinned source inspection supplies that evidence. Its reference to existing SMS/power services is an architectural allocation, not evidence that this project has implemented them.

BRIDGE VERIFIED: Chat selected, Pro 5 of 5 selected, terminal Copy/Rate/Share controls present and Stop absent, marker `PRO-BRIDGE-20261012-qt-91b3` matched. Initial framework review completed with zero context rounds. A language-choice follow-up is tracked separately.
