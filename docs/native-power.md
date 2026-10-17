# Native power request arbitration

`native-power` implements the first portable Rust power-policy component. It
owns bounded requests and application bookkeeping; it is not a deployed power
service, suspend authority or qualified Pixel backend. No physical phone,
vendor target, host suspend control or sysfs power node was accessed.

## Request ownership and application

A `ResourcePolicy` contains a logical resource, ordered logical values and a
default index. Earlier requested values win; the default participates only when
no active lease remains. The shared component rejects path-shaped identifiers.
Pixel node names, paths, firmware policy, properties and value encoding belong
to a separate backend profile. The current fixtures use only synthetic values.

The service assigns fresh `OwnerId` identities after authenticating clients;
these are not bare PIDs or client-selected identities. The crate does not yet
implement authentication or a service transport. Each acquisition returns an
opaque lease scoped to that component instance. Only its owner can renew or
cancel it. Renewal extends the existing deadline and never shortens it;
`UntilCancelled` requires explicit cancellation. Disconnect/revocation cleanup
removes all requests for the authenticated owner. Limits are 32 resources,
32 values per resource, 1,024 leases total and 64 leases per owner.

`BootClock` samples Linux `CLOCK_BOOTTIME`. Native policy deadlines therefore
include time spent suspended. Callers must use one trusted clock, advance the
component after resume, and drive their event loop using `next_deadline()`.
Backward samples and deadline overflow are rejected. The component creates no
timer and does not wake hardware. A controlled clock test advances boot time
while awake time stays constant; it does not suspend the host.

Desired state is separate from applied state. Startup, backend loss, in-flight
application and failed application report `Unknown`. `last_confirmed` is
historical successful application in the current backend generation. It is not
physical readback. One application may be outstanding per logical resource;
completion names the issued value even if cancellation or expiry changed the
current desired value. The next application reconciles the current intent.
Duplicate, replaced-backend and previous-instance completions are rejected.
Failure preserves history and waits for the configured retry delay; an
overflowing retry deadline blocks retry until explicit backend recovery.

The caller still owns the actual backend IO. Tokens fence bookkeeping and
callbacks, not a write already issued to a device. A deployed adapter must
serialize actual mutable resources, revalidate intent before dispatch, handle
partial application, and stop/reap old backend work before replacement. This
crate starts no worker or child process and supplies no process-death proof.

## Original source comparison

The reference tool fetches only six public source/header files at immutable
Pixel repository commit `8612656e216075f79a1efa3bb9be6b6943d6e8cc`, checks their
SHA-256 identities and compiles unmodified `RequestGroup.cc`, `Node.cc` and
`FileNode.cc`. Our small Android-base shims use real POSIX temporary-file IO;
properties return their fallback and tracing/logging are simplified. The
source retains its license and stays in ignored artifacts. Published files are
our harness, fetch manifest and synthetic outcome fixture.

The Rust comparison matches 18 original-source outcomes for renewal, expiry,
indefinite lifetime, owner cancellation, ordered priority, default restoration,
invalid values and 500 ms failed-write retry. The original source also confirms
two descriptor-lifetime outcomes. Those two are marked `reference-only`; the
Rust arbitration component does not implement a device file-descriptor owner.
Both implementations exercise real bounded temporary files for application,
failure, retry and default restoration.

There are deliberate differences. Public `RequestGroup` uses `steady_clock`
and rounds remaining time to milliseconds; native policy uses boot time and
full `Duration` comparisons. Native acquisition rejects an already elapsed
expiry, while the reference accepts and then prunes it. Native renewal reports
whether a deadline extended; public `AddRequest` reports whether an entry was
new. Native startup does not assume a known default, and native failed or
in-flight application reports uncertainty. These are portable policy decisions,
not backward compatibility or proof of factory-binary equivalence. The factory
libperfmgr audit found a newer queued worker and additional node behavior;
Pixel adaptation still needs its exact contract.

## Validation and reproduction

Two strict runtime red phases were observed: the initial default-only baseline
compiled but failed 11 of 14 tests; after priority arbitration the same tests
passed. A later test exposed stale confirmation during dispatch, failed at
runtime, and passed after dispatch marked current application unknown.

Current validation: formatting and clippy with warnings denied pass; 17
arbitration tests plus one 18-outcome original-source comparison pass on native
x86 Linux and on ARM64 user-mode QEMU. Tests cover owner/resource cleanup,
exact expiry, suspend-aware clock injection, real Linux boot-clock sampling,
clock/deadline errors, capacity, stale instance/backend tokens, completion after
cancellation, in-flight uncertainty, retry scheduling and real temporary-file
failure/recovery. Shell and Python tool syntax checks pass. See
[evidence/native-power-provenance.json](evidence/native-power-provenance.json).

On the prepared Harbour images:

```sh
./tools/build-power-reference.sh fetch
./tools/build-power-reference.sh build
./tools/build-native-power.sh lock
./tools/build-native-power.sh build
```

Only `fetch` uses the network. Reference source, compilation products and logs
stay below ignored `artifacts/native-power/`. ARM64 tests use user-mode QEMU
with host-kernel syscalls; they do not establish an ARM64 kernel, Binder, process
supervisor or physical-device result.

## Remaining integration

A native service transport, authenticated owner lifecycle, durable/restart
policy, actual backend ownership and cancellation, exact Power V7 Binder glue,
ADPF sessions, thermal/charging policy and vendor libpower participation are
still required. The one suspend authority must account for kernel wake sources,
the sensor FMQ acknowledgement obligation and the separate USF wake lock.
Stock boot/property and optional/reset node behavior need explicit backend
qualification. The separate [native battery observation service](native-battery.md)
now passes seven filesystem/socket/process groups, host/ARM crossbuilds and
actual virtual-kernel driver observations with installed PID1 startup and fresh
automatic cold boot. Its readonly Rust client and normal Qt build also pass;
a prepared normal Messages window now visibly follows all three states, with
original Pixel10-profile captures. Physical battery integration remains open. Display/brightness, actual
suspend/wakeup, charging, thermal measurements and drain remain separate
implementation and later authorized hardware gates. This component supplies no production or
hardware completion claim.
