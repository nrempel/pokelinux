# Current native smartphone integration map

This map identifies what is composed today and the next integration boundary. It
supplements the [completion matrix](smartphone-goal.md); a passing component or
synthetic transport does not qualify a complete service, app or hardware backend.
Android framework/Java peers remain reference and test infrastructure.

| Function | Existing native component or boundary | Installed service and app composition | Backend and evidence still required |
| --- | --- | --- | --- |
| Permission/lifecycle | One Rust principal/policy actor; exact per-packet Linux credentials, pidfds, generations; owned policy-storage child | Distinct-UID launcher and controls pass installed process fixtures on x86 and an ARM64 kernel. Local Messages reads compose this actor with its own storage child; trusted Omarchy ingress and hardware dispatch remain open | Trusted lock/focus/session ingress, complete process-tree supervision, startup/launch IO and device SELinux. [Authority](native-lifecycle.md), [storage](native-lifecycle-storage.md) |
| Incoming messages | Durable receipt owner, bounded AOSP-qualified codecs, private inbox, exact local occurrence IDs and deletion exclusions | Installed local-history Messages service/Qt window now pass11 ARM64 desktop groups with10 service groups and2 retained-descriptor cases; current read authority, responsive storage and local recovery qualify. A separately built receiving fixture now composes actual ARM64 Binder arrivals with this actor and sole raw/inbox child (18 groups); normal installed service stays history-only | Producer/reset/ACK identity, raw-retention/backlog policy, multipart association, notifications and carrier receive. [Receipt](native-sms-receipt-owner.md), [inbox](native-inbox.md), [installed history](native-messages-service.md), [synthetic incoming](native-messages-incoming.md) |
| Outgoing messages | Durable outbox and one-attempt submission owner; frozen V3 Binder synthetic boundary | No installed Messages send path through the single current permission authority yet | Final modem/error policy, receive/ACK identity, separately qualified delivery correlation, retries/reconciliation and carrier SMS/MMS. [Submission](native-sms-transport.md) |
| IMS/calls | Original-compared wire encoders/reader/registration payload, single-attempt owned transport/death watch, supervised child reclamation and scoped ARM64 kernel Binder/NDK interop | Existing Phone UI/Android bridge is reference infrastructure; a production native IMS/call service is not composed | Producer/response identity and reset, registration state machine, bearer/data/SIM/eSIM, call/media/audio and carrier operation. [Radio owner](native-radio-owner.md), [reader](native-ims-reader.md), [registration payload](native-ims-registration.md) |
| Camera | Frozen camera NDK/Rust Binder types, transferred FDs and owned completion/release/quarantine | No installed Camera service/app with permission, media publication and background ownership | Exact factory interface identity, gralloc/DMA/sync_file/FMQ, capture/streams and reset. [Camera owner](native-camera-owner.md) |
| Power/suspend | Rust lease/arbitration/boot-time fences; Android source ordering and frozen V1 wake-object Binder boundary | No complete installed shared power/suspend authority consumed by all services | Wake-source duties, actual kernel abort/count semantics, charging/thermal, driver supervision and device suspend/resume. [Suspend](native-suspend.md), [boundary](native-suspend-boundary.md) |
| Audio | Rust routing policy and owned helper lifecycle against actual PipeWire/WirePlumber | Dedicated process/CLI fixtures; no installed authorized PCM broker and complete call/media routing | AoC/DSP, physical routes/mute, volume/DND/haptics, durable reconciliation and idle power. [Audio](native-audio.md) |
| Connectivity | Rust reads against actual NetworkManager/private D-Bus with owner/death/late-response fences | No installed permission/app policy for scan, activation, background or portal workflows | Wi-Fi activation/IP/portal, BlueZ Bluetooth, coexistence, USB and device RF. [Connectivity](native-connectivity.md) |
| Sensors/location | Frozen V3 Binder/original FMQ, bounded subscription/read/ACK and wake-lease ownership | No installed app delivery tied to the single lifecycle/permission owner | Source/reader wake duties, USF200ms/HAL1s, calibration, GNSS/location, restart reconciliation and physical sensors. [Sensors](native-sensors-boundary.md), [wake](native-sensor-wake.md) |
| Omarchy appearance | Production Qt theme/view reads original generated files/settings and resolves the live font alias; original commands/rendering pass in ARM64 Hyprland | Original shell plus production-component renderer is qualified; the installed distinct-UID Messages service/window also qualifies in that session; the full phone bundle remains open | User plugin activation, desktop services/fonts/browser provisioning, full wallpapers/decorations/themes, mobile GPU/display/touch. [Appearance](native-theme.md), [worker](native-theme-worker.md) |
| Deployment | Explicit device/profile and read-only staging admission pass29 x86/ARM64 user-QEMU groups; actual ELF geometry and streaming inspection qualify | No production native boot image, installed service closure or launcher consumes a qualified deployment report; selected Pixel profile refuses admission | Nine offline requirements, one physical gate, three absent artifact pins: authenticated set, kernel/KMI/modules/firmware, vendor userspace/Bionic closure, init/SELinux/discovery and service composition. [Deployment](native-deployment.md) |

Contacts, notifications, shared media storage, phone settings, updates and the
remaining background/lock behavior also need native implementation and installed
integration. These are offline engineering gates, in addition to later physical
qualification. No physical phone has been accessed.

## Next composed path

The installed local-history Messages path is qualified separately from continuous
receiving. One existing lifecycle actor owns current read authority and every
actual send/EAGAIN retry; one supervised writer owns receipt/inbox IO and a separate
readonly child owns each history transaction. The native
window is installed in original Omarchy. Root fixture focus/lock controls and
appearance ACL setup remain explicit test infrastructure.

1. Preserve the qualified synthetic incoming composition: genuine ARM64 Binder
   callbacks reach the sole storage child; raw commit and projection run without
   a window/read. The parent drains unsolicited start/raw/completion/failure and
   enforces original boot-time deadlines. Normal production service stays history-only.
2. Preserve durable source uncertainty across child and whole-service restart,
   including installed systemd restart. Only local history recovers; actual reap
   and authenticated stop-to-service-exit never grant reattachment or vendor cleanup.
   Explicit synthetic successors require fresh independent peers and reset context.
   The [owned read split](native-messages-receive-isolation.md) preserves the same
   qualified receiver through close, expiry and actual SQLite-lock contention.
   Exact-occurrence permits and original deadlines still govern projection;
   production attachment and broad cold app/session availability remain separate.
   No production ACK/send is enabled.
3. Complete receipt retention/capacity and cold-backlog availability. The
   [bounded enumeration](native-messages-enumeration.md) now qualifies every ID
   within the append-only4096 bound using fixed-upper pages/eight-import turns;
   no partial Ready or callback recreation occurs. Its six ARM greens use the
   frozen `f733d3b` actor, separately from later session composition. Raw eviction,
   inbox bounds and throughput under unchanged deadlines remain open.
4. Add trusted lock/focus/session ingress and native notifications through this
   same authority. Source failure/default lock denies reads. Distinct UIDs and
   Wayland socket ACLs do not establish compositor protocol containment. Qualify
   wake ownership separately; closed windows do not prove suspended delivery.
   The [matching compositor build probe](native-session-build-boundary.md) now
   executes11 original API/protocol/kernel groups; the subsequent
   historical [ingress prototype](native-session-ingress.md) qualifies17 prepared original
   compositor/observer groups and13 synthetic-source/current-authority ARM groups
   separately. Joint composition, cold availability and retained-kernel identity
   remain open; that earlier scalar-pidfs prototype cannot substitute for Pixel
   capability qualification.
   Admission callbacks precede the original locked flag. The
   [session contract](native-session-contract.md) retains trusted locker/restore
   containment and external-transition publication ordering as explicit gates.
   Current [owned focus](native-session-owned-focus.md) replaces scalar identity:
   14 leaf/26 observer checks pass on host/ARM,15 host/14 ARM actor groups and17
   original-compositor groups qualify separate paths. The
   [joint installed app path](native-session-installed-app.md) now passes six normal
   production Qt groups and three separate10-group diagnostic source epochs for
   local synthetic history. The writer/readonly split now passes a fresh six-group normal Qt replay and
   ten separately labeled diagnostic groups; see [combined reads](native-session-receive-read-combined.md).
   The [display primitive](native-display-containment.md) now qualifies original
   privileged bind filtering and supervised SDK ownership separately. Compose its
   restricted listener with per-app context generations/relaunch. The
   [service-epoch actor composition](native-session-display-composition.md) now
   passes25 ARM observations after two integration reds, with source admission
   fencing and actual SDK reaping within the original actor cleanup budget.
   [Incoming/session/normal Qt](native-incoming-session-installed-app.md) now
   qualifies six narrow groups in each of two epochs and eight separate source-loss
   groups. Both historical full close/reopen matrices fail cold reopening under
   the unchanged lease. The later [source-queue correction](native-session-freshness-chain.md)
   passes34 process cases per host/ARM kernel and nine separately selected original
   compositor/incoming/normal-Qt groups, including cold reopen and source/peer loss.
   It accepts only fresh continuous queued samples; actual expiry remains terminal.
   The observed99.2ms publisher gap leaves broad availability open. Actual locked
   graphics can expire the original100ms lease and restoration remains unavailable.
   Protected procfs/common namespaces and matched Pixel kernel remain gates.
5. Add one authorized outbound operation through the same authority to the frozen
   synthetic backend. Resolve production identity/ACK/reset and device backend
   gates separately, then extend the installed path to the remaining phone apps.

Reuse this installed authority for camera/audio/sensors when their real backend
and app boundary is ready. Keep hardware details in device adapters. An abstract
universal service framework is not a substitute for the composed path.

## Evidence classes

Original-code execution answers how the retained original behaves on authored
inputs. Synthetic x86 Binder tests answer IPC/type/lifetime questions on the owned
Android image. ARM64 user-QEMU answers CPU/runtime questions. ARM64 system-QEMU
answers actual guest-kernel process/filesystem, Binder or compositor questions for
the recorded test. The native radio Binder milestone separates nine Rust/manual
peer scenarios from three genuine Android runtime/NDK interop scenarios. None proves matched Pixel kernel policy, carrier behavior or
physical GPU, DMA, audio, charging, sensors or suspend/wake.
