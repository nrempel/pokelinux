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
