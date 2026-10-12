# Native device contracts

Harbour checkpoint, 12 October 2026. Shared Rust services own permission,
policy, lifecycle and publication; device backends own wire IDs, calibration,
firmware paths and native HAL details. Google's matched kernel/modules/firmware
remain the initial base. Android framework/Java components remain references
and test infrastructure. Retained vendor userspace requires an explicit
unresolved dependency exception. No physical phone was accessed.

Public frozen schemas were recovered from `android-17.0.0_r1`; missing versions
on the main branch were not evidence that their source was unavailable.
[Provenance](evidence/native-device-contract-provenance.json) separates factory
static observations, public original-source/compiler execution and unexecuted
integration. Raw ELF, configs, REA responses and snapshots remain private.

| Backend | Exact contract checkpoint | Execution still required |
|---|---|---|
| Camera | AIDL provider/device v4, despite the executable’s `@2.7` name; public frozen schema available | Factory hash equality, real Binder/fd/FMQ, allocator/mapper, fences, processing and safe cleanup |
| Audio | Core v4 hash matches bytes in the verified factory NDK library; common v5 schema available | Live interface handshake, FMQ/mmap/stream teardown and DSP/route integration |
| Power | Power v7 public hash matches the factory NDK library | Native policy, oneway/application distinction, suspend authority, charging/thermal integration |
| Sensors | Sensors v3 public hash matches the factory NDK library | Mapped queues/EventFlag, wake accounting, calibration, restart and dependencies |

## Camera

The service owns exclusive permission/privacy admission, session/configuration
generations, async capture state and bounded buffer/fd/fence leases. The Pixel
backend maps frame/stream IDs, metadata/FMQ, allocator imports and error codes.
Buffer return, complete capture and signalled release fence are different events.
A returned buffer is not reusable merely because its callback arrived.

Static factory evidence shows cache removal, request conversion and delegation
to Google HAL, plus acquire-fence duplication and GraphicBufferMapper imports.
The public original AIDL compiler produced proxy/dispatcher code that omits
return values on error. Thus a wrapper’s locally updated partial count cannot
prove acceptance after a failed Binder reply. Retain uncertain ownership until
qualified callback/drain/reset reconciliation. This is compiler execution and
static reasoning; the generated NDK/Rust runtime boundary now passes the [real Binder error/FD fixture](native-camera-boundary.md). Pipe FD ownership does not qualify camera DMA, gralloc or fences.

Normal flush delegates to Google HAL. The recovered `signalStreamFlush` is a
success stub, so it establishes no drainage. Close/destructor cleanup, failed
callback delivery and safe reopen remain unqualified. Local child reaping does
not prove camera DMA ceased. Actual supported combinations, vendor tags, image
quality, video, allocator/mapper imports and Wayland/PipeWire rendering remain
execution/hardware gates.

## Audio and connectivity

One native authority coordinates focus, routes, microphone permission and call
media. PipeWire provides transport and WirePlumber session policy controls graph
links. Privileged call/alert leases require authenticated authority; an app’s
`media.role` property cannot grant it. Desired route and observed application
remain distinct, including unplug/restart and lock/background behavior.

Pixel IMS media opens `/dev/acd-audio_rtp_rx` and `/dev/acd-audio_rtp_tx`; recovered
paths contain command/reply validation and close/reopen/reinitialization. AOC
microphone mute reaches both ordinary streams and telephony, so ordinary capture
mute does not establish voice-uplink mute. Call mode, patch application, codec
readiness, stream start/stop and actual media progress are separate states.
These are static deductions, not successful DSP operation. Audio core v4 mmap
uses `IStreamCommon.createMmapBuffer`; the older open-stream FD is not the usable
audio buffer. Stream close and device disconnect have ordered teardown contracts.

The [read-only native connectivity boundary](native-connectivity.md) now qualifies
original NetworkManager owner identity, observation expiry and daemon/bus death
against a private loopback-only namespace. It performs no radio mutation.
Native connectivity should coordinate one qualified NetworkManager Wi-Fi path
and BlueZ ownership. Scan completion, association, IP configuration and reachability
need separate observations. iwd `Network.Connected` can describe a connection
still in progress. Bluetooth discovery sessions, pairing, connected state and
active media are separate; FD/MTU and endpoint lifetimes belong to audio.
PipeWire’s native HFP call commands use ModemManager, so the Samsung backend
needs explicit call-service integration rather than a second RFCOMM owner.

BCM4383 calibration/regulatory inputs, Bluetooth AoC/serial firmware setup, modem
coexistence, tethering and USB Type-C/gadget ownership remain backend gates.
No physical interface was controlled. Next tests use isolated software graphs,
mac80211_hwsim and virtual Bluetooth controllers.

## Power, sensors and haptics

Exactly one native suspend authority must account for both shared-service leases
and retained vendor/kernel wake responsibility. An independent autosuspend loop
would bypass that accounting. Battery observation through power_supply is
separate from charging policy; preserve units, signed values, freshness and
unknown fields. Native display intent must coordinate with the sole compositor.

Factory power handling includes queued node updates, performance/thermal budgets
and mode capabilities. Positive boost duration overrides defaults; zero selects
configured defaults; negative cancels. Oneway acceptance does not prove node
application. The public libperfmgr RequestGroup/Node/FileNode implementation
passed 25 reference checks with POSIX helper shims and actual temporary-file I/O.
Factory paths corroborate selected renewal/retry behavior but the shipped worker
is newer; these results do not establish broad source/binary equivalence.

Sensor ownership includes subscriptions, queue acknowledgements and generation
fencing across deactivate/reinitialize. USF holds a separate sliding 200 ms
libpower wake lock. The top-level frozen Sensors v3 FMQ missing-ack contract is
one second. Both obligations matter; handing an event to an app does not prove
that its work is protected against suspend. USF/Suez/AoC, display and proximity
dependencies, direct-channel teardown and calibration remain unqualified.

Factory haptics uses Linux force-feedback discovery/upload/playback with
calibrated controls. Its completion worker can wait indefinitely for stopped
state and does not check that poll’s result before reporting completion. An
accepted command or callback therefore does not prove actuator stop. The native
backend needs bounded ownership/recovery and one authority coordinating audio
external control, DND, ringtone and app permissions.

## Completion boundary

These recovered contracts guide implementation. They do not deliver installed
services or qualify hardware. Real generated ABI/FMQ, isolated audio/network
stack, service death/restart, permission/revocation and shared lock/background
tests are required before later authorized Pixel boot, modem/carrier, camera,
audio, charging/thermal/drain, sensor and suspend/wake measurements.
