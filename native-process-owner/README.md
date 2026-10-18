# Native process owner

Shared single-attempt Linux process custody used by the radio worker and its
supervisor. It contains the extracted `bootstrap`, `supervisor` and `deadline`
modules; radio and fixture callers import this crate directly, without aliases
for the former radio-local modules.

- `WorkerCommand` prepends `--pokelinux-process-worker` to a trusted executable.
  `BoundParent::from_stdin` validates the retained parent/socket identity,
  credentials, original deadline and READY/GO exchange before Binder startup.
- `Supervisor::start_worker`, `poll_worker`, `stop` and `poll_reap` keep one child
  occupied through actual wait and stdout EOF. Actual wait revokes publication
  immediately; `poll_reap` caches that status and returns it only after EOF.
  Each poll discards at most4096 stale bytes without blocking. Empty reads,
  `Interrupted` and `WouldBlock` do not establish EOF. Stop revokes local
  admission first and never signals a child whose wait is already cached. The
  mode-0600 journal remains across failure, Stop, wait and owner recreation;
  `StartError::AdmissionUncertain` forbids another admission.
- `send_message`/`receive_message` reuse the authenticated retained pair after
  GO with1..8192 copied bytes, exact live credentials/pidfds and fresh original
  application BOOTTIME deadlines. No raw handle, queue or resend is exposed.
  The [real host channel consumer](../docs/native-process-channel.md) passes five
  groups, including actual rights overflow, EAGAIN and Stop-before-wait fencing.
- `ProcessIdentity::{from_bytes,as_bytes,from_hex,hex}` are explicit codecs.
  Decoding an identity grants no custody or publication authority: admission
  also binds the current source connection and attempt.
- `BootClock` and `Deadline` use absolute Linux CLOCK_BOOTTIME. This includes
  suspend in expiry calculations and does not provide wake delivery.

Cooperative launch requires the process main thread, equal real/effective
UID/GID, trusted executable/procfs provisioning and a common PID namespace. The
non-Send/non-Sync supervisor must remain on the spawning thread until actual
child wait and stdout EOF. Dropping an unreaped child kills then aborts;
dropping a waited child without witnessed stdout EOF also aborts. Neither path
detaches or waits in Drop. Deployment still needs service-manager supervision and current
permission/containment policy.

The [shared custody regression](../docs/native-process-custody.md) passes eight
shared and 59 radio host tests (two radio tests ignored), then the existing
production radio-worker oracle through actual generic ARM Binder. This new
selection qualifies normal lookup/RAW/death/Stop/wait/refusal; it does not rerun
the earlier parent-death or four bootstrap-policy controls. Local process death
and wait prove no vendor cleanup and authorize no retry. The later Data
selection below is separately qualified; APN acquisition, normal deployment and
physical hardware remain separate work.

## Later Data consumer

A host actual failed-spawn admission-receipt RED→GREEN passes nine shared tests
and exposes readonly metadata for returned file/directory sync before spawn.
This does not simulate power loss or clear the admission fence. The historical
eight-shared/59-radio proof above remains byte-for-byte separate.

The [Data service tracer](../docs/native-cellular-service.md) then runs the three unchanged final-core
production executables through actual generic ARM Binder and an independently
controlled TUN. Main actually waits its worker9 before guest-owned Main0, and
unchanged-journal restart78 starts no new worker/setup. All34 guest direct
waits/EOF are separately counted from that nested native wait. No Binder pool
join, remote provider cleanup, safe successor operation, permission/APN grant
or normal systemd installation is inferred; manager fallback remains untested.

## Initial SIT attempt consumer

The [initial SIT attempt gate](../docs/native-sit-attempt.md) consumes source-
generation identity while actual custody stays here. Its explicit Unix-record
fixture binds the worker through parent bootstrap, accepted peer credentials and
the retained supervisor. Compiled13-pass/two-missing-completion baseline plus
actual host/ARM kernel RED precede15host/15ARM user-QEMU GREEN and identical
host/ARM process completion/expiry/cancel passes. The40 codec tests remain
separately qualified unchanged prerequisites.

Stop revokes admission and retains the child through actual signal9 wait. The
durable `AdmissionUncertain` fence survives and blocks a successor. Attempt
snapshots and identity codecs clear no journal and establish no vendor cleanup,
safe retry, provider/APN readiness or CPIF/physical-phone compatibility.

## Later stdout EOF custody repair

A separate host real-pipe regression first fails101: a child exits after
transferring its stdout writer to the parent, and the old owner clears capacity
before EOF. The unchanged test then passes with the retained exit-status/EOF
repair, alongside the nine existing Supervisor unit controls, production-file
formatting and strict all-target Clippy. Its empty-read/EAGAIN controls and
caller-observed EOF exercise actual pipe behavior; its native wait is witnessed
independently with `waitid(WNOWAIT)` returning `ECHILD`.

[Summarized proof](../docs/evidence/native-process-stdout-eof-provenance.json).
This host selection does not rerun the historical Radio, Data, channel, SIT or
ARM consumers above. Local wait and EOF establish neither remote cleanup nor a
safe successor, and the durable admission marker remains unchanged.
