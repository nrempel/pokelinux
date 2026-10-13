# Native camera process owner

This Rust library supplies one owned camera backend process to the existing
`native-lifecycle` actor. The shared contract uses the camera core's generation
and capture ID, an original BOOTTIME deadline, and bounded state. Device adapters
own their hardware interfaces and buffer policy.

`Supervisor::spawn` retains the actual `Child` and pidfd. The trusted worker
creates and syncs a root-protected attempt marker, closes its writable marker
descriptor, drops to the configured unprivileged UID, rearms its parent-death
signal, and verifies the actual parent pidfd race. Bootstrap performs no Binder
lookup, callback registration, or submission. The current supported backend UID
range follows the lifecycle installation range, 10000 through 60000.

Only the actor calls nonblocking `submit` after its final current-authority check.
WouldBlock sends no command and creates no output queue. `poll` observes readiness,
one command state, terminal facts, and actual child waits. The child checks the
original deadline before each adapter effect. Revocation and terminal facts fence
publication before signal delivery; occupancy ends only after actual parent wait.

An accepted receipt, returned buffer, capture completion, and fence readiness are
separate observations. The application receives no frame, FD, image-success claim,
reuse permission, or reset capability. Callback progress stays in the child. The
attempt marker survives EOF, death, reaping, and healthy shutdown; there is no
restart method or implicit marker removal.

The authored `tools/native-camera-service-boundary` fixture passed 17 actual ARM
kernel Binder groups after preserving the unsupported-route RED and an actual
Supervisor-socket EAGAIN cancellation RED. It uses public generated CameraV4
interfaces, synthetic pipe buffers/fences, real app PID/UID/pidfd binding, and
headless root lifecycle controls. It establishes no original NDK, Source,
vendor, media-publication, physical DMA/sync-file, or phone evidence. Installed
service/app composition and stronger worker containment remain separate work.
