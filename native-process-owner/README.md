# Native process owner

Shared single-attempt Linux process custody used by the radio worker and its
supervisor. It contains the extracted `bootstrap`, `supervisor` and `deadline`
modules; radio and fixture callers import this crate directly, without aliases
for the former radio-local modules.

- `WorkerCommand` prepends `--pokelinux-process-worker` to a trusted executable.
  `BoundParent::from_stdin` validates the retained parent/socket identity,
  credentials, original deadline and READY/GO exchange before Binder startup.
- `Supervisor::start_worker`, `poll_worker`, `stop` and `poll_reap` keep one child
  occupied through its actual wait. Stop revokes local admission first. The
  mode-0600 journal remains across failure, Stop, wait and owner recreation;
  `StartError::AdmissionUncertain` forbids another admission.
- `ProcessIdentity::{from_bytes,as_bytes,from_hex,hex}` are explicit codecs.
  Decoding an identity grants no custody or publication authority: admission
  also binds the current source connection and attempt.
- `BootClock` and `Deadline` use absolute Linux CLOCK_BOOTTIME. This includes
  suspend in expiry calculations and does not provide wake delivery.

Cooperative launch requires the process main thread, equal real/effective
UID/GID, trusted executable/procfs provisioning and a common PID namespace. The
non-Send/non-Sync supervisor must remain on the spawning thread until actual
child wait. Dropping an unreaped child kills then aborts; it does not detach or
wait in Drop. Deployment still needs service-manager supervision and current
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
