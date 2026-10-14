# Native Rust PAM worker

The native Rust worker passes wrong/matching password controls through the
installed public Linux-PAM ABI on an actual Arch ARM7.1.6 kernel. A closed-worker
missing-prompt RED, a failed first real PAM run and a separate original-helper
capability comparison precede the passing default-production-feature run. Prior
failed aggregates remain failed. This is component authentication, with no actor
attempt, unlock receipt, durable restoration or normal Messages reopening enabled.
See [worker protocol](../native-pam/README.md) and
[bounded provenance](evidence/native-pam-worker-provenance.json).

## Fixed installed boundary

The worker resolves UID/GID1000 through NSS in both directions, changes directly
from its root launcher to that account, then loads the trusted installed libpam
and fixed original Omarchy password service. Responses enter only private pipes;
apps cannot choose an account, policy, library, executable or success flag.
The public C ABI uses its documented pointer-to-message-pointer array, libc
response allocation and exactly one `pam_end` after successful `pam_start_confdir`.
Raw PAM text is never read or published. Unexpected message styles, conversation
failure, changed PAM user, unsuccessful teardown or an expired original BOOTTIME
budget cannot yield Authenticated. Completed PAM responses follow PAM's allocation
ownership; locally owned buffers and partial replies are scrubbed.

The worker's typed terminal packet is a candidate. A parent ACK and clean exit0
are separate protocol steps; exit0 alone is never success. The future supervisor
must retain the result through actual worker/helper waits, ECHILD, empty workload
cgroup and EOF within the original bound. Deadline readbacks cannot interrupt a
blocked foreign call or pipe. There is no detached-worker or implicit retry path.

The protected credential transition requires `fs.suid_dumpable=0`, CORE soft/hard0,
empty groups, no exec after the UID drop, no inherited descriptors beyond private
stdio, explicit active-capability clearing/readback and rearmed parent-death
protection. The selected Arch guest normally has `fs.suid_dumpable=2`; disposable
runs explicitly record and change the guest setting to0. No host sysctl/account
or PAM file is changed. That native prerequisite needs installed deployment policy.

The Messages actor's `NoNewPrivileges=yes` unit remains. It cannot launch this
NNP0 transition by inheritance. A separate fixed narrow root authentication
broker, actor-instance connection ownership and scoped workload launcher remain
required; weakening the whole actor unit is not the chosen architecture.

## Actual closed-worker RED

Selection `13ca4a0b724dea330cb3fb0b82b1ea98ae5375f318b2df88b64be40fbc3a33d2`
executes the closed native worker on an actual Arch ARM7.1.6 kernel. Both wrong
and matching cases receive Ready then Unavailable, with no prompt or response.
The aggregate stays failed; the missing required hidden prompt is a qualified
component RED. No PAM execution is claimed for that closed implementation.

Root independently reviews all69 input bindings and actual role/cleanup records
before implementing FFI. At the withheld ACK boundary both workers have UID/GID1000,
empty groups, all five capability sets0, NNP0, protected proc ownership, CORE0,
private pipe identity and actual executable identity. Both owned worker waits0,
pidfd death/proc absence, one provisioner wait0, no adopted descendants, final
ECHILD and same-daemon exact-container absence close the selected run.

## First real PAM attempt remains failed

The fresh unchanged-credential oracle selection
`ff6e33d16676a9e732a5edb4674a10566daddd2e4eb5f6e4917b75eea181d54e`
executes the repaired worker. Both cases now produce one real hidden prompt;
libpam and pam_unix are mapped before either secret is supplied. Wrong yields
Denied as required. Matching also yields Denied, violating the positive contract.
This is a real integration failure, not passing native authentication evidence.

Root independently verifies all101 input bindings, the exact failed result,
UID/GID/group/capability observations, actual worker waits0, final ECHILD without
adoptions and fresh exact-container absence. The provisioner plus two worker
waits are direct guest waits; internal PAM-helper wait syscalls are not separately
traced. No raw credential, credential hash, shadow file, raw PAM text, backing
overlay or proprietary binary is published. Fresh credentials exist only inside
the disposable guest and its owned private provisioning/response pipes.

The [original public helper](https://github.com/linux-pam/linux-pam/blob/28fce1fd55b003fec90f34e5bc785e9439b216f2/modules/pam_unix/unix_chkpwd.c)
returns the result of audit logging after a successful password verification when
its real UID is nonzero. Its [audit implementation](https://github.com/linux-pam/linux-pam/blob/28fce1fd55b003fec90f34e5bc785e9439b216f2/modules/pam_unix/audit.c)
can turn an audit failure into PAM_AUTH_ERR. The failed typed result alone does
not identify an audit errno or establish that full-PAM failure's cause.

A separately frozen actual helper comparison then qualifies the narrow bounding
requirement. With the same fresh guest credential and unchanged original helper,
correct with Bnd0 exits7; correct with only CAP_AUDIT_WRITE29 exits0; wrong with
that audit-only bound exits7. Before any response Root observes the exact helper
executable/generation blocked in ARM read63 on stdin0, real UID/GID1000 and
effective/saved/filesystem IDs0. Its permitted/effective/bounding sets match the
selected mask; inheritable/ambient remain0. Each UID1000 fixture parent has empty
active sets. Private pipe identity, CORE0 and NNP0 are checked before the secret.

Root independently verifies all120 bindings, six direct fixture waits plus three
helper waits by those owned parents, eight captured process deaths, final ECHILD
without adoptions and fresh same-daemon exact-container absence. The native
wrong/matching baseline is repeated unchanged and remains Denied/Denied, keeping
the aggregate failed. This demonstrates the selected helper's bounding-set
interoperability difference; it does not observe the precise audit syscall/errno.

The revised native entry retains only CAP_AUDIT_WRITE in bounding while clearing
and reading back effective/permitted/inheritable/ambient sets0. It does not grant
the Rust worker an active audit capability. The fixed production default build
and separately selected synthetic fixture compile.

## Passing default-worker PAM boundary

Fresh selection `18188bcd9b616cead2a5dbab5463958726761a8c9eb4b430ae80d7046c3c6f2b`
executes the default-feature production worker with that narrow bounding profile.
The wrong response yields Denied1; the matching response yields Authenticated0.
Each has one real hidden prompt, one private response, actual installed libpam/
pam_unix mappings before the secret and a protected role observation while the
response is withheld. Worker UID/GID1000, groups[], E/P/I/A0, Bnd0x20000000,
NNP0, CORE0 and root-owned protected proc identity match before and after sampling.

The authentication oracle, original PAM stack, private conversation, ACK, actual
wait0, EOF/death checks and deadlines remain unchanged from the first attempt.
Only the guest's explicit expected bounding predicate changes. Root independently
verifies all156 selected bindings and the passing result, three direct guest
waits0, two captured worker pidfd deaths/proc absences, final ECHILD without
adoptions and fresh same-daemon exact-container absence. Internal PAM-helper
wait syscalls remain untraced; this does not qualify generic descendant cleanup.
The selected envelope remains2 guest CPUs/1GiB,2 container CPUs/2GiB and512MiB
overlay tmpfs, with30s conversations,80s guest,110s QEMU plus5s grace and120s outer.
No NIC, host device, phone, host account/PAM or host sysctl change is selected.

## Separate ownership and restoration gates

A fixed Cargo-feature-gated synthetic ownership worker selects only an isolated
root-owned test policy under `/run/pokelinux-auth-scope-test`. Its stderr readiness
records are not authentication authority. It prepares actual blocked PAM, fork,
setsid and descendant cleanup controls; compilation alone qualifies no cleanup.

The per-attempt supervisor must run outside a workload cgroup, admit the worker
before START/NSS/PAM and retain occupancy through recursive kill plus actual waits
and populated0. The shared parent keeps Stop/snapshots local if any supervisor
syscall blocks. Root-owned cgroup controls alone do not prevent an EUID0 helper
from migrating through owner DAC permissions. Namespace/proc-alias containment,
trusted dependency policy and installed supervision require separate qualification.
Google's matched kernel lacks the PID controller and Landlock; neither can be an
unconditional shared requirement. The current Arch ARM runs qualify neither that
Google kernel's PAM stack nor physical hardware.

Successful supervised authentication still must bind an actor/Source incarnation,
exact accepted lock operation, retained live locker and lock-admission epoch.
Only a current clean result may arm a one-shot internal receipt; genuine exact
unlock must follow arming. Versioned synced durable clear must serialize with
new lock admission, preserving grants/history and denial across uncertainty.
The existing generic history-completion path is not an unlock-clear primitive.
Account/session policy, secure frames, suspend/resume and physical-device behavior
remain explicit gates. No physical phone is accessed or qualified.
