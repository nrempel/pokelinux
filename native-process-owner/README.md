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
and wait prove no vendor cleanup and authorize no retry. A supervised Data
worker/service, APN acquisition and physical hardware remain separate work.
