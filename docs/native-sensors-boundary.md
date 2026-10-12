# Native sensor ownership across SensorsV3 and FMQ

This records the original11-group ownership boundary from7752354. The separate
[current sensor wake integration](native-sensor-wake.md) adds actual coordinator
leases around reads/ACKs and its own10-group original-FMQ/frozen-V1 evidence.

`native-sensors` provides a bounded, device-neutral owner for authenticated sensor subscriptions, event admission, normalized acceleration samples, processing and wake acknowledgements. `native-sensors-boundary` drives it with the frozen Android 17 SensorsV3 interface and an independent original NDK peer. The fixture uses the original public `AidlMessageQueue`, `MessageQueueBase`, `EventFlag` and native-handle implementation over actual Binder, shared memory and futex event words in an owned SDK37 x86_64 emulator. No Android sensor framework implementation is part of the native service owner.

The owner holds logical kinds and client tokens. The backend adapter holds raw handle 77, the fixture's sensor type and its generated object layout. Location is an explicit unsupported permission path. The test app's installed UID comes from the owned emulator's package manager. Nested NDK-to-Rust Binder calls authenticate the kernel caller UID, check the payload claim against it and require a service grant. A claimed UID of zero cannot authenticate the app. Revocation prevents further admission and publication eligibility; already admitted, correctly processed service-owned work may still be disposed and acknowledged.

Admission reserves a wake responsibility before a synchronized queue read advances its counter. The service mutex covers the bounded local reserve/read/decode operation. A failed or unsupported decoding retains a quarantined responsibility. Processing is single-shot. Acknowledgement preparation refuses unprocessed, duplicate, unknown, stale or quarantined tickets and computes the count from owned wake work. Preparation retains ownership. A successful queue write transfers the exact count; the reader commits it and signals the original wake-ack event word. A full acknowledgement queue leaves the responsibility owned and retryable. The backend's synthetic count is decremented only when its original queue reader receives an acknowledgement.

Generation replacement invalidates client tokens and quarantines admitted old work. Closing or replacing the peer's queues does not erase native responsibilities. Old shared-memory mappings and event words remain independent of a new generation and of receiver FD closure. Real Binder death disables admission/publication/acknowledgement and retains quarantine. This owner deliberately has no automatic quarantine retirement or silent restart recovery.

## Qualified ABI glue

The official pinned SDK37 AIDL compiler emits a fixed-size, standard-layout native `Event` with size 96, alignment 8 and a one-byte union tag. The peer obtains field offsets from an actual valid generated object and accessors; the Rust client checks the exact layout before reading events. An NDK-compiled object symbol supplies the opaque quantum. The runtime-checked offsets are timestamp 0, handle 8, type 12, payload tag 16, and Vec3 fields 24/28/32/36.

Rust maps `RawEvent`, an aligned 96-byte array with every bit pattern valid, rather than a generated Rust enum or foreign C++ object. The single audited `unsafe impl rsbinder::fmq::Element` declares this byte representation safe to copy. The decoder checks tag, backend handle/type, scalar bounds, positive timestamp, finite values and status before exposing the normalized sample. This qualification is specific to the pinned x86_64 compiler and little-endian target; another architecture requires its own original-code layout probe.

The frozen schemas use qualified references rather than explicit imports. Both generators therefore consume the full pinned import closure. The Rust adapter transfers validated `rsbinder` descriptor fields into the generated frozen parcelable; it never patches an upstream schema. Original libfmq raw-copy support explicitly accepts fixed-size AIDL types even though the generated native union is not trivially copyable. The build suppresses only the relevant original-code compatibility diagnostics (`nontrivial-memcall` and `deprecated-copy-with-user-provided-copy`) and leaves the source unchanged.

Own support glue supplies logging/check functions, the ashmem entry points through sealed memfd allocation, and elapsed-time functions through `CLOCK_MONOTONIC`. The executed NDK peer attaches to queues created by Rust; its ashmem allocator shim is compiled infrastructure, not an exercised allocation oracle. The ring operations, descriptor attachment/cloning, queue counters, native-handle FD duplication and EventFlag implementation are original public code.

## Reproduce

Use the existing ignored SDK37 emulator/toolchain installation and `pixel-native-radio:20261012` / `pixel-android-framework:20261011-b472` build images on Harbour. The manifest pins all public source files, frozen schemas, compiler and Binder helper inputs. The prepare tool validates every source hash and refuses unmanifested frozen schemas. The official NDK r30 archive was separately pinned and qualified in the camera boundary; sensor preparation validates and reads those immutable installed tools without editing camera paths.

```sh
python3 tools/prepare-sensors-boundary.py
# Inside the existing Android build image mounted at /workspace:
bash tools/build-sensor-peer.sh
# On Harbour, builds static Rust client and runs fmt/clippy checks:
bash tools/build-native-sensors-boundary.sh
# Owns suite.lock, internal network, emulator, app and bounded client:
bash tools/sensors-boundary-suite.sh green
```

The runner fails if another framework suite owns the workspace. Its exit trap stops/removes only its named emulator container/network. The peer APK's Java instrumentation and private framework reflection oracle bootstrap the disposable test app; the owner, policy and FMQ client are native Rust. Raw upstream sources, generated outputs, APKs, binaries, red snapshots, emulator metadata and logs remain ignored under `artifacts/sensors-boundary`.

## Evidence and limits

The public [evidence record](evidence/native-sensors-boundary.json) distinguishes prerequisite execution, two actual runtime red failures and the final green. The first failure sent an acknowledgement before processing: the original queue received count 1 and the synthetic source count dropped from 2 to 1; the client exited 101. The second failure consumed a malformed original generated Event without retaining ownership: the queue became empty while the native wake responsibility count remained zero; the client exited 101. An earlier one-byte-tag qualification failure is retained as a prerequisite failure, separate from those reds.

The runtime checks exact version/hash, actual authenticated Binder caller, independent FD ownership, original queue/event-word interoperability, bounded admission and queue backpressure, split/out-of-order/duplicate/count checks, revoked publication eligibility, stale generation rejection, malformed-event quarantine, 32 FD replacement cycles and real peer death. The original EventFlag waits for the frozen one-second interval with no acknowledgement; expiry leaves the synthetic responsibility pending. A separate short bounded wait checks timeout behavior after peer death.

The fixture's pending count is synthetic. Its defensive oversized-delta clamp follows a retained public multihal reference, but that reference's wake algorithm is not executed here. Native responsibilities are accounting records, not acquired SystemSuspend wake locks. Physical suspend protection requires connecting the reader's acquire-before-read and acknowledgement transfer to the project's single qualified suspend authority. The Android one-second missing-ack contract and the separately recovered USF sliding 200 ms libpower obligation remain distinct. A tested EventFlag wait proves neither top-level HAL timeout cleanup nor USF behavior.

Remaining gates include actual wake-lock acquisition/release and failure handoff; durable quarantine, restart reconciliation and concurrently held producer/reinitialize calls; app delivery/permission authority integration beyond this authenticated fixture and eligibility guard; independent client death and multiple app UIDs; rate aggregation, non-wake events, flush/one-shot ordering, dynamic sensor callbacks and direct-channel cleanup; AOC/Suez/USF transport, calibration registry, axes/units/timestamps, display/proximity dependencies; ARM64 ABI and kernel qualification; physical sensor/location and suspend/resume evidence. No physical phone, host sensor or vendor code was executed. Synthetic vector values establish transport and ownership, not calibration or location.
