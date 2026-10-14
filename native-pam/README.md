# Native PAM worker

A dedicated Rust process owns the private conversation with the fixed installed
Linux-PAM password policy. It is a component under qualification, not an unlock
API or enabled production authentication broker. See
[scope and evidence](../docs/native-pam-worker.md).

`pam-worker --parent <owning-parent-pid>` accepts no account, service, configuration,
program or response on its command line. The root launcher must have already put
the worker into an owned per-attempt workload scope before sending START. The
worker enters the fixed UID/GID1000 account directly, without a post-drop exec.
The account must resolve consistently in both NSS directions. Root-owned trusted
`/usr/lib/libpam.so.0`, `/etc/pam.d/omarchy-lock-password` and the original
`/usr/bin/unix_chkpwd` supply the installed dependency boundary. Authentication
only is selected; no account, credentials or session establishment API runs.

The launcher requires `NoNewPrivileges=0`; the worker checks that property rather
than weakening an existing service. The Messages actor retains its own
`NoNewPrivileges=yes` unit. Installation of a separate narrow root broker and
its private authenticated actor connection remains required. The protected
credential transition requires kernel `fs.suid_dumpable=0`, zero core limits,
empty supplementary groups and explicit capability readbacks. Only
CAP_AUDIT_WRITE remains in bounding for the unchanged original helper; the Rust
worker effective/permitted/inheritable/ambient sets are0. Actual helper controls
establish the selected bounding requirement, separately from full PAM behavior. This is a native
installation prerequisite; the test guest originally has a different dump policy.

Each attempt uses fresh private stdin/stdout pipes. START is `PKPA` followed by
one absolute BOOTTIME nanosecond deadline as little-endian u64, at most30seconds
ahead. Ready is `0x10`. A hidden prompt is `0x11` plus its little-endian nonzero
u16 sequence; a response is `0x20`, matching sequence, little-endian u16 length
and1..512 non-NUL bytes. At most32 prompts/messages are accepted. No raw PAM
message text leaves the worker. Response buffers are zeroized on local release;
PAM owns completed libc response allocations as required by its public ABI.

Terminal candidate is `0x30` plus Authenticated0, Denied1 or Unavailable2. The
parent acknowledges with `0x40`. The parent must use strict
START→Ready→Prompt→Response ordering, buffer any candidate through actual clean
worker exit/EOF and independently close the whole helper scope before granting
any authority. Exit0 is never an authentication result. Blocking pipe/PAM work
is not interrupted by the worker's deadline checks; an external owned supervisor
must enforce the same original deadline, retire the attempt and retain occupancy
until actual cleanup. No detached blocked worker is permitted.

The `ownership-fixture` Cargo feature builds the separate
`pam-ownership-fixture` binary with only the fixed isolated service
`/run/pokelinux-auth-scope-test/pam.d/pokelinux-auth-ownership-test`. The production
entry retains its one fixed policy even when this test feature is compiled. The
fixture uses stderr FD2 for synthetic module readiness; it is not authentication
or unlock authority. Default builds do not include that entry.

The original closed-worker RED and failed first real PAM attempt are preserved.
A subsequent original-helper comparison qualifies the narrow audit-only bounding
requirement, then the default production worker passes actual wrong→Denied and
matching→Authenticated controls on Arch ARM7.1.6. Root independently verifies
all156 inputs and actual waits/EOF/death/ECHILD/container closure. Generic helper
scope cancellation, installed broker, actor receipt, durable restoration, secure
frames, suspend and physical devices remain unqualified; production authentication
is not enabled by this component checkpoint.
