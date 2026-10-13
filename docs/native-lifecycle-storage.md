# Owned lifecycle storage process

Policy loading and durable admission writes now run in one retained native Rust
storage process. The authority actor serves status, lifecycle controls and stop
while that process is paused. The final x86 and actual ARM64 guest-kernel runs each pass27 installed/process
groups and seven real file/pidfd components. The ARM64 guest verifies the exact
final indexed daemon/launcher/client/test identities before execution. This extends the installed authority milestone
[a536634](evidence/native-lifecycle-provenance.json); its historical 18-group
ARM64 proof remains separate. The current storage extension's exact validation
and source identities belong to
[storage provenance](evidence/native-lifecycle-storage-provenance.json).

The process fixture uses actual AF_UNIX credentials, distinct installed app
UIDs, owned child processes, pidfds and bounded temporary files. SIGSTOP pauses
an owned process. It does not reproduce an uninterruptible filesystem syscall,
make filesystem operations nonblocking, or establish phone storage behavior.
The separate final-cleanup cases use a real child with explicitly injected
wait/reap errors or withheld wait calls.

## Authority and request lifetime

The daemon keeps all current principals, prepared permits, policy/lifecycle
revisions and response ownership in its actor. Only the storage process opens,
locks, reads, writes, renames or syncs the policy directory during operation.
Startup opens control sockets first and reports `ready=false` until a qualified
load completes. Profile and executable trust checks still occur during startup
or app launch; this change does not supervise every possible filesystem access.

A private mode-0600 sequence-packet socket binds the worker to the daemon's
actual launched PID. The daemon checks the peer and each packet's kernel UID/PID
against its owned child, then checks a fresh source nonce, increasing operation
token and absolute `CLOCK_BOOTTIME` deadline. The worker checks the actual
parent's per-packet credentials and source/token/deadline. There is one pending
operation and no queued work. Accepted response publication checks the deadline
after receive, actual pidfd liveness and the still-current principal, policy and
lifecycle revisions. EINTR and retries retain the original deadline.

Failure, expiry, caller loss or stop invalidates publication before signalling
the worker. Retired work remains owned until the actual child is reaped. An exit
notification alone does not release occupied capacity. The same owner cannot
spawn a replacement or replay an admission. While a write is busy, an
unavailable admission consumes that principal's prepared permit; a foreign
principal cannot consume it. Status and trusted lifecycle controls remain
available without waiting for disk IO.

A synced response arriving after lock/background/suspend changes retains known
receipt history but grants no current admission and latches degraded state.
`AdmittedUncertain` remains a historical receipt, never a token for a later
hardware effect. Atomic backend effect dispatch is still a separate integration
gate. Revocation and process death establish no remote cleanup.

## Durable ownership and uncertain recovery

The worker holds the exclusive policy flock. Existing grant/receipt bounds and
privacy-safe opaque receipt IDs remain: 32 grants, 128 receipts and a 32 KiB
serialized state. File replacement writes an exclusive pending file, syncs it,
renames it, then syncs the directory. Linux documents the separate directory
sync obligation in [fsync(2)](https://man7.org/linux/man-pages/man2/fsync.2.html).
No filesystem durability claim extends beyond the tested temporary filesystem
or a future qualified platform storage stack.

Before activation, a new owner writes and syncs a fresh `owner.live` epoch.
Healthy quiescent stop writes an `owner.clean` seal containing that epoch and
the SHA-256 hash of the exact known policy bytes. A successor accepts only a
matching seal/hash, refuses every pending file and starts a fresh live epoch.
A killed or failed owner cannot inherit the old seal; it leaves durable
uncertainty that prevents automatic restart, even after its lock is released
and its child is reaped. There is no automatic file deletion, admission replay,
receipt eviction, reset or recovery API. Root or a future trusted recovery
service must qualify storage state and backend uncertainty before deliberately
changing this state. Removing files is not a provided recovery procedure.

## Bounded stop and final ownership

Ordinary stop fences authority, acknowledges promptly and polls owned children.
A healthy idle worker receives a bounded clean-close request; an uncertain
worker is killed without a clean seal. The stop and fatal-cleanup path share one
absolute two-second boot-time budget across storage and app children. Cleanup errors or
budget exhaustion abort the owner while it still retains every unreaped Child;
there is no detached-success return or occupied-slot release. The Worker drop
guard also aborts if a live Child would be relinquished.

Actual [wait/reap](https://man7.org/linux/man-pages/man2/wait.2.html) releases a
terminated child's remaining kernel resources. The cleanup qualifier records a
real child, injects an error or withholds `try_wait`, observes SIGABRT and lets
the isolated fixture adopt and reap the terminated child using the public
[child-subreaper contract](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html). These injected
conditions prove our retained ownership and fixed-budget behavior. They do not
prove a Linux uninterruptible task can be killed. A service-manager/cgroup and
kernel/filesystem qualification remain necessary for an unreapable task and
for the complete app process tree.

## Reproduction and limits

`tools/build-native-lifecycle-storage.sh build` runs formatting, Clippy with
warnings denied, and exact Cargo JSON builds for x86 GNU and ARM64 GNU.
`tools/index-native-lifecycle-storage.py --architecture aarch64` selects the
three binaries and one library-test binary from those records and records
paths/hashes. It never chooses an executable by a filesystem glob.

The installed fixture accepts explicit `--binary-root`, `--results-dir` and
`--baseline-fixture`. It executes the unchanged a536634 18-case body with an
asynchronous readiness bootstrap, then seven storage cases. The independent
cleanup fixture accepts binary/results roots and `--mode error|withheld`.
Together both qualified runtimes pass27 installed/process groups plus seven real
pidfd/file components. The actual ARM64 result is separate from x86 and injected
reaper faults; there is no ARM64 user-mode process substitution. Runtime requires an isolated root Linux kernel,
Python3, GNU libc, writable executable `/opt`, private `/run` and `/var/lib`,
core dumps disabled and a fixture-owned child subreaper. ARM64 user-QEMU cannot replace
these process/kernel tests. The library component's helper is the exact
recorded daemon from `NATIVE_LIFECYCLE_STORAGE_HELPER`.

Only our Rust implementation, synthetic fixture, tool manifests and concise
provenance are publishable. Build inputs, binaries, receipts and detailed logs
remain ignored. No Android/Java production dependency, vendor target, host
power control or physical phone participates. Production service installation,
Omarchy package/UID and lifecycle ingress, backend dispatch, complete process
trees, storage crash/power-loss qualification and uninterruptible kernel tasks
remain open.

`python3 tools/lifecycle-storage-process-vm.py` runs the final exact selection
under isolated Arch ARM64 Linux7.1.6. It verifies manifests, Cargo selections and
fixture digests; executes all seven components, the unchanged18-case body, seven
storage cases and two explicit cleanup faults; and compares exact case names.
The VM uses no NIC, a snapshot root disk and a read-only project export. Root in
the guest controls only disposable installed fixtures; it grants no host authority.
