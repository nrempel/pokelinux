# Native authentication worker-tree scope

Safe Rust per-attempt ownership for a separately supervised PAM worker. The fixed
isolated test entry is under qualification; this is not an installed root broker
or unlock authority. The entire component is currently gated by
`isolated-fixture`; a default build exposes no authentication implementation.
The dedicated supervisor becomes non-dumpable and a subreaper,
stays outside its private workload leaf, and owns its worker Child/pidfd, private
pipes, original absolute BOOTTIME deadline, anchored cgroup controls and adopted
waits. It verifies the still-unreaped worker's membership before START/NSS/PAM.

Cancellation retires the candidate before recursive `cgroup.kill`. Exact-worker
pidfd signalling is retained because admission may fail before workload migration.
A successful write, signal, deadline or pidfd death never substitutes for actual
worker/adopted waits, ECHILD, worker EOF and populated0. Observation errors latch
occupied failure. The native parent owns the supervisor Child/pidfd and keeps
Cancel/Stop/snapshots local; potentially blocking scope syscalls stay in that
owned subprocess. Slot release also requires consistent wait receipts/monotonic
scope state, one terminal report, actual supervisor wait0, stdout EOF and an empty
frame buffer. Occupied Drop aborts rather than detaching a live owner/tree.

The first actual ARM selection exposed process-group-only `waitpid(None)` being
misreported as global ECHILD. That failed selection is preserved. Root changed
the pinned rustix call to `wait(NOHANG)`; the unchanged observer then qualifies
the intended direct-worker-only descendant-survival RED. Two detached processes
stay live after worker wait9; Stop/snapshot responds while occupancy remains.
Only separately labeled external cleanup allows two native adopted waits and
ECHILD/populated0/EOF closure. The recursive repair passes two actual
Arch ARM controls: Cancel/Stop while the owned supervisor is stopped, then a fresh
attempt held past its original deadline. Both close without external SIGKILL, with
three native workload waits, actual supervisor/parent waits, ECHILD, EOF and empty
leaf. See [scope report](../docs/native-auth-scope.md) and
[bounded provenance](../docs/evidence/native-auth-scope-provenance.json).

`isolated-fixture` selects the fixed `/run/pokelinux-auth-scope-test/pam-worker`
and actual membership under `pokelinux-auth-scope-test.service`; no caller chooses
an executable, account, PAM policy or cgroup. Ordinary isolated entries are
`--isolated-parent` and `--isolated-supervisor`, each with exact parent PID and
original deadline. PKSCOPE3 provides48-byte credential-free state, wait and hidden-
prompt records; no historical decoder is retained. Response bytes travel only on
the private command pipe, in bounded zeroizing frames.
The synthetic PAM module uses a checked nonblocking writable FIFO at stderr FD2.
The historical fixture rejected hidden prompts. The later host-only parser and
standalone zeroizing frame passed13 components after an actual private-pipe RED.
A new extraction exercises the exact private Conversation methods through real
pipes: two prompt/response rounds, cancellation under EAGAIN, invalid responses,
failed setup and calls begun after expiry. Three runtime failures precede19
passing components. Both endpoints must become nonblocking before use; local
expiry checks surround I/O and precede parsing. They do not make syscall timing
atomic. Attempt retains every actual child, pipe, cgroup and wait; these tests do
not construct it. Strict Clippy and separate ARM64 compilation pass, without ARM
execution. That19-component selection is historical. The current parent and
supervisor loops forward prompts/responses in source, with40 passing host
components and separate ARM64 compilation. A later unchanged40-component library
suite also passes as UID1000 under an actual Arch ARM64 kernel, including the
ordinary helper's natural wait/EOF control. This is separate from the unexecuted
ARM fixture binary; see the [ARM report](../docs/native-auth-components-arm.md).
Exact parent queue/command codec
composition crosses real pipes, but neither actual owner nor either complete
fixture loop runs in those controls. Actual forwarding and original PAM+scope
remain runtime gates. A candidate is not authentication authority. See the
[current forwarding report](../docs/native-auth-parent-forwarding.md),
[earlier I/O report](../docs/native-auth-conversation-io.md) and
[earlier primitives](../docs/native-auth-conversation.md).

Root-owned controls exclude UID1000 writes but do not prevent an EUID0 helper
from using owner permissions or proc-root aliases to migrate. A separate authored direct-path
[containment control](../docs/native-auth-containment.md) passes; complete helper
alias/filter qualification, non-SIGCHLD clone cases, provisioning and actual
Original-helper+scope composition remain gates. The existing actor keeps NNP1;
a separate fixed-policy NNP0 root broker must be provisioned. Account/session
APIs, actor receipts, exact unlock, synced durable restoration, actual suspend,
Google matched-kernel scope and physical devices remain unqualified.

A separate actual missing-handle setup RED reproduces blocking parent pipes.
After repair, the parent disarms both child endpoints while retaining its Child,
sticky Setup fault and occupied state. The hardened ARM control proves a fresh
Stop/snapshot response while the child stays held; separate driver teardown is
not native cleanup. A new normal two-group regression also passes. The explicit
`setup-fault-fixture` feature adds only fixed no-workload test entries and is
absent from the selected normal ELF. See [setup report](../docs/native-auth-setup.md)
and [proof](../docs/evidence/native-auth-setup-provenance.json).

Run the forty host component tests with `cargo test --features isolated-fixture`
from this crate. The actual cgroup fixture needs the separately provisioned
disposable guest; do not run its root entry on the Harbour host.
One additional ignored helper entry runs only as the exact subprocess of its
parent regression.
