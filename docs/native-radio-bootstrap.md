# Cooperative native radio bootstrap

The production radio library now provides an explicit cooperative worker entry
before Binder initialization. An actual generic Arch ARM64 kernel Binder test
first demonstrates the new requirement missing from ordinary process launch,
then passes with the cooperative entry. This is local process custody. A complete
production radio worker, installed service supervision and native IMS remain open.
[Bounded execution record](evidence/native-radio-bootstrap-provenance.json).

`WorkerCommand` selects a trusted executable and prepends the fixed
`--pokelinux-radio-worker` selector. `Supervisor::start_worker` requires the
process main thread and inherits equal real/effective UID/GID. Its owner is
explicitly non-Send/non-Sync and must retain that spawning thread through actual
child wait. The caller protects executable provisioning, procfs and the common
PID namespace. This profile supplies no credential transitions, supplementary
Group policy, containment or service installation.

Before spawn, the existing exclusive mode-0600 admission marker is synced with
its private service-owned directory. It survives every spawn/bootstrap failure,
Stop, reap and supervisor restart. No marker reset, vendor `clear`, implicit
retry or successor authorization is added.

Bootstrap uses an owned nonblocking CLOEXEC Unix SEQPACKET channel transferred as
stdin. The parent drops its configured child endpoint after spawn. The child
retains a CLOEXEC duplicate and replaces fd0 with `/dev/null`. BOOT carries the
incarnation, original absolute three-second CLOCK_BOOTTIME deadline and an owned
parent pidfd. The child compares it with the independent socket-origin pidfd
through `ProcessView`, checks kernel peer credentials and its actual parent,
arms/readbacks SIGKILL parent-death after final credentials, and rechecks before
READY and after GO. Failure returns before Binder initialization or pool startup.

The parent validates READY's actual SCM_CREDENTIALS against its retained direct
child pidfd, inherited UID/GID, incarnation and original deadline. Only successful
GO followed by child-liveness and deadline checks enables publication admission.
`poll_worker` handles one bounded packet without a waiting thread. Stop drops
bootstrap admission before signalling. A reap error terminally drops that
admission while retaining Child occupancy; polling cannot restore it. Only actual
wait releases process custody. The existing generic `start(Command)` remains a
separate low-level explicit-Stop primitive and promises no cooperative bootstrap
or intrinsic parent-death behavior.

Linux attaches parent-death signaling to the spawning thread and clears it on
specified credential changes. These restrictions inform this main-thread,
final-credential contract; they do not replace service-manager supervision.
See the [upstream Linux manual](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html).
Socketpair peer credentials identify its creator, so parent-side READY uses
message credentials rather than treating its socket-origin pidfd as the child.
See [Unix socket credentials](https://man7.org/linux/man-pages/man7/unix.7.html).

## Executed evidence

The test worker body is public Rust/manual-peer infrastructure in
`tools/native-arm64-binder/fixture`; its main enters production `BoundParent`
before `ProcessState` or Binder pool setup. No Android runtime, original vendor
execution, private REA inputs, authentication/PAM or physical phone is selected.
The kernel is unchanged generic Arch ARM7.1.6, separately from Google's matched
Pixel kernel/modules/firmware.

- The unchanged production supervisor baseline reaches an actual permanent Binder
  hold, cancellation and responsive stop/snapshots. The verified guest subreaper
  retains exact parent/worker pidfds and kills only the supervisor. Its actual
  wait is SIGKILL, while the worker remains alive as an adopted orphan after the
  fixed first-second oracle. The driver persists this RED before signalling the
  worker, then actually waits it within the original two-second cleanup cutoff.
  This is a new requirement, not an earlier generic-start contract regression.
- The repaired selection uses the same frozen outer oracle and deadlines. The
  worker dies and is actually waited as an adopted SIGKILL child7.528539ms after
  the parent-kill request, without a driver worker signal. The independent peer
  stays live. This observation is not a realtime or unkillable-task guarantee.
- Both epochs retain the durable marker and refuse a recreated supervisor. Both
  separately run explicit Stop through held Binder, revoke old output, retain
  occupied capacity through actual wait and refuse uncertain registration.
- A separate ARM guest transfers a real live independent peer pidfd as the parent
  handle, with otherwise valid BOOT. The production child returns PermissionDenied
  and actual exit1 before READY or worker-body entry. The Rust fixture is its sole
  reaper; the outer client has actual wait0, the peer remains live, and its actual
  registration count remains0.
- Review found a new cooperative poll could undo existing reap-error revocation.
  An explicitly authored ready-state component test first fails with Ready instead
  of Terminal; clearing bootstrap on wait error repairs it. This is not a child
  handshake execution claim. All36 host radio library tests then pass.

Strict host formatting/all-target/all-feature radio Clippy, fixture and existing
containment-tool Clippy, and separate ARM client/peer compilation pass. A fixture
error-conversion compile failure and formatting mismatch remain prerequisites,
not behavioral REDs. Root independently rehashes67 source bindings, all three
runtime selections and the unchanged core harness. Actual guest child waits,
QEMU/container CLI waits, fresh exact-CID absence, same-daemon and frozen-input
checks all close. Raw logs, proc/mount records and binaries stay private.

The supplied public fixture modes are `parent-death-supervisor JOURNAL`,
`parent-death-refuse JOURNAL`, `supervised-permanent` and
`bootstrap-wrong-parent PEER_PID`. The first requires an outer verified subreaper
with exact orphan custody; it deliberately parks and must not be run unowned.
The separate private frozen guest envelope supplies that ownership. The older
nine-scenario runner is separate historical qualification, not this new oracle.

## Remaining gates

Queued READY after the original deadline, Stop during Initializing with queued
READY, and actual late-GO refusal still need their own runtime controls. Deadline
checks are source-level here; this selection performs no actual suspend/wake.
Installed service-manager/cgroup and descendant cleanup, stronger containment,
matched Google kernel/policy and unkillable kernel work remain separate gates.

A complete production transport-worker executable, vendor discovery and reset,
producer identity, decoded IMS/call/message receipts, modem/carrier and physical
hardware remain unfinished. Killing and reaping a local worker proves no remote
cleanup and authorizes no registration retry. Zero physical phones are qualified.
