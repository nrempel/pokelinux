# Rust first-Stop observations

The normal compositor reaches STOPPING before its startup deadline. This change
records the first observed Stop path so the next actual startup run can identify
its cause. It does not make startup succeed or change authority decisions.

## Ownership and migration

Rust owns the fixed first record, immutable update rules, saturating call count,
clock observation and creating-thread identity. The independent CPU-only handle
is allocated before broker construction. It retains no executor, native object,
file descriptor, callback, queue or worker. Foreign-thread refusals preserve
output bytes and use a cached, nonreused Rust ThreadId; C++ thread identities are
not used as ownership proof.

The C++ adapter copies scalar facts at existing native Stop branches. Terminal
latching, readiness clearing and the existing Rust Stop happen before recording.
Direct revoke session/reason arguments are copied before the callback returns.
Broker fields and CLOCK_BOOTTIME are post-authority observations; the timestamp
is not a transition timestamp. A missing pending head does not prove an empty
queue. Observer failure publishes unavailable fields without suppressing Stop.

`stop_requests` counts valid recording calls, including terminal reobservations
from snapshot polling. It does not count distinct revocations or user actions.
The first record is never replaced. The public wire contract uses `first_stop`
and `stop_requests`; the frozen sources and incremental native adapter patch are
in the [source package](../tools/native-render-owner/first-stop/README.md).

Substantial Hyprland, Aquamarine and Qt C++ remains in the current system. The
Rust migration is unfinished. New event ordering, lifecycle and scheduling state
belongs in Rust; native adapters translate facts and perform worker-local calls.
This diagnostic is not a reason to add a general C++ control framework.

## Actual red-green evidence

The actual normal-compositor wire RED authenticates two public socket replies:
STARTING/stage8 then clean STOPPING/stage5, before any public Stop and before the
20s startup deadline. It fails precisely because the two observation fields are
missing. Its28 direct waits and35 PIPE EOFs close; separate QMP/public socket EOFs
are retained separately. Process disposal is not native retirement or Rust join.

A separate actual x86 KVM compiler RED uses all three genuine original public
Rust headers. The unchanged control exits0 with zero diagnostics; the target
fails on exactly the four absent first-Stop function declarations. No future
header or substitute declaration is admitted.

After those REDs, the original Rust implementation passes formatting, strict
Clippy, all64 active Rust tests and Release compilation. The original62 passing
names are preserved; two authored child modes remain explicitly ignored in the
parent suite. Two new private tests cover counter saturation, immutable first
facts, unavailable clock and invalid-input atomicity.

Four actual compiled C++ consumers link the same new Rust archive: snapshot5,
lifecycle, executor5 and first-Stop5 groups pass. The last includes two actual
joined foreign threads and refusal even with observed C++ identity reuse. The
existing snapshot producer has an actual wait and EOF, and executor checks retain
four post-join task-absence observations. These are real CPU/FFI integration
checks in disposable host containers; they do not execute the native adapter.
Both container CLIs exit0 and both exact container IDs are absent, with unchanged
source snapshots and all10 direct waits/PIPE EOFs closed.

The new Rust archive is22,903,680 bytes, SHA256
`0fdc60b68c02d1e94bd2f3543b56627e683b41d2b592bb3ff16bd5869000e8a9`.
Its executor header is13,386 bytes, SHA256
`338dd308fda306a3e0774800bf3ea852119fb938dc38143df2d4829a161855ec`.
The earlier normal-profile archive remains earlier evidence, not this provider.

## Remaining integration gate

The matching normal compositor and535-regular/two-link SDK now pass in one
actual x86 KVM build. All458 configured C++/PCH entries omit the test definition;
the exact main exports27 Rust functions and selects the normal AQ15 library.
Six strict SDK API checks, the15 existing CPU groups, published texture5 and
first-Stop5 pass against the same fresh Rust provider. All78 distinct direct
waits/PIPE EOFs close, including the early mask receipt once; the embedded
snapshot producer wait/two EOFs and QMP socket EOF remain separate. Main is built
and loader-checked, without being invoked by this build.

The unchanged actual wire GREEN now passes in a fresh actual normal-compositor
run. Five kernel-peer-authenticated EOF-framed replies total3,455 bytes; the
maximum observed request takes10,590,931ns. Original28 direct waits/35 PIPE EOFs
close, plus separate QMP and five public-response EOFs. The full owned outer
closure passes; this is first-Stop observability, not normal startup success.

The real first origin is `AQ_REVOKE` (3), with the actual callback reason
`AQ_REVOKE_DEVICE_CHANGE` (5) and matching session generation. Its post-authority
clock observation is116,433,680ns after that generation, before the20s deadline.
The first record survives a later actual public Stop and two snapshots; the
recording-call counter advances4→6→7→8. The copied broker reports no transport
failure and no observed pending head. The callback runs before enqueue, so this
is no proof of an empty queue. The particular device and HOTPLUG versus LEASE
subtype remain unobserved.

The first GREEN attempt remains a failed setup run, before main/case launch.
Loader checks and the seatd spawn complete; the controller requires GID1000
immediately after observing the socket. The corrected fixture waits for both
socket and requested group under the same original5s cutoff, retaining the final
GID assertion. It changes no permissions, production source, frozen target or
bounds. The readiness-race explanation is an inference: initial GID was not
retained. Upstream seatd creates/listens before setting ownership; see the
[primary source](https://github.com/kennylevinsen/seatd/blob/0.9.3/seatd/seatd.c).
The failed attempt's27 waits/34 PIPE EOFs, QEMU0/CLI1, exact-container absence and
unchanged inputs remain separate. This fixture correction is not a production
bug-fix TDD claim.

The next substantive gate is Rust-owned recoverable device-change handling:
immediate authority invalidation and suspended discovery/frame admission, bounded
ordered copied-event custody, worker-local application and fresh discovery
accepted only through the latest reconciled change. Invalidation alone must not
permit stale discovery to establish Ready. Session and security generations are
distinct identities. Disable, REMOVE and transport failure remain conservative;
already-sealed attempts retain their completion custody.

Normal readiness, real client frames, output presentation, native retirement,
Source7 acceptance, new ARM execution and physical-phone gates remain open. The
main is disposed by actual SIGKILL/wait-9, which proves no native join or cleanup.
No phone was accessed.
