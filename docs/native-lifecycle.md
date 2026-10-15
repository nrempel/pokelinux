# Native installed permission and lifecycle authority

The initial authority milestone passed18 installed Linux process groups and
four component tests on x86 and an actual ARM64 Arch kernel. The current
[owned storage extension](native-lifecycle-storage.md) passes27 installed/process
groups plus seven components on both kernels, with responsive controls and
absolute bounded cleanup. The fixtures install our binaries into a disposable container/guest,
assign two distinct app UIDs and run actual separate processes. Exact compiled
ARM64 test/daemon/launcher/client identities are checked before guest execution. No Android/Java runtime,
phone, host power interface or vendor executable participates.
[Provenance](evidence/native-lifecycle-provenance.json) separates these results.

This historic milestone is a local permission/admission boundary. The new
[installed Messages extension](native-messages-service.md) adds current data
release and a distinct-UID native window to the same actor. Production hardware
dispatch and trusted Omarchy lock/focus ingress remain open. Its
`AdmittedUncertain` receipt records conservative authorization history; it is not
a transferable token granting a later hardware operation.

## Installed identity and ownership

The installer owns a bounded profile mapping app IDs to unique UIDs, absolute
executables and fixed arguments. Only the privileged admin socket can request
launch arbitrary configured apps, grant/revoke policy, change lifecycle or stop.
The optional Messages provider also admits one fixed launch-only request from the
actual designated desktop UID; it grants no policy or data authority. A privileged
launch request supplies an app ID; it cannot select executable, UID, PID or arguments. Executables and
containing directories must belong to root and be unwritable by apps. Set-ID
executables and symlink executables are refused.

The daemon owns every spawned `Child`, opens its pidfd before reaping and admits
only messages whose kernel PID and UID match that launched generation. The
launcher entry clears supplementary groups, sets real/effective/saved UID and
GID, sets `no_new_privs`, then executes the installed app with a cleared
environment. It arms the parent-death signal after credential changes and checks
the parent again. Linux clears that signal when credentials change, as recovered
from [the original credential implementation](https://github.com/torvalds/linux/blob/e8f897f4afef0031fe618a8e94127a0934896aba/kernel/cred.c).

The daemon polls pidfds and `try_wait`; it never joins a worker or blocks on a
child wait in its request loop. Death removes the principal and prepared work,
even when another process retains the app's actual connection. Termination
revokes before signalling. A successor cannot launch until the previous owned
child has been reaped. The later [normal app Stop](native-app-stop.md) now fences
and withdraws first, permits one bounded TERM grace for Running apps, and retains
all children/contexts until actual wait/drain; failure/cancel paths remain forced.
Stop and fatal cleanup share one absolute two-second boot-time budget and abort
before relinquishing unreaped children. An uninterruptible kernel task still
requires an external supervisor/cgroup. Process reaping establishes
no remote backend cleanup.

Omarchy plugins and caller-supplied app labels are not security principals. The
present contract requires distinct assigned UIDs and authorizes the launched
process only. Shared-UID applications and multiprocess app delegation require a
qualified launcher/sandbox/capability contract before admission. The fixture
proves our installed profile; it does not install this service into an actual
Omarchy user session or prove a package-signing/update authority.

## IPC and admission

AF_UNIX `SOCK_SEQPACKET` carries one JSON object per packet. `SO_PASSCRED` is
enabled on listeners and accepted sockets. Each receive requires the actual
`SCM_CREDENTIALS`, rejects truncated data/control and received descriptors, and
closes malformed transports. Claimed PID, UID and app fields are rejected.
`SO_PEERCRED` alone would preserve the connection creator's credentials after
FD inheritance; [Linux documents both credential interfaces](https://man7.org/linux/man-pages/man7/unix.7.html).
The executed fixture transfers the real connection FD, then proves another PID
with the same UID cannot use it. Separate unprivileged children attempt forged
UID and PID credentials; the actual kernel rejects them. This agrees with
[the original SCM checks](https://github.com/torvalds/linux/blob/e8f897f4afef0031fe618a8e94127a0934896aba/net/core/scm.c).

Pidfds retain task identity while numeric PIDs can be recycled and expose task
termination through polling. The [public pidfd contract](https://man7.org/linux/man-pages/man2/pidfd_open.2.html)
informed the implementation. The test exercises real termination/reaping and
replacement; it does not force kernel PID-number wraparound. The selected Linux
v6.8 sources are inspection references, not a kernel build or an assertion that
Ubuntu's patched kernel equals that source revision.

`prepare` reserves bounded work for capture or message-send permission. `admit`
checks the current principal, policy revision, lifecycle revision, foreground,
unlocked/awake policy and absolute deadline. A foreign principal cannot consume
another app's reservation. Successful or failed admission consumes the permit;
a repeated permit never authorizes another attempt. Lock/unlock,
background/foreground, suspend/resume and policy changes fence older work.
Revisions cannot wrap into usable authority. Query authenticates the principal
without dispatching an effect or granting permission.

All request lifetimes use absolute `CLOCK_BOOTTIME`, including time spent
suspended. Expiry is checked at the gate without relying on timer delivery and
after durable IO. Late durable success retains uncertainty but returns no current
admission. Nonblocking IPC uses one absolute two-second deadline across retries
and checks it after receive; EINTR never resets it. Synthetic trusted lifecycle
controls test these transitions. Actual compositor lock, app background and
suspend ingress still need ownership/authentication integration.

## Durable policy and bounded uncertainty

A private mode-0700 directory and mode-0600 files belong to the service UID
(root in this first installed contract). One held exclusive flock owns policy.
Updates create an exclusive pending file, sync it, rename it, then sync the
directory. An admission receipt is persisted before returning authorization.
Write/sync failure latches unavailable and clears prepared work; removing a
failure fixture cannot restore that owner. Any pending file at startup refuses
restart. There is no automatic pending-file deletion, receipt eviction, retry,
policy migration or backend reset.

Policy contains only app grants and opaque receipt IDs. Receipts contain no PID,
UID, app label, message/capture data or wall-clock timestamp. Revocation affects
new admission and prepared work; it never deletes an earlier uncertain receipt
or claims the remote effect was undone. The installed test reaches the receipt
limit and proves the next admission is unavailable without eviction.

| Resource | Bound |
|---|---:|
| Installed apps and owned child slots | 16 |
| Profile input | 16 KiB |
| JSON packet / local pending response | 4 KiB |
| Accepted app / privileged connections | 12 / 4 |
| Prepared work, total / per app | 64 / 4 |
| Permit lifetime | 1–5,000 ms |
| Durable grants / uncertain receipts | 32 / 128 |
| Durable input and serialized state | 32 KiB |
| Accepted idle / pending response lifetime | 30 s / 1 s |

Kernel socket queues and the complete app process tree still require installed
service-manager/cgroup budgets. The current actor delegates policy IO to one
owned storage process. Actual SIGSTOP tests qualify responsive snapshots/stop,
late-response fences and reaping; this does not make filesystem calls nonblocking
or qualify an uninterruptible task. Startup/profile/launch trust checks remain
separate filesystem operations. Healthy storage shutdown seals its exact epoch
and policy hash; uncertain ownership refuses automatic recovery after restart.

## Execution and remaining gates

Four executed regressions failed before their fixes: a stale permit surviving
lock/unlock; extra identity fields accepted by Serde's unit query variant; an
unreconciled durable pending file permitting restart; and revision exhaustion
retaining usable authority. The final runtime passes 18 groups. Four real-file /
pidfd component tests cover withheld-timer expiry, expiry after durable success,
failed admission persistence and revision exhaustion. Injected core credentials
in those component tests are separate from the actual kernel credential tests.

The original18/four result belongs to commit a536634 and its historical artifact
selection. For current source, run `tools/build-native-lifecycle-storage.sh build`,
`tools/test-native-lifecycle-storage-components.sh`,
`tools/test-native-lifecycle-storage.sh green` and the separate cleanup qualifier;
see [storage reproduction](native-lifecycle-storage.md). The build records exact Cargo JSON
artifacts for ARM64. `tools/index-native-lifecycle.py` generates a private
manifest of paths/hashes for the shared VM runner. The reusable Python fixture
accepts explicit binary, result, install, runtime and policy roots, and requires
`NATIVE_LIFECYCLE_ISOLATED=1` in a disposable root container/guest. The command
client's driver socket is test infrastructure. Existing native audio/radio
services do not consume these permits yet.

Remaining gates are the installed Omarchy
service-manager and package/UID policy, app delegation/sandboxing, authenticated
Omarchy lifecycle ingress, complete storage/platform supervision, and trusted backend dispatch
at the same admission boundary. Phone hardware and remote cleanup remain
unqualified. A later backend must not accept a historical receipt as current
permission or infer cleanup from revocation or process death.

The separate `tools/lifecycle-process-vm.py` runner executes the indexed native
ARM64 artifacts under Arch kernel7.1.6 with no NIC, a snapshot root disk and a
read-only project mount. Its guest fixture installs into root-owned `/opt`, `/run`
and `/var/lib` paths and exercises the same18 Linux groups plus4 component tests.
Root inside that disposable guest is the trusted test controller; it grants no
host authority. Those are actual ARM64 kernel/process/credential results, not
ARM64 user-QEMU or phone evidence. Kernel/driver storage stalls, complete app
trees and production Omarchy service installation remain separate gates.

The [installed Messages correction](native-messages-service.md) now qualifies
current read publication through this actor in original Omarchy, including
actual EAGAIN, retained old descriptors and source-bound recovery after real
reaping. The storage child holds no app read authority; new app/worker generations
need fresh stamps. The [native client worker](native-messages-ui-worker.md) keeps
thread ownership until confirmed exit. Trusted compositor lock/focus/session
ingress and hardware effect composition remain separate open gates.

The later [Messages receive/read correction](native-messages-receive-isolation.md)
uses a required exact-token read cancellation method and explicit read-only
occupancy. Those two pending-read EOF/deadline sites preserve the separate live
receiver. Aggregate service occupancy and terminal Stop still own every child
until actual reap. Its ARM Binder qualification uses no session Provider; joint
session/installed app integration remains separately selected.
