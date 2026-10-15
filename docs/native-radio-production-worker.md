# Production radio worker tracer

The production `radio-worker` executable now takes a cooperative parent bootstrap,
an explicit Binder device/service and an absolute BOOTTIME session deadline. It
resolves that service, owns one registration and its death watch, and publishes
bounded historical transport observations to its supervisor. This remains the
Samsung radioExternal backend; shared phone services must translate its vendor
metadata into separately qualified domain state.

## Actual RED and repair

The first ARM run reaches the actual production worker's Active registration 42.
The guest independently binds the four selected executables, worker parent,
UID/GID1000 with no supplementary groups or effective capabilities, one named
`IServiceManager.checkService` lookup, one registration and one successful public
peer callback submission. Only then does it release the RAW observation oracle.
The baseline never consumes that callback: RAW remains absent for the original 1s
window. The production Supervisor revokes and kills the worker, actually waits
SIGKILL 9 inside its original cleanup deadline, rejects old output and refuses
same-owner/recreated-owner retries from the unchanged durable journal. Client
wait 1 is the intended new-capability RED; the still-live peer is then separately
killed and waited by its guest owner. Submission alone proves no receipt.

The repair changes only `native-radio/src/bin/radio-worker.rs`: after a successful
Active write, consume at most one ready callback per turn and commit it through
`TransportSession` and the existing `RadioOwner` gate. The same client, peer and
manager ELF bytes, and the unchanged outer oracle, now receive exactly the
expected raw metadata/payload. The peer then exits through its one test control;
the retained death watch publishes `Failed(ServiceDied)`. Worker Stop again has
actual wait 9 and durable retry refusal; client and peer actual waits are 0.

Both runs close all guest direct waits, actual QEMU/Docker CLI wait 0, fresh exact
container absence and frozen inputs. Root independently rehashes the selected
source/runtime bindings. All 41 host library tests and strict checks pass; all
four executables compile for ARM64. Two earlier import/lint build failures execute
no tests and are preserved separately from behavioral RED. See the bounded
[source and execution provenance](evidence/native-radio-production-worker-provenance.json).

## Worker and observation contract

`Supervisor::start_worker` launches `WorkerCommand` on its owning main thread.
The selected child accepts `--device`, `--service` and `--until-ns`; there is no
implicit endpoint, context-manager fixture shortcut, automatic retry or clear.
`BoundParent` completes before Binder setup. The current public service-manager
method resolves the radio object inside that process; a Binder handle is not
transferred through the bootstrap Unix socket. `RadioTransport` checks that it
is remote and has the selected backend descriptor. These checks do not qualify
an installed device service's identity, interface version or permission policy.

Stdout must be a nonblocking FIFO. Each binary frame is at most 2116 bytes and is
written once, below Linux PIPE_BUF 4096. EAGAIN/INTR causes a fresh snapshot next
turn; there is no detached writer, partial-frame outbox or unbounded queue.
The stream carries incarnation, attempt 1 and revision, followed by Active,
Raw or Failed data. A reader must bind the actual owned source connection,
check current Supervisor admission and increasing revision, bound its incremental
buffer, and reject malformed/truncated frames. The actual reader in this
qualification is test infrastructure around the production Supervisor primitive;
installed observation belongs to the later [service selection](native-radio-service.md).

These are historical latest-snapshot observations. Multiple raw revisions can
coalesce under pressure; receipt does not establish current peer liveness,
lossless callback delivery, IMS registration, decoded SMS, call state or operation
success. Declared vendor length and actual payload length are preserved as
separate fields. Indications/segmented responses remain unsupported by this raw
publication path. Unsupported events release their preparation; terminal reasons
stay terminal. The source uses one bounded callback turn and existing BOOTTIME
gates; actual output pressure and suspend are not exercised in this selection.

The session retains its watch and any blocked registration handle through terminal
states. It never detaches a worker or drops a possibly blocking watch to finish
Stop. The parent terminates and actually reaps the process. Fatal worker setup/IO
errors exit without running those ownership destructors; the parent still owes
actual wait. Reaping never clears the durable admission marker or proves remote
vendor cleanup. Service-manager containment and original cleanup/reset remain
required before any successor registration can be authorized.

## Evidence and remaining work

This executes public Rust manager/peer fixtures on an actual generic Arch ARM
kernel, with the unchanged production worker. It adds no Android/vendor runtime,
original-code execution, private REA inputs, UI, authentication or phone access.
Earlier emulator, original-code, ARM user-mode and Google-kernel results retain
their separately documented scopes. Zero physical phone models are qualified.

A later [installed service selection](native-radio-service.md) connects the
production supervisor and status client through the actual packaged unit. Its
separate RED/repair, peer-loss withdrawal, native wait, Main exit0 and restart
refusal do not change this earlier four-executable evidence. Next qualify the
exact installed worker under blocked lookup and output backpressure. The later
[held-registration control](native-radio-service-blocked.md) separately qualifies
normal installed status/Stop with the registration call permanently held.
Production discovery identity and permissions, recovered producer semantics,
requests/ACK/reset, native IMS/calls/SMS, actual suspend and Pixel kernel policy
remain gates. The full smartphone matrix also still requires device graphics,
camera, audio, modem/carrier, power and the other hardware tests. Keep Google's
matched kernel/modules/firmware unchanged during this userspace work.
