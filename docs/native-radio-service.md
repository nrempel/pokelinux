# Installed native radio service

The packaged `radio-service` now owns the production `radio-worker` and serves a
restricted local `radio-status` client. An actual installed systemd unit on a
generic ARM kernel completes named Binder discovery, one registration, raw
callback receipt, status publication, peer-loss withdrawal, normal Stop and
same-journal restart refusal. This is the Samsung radioExternal transport backend
with public synthetic inputs. It does not provide native IMS, cellular state or
a verified phone deployment.

## Integration RED and repair

The baseline reaches the installed unit's actual Main process and worker. The
guest independently checks their selected ELF bytes, argv, parent relationship,
UID/GID1500, effective capabilities and actual unit cgroup membership. The original
unit/sysusers bytes are installed, with only a private `ExecStopPost` outcome
observer added. The named public manager sees one worker lookup; the peer sees
one registration and successfully submits one authored raw callback. The native
service decodes that frame and emits its first metadata-only receipt. A real
status client must then complete a valid active response before the RAW oracle.

Only after those prerequisites does the original 1s oracle run. The baseline's
status remains `last_raw:null`, including a post-cutoff negative snapshot, while
the worker and peer stay live. Normal Stop has actual native worker SIGKILL wait,
systemd Main exit0 and empty/removed unit cgroup. Restart from the unchanged
journal exits78 without another child or registration. The peer remains live
through that refusal and is then separately killed and waited by its guest owner.
This fully closed missing-projection result is the behavioral RED.

The repair adds one assignment in `radio-service.rs`: retain the already admitted
Raw frame in `state.last_raw`. All five other ELF bytes and all five outer oracle
files are identical between the RED and GREEN selections. The same real client
now receives exactly slot0, serial7, message137, declared length1, error0 and
payload `07`. The independent public peer exits through its existing control107;
status becomes `unavailable` with `service-died`, retaining the historical Raw
frame. The native owner actually waits its worker before operator Stop. The unit
again has Main exit0, empty cgroup and same-journal exit78 with no new registration.
The control helper and peer both actually exit0.

Both selections have actual guest child waits, client pipe EOF, QEMU/Docker CLI
wait0, fresh exact container absence, unchanged daemon and frozen-input closure.
All 45 host library tests and strict checks pass; six executables compile for
ARM64. The four new host cases cover bounded terminal history, malformed packet
shapes, actual SEQPACKET truncation/descriptor rejection, and inconsistent history
or live/terminal state. An additional actual client control rejects regular-file
stdout with exit1 and zero bytes, followed by a successful pipe status query;
that is additional coverage, not the RAW behavioral RED. Earlier build-only
baseline1 remains a separate successful source/build checkpoint before the output
sink prerequisite; it supplies no installed runtime evidence.

Exact source, executable and execution bindings are in the
[bounded provenance](evidence/native-radio-service-provenance.json).

## State and local interface

Installation paths, selected device/service/lease and account provisioning are
specified in [packaging](../native-radio/packaging/README.md). The unit requests
private0700 runtime/state directories and a0600 status socket/journal, an empty
capability bounding set, `NoNewPrivileges=yes`, `Restart=no` and
`KillMode=control-group`. This run checks the actual selected credentials,
directories and membership/empty lifecycle; it does not qualify universal
confinement, resource-limit enforcement or manager fallback.

The nonblocking Unix SEQPACKET interface accepts only the service account's
UID/GID. Each packet must carry credentials matching the connected peer's
PID/UID/GID. The client retains the actual socket-origin server pidfd. Commands
are Status and Stop; status is one bounded packet with one optional historical
Raw frame. Unknown shapes, truncation, passed descriptors and credential
mismatches are rejected. This is an operator diagnostic interface, not an app
permission or phone-state API. Cross-UID installed refusal still needs its own
runtime control; source checks and component coverage do not replace it.

| Status field | Meaning |
| --- | --- |
| `phase` | `initializing`, `observed-active`, `unavailable` or `stopping`, according to the owner's latest local observation |
| `incarnation` | The one admitted process identity |
| `revision` | Latest accepted worker revision; local phase changes need not increment it |
| `failure` | First terminal reason, retained through later cleanup |
| `last_raw` | Optional historical metadata/payload; never current carrier state or a final request receipt |

Frames must match the actual owned stream, current admission, incarnation,
attempt1 and increasing worker revision. Revocation clears buffered input and
prevents later frames from restoring availability. Retained history survives a
terminal phase. Raw revisions may coalesce under pressure, so this is not lossless
callback delivery. `observed-active` records transport activation; it does not
independently prove continuing remote liveness or IMS registration.

## Ownership and bounded work

The service's main loop owns the child and never calls Binder. A turn processes
at most four bounded worker read/decode steps, accepts at most two clients, and
retains at most eight client slots. Each slot has one original500ms BOOTTIME
cutoff. Pending status replies are rebuilt after local reap/expiry checks on every
send retry, with no stale encoded outbox. The worker pipe buffer is bounded at
4232 bytes. Service lifecycle logging is one best-effort nonblocking write of at
most1024 bytes; incomplete diagnostics cannot qualify a test.

The selected lease becomes one absolute CLOCK_BOOTTIME deadline, with no renewal.
The client has one3s deadline across connection, request, response and Stop exit
observation. Status writes once to a pipe, socket or actual terminal, with a5120-byte
maximum and no retry queue; short/EAGAIN output fails. Require exit0 and a complete
line because a failed large pipe write can leave a prefix. Regular-file and
block-device stdout are refused. Stop is silent. Pipe output is exercised here;
TTY output and output-pressure behavior remain unqualified.

Retirement revokes admission before killing the worker and starts one original2s
cleanup cutoff. Repeated Stop or errors cannot renew it. The production Supervisor
retains and actually waits its child; an occupied child at cutoff aborts the
service for manager fallback rather than detaching it. Normal service exit waits
for native child closure and pending Stop replies. The configured manager stop
bound is5s, a separate fallback contract that this normal run does not exercise.

The Stop client observes ACK followed by server pidfd exit. That alone cannot
prove the server's exit code, parent wait or successful worker cleanup: an abort
also makes the pidfd readable. This test separately binds the native worker wait
and systemd's Main outcome. The private observer records distinct invocation IDs
and `EXIT_CODE`, `EXIT_STATUS`, `SERVICE_RESULT` from `ExecStopPost`, then checks
Main process absence and the previously bound unit cgroup. These variables report
the identified Main process's exit outcome; the observer performs no independent
reap. See the [systemd execution contract](https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml).

The synced admission journal survives Stop, process death and clean reaping.
A new service refuses that same journal with exit78 before spawning a worker.
There is no reset command, automatic registration retry or vendor-cleanup claim.
Runtime socket removal belongs to the manager's RuntimeDirectory lifecycle and
does not clear persistent uncertainty.

## Remaining gates

This selection uses public Rust peers and an unchanged generic Arch ARM kernel.
It executes no new original vendor/Android runtime, private REA input, UI,
authentication path or physical phone. Prior emulator, original-code, user-mode
ARM and Google-kernel evidence retain their separate scopes. Zero phone models
are qualified.

Next qualify permanently blocked registration in this exact installed service,
then remaining discovery/output pressure, installed principal negatives and
startup filesystem stalls. Synchronous startup metadata/journal fsync can stall;
the event-loop budgets do not make those filesystem operations bounded. Actual
suspend/wake, unkillable tasks and manager fallback remain open.

Production endpoint identity/version/access policy, original producer and decoded
IMS/call/SMS semantics, acknowledged remote cleanup/reset and safe successor
registration remain necessary. Pixel deployment, modem/carrier, graphics, camera,
audio, power and the full smartphone matrix still require hardware evidence.
Keep Google's matched kernel/modules/firmware while replacing userspace.
