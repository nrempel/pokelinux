# Development handoff

## Goal and constraints

Finish the full Omarchy smartphone system described in `smartphone-goal.md`.
The earlier call-bridge goal was expanded to native Rust phone userspace: calls,
cellular/IMS, SMS, camera, power, audio, connectivity, sensors and background/lock
behavior, with secure local controls, packaging and Omarchy theme integration.
Pixel 10 is the first target; common services must support separate hardware
backends. Retain matched Google kernel/modules/firmware initially. Android/Java
components are reference and test infrastructure, not the intended final runtime.
Use open source first and REA for closed interfaces. No backwards compatibility.

Build and test everything possible offline. Do not access or modify a physical
phone. Keep synthetic emulator, original-code, ARM64/QEMU and physical evidence
separate. Do not equate registration, submission or API success with carrier
acceptance, delivery, native call state or working physical hardware.

## Current stopping point

The native raw-response publication gate, absolute suspend-aware session expiry,
owned registration worker/death watch, and local process reclamation are implemented.
They are not native IMS semantics or a complete phone service.

- `RadioOwner` fences session/revision/preparation identity and checks Linux
  `CLOCK_BOOTTIME` expiry under the gate. Withheld timer delivery cannot permit
  late activation or publication. Candidate/operation lifetimes remain open.
- `TransportSession` owns one registration worker and retained death watch. Stop
  and snapshots do not join or unlink. Cancellation/expiry retain occupied worker
  capacity; completion is independent of the failed callback stream. Late success
  cannot activate, and every second attempt is refused.
- `Supervisor` revokes process output before SIGKILL and retains ownership until
  reaping. Its exclusive, synced private admission marker persists uncertainty
  before launch and survives supervisor restart. It never removes that marker or
  treats reaping as remote cleanup. No successor registration is permitted.
- Real Binder passes held cancellation/expiry, death before/after activation,
  permanent blocked-child termination/reaping, stale-tag refusal and durable
  restart refusal. Component red/green tests include worker detachment, death-result
  ordering, output revocation and stalled reaping. Existing IPC and six Binder
  dispatch-wrapper tests also pass on Harbour with SELinux enforcing.

Evidence: `evidence/radio-deadline-provenance.json`,
`evidence/radio-worker-provenance.json`, `evidence/radio-supervisor-provenance.json`.
Java changes are synthetic peer controls only; no physical phone was accessed.
Harbour now has authenticated `gh` write access and the repository-local existing
author identity. Authentication remains outside source control.

ARM64 user-mode QEMU passes component state, journal and provenance tests, but
its failed-exec subprocess test unexpectedly returns a successful child for a
missing executable. The two process execution tests are now compiled for ARM64
and explicitly ignored by the user-mode suite. `tools/native-process-vm.py` runs
them under an Arch Linux ARM guest kernel in system-QEMU, as the unprivileged
`alarm` user. Both pass, along with the component tests. See
`evidence/arm64-radio-process-provenance.json`. This closes the process-specific
gate; ARM64 Binder, matched Pixel kernel/policy and physical hardware remain open.

Native messaging now has a device-neutral SMS-SUBMIT encoder and bounded multipart
planner. Twenty-seven byte comparisons match actual AOSP encoder execution in the
owned SDK37 emulator, including forced UTF-16 ASCII fragments. The comparison
found and fixed an upstream form-feed mapping omission. Five planner tests and
x86/ARM64 user-QEMU builds pass. The durable outbox now reserves references/payloads atomically, commits uncertain
state before dispatch permission, fences stale owners and preserves cancellation
after late acceptance. Twelve actual SQLite tests and x86/ARM64-kernel SIGKILL
restart tests pass. This does not implement send/receive, delivery, permissions,
a messaging service or UI. See `native-messaging.md` and
`evidence/sms-submit-provenance.json` and `evidence/sms-outbox-provenance.json`.

Incoming SMS text decoding now matches 33 actual AOSP parser outputs for text,
sender, timestamp, reply-path and8/16-bit concatenation. Five bounds/malformed
tests and x86/ARM64 user-QEMU builds pass. Parts are parsed only; durable incoming
deduplication/assembly, vendor acknowledgement, delivery and UI remain open.
See `evidence/sms-receive-provenance.json`.

The frozen standard SMS messaging V3 contract is now recovered. An isolated
original Shannon APK oracle passes50 codec/tracker outcomes and records8 message
IDs without radio dispatch or service startup. Signed response lengths, token
reuse, message-reference ambiguity and removal before failed acknowledgement
are explicit observations. Last-incoming GSM acknowledgement has no durable
receive identity; replay after reconnect remains forbidden without qualification.
See `native-sms-transport.md` and its provenance. The next synthetic frozen Binder
boundary must distinguish one-way return/intermediate ACK from an exact final
send callback, preserve durable uncertainty and fence old endpoint generations.

Focused REA/public-source investigations now recover camera AIDL4, audio core4,
Power7 and Sensors3. Selected audio/power/sensor frozen hashes match shipped
libraries; camera hash equality remains open. Camera failed-return/count and
flush semantics, AoC call audio, two sensor wake obligations and potentially
wedged haptic completion remain explicit boundaries. Twenty-five public power
source checks and AIDL compiler generation passed, separately from runtime.
See `native-device-contracts.md` and its provenance; raw factory/REA records stay
private. The camera error-reply/FD fixture now passes through real Binder using original
generated NDK proxy/dispatcher and an independent Rust client. Both pass
64-cycle FD budgets, transfer/reverse-transfer lifetime, error-count omission,
DEAD_OBJECT and death notification. See `native-camera-boundary.md`; pipe FDs
do not prove gralloc/sync_file/DMA or capture. Native power arbitration now passes18 tests on x86/ARM64 user-QEMU and18
outcomes against actual pinned public RequestGroup/Node/FileNode execution.
Owned leases, cancellation, boot-time expiry, uncertain application, retry and
stale completion fencing are implemented. Authentication, actual backend IO
ownership and a deployed suspend authority remain open; see `native-power.md`.
Native audio now passes five policy tests on x86/ARM64 user-QEMU and16 actual
PipeWire/WirePlumber stages plus an authenticated stop run. One owned backend
process isolates original-stack IO from responsive status/stop. Nonblocking
worker pipes, boot-time deadlines and intent tokens fence late replies; expired
work is killed, retained until reaped and requires explicit recovery. `ok:true`
admits intent; observed application and pending cleanup remain separate. Actual
stalls measured status2.3ms/stop0.64ms, without claiming hard latency. Capture
revocation, capacity reservation, parent death and graph-cookie restoration pass
through real daemons. This qualifies a virtual mono graph; installed service
management, PCM brokerage, durable call recovery, physical mute and Pixel AoC/DSP
routing remain open; see `native-audio.md` and its process provenance.

Native camera ownership now passes nine checks with exact frozen cameraV4
requests/results and original generated NDK callbacks over real Binder. Buffer
return, capture completion and release readiness are distinct. Cached reuse
preserves older capture accounting; duplicate/resolved receipts cannot reclassify
acceptance. Held-call revocation/configuration fences stay responsive, and peer
death retains quarantine; EOF is not a release signal. Three actual runtime reds
preceded fixes. Production publication/storage, durable quarantine/reset,
multi-stream/FMQ, sync_file/gralloc/DMA and physical capture remain open. See
`native-camera-owner.md` and its evidence.

The read-only native connectivity adapter now pins the original NetworkManager
owner and bus identity, targets unique-name RPCs, and checks absolute boot-time
expiry before publication. Eight real-daemon/private-D-Bus stages pass with x86
and ARM64 user-QEMU Rust clients; the daemon stays x86 in both runs. No host NIC
or bus is accessed. Installed authority, authenticated permissions, mutations,
BlueZ and device behavior remain open; see `native-connectivity.md`.

The native suspend coordinator now has one cooperating writer, owned wake
objects, atomic responsive stop/snapshots, abort-before-counter ordering and
terminal uncertainty. Absolute CLOCK_BOOTTIME budgets fence late successful FD
operations without resetting after EINTR. Twenty-seven checks pass on x86 and
under an actual ARM64 Arch kernel, including three process/Unix lifetime cases;
24 component checks pass under ARM64 user-QEMU. Eight actual unmodified Android17
source cases qualify the reference ordering/lifetime separately. Pixel kernel
count/abort controls and driver-blocked process supervision remain open; see
`native-suspend.md`.

The retained frozen V1 suspend Binder boundary now passes18 actual x86 outcomes:
original generated NDK/Rust interoperability in both directions, explicit and
final-reference release, strong-object transfer across client exit, second-client
death, and a retained authority death recipient permanently fencing the native
coordinator. Both unchanged original libpower cache branches execute actual RPCs
through a link-time discovery adapter. A held-name success after service death
does not prove a current wake blocker. SELinux stays enforcing; default is occupied
and custom registration is denied. Installed standard discovery/permissions,
ARM64 Binder and actual suspend remain open; see `native-suspend-boundary.md`.

Native SMS submission ownership now drives the durable outbox through exact
frozen messaging V3 original NDK/Rust Binder. Ten live groups and seven additional
SQLite owner tests pass. One-way return/intermediate ACK cannot accept a part;
only a matching final qualified result can. Malformed acknowledgement hex,
wrong endpoint/method/serial/generation, duplicates, cancellation and death
cannot consume or revive work. Both live defects went red before their fixes.
This remains a synthetic SOLICITED/NONE/errorCode-1 qualification, with modem
mapping, receive identity/ACK replay, delivery and installed integration open.
See `native-sms-transport.md` and `evidence/native-sms-binder-provenance.json`.

Native sensor ownership now passes11 actual x86 SensorsV3 Binder/original FMQ
and EventFlag groups. The fixed96-byte Event ABI is probed by original NDK code;
Rust copies opaque aligned bytes and validates them before normalization.
Authenticated subscriptions, reserve-before-read, processing-before-ACK, exact
split counts, full-queue retention, revocation, stale FD generations, malformed
Event ownership,32 FD cycles and peer-death quarantine pass after two live reds.
The source wake counter is synthetic and there is no acquired SystemSuspend
reader wake lock yet. USF200ms, top-level HAL1s cleanup, actual suspend integration,
app publication/lifecycle, durable reconciliation, calibration and ARM64/device
proof remain open; see `native-sensors-boundary.md`.

Native GSM status-report parsing now passes57 field comparisons against actual
original AOSP execution plus12 focused tests on x86 and ARM64 user-QEMU. Both
receipt/discharge timestamps, numeric addresses, status, optional PID/DCS and
qualified text are bounded. AOSP parsed41 deliberately rejected native cases;
that divergence remains explicit. TPDU/full-PDU limits are164/177 bytes. Parsing
provides no unique outbox delivery key or modem acknowledgement authority.
Incoming receipt/ACK ownership is currently a design with no implied deduplication
or replay; see `native-sms-status-report.md` and `native-sms-receipt-owner.md`.

## Next work

1. Recover and test the actual vendor cleanup/reset and registration-overlap
   contract. `clear` is one-way. Do not remove uncertainty markers, retry, use a
   recycled numeric registration ID for delayed cleanup, or call local reaping
   vendor cleanup. Keep registration unavailable without proven remote recovery.
2. Integrate these primitives into an installed Rust service and its service
   manager. Define authenticated permissions and observer IPC with process/session
   fencing, bounded output, safe durable admission and bounded diagnostics.
   Qualify ARM64 Binder and matched Pixel kernel/policy separately; the owned
   ARM64 system-QEMU process gate now passes.
3. Extend ownership to request transport, operation receipts, absolute candidate
   and operation deadlines, timer delivery, notification watches and aggregate
   budgets. Recover producer identity before enabling fragmented semantic output.
4. Recover native IMS state/response contracts with original-code comparison and
   integrate the shared phone service. Continue calls, SMS, camera, power, audio,
   connectivity, sensors, background/lock behavior, permissions, apps, deployment
   and genuine Omarchy integration in the full smartphone matrix.

An advisory Pro worker-lifecycle consultation was stopped at the user's handoff
request before a terminal answer. It is not a completed review or accepted design:
https://chatgpt.com/c/6acd2fad-19c0-83e8-b059-f88452119b16
Earlier completed owner reviews are recorded in `native-radio-owner.md`.

## Verification and publication

Use the existing prepared Linux checkout and its private inputs. Follow
`AGENTS.md`, strict red-green TDD, and `remote-development.md`. Update README and
affected docs each work turn when implementation or evidence changes. Run:

```sh
bash tools/build-native-radio.sh build
bash tools/build-native-messaging.sh build
bash tools/build-power-reference.sh build
bash tools/build-native-power.sh build
bash tools/build-suspend-reference.sh build
bash tools/build-native-suspend.sh build
bash tools/build-native-suspend.sh arm64-process
bash tools/build-native-suspend-boundary.sh build
bash tools/build-native-suspend-boundary.sh arm64-check
bash tools/suspend-boundary-suite.sh green
bash tools/build-native-sensors-boundary.sh
bash tools/sensors-boundary-suite.sh green
bash tools/build-native-audio.sh build
bash tools/build-native-connectivity.sh build
bash tools/build-native-audio.sh integration artifacts/native-audio/NEW_RUN
bash tools/framework-suite.sh sms-codec
bash tools/framework-suite.sh sms-transport-oracle
bash tools/framework-suite.sh sms-binder
bash tools/framework-suite.sh binder-kernel
bash tools/framework-suite.sh radio-ipc
bash tools/framework-suite.sh camera-owner
# In the prepared network-disabled graphics simulation container:
python3 tools/native-process-vm.py
```

These are focused native-radio gates, not proof that the full smartphone goal is
complete. Keep scope-specific evidence and explicit remaining gates. Public source
is on `main` at https://github.com/nrempel/pokelinux. Commit and push source/docs
as work progresses, after checking staged files. Preserve private artifacts and
pre-existing untracked files; never stage them wholesale. Check GitHub write
authentication on a new host without copying credentials from another machine.
