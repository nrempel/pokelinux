# Current native smartphone integration map

This map identifies what is composed today and the next integration boundary. It
supplements the [completion matrix](smartphone-goal.md); a passing component or
synthetic transport does not qualify a complete service, app or hardware backend.
Android framework/Java peers remain reference and test infrastructure.

| Function | Existing native component or boundary | Installed service and app composition | Backend and evidence still required |
| --- | --- | --- | --- |
| Permission/lifecycle | One Rust principal/policy actor; exact per-packet Linux credentials, pidfds, generations; owned policy-storage child | Distinct-UID launcher and controls pass installed process fixtures on x86 and an ARM64 kernel. Actual Omarchy ingress and backend dispatch composition remain open | Trusted lock/focus/session ingress, complete process-tree supervision, startup/launch IO and device SELinux. [Authority](native-lifecycle.md), [storage](native-lifecycle-storage.md) |
| Incoming messages | Durable receipt owner, bounded AOSP-qualified codecs, private inbox, exact local occurrence IDs and deletion exclusions | Native Messages service/window integration is in progress; the separately committed synchronous components do not establish current OS authorization or responsive service IO | Producer/reset/ACK identity, raw-retention/backlog policy, multipart association, notifications and carrier receive. [Receipt](native-sms-receipt-owner.md), [inbox](native-inbox.md) |
| Outgoing messages | Durable outbox and one-attempt submission owner; frozen V3 Binder synthetic boundary | No installed Messages send path through the single current permission authority yet | Final modem/error policy, receive/ACK identity, separately qualified delivery correlation, retries/reconciliation and carrier SMS/MMS. [Submission](native-sms-transport.md) |
| IMS/calls | Original-compared wire encoders/reader, single-attempt owned transport/death watch, supervised child reclamation | Existing Phone UI/Android bridge is reference infrastructure; a production native IMS/call service is not composed | Producer/response identity and reset, registration state machine, bearer/data/SIM/eSIM, call/media/audio and carrier operation. [Radio owner](native-radio-owner.md), [reader](native-ims-reader.md) |
| Camera | Frozen camera NDK/Rust Binder types, transferred FDs and owned completion/release/quarantine | No installed Camera service/app with permission, media publication and background ownership | Exact factory interface identity, gralloc/DMA/sync_file/FMQ, capture/streams and reset. [Camera owner](native-camera-owner.md) |
| Power/suspend | Rust lease/arbitration/boot-time fences; Android source ordering and frozen V1 wake-object Binder boundary | No complete installed shared power/suspend authority consumed by all services | Wake-source duties, actual kernel abort/count semantics, charging/thermal, driver supervision and device suspend/resume. [Suspend](native-suspend.md), [boundary](native-suspend-boundary.md) |
| Audio | Rust routing policy and owned helper lifecycle against actual PipeWire/WirePlumber | Dedicated process/CLI fixtures; no installed authorized PCM broker and complete call/media routing | AoC/DSP, physical routes/mute, volume/DND/haptics, durable reconciliation and idle power. [Audio](native-audio.md) |
| Connectivity | Rust reads against actual NetworkManager/private D-Bus with owner/death/late-response fences | No installed permission/app policy for scan, activation, background or portal workflows | Wi-Fi activation/IP/portal, BlueZ Bluetooth, coexistence, USB and device RF. [Connectivity](native-connectivity.md) |
| Sensors/location | Frozen V3 Binder/original FMQ, bounded subscription/read/ACK and wake-lease ownership | No installed app delivery tied to the single lifecycle/permission owner | Source/reader wake duties, USF200ms/HAL1s, calibration, GNSS/location, restart reconciliation and physical sensors. [Sensors](native-sensors-boundary.md), [wake](native-sensor-wake.md) |
| Omarchy appearance | Production Qt theme/view reads original generated files/settings and resolves the live font alias; original commands/rendering pass in ARM64 Hyprland | Original shell plus production-component renderer is qualified; installed enrolled native app/service composition in that same session remains separate | User plugin activation, desktop services/fonts/browser provisioning, full wallpapers/decorations/themes, mobile GPU/display/touch. [Appearance](native-theme.md), [worker](native-theme-worker.md) |
| Deployment | Explicit device/profile and read-only staging admission work is being qualified | No production native boot image, installed service closure or launcher consumes a qualified deployment report | Complete authenticated factory set, exact kernel/KMI/modules/firmware, reduced vendor userspace/Bionic closure, init/SELinux/discovery and actual phone boot |

Contacts, notifications, shared media storage, phone settings, updates and the
remaining background/lock behavior also need native implementation and installed
integration. These are offline engineering gates, in addition to later physical
qualification. No physical phone has been accessed.

## Next composed path

Finish one installed Messages path before multiplying independent service owners:

1. The lifecycle actor launches the actual distinct-UID app and authenticates each
   packet using current kernel identity. Desktop launch ingress carries a fixed app
   identity and cannot grant policy or read messages.
2. A private, supervised Messages worker projects service-owned durable occurrences
   independently of open windows. Current read authorization binds the principal,
   operation, policy/lifecycle generation, worker incarnation, request and absolute
   boot-time deadline. Historical durable admission is not a later capability.
3. Publication rechecks that current authority at actual nonblocking send, including
   delayed/EAGAIN attempts, without holding the authority lock across storage IO.
   Lock, revocation, death, expiry and old-worker replies cannot publish text.
4. Qualify the installed window in original Omarchy/Hyprland, private filesystem
   access, close/reopen and explicit local worker recovery after actual reap.
   Ordinary local reads/projection can recover under qualified SQLite rules;
   uncertain modem effects retain their separate no-replay fence.
5. Add notifications and actual lock/background ingress, then one authorized
   outbound operation through the same authority to the frozen synthetic backend.
   Resolve production identity/ACK/reset and device backend gates separately.

Reuse this installed authority for camera/audio/sensors when their real backend
and app boundary is ready. Keep hardware details in device adapters. An abstract
universal service framework is not a substitute for the composed path.

## Evidence classes

Original-code execution answers how the retained original behaves on authored
inputs. Synthetic x86 Binder tests answer IPC/type/lifetime questions on the owned
Android image. ARM64 user-QEMU answers CPU/runtime questions. ARM64 system-QEMU
answers actual guest-kernel process/filesystem or compositor questions for the
recorded test. None proves matched Pixel kernel policy, carrier behavior or
physical GPU, DMA, audio, charging, sensors or suspend/wake.
