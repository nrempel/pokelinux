# Original Quickshell/PAM reference prerequisite

Two reference authentication controls pass through the installed original
Quickshell 0.3.2-1 PamContext and Linux-PAM 1.7.3-1 on an actual Arch ARM 7.1.6 kernel
in disposable QEMU. A wrong response yields `PamResult.Failed=1`; the matching
fresh nonempty guest credential yields `PamResult.Success=0`. Each has one hidden
response-required prompt, one completion and an inactive context afterward.
See [bounded provenance](evidence/native-original-pam-prerequisite-provenance.json).
This qualifies original reference authentication, with no native unlock or
permission/session restoration attempted.

## Original stack and private fixture

The selected backing lacks `/etc/pam.d/omarchy-lock-password`: readonly preflight
and actual guest absence precede installation of the exact retained original
Omarchy service, SHA256
`1740b095412602a0db96fb810e0782ce9f3f155d489da4cb98db36d5b592baac`,
as `0:0/0644` in the disposable overlay only. This missing baseline is a
provisioning prerequisite, not an authentication RED. Original faillock preauth,
authfail/authsucc and delay behavior, optional homed, pam_unix, pam_permit and
pam_env remain; no permissive stack substitutes for password authentication.
PAM account/session APIs are not invoked by the original PamContext.

The fixture uses the unchanged installed Quickshell executable with authored
windowless QML and the existing offscreen Qt plugin. Quickshell and its captured
PAM children run as UID/GID 1000 with empty supplementary groups, CapEff 0 and
NNP 0. The original root-owned `06755` unix_chkpwd and its privilege behavior remain;
there is no guest NNP, ptrace or preload change to that path. Guest Root provisions
one temporary credential through an owned chpasswd stdin pipe. Responses stay in
guest memory and a private `0600` `/run` file. No account name, credential, credential
hash, shadow content, raw PAM output, overlay or binary is published. No host
account or PAM configuration changes, network, host device or phone are involved.

Version-matched original Quickshell and Linux-PAM source is pinned separately
from installed package bytes; a reproduced source-to-package build is not
claimed. Both actual PAM children map 121 root-safe original files, including
libpam, libcrypt and the selected PAM modules. Executable/helper, service/includes,
NSS, environment and faillock configuration hashes and ownership are retained.
Loading an account module does not establish that account management ran.

## Actual outcomes and closure

The authentication oracle is original typed completion. The [original subprocess](https://github.com/quickshell-mirror/quickshell/blob/4f508be500dea6e5732cc3d50382a0048b17e7b1/src/services/pam/subprocess.cpp)
can send Failed and then exit 0; exit 0 alone cannot mean success.
The fixture directly waits three children: the guest-only provisioner and two
Quickshell processes, all exit 0. Two separately captured PAM children have owned
parent/start/pidfd observations, then pidfd-dead and proc-absent observations.
Their own exit statuses and PAM helper wait syscalls are not independently
traced. Original Quickshell's abort/wait path remains a source-bound distinction.
The fixture subreaper ends with actual waitpid(-1,WNOHANG)=ECHILD and no adopted
children. Direct Docker CLI wait0, fresh same-daemon exact-CID absence and all 64
selected input bindings close unchanged.

The selected envelope is 2 guest CPUs/1GiB, 2 container CPUs/2GiB and a 512MiB
container tmpfs qcow2 overlay over the shared readonly backing. No backing clone
is made. Bounds are 30s per conversation, 80s guest fixture, 110s QEMU plus 5s
termination grace and 120s outer runtime. A one-second fixture response delay
allows child/map observation within those limits. No retry or faillock reset is
used. Existing guest service teardown owns poweroff. This is a generic Arch ARM
reference checkpoint, separate from Google-kernel and physical Pixel evidence.

## Remaining native restoration gates

The next [owned locker](native-session-owned-locker.md) work must bind the exact
accepted protocol generation and exact-object unlock, beyond mapped PID or an
observer revision. A supervised native PAM worker must own its typed result and
private conversation, fixed trusted service/account and original absolute bounds;
UID1000 supplies responses, never a trusted success flag. That worker's own
privilege/dependency and account/session policy require qualification. Source
expiry, locker replacement/death, timeout or worker failure must retire the attempt.

Only a successful current owned attempt may arm an internal one-shot receipt.
Consumption still requires that exact owner's real unlock, fresh current Source
reconciliation and synced [durable lock-intent restoration](native-lock-admission.md),
while preserving existing policy, grants and history. Global unlock, root test
clear, same-UID success forwarding or a renewed expired Source cannot provide
that receipt. Secure frames, installed supervision and physical hardware remain
separate. This reference run selects no 100ms Source lease or restoration path.
