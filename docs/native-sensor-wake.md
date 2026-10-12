# Sensor admission under one native suspend authority

`native-sensors::wake::WakeOwner` now retains an actual `native-suspend::WakeLock` for each consumed wake event. The integration executes against the existing original SensorsV3 NDK/FMQ peer and exposes the same coordinator through the committed frozen V1 `ISystemSuspend`/`IWakeLock` frontend. The owned x86 SDK37 runtime proves native ownership and real Binder/shared-memory boundaries. Kernel power IO remains injected into private regular files and sockets; no hardware suspend, physical sensor or vendor execution is claimed.

The prior sensor milestone is7752354 and the suspend Binder milestone is e8ff6ea. Their original public evidence remains historical and unchanged. This scope adds ownership hooks and a new executable, runner and [provenance record](evidence/native-sensor-wake-provenance.json). The original peer sources and `native-suspend` coordinator/frontend were not changed.

A bounded policy reservation precedes acquisition. Acquisition executes outside the sensor mutex: actual abort/unlock IO and coordinator-gate waits cannot block revocation or sensor snapshots. After acquisition, the service rechecks generation and permissions, attaches its private native wake object to the reservation, and then invokes the bounded local queue read. Failed acquisition never invokes that read. A refused admission releases only its unconsumed reservation. An empty synchronized read returns the unconsumed lease; a malformed or uncertain receipt keeps its consumed responsibility quarantined. Reader callbacks must perform one bounded local read/normalization, and the adapter must bind its queues to the supplied service generation.

Correctly decoded events retain the lease through processing. Revocation removes publication eligibility; service-owned processed disposal can still hand back its acknowledgement. Preparation computes the exact count from processed tickets. A full ACK queue permits retry while retaining the actual lease. Successful full queue write plus successful event-word notification qualifies the handoff; the policy commits before the private native objects are released. An uncertain write or a successful write followed by a failed notification retains a marked acknowledgement and native lease, and refuses duplicate retry. Lease destruction occurs outside the sensor mutex so a coordinator gate cannot stall policy access.

Generation replacement quarantines old records without creating another coordinator or resetting its wake objects. Source Binder death marks records unavailable and stops the coordinator, even when queued work cannot be identified by a current ticket. Last owner destruction also stops the coordinator before Rust automatically destroys its retained native objects. Thus a later count of zero after local teardown is accompanied by a terminal authority fence, never a successful cleanup or rearm assertion. No persistent quarantine or new-process recovery is implemented.

## Executed boundaries

The new runtime compares original generated NDK frozen V1 proxies to the unchanged Rust dispatcher using the same coordinator used by the sensor owner. Original client checks version/hash, actual UID/PID/SID, one-way release receipt, explicit release once, final reference destruction and foreign-caller rejection. A Binder wake object retained by the instrumentation broker and a consumed sensor ticket simultaneously counted two. Qualified sensor ACK reduced it to one, while a native attempt stayed blocked. Discarding the final retained Binder reference reduced it to zero; only then did one explicit injected `mem` write return. The broker is synthetic discovery, and the existing stock default SystemSuspend was not replaced.

Two intentional runtime faults failed with Rust exit101 before their fixes:

- Reading the real original-produced FMQ event before acquisition advanced the read counter with native wake count zero instead of one.
- Releasing the native object at processing let the injected suspend attempt return while the real ACK queue was full and unable to transfer the count.

Final green passed ten groups. Read-only abort and unlock descriptors caused actual acquisition errors and degradation, with the original event still unread and no acknowledgement. A full real Unix socket held abort IO while snapshot/revocation completed within100ms; after IO resumed, the revoked reader declined admission and left the event unread. Processing/ACK pressure, exact V1 shared authority, malformed data, generation replacement, real source death and teardown fencing all executed. Native `stop()` and `snapshot()` retain their committed atomic behavior. Sensor and coordinator snapshot values describe separate observations; foreign Binder objects contribute to the coordinator total.

The signal-failure test executes an actual syscall error. A test-only x86_64 worker installs a per-thread seccomp filter returning `EPERM` for `FUTEX_WAKE_BITSET`. It writes the valid ACK into real shared memory and calls the normal Rust EventFlag implementation. The atomic bit publication happens before the rejected syscall, so the original NDK reader can still observe the flag and consume the count. The native owner keeps quarantine and refuses another ACK despite that source observation. A syscall error can therefore coexist with peer-visible publication; an unsuccessful notification is not a safe retry or retirement signal.

The seccomp ABI leaf is limited to the `wake-fault-test` feature and required test binary. Its audited unsafe calls pass initialized libc kernel POD filter arrays to synchronous `prctl`; the filter only adds a restriction to the owned guest worker, uses no TSYNC and allows mutex wake/thread exit. The production sensor owner forbids unsafe code. The inherited opaque96-byte/tag1 FMQ ABI qualification is unchanged and remains x86-specific.

## Reproduce and scope

With the existing ignored qualified public inputs and both peer APKs prepared:

```sh
bash tools/build-native-sensor-wake.sh
bash tools/sensor-wake-suite.sh green
```

The build verifies both public input closures, performs formatting and warnings-denied clippy checks, locks dependencies and checks the static musl ELF has no interpreter. The runner exclusively owns `android-framework/suite.lock`, a named internal network/emulator, both disposable apps, bounded native clients and joined workers. It gathers the private descriptor traces and removes its resources. Historical sensor/suspend artifacts are read-only inputs; new outputs and immutable reds are under ignored `artifacts/sensor-wake`.

The source pending-wake count is still a synthetic test ledger. Reader ownership now uses actual native coordinator objects, but the original sensor fixture does not implement vendor wake acquisition. The USF sliding200ms libpower protection and the top-level HAL one-second missing-ack contract remain separate obligations. Neither source process reaping, queue replacement, local count zero nor native authority fencing establishes vendor cleanup, DMA quiescence or physical suspend safety.

Remaining gates are durable quarantine/restart reconciliation; a supervised production sensor service and atomic app publication authority; authenticated production permission/lifecycle integration beyond the existing Binder fixture; transport-generation binding across concurrently held producer/reinitialize calls; source acquire-before-publish and failure participation in the same installed authority; standard V1 service discovery/VINTF/SELinux and libpower death-cache reconciliation; qualified kernel paths/count rejection/abort behavior and stuck-driver supervision; ARM64 FMQ/Binder integration; AOC/Suez/USF/calibration/display/proximity; physical sensors/location and suspend/resume. The shared owner contains logical kinds and normalized samples; raw sensor handles and layouts remain in the backend adapter.
