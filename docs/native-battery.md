# Native battery observations

The Rust battery service observes capacity and charging status through Linux
`power_supply`, independently of the suspend authority and charging policy.
Unavailable, malformed, removed or expired input clears the percentage and status.
Device-neutral view types live in `native-phone-state`; the selected supply name
belongs to deployment. No Android service, Java runtime or vendor interface is
required by this backend. Pixel attribute availability remains unqualified.

Linux defines capacity in percent from 0 to 100 and allows drivers to omit
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

Collection deadlines use CLOCK_BOOTTIME and expire at 1 s. A successful observation
expires 2 s from collection start, rather than receipt. Normal idle collection runs
at 1 Hz; listener/reader readiness wakes Main, without a 5 ms idle polling loop.
Snapshot has no request or mutation operation: connect, receive one bounded JSON
line, then EOF. Coarse battery state is explicitly public. It never grants an app
permission or controls charging, suspend, wake, thermal or performance policy.

SIGTERM withdraws state and drains the retained reader before normal Main exit.
An uninterruptible kernel read may prevent prompt reaping; custody stays retained
and the service manager's shutdown bound exposes that failure. The
[unit](../native-battery/packaging/pokelinux-battery.service) uses root 0:0, a
root-owned runtime directory, control-group shutdown and no automatic restart.
Its configuration selects one supply under `/sys/class/power_supply`. Trusted
local executable setup, default SIGCHLD disposition and service-manager custody
are deployment requirements. Permanently blocked exec setup is not tested here.

## Component RED and GREEN

The original compilable stub fails five assertions; its malformed-input negative
already passes. The implementation then passes the same six tests. That
[historical six-test provenance](evidence/native-battery-provenance.json) remains
unchanged and does not qualify the later kernel integration.

A real kernel trial exposed a parser defect: `test_power` repeats the identical
`POWER_SUPPLY_TYPE=Battery` record. The old production reader returned unavailable
for both actual sysfs and a byte-identical regular-file copy. A new regression ran
against that production parser before repair: the original six tests passed and
only the repeated-TYPE positive failed. The repair accepts identical TYPE records
and still rejects conflicting types and repeated capacity/status fields. All
seven tests then pass, along with format checks, strict Clippy and fresh optimized
host/ARM64 Release builds. The [current hash-only provenance](evidence/native-battery-kernel-provenance.json)
binds the exact twelve-source cohort, tests, providers and closed results.

The service tests use real Unix sockets, children and FIFOs. A blocked reader
leaves snapshots responsive and is reaped on Main SIGTERM. Another test stops Main
while its reader writes 99% into the retained pipe and reaches zombie state
with raw exit status 0. Main resumes after the original deadline, rejects the queued success and
later observes 42% through a new reader. Source loss clears the previous
percentage in the same Main. Controlled absolute-clock samples separately test
suspend-sized expiry and token fencing; they do not execute physical suspend.
Each final component RED/GREEN envelope closes 10 actual CLI waits and 20 PIPE EOFs,
with exact container absence, the same daemon and unchanged selected inputs.

## Installed kernel and PID1 integration

Two separate x86_64 KVM boots execute the public Arch 7.2.9 kernel's synthetic
`test_power` driver with the repaired production service. Preparation installs
exactly the service ELF, unit and explicit `test_battery` configuration through
the existing Make target. A later cold boot uses that retained image: no install,
reload, enable or manual service start occurs before the probe, and the unit has
a fresh invocation. Both use the same frozen
[`battery-probe.py`](../tools/native-installed-services/battery-probe.py) and public
owned-process helper.

Both boots observe 73%/charging, then 51%/discharging, then absent with
null percentage/status through credentialed sockets from the same Main. The
kernel `uevent` independently confirms each state. The probe retains an actual
reader, observes normal PID1 Main exit 0 on Stop, and requires empty cgroups, dead
Main/reader pidfds, process absence and socket removal. QEMU shutdown, container
absence and selected-input closure are recorded separately. Across the two
successful VM epochs, 47 in-guest waits/47 merged PIPE EOFs and 14 host CLI waits/28 PIPE
EOFs close; the separate host Make stage closes 3 waits/3 merged PIPE EOFs. QMP
socket EOF is counted separately. PID1 service outcomes are not described as
guest-parent waits.

The original image first failed the missing-unit requirement. The first installed
trial never obtained a valid reading; a diagnostic exposed the duplicate TYPE. After its repair, a
fixture incorrectly required the boolean module getter to echo numeric 0 and also
changed selected Python bytecode: its original input-closure result remains false.
A separate post-check found the bytecode-only difference without upgrading that
run. The corrected frozen probe uses `true`/`false`, records the raw getter, and
still requires kernel PRESENT=0 plus unavailable/null service state; it disables
bytecode generation. Preparation and cold boot both pass this corrected probe.

These are headless, fixture-controlled kernel observations and installed-service
lifecycle evidence. ARM64 is a crossbuild only here. Normal Qt presentation,
physical Pixel attributes, charging, suspend, wake and drain remain separate
gates. No physical phone is accessed.
