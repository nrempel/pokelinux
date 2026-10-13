# Native camera actor and process ownership

2026-10-13. The lifecycle actor now has a typed `camera_capture` request and one
owned camera backend. The actual current app PID/UID needs an independent Capture
grant. Caller input is only `ttl_ms`; the actor creates capture/generation identity
and retains a nonserializable stamp binding principal, policy, lifecycle, backend,
original BOOTTIME deadline and, when configured, exact Source revision/lease.

The actor durably records uncertain history before dispatch. That receipt is not
an effect capability. Immediately before every nonblocking submit/EAGAIN retry it
checks current authority and owning-client EOF. Provider polling observes facts;
it never queues or flushes a command. A successful app reply retains the stamp so
subsequent revoke/EOF/death/expiry still retires the worker. The fixed RPC response
grace does not extend the effect or captured Source deadline.

The [native camera supervisor](../native-camera-service/README.md) retains Child
and pidfd before fallible setup, creates/fsyncs a root-protected attempt marker,
drops groups/UID/GID and rearms parent-death detection. Bootstrap performs no
Binder lookup, registration or submission. One accepted command may report one
bounded observation; callback progress remains in the child. Publication fencing
precedes signal delivery, and occupied capacity ends only after actual wait.
The marker survives healthy shutdown, crash and reaping; no implicit reopen or
reset follows. Pre-dispatch cancellation stops the ready worker even when its
channel is full and no capture ID has reached the provider.

Two genuine integration REDs precede the final implementation. The unchanged
actor denied a real current UID11020 typed request while the fresh Binder peer
count stayed0 rather than1. Later, the actual retained Supervisor SEQPACKET was
filled to EAGAIN with its ready worker pidfd-stopped. Grant revocation removed the
pending stamp but left that worker occupied/alive. The phase-aware stop fixes this
second defect without an output queue, deferred flush or implicit retry.

Seventeen actual Arch ARM7.1.6 kernel groups pass through real Binder. Independent
permission/bootstrap controls yield zero effects; current Capture permission
produces exactly one registration/submission and a bounded Accepted observation,
with returned/completed/fence-ready false. Eight held-Binder lifecycle cases,
three actual-channel EAGAIN cancellation cases, original expiry before lookup,
peer death and post-UID-drop parent death qualify fencing and actual reclamation.
The remote held count stays1 after local reaping; this proves no remote cleanup.
Separate FD/core-owner smoke uses public frozen CameraV4 and synthetic pipes.

Root independently verifies198 GREEN bindings,80 EAGAIN RED bindings and the
earlier95 missing-route bindings. Both pre-boot QEMU setup failures remain failed.
The final Docker/QEMU waits return0, the canonical shared input lock spans wait
and posthash, and all kernel/initrd/backing hashes remain unchanged. Eight existing
host lifecycle/file/pidfd regressions passed separately. See
[provenance](evidence/native-camera-service-provenance.json) and
[fixture](../tools/native-camera-service-boundary/README.md).

This is a headless public Rust/Rust Binder service composition with synthetic Root
foreground/lock controls and no Source provider. The ordinary installed daemon
still needs a qualified actual backend/invocation; the fixture is not a production
transport. Earlier original-NDK camera evidence is a separate class. No vendor
runtime, image, hardware capture or physical phone is exercised.

Installed Camera app/Source/display composition, media/storage publication,
stronger worker containment, durable qualified reset/reopen, factory interface
and ABI, FMQ/multi-stream/gralloc/DMA/sync_file, preview/photo/video, privacy
indicators, Pixel SELinux/kernel/firmware and later hardware qualification remain
required. Neither a receipt nor a pipe fence enables image publication.
