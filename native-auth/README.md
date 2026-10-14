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
an executable, account, PAM policy or cgroup. Current private entries are
`--isolated-parent` and `--isolated-supervisor`, each with exact parent PID and
original deadline. PKSCOPE2 provides48-byte credential-free records with separate
recursive-write and direct-signal observations; no historical decoder is retained.
The synthetic PAM module uses a checked nonblocking writable FIFO at stderr FD2.
This fixture has no credential bridge: a hidden prompt retires it. A candidate is
not authentication authority. The independently qualified production PAM worker
and this synthetic workload remain separately selected until joined integration.

Root-owned controls exclude UID1000 writes but do not prevent an EUID0 helper
from using owner permissions or proc-root aliases to migrate. Privileged-helper
containment, non-SIGCHLD clone cases, namespace/dependency provisioning and actual
Original-helper+scope composition remain gates. The existing actor keeps NNP1;
a separate fixed-policy NNP0 root broker must be provisioned. Account/session
APIs, actor receipts, exact unlock, synced durable restoration, actual suspend,
Google matched-kernel scope and physical devices remain unqualified.

A separate source review identifies an unqualified setup-error path: failed
supervisor pidfd setup can leave the parent pipes blocking before their flags are
set. The next poll can then stall before servicing Stop/snapshots. Successful
admission controls do not qualify that path; an actual injected-syscall-failure
RED and repair are required before production integration.

Run the seven host component tests with `cargo test --features isolated-fixture`
from this crate. The actual cgroup fixture needs the separately provisioned
disposable guest; do not run its root entry on the Harbour host.
