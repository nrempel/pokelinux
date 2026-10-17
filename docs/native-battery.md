# Native battery observations

The new Rust battery service observes capacity and charging status through Linux
`power_supply`, independently of the suspend authority and charging policy.
Unavailable, malformed, removed or expired input clears the percentage and status.
Device-neutral view types live in `native-phone-state`; the selected supply name
belongs to deployment. No Android service, Java runtime or vendor interface is
required by this backend. Pixel attribute availability remains unqualified.

Linux defines capacity in percent from0to100 and allows drivers to omit
unsupported properties. The backend reads the selected supply's bounded `uevent`,
requires Battery type, capacity and a recognized status, and honors `present=0`
when present is exported. Other attributes are ignored without converting their
units or deriving estimates. This is one sampled observation, not an atomic
hardware transaction. See the [Linux power-supply API](https://docs.kernel.org/power/power_supply_class.html).

## Service and publication

Main owns one reader process, its bounded nonblocking pipe, original collection
deadline, observation view and readonly socket. Atomic nonblocking pipe creation
precedes spawn. All attribute I/O runs in that child. Main requires actual child
wait and pipe EOF before considering the result; it admits no successor while
that reader is retained. Collection expiry withdraws publication before kill,
and a late buffered success cannot reactivate it. Readonly measurement can resume
after the old reader is reaped; this grants no retry authority to radio or other
mutable vendor operations.

Collection deadlines use CLOCK_BOOTTIME and expire at1s. A successful observation
expires2s from collection start, rather than receipt. Normal idle collection runs
at1Hz; listener/reader readiness wakes Main, without a5ms idle polling loop.
Snapshot has no request or mutation operation: connect, receive one bounded JSON
line, then EOF. Coarse battery state is explicitly public. It never grants an app
permission or controls charging, suspend, wake, thermal or performance policy.

SIGTERM withdraws state and drains the retained reader before normal Main exit.
An uninterruptible kernel read may prevent prompt reaping; custody stays retained
and the service manager's shutdown bound exposes that failure. The
[unit](../native-battery/packaging/pokelinux-battery.service) uses root0:0, a
root-owned runtime directory, control-group shutdown and no automatic restart.
Its configuration selects one supply under `/sys/class/power_supply`. Trusted
local executable setup, default SIGCHLD disposition and service-manager custody
are deployment requirements. Permanently blocked exec setup is not tested here.

## Actual component RED and GREEN

The frozen compilable baseline fails five assertions: valid filesystem readings,
observation admission, real service values and the two real reader controls.
The malformed-input negative already passes. The identical final tests then pass
all six groups with the implementation, formatting, strict Clippy and fresh
optimized host/ARM64 release builds. Root verifies all12 selected source inputs
against the working tree. Earlier preparation refusals and intermediate results
remain separately recorded in the [hash-only provenance](evidence/native-battery-provenance.json).

The service tests execute real Unix sockets, children and FIFOs. One blocked
reader leaves snapshots responsive and is actually reaped on Main SIGTERM. In
another test, Main is stopped while its reader successfully writes99percent and
exits0 into the retained pipe. Main resumes after the original deadline, rejects
that actual queued success and later observes42percent through a new reader.
Source loss clears a previously observed percentage in the same Main process.
Controlled clock samples separately test suspend-sized expiry and token fencing;
they do not simulate physical suspend or kernel wake delivery.

Each final host RED/GREEN envelope closes10 actual CLI waits and20 pipe EOFs,
with exact container absence, the same daemon and unchanged selected inputs.
ARM64 is a crossbuild only. Installed PID1 startup, a real kernel battery driver,
normal Qt presentation, physical Pixel readings, charging, suspend and drain
remain separate integration/hardware gates. No physical phone is accessed.
