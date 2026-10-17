# Owned Linux peer identity

This leaf obtains the exact socket peer reference with `SO_PEERPIDFD` and compares
already-owned pidfds through `ProcessView`. These public operations fail closed;
neither opens a pidfd from an observed numeric PID. Callers can forbid unsafe
code. The pinned rustix1.1.5 dependency has no `SO_PEERPIDFD` getter, so that one
fixed socket operation remains an isolated audited libc boundary.

`socket_peer_pidfd` borrows the connected socket and returns a newly installed,
owned CLOEXEC descriptor. It checks fixed output shape and atomic CLOEXEC;
rejected successful replies release the actual transferred descriptor. A failed
syscall transfers no descriptor. Linux obtains the retained `sk_peer_pid` and
installs only after successful output copies: [Linux6.8 socket
source](https://github.com/torvalds/linux/blob/e8f897f4afef0031fe618a8e94127a0934896aba/net/core/sock.c).
Existing source hashes remain in `public-api-inputs.json`.

`ProcessView::open()` pins an actual procfs root, resolves its kernel-generated
`self` link, checks the current process PID, and opens that process's `fd` and
`fdinfo` directories. Every directory is checked for `PROC_SUPER_MAGIC` and opened
with CLOEXEC. Descriptor children are opened relative to these retained directories;
callers cannot supply a path or an inherited directory. A view used after fork
returns an error before inspecting the parent's descriptor table.

`is_pidfd(&owned)` checks the creator process and pinned kernel type name without
requiring liveness or a positive Pid. Correctly typed dead/reaped handles remain
valid protocol descriptors; `matches` still denies their use for live focus.
Non-pidfd ancillary input cannot pass this type check. Two additional actual red
groups demonstrate the type-only non-pidfd and inherited-view fences before the
check was factored from the existing comparison.

`matches(&left, &right, expected_pid)` keeps both `OwnedFd` values borrowed through
the operation. Their procfs links must have the exact kernel pidfd type name,
`anon_inode:[pidfd]`; scalar device/inode equality is unused. Each bounded fdinfo
read must contain exactly one canonical positive `Pid` within the signed Linux
PID range. Missing, duplicate, malformed, oversized, invisible0 and reaped-1
records fail closed. Both reads precede a final joint nonblocking poll; any
readiness or error flag refuses a match. Every temporary read descriptor drops on
success or error. The new comparison module forbids unsafe code.

Deployment must protect the procfs mount and qualify **one common PID namespace**
for the source's actual credentials, registered app PID, expected PID and pinned
procfs view. `Pid` is expressed in the procfs mount's namespace. `NSpid` is a list
of translated numeric PIDs, not a namespace identifier. The open-time self-PID
check refuses an observed translation; it does not authenticate a deployment's
namespace or mount configuration. No caller-controlled proc mount or path is
admitted. A positive match is an instantaneous identity/liveness observation;
publication must still recheck its retained handles and independent authority.

The stable fdinfo semantics and final-poll rationale are documented by
[Linux6.6 kernel/fork.c](https://github.com/torvalds/linux/blob/v6.6/kernel/fork.c)
and [Linux6.8 kernel/fork.c](https://github.com/torvalds/linux/blob/v6.8/kernel/fork.c).
The actual descriptor table is read through
[Linux6.8 fs/proc/fd.c](https://github.com/torvalds/linux/blob/v6.8/fs/proc/fd.c).
[Linux7.1 fs/pidfs.c](https://github.com/torvalds/linux/blob/v7.1/fs/pidfs.c)
retains these `Pid` semantics and deliberately retains the same pidfd type name.
The two already-owned references cannot retarget while held; equal positive PIDs
in that one qualified namespace and both references still living establish the
same process. This uses the common owned-reference ABI on anon-inode and pidfs
kernels, with no scalar-identity fallback.

Executed host evidence on Linux `6.8.0-136-generic` records six failing actual
kernel/process groups before implementation and four further failing component
groups. The corrected complete selection passes fourteen tests: five library
cases, seven owned-pidfd groups and two existing socket cases. Ignored helper
entrypoints run as owned subprocesses from their parent cases. Real boundaries
include different living peers with aliased anon-inode scalar identities;
wrong expected PIDs; zombie positive-Pid denial; reaped-1 denial; unrelated FD
slot replacement while handles stay held; non-pidfd inputs; descriptor cleanup;
an actual inherited-view fork; and termination after reads but before final poll.
The bounded/malformed parser fixtures are component injections, not claims that
the kernel emitted malformed fdinfo. The FD replacement case does not force
numeric PID reuse. Child exit/reap evidence is local process evidence, not remote
cleanup or hardware health.

Exact sources, red/green logs, host/static-ARM test executables and their hashes
are retained in ignored `artifacts/pidfd-identity/`; `proof.json` indexes the
selection. The same14 checks now also pass under the actual ARM7.1.6 guest kernel in
`artifacts/native-session-compositor/green-owned-focus-1/`, separately from the
original compositor groups; the static binaries were not qualified with
user-mode QEMU. The earlier leaf `proof.json` remains the frozen pre-execution
build/host record. Current combined provenance is linked in
[owned focus](../docs/native-session-owned-focus.md). The retained Pixel profile
still records matched6.6 assets. Upstream6.6 source availability and a6.8 host
pass do not qualify that exact Pixel image/config/KMI or a physical device. No
physical phone, privileged service configuration or retained kernel was changed.

## Process termination latch

`TerminationGuard::install()` owns one Linux SIGTERM action and
`termination_requested()` reads its atomic latch. The handler only stores that
flag. The caller must feed it into its existing actor Stop/drain path; this leaf
adds no Stop implementation, signal mask, thread, worker, descriptor or retry.
The guard is neither Send nor Sync and must live through all drain custody.

Install and Drop require the main thread and one actual task under trusted
`/proc`. The process owner must exclude concurrent thread creation and external
signal reconfiguration at those boundaries; threads created during the lifetime
must have joined before Drop. Installation refuses an active guard, an
unqualified thread set or an inherited blocked SIGTERM mask. It never changes
the calling mask. Drop restores the complete saved signal action; a failed
restoration or unqualified destruction aborts instead of releasing a possibly
still-active handler lease. A fork child cannot consume the copied parent latch
or release its guard. Caught handlers reset to default across exec, while the
unchanged signal mask carries across exec. Children must exec before independent
work. See [Linux signal semantics](https://man7.org/linux/man-pages/man7/signal.7.html)
and [execve](https://man7.org/linux/man-pages/man2/execve.2.html).

A frozen independent consumer first failed with exactly the two absent public
interfaces. The same consumer now passes five real host component groups:
actual SIGTERM latch/duplicate delivery and prior-action restoration; fresh
lifetimes; blocked-mask refusal; actual main/foreign-thread refusal; and a
pidfd-signalled exec child with signal15 parent wait and both PIPE EOFs. The test
uses one joined test-only thread; the implementation adds none. Its original
BOOTTIME bounds remain5s work and8s close.

All fourteen existing active crate tests and the five new groups pass with
`RUST_TEST_THREADS=1`; three isolated helper entrypoints remain ignored in the
ordinary listing and are invoked by their actual parent cases. An earlier
parallel run failed an unchanged numeric FD-slot absence assertion while another
test opened descriptors; that failed run remains separate. Formatting, strict
all-target Clippy and Release build pass. These are Linux host component checks,
not owner signal-to-Stop/drain integration, systemd shutdown, ARM or phone proof.
The later [owner integration](../docs/native-session-signal-stop.md) separately
passes actual SIGTERM before readiness, with Main exit0 and no adopted children.
That combined result does not follow merely from this leaf component pass.
