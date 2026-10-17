# Native Linux and Android hardware boundaries

Research checkpoint: 12 October 2026 UTC. No physical phone accessed. This is an architecture decision, not a claim of a bootable Pixel system.

## Decision

Use Google's matched Pixel kernel, modules, firmware and device description as the initial hardware base. Run native Arch Linux ARM / Omarchy applications above it. Use standard Linux interfaces directly where the Pixel implementation supports them. Shared phone services are native Rust; Android/Java components are reference/test infrastructure. Any temporarily retained vendor userspace is an explicit unresolved reduction boundary. Kernel changes are permitted for specific missing mechanisms, but a custom kernel is not a replacement for the vendor userspace stack.

The smartphone product can be a preinstalled Omarchy app bundle plus shared system services. Phone, Messages, Camera and Settings should not each implement their own hardware policy. The native Phone application is separate from the always-running incoming-call, audio, notification and power services. A closed window must not stop those services.

There is no decision to route every feature through the phone APK. Use the lowest stable interface that supplies the required behavior: Linux device API or a qualified device-specific native HAL/service adapter. Android framework APIs supply reference comparisons, not the intended production service architecture. Narrow interface size and retained runtime size are different: a small Telecom bridge can still require a substantial Android framework underneath it.

## Evidence and selected boundaries

| Function | Selected initial boundary | Evidence and remaining uncertainty |
|---|---|---|
| Apps, shell, navigation, keyboard and local files | Native Linux | The project already runs ARM64 Arch, Hyprland and the Omarchy shell in a VM. This is not Pixel boot proof. |
| Touch and physical buttons | Linux input/evdev to libinput, if exposed correctly | Linux has a standard event interface. Physical Pixel event nodes, permissions, gestures and wake behavior remain untested. Do not add an APK relay merely because the original OS is Android. |
| Battery status | Read standard power_supply attributes where available; adapt missing vendor data separately | The [Rust observation service](native-battery.md) passes seven filesystem/socket/process groups and host/ARM builds, plus actual virtual-kernel driver readings and installed PID1/fresh cold startup. A normal Qt build links the readonly Rust client; GUI and physical Pixel integration remain open. Linux defines this interface. Attribute availability and units must be checked against the Pixel driver; a generic API is not device validation. Reading status is distinct from charging policy. |
| Suspend, charging, thermal and performance policy | One native suspend authority accounting for qualified vendor/kernel wake sources; native policy above hardware backends | Pixel power init starts libperfmgr, reacts to Zygote/property changes, and creates vendor state. Android SystemSuspend coordinates userspace wake locks before writing kernel suspend controls. A second independent Linux autosuspend loop could ignore phone-service activity. The exact safe minimum has not been established. |
| Camera | Native Rust camera ownership and UI above a Pixel AIDL v4 provider/processing backend | Factory init launches the AIDL v4 camera provider from com.google.pixel.camera.hal APEX despite its @2.7 filename. Frozen public v4 source is recovered; native ownership must replace CameraService’s buffer, metadata, permissions and lifecycle responsibilities. Retaining the HAL does not promise Pixel Camera app feature or image-quality parity. |
| Cellular, IMS and call audio | Native Rust radio/IMS and call services above a qualified Pixel vendor backend | The audited factory overlay selects Shannon IMS. Its APK references carrier configuration, connectivity, subscriptions and power APIs. IMS offload uses vendor extensions; standard radio AIDL alone does not replace that contract. Native registration ownership/reaping is tested; full IMS state, recovery and call media remain open. Android Telecom remains a reference boundary. |
| Graphics | Native compositor with vendor EGL compatibility; prefer direct DRM/KMS presentation if validated | Factory vs_drm has PRIME, atomic modesetting and allocation symbols. Vendor EGL contains dma-buf import names. These are strong probe targets, not runtime proof. Use allocator/mapper or composer adapters only where direct paths fail their actual contracts. |
| Wi-Fi, Bluetooth, GNSS, sensors and haptics | Decide per interface after audit | Do not infer native support from device names or route all of them through Telecom. Firmware initialization, HAL policy, calibration and coexistence may still require vendor services. Assign one controller to each mutable resource. |

## Why a custom kernel does not eliminate the bridges

The kernel provides mechanisms such as input events, interrupts, memory allocation, device control and suspend. Vendor camera processing, EGL/GLES drivers, radio/IMS integration and much of power policy are userspace components. Rebuilding the kernel does not convert those Bionic libraries and Android services into glibc programs or standard Linux APIs.

The [retained graphics boundary](native-pixel-graphics-boundary.md) records PowerVR DDK25.3, firmware identifier `71.2.2448.1212` and retained module metadata with `6.6.127-android15-8-gee206ffe8752-ab16254410-4k` vermagic. The module-to-authenticated-image extraction chain, actual loading and KMI remain unqualified. Current upstream kernel and Mesa supported-device tables do not list that identifier. Therefore an upstream kernel/Mesa replacement is not an established shortcut. The existing original EGL run reaches an absent-device failure; successful rendering and separate display scanout remain open.

Google's kernel ABI documentation scopes stability to specific KMI branches. Keep the matched vendor module interface and userspace driver contract when changing configuration or adding patches. The public Pixel build guide inspected in this review does not list Pixel 10; it is not evidence of an exact reproducible source manifest for this factory release. Identifying and rebuilding that exact source/configuration remains an offline task, not a completed result.

## Ownership and implementation order

1. Inventory kernel configuration, module dependencies and exposed userspace interfaces from the pinned factory build. Record a concrete missing feature before patching the kernel.
2. Complete native Rust radio/IMS and shared phone ownership. Preserve Android Telecom comparison tests as reference evidence; do not extend the APK into production hardware ownership.
3. Keep graphics as an independent native integration. Test allocation, import, fences and display ownership separately. SurfaceFlinger/HWC and Hyprland must not compete for the physical display.
4. Build a native camera service adapter and explicit frame/metadata contract. Choose PipeWire or another Linux-facing transport only after validating buffer lifetime, formats, fences, access control and restart behavior. Do not claim zero-copy without actual shared-buffer proof.
5. Define one native Rust power coordinator accounting for retained vendor wake-lock participation. Keep vendor thermal/charging policy until a replacement passes equivalent tests. Linux can provide the Settings UI without owning every underlying policy.
6. Remove retained services one dependency at a time, with failure/restart tests. Offline tests establish software contracts. Actual GPU, camera, modem, audio, charging, thermal and suspend/wake behavior require later authorized hardware tests.

The exact native boot/init, Bionic ABI closure and SELinux arrangement remains unqualified. The two-VM reference tests do not prove a production device runtime. Any temporarily retained vendor service is a reduction gate, not completion.

Public Pixel Power HAL source provides a concrete reduction candidate: `Power.cpp` implements modes and boosts through HintManager, Android properties and display-power integration. A native adapter or a port of this userspace policy is worth evaluating; it does not belong in a new kernel driver merely to avoid Binder. This public source is not yet proven to match the audited factory binary. Suspend coordination, thermal control and charging remain separate responsibilities. Native camera clients likewise retain the service's permission and privacy requirements; native code does not imply framework independence.

## Sources

- [Native Pixel graphics boundary](native-pixel-graphics-boundary.md): CP3A.261005.005 runtime hashes, retained original EGL execution, static module provenance limits and remaining bringup gates. Module membership/loading/KMI and all device execution remain explicitly unqualified.
- [Cellular and IMS audit](cellular-ims-audit.md): package selection, native dependencies and Android service relationships. Its old test-status section is superseded by later completion checkpoints.
- Actual retained init files: `artifacts/telephony-audit/input/vendor/etc/init/android.hardware.power-service.pixel-libperfmgr.rc` and `android.hardware.camera.provider@2.7-service-google-apex.rc`.
- [Android kernel modules](https://source.android.com/docs/core/architecture/kernel/modules) and [KMI monitoring](https://source.android.com/docs/core/architecture/kernel/abi-monitor).
- [Pixel kernel build guide](https://source.android.com/docs/setup/build/building-pixel-kernels).
- [Linux input](https://docs.kernel.org/input/input.html) and [power supply API](https://docs.kernel.org/power/power_supply_class.html).
- [Android SystemSuspend](https://source.android.com/docs/core/power/systemsuspend) and [thermal mitigation](https://source.android.com/docs/core/power/thermal-mitigation).
- [Public Pixel Power HAL source](https://android.googlesource.com/platform/hardware/google/pixel/+/refs/heads/master/power-libperfmgr/aidl/Power.cpp), inspected blob `50ec54390fd625fe0fc833f90ef51952759c4c78`.
- [Android camera architecture](https://source.android.com/docs/core/camera) and [libcamera architecture](https://docs.libcamera.org/master/). libcamera's Android HAL is an implementation over libcamera, not evidence of a generic importer for Pixel's proprietary HAL.
- [Android IMS](https://source.android.com/docs/core/connect/ims).
- [Upstream PowerVR kernel support](https://docs.kernel.org/gpu/imagination/index.html) and [Mesa PowerVR support](https://docs.mesa3d.org/drivers/powervr.html).
- [Halium scope](https://docs.halium.org/en/latest/project/Scope.html): established precedent for native Linux plus Android hardware adapters. This older architecture document does not establish Pixel 10 or Android 17 support.

## Independent review and local verification

BRIDGE VERIFIED: Chat selected, Pro 5 of 5 verified, terminal response controls present and Stop absent, marker PRO-BRIDGE-20261012-hardware-c739 matched. One context round repaired an initial marker-only submission. Review conversation (private transcript). The review supports subsystem-specific boundaries and highlights single ownership of display, suspend, radio and audio. Its advice is not hardware evidence. Public Pixel Power HAL source was independently opened after the review identified it.

Locally rehashed the retained Shannon APK plus power, camera and audio init files against the prior factory evidence inventory; all four matched. Results are in `artifacts/telephony-audit/results/hardware-boundary-recheck.json`. No new device execution or hardware validation occurred.

The later [native device contract checkpoint](native-device-contracts.md) records exact Android17 frozen schemas, selected factory hash matches and remaining execution gates across camera, audio, power, sensors and connectivity.
