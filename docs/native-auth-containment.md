# Native authentication helper containment

An actual Arch ARM7.1.6 RED demonstrates a real gap: root ownership/mode0644 of
cgroup controls cannot confine the already qualified privileged helper profile.
An authored process with real UID/GID1000, effective/saved/filesystem IDs0 and
only audit capability29 moves itself into an explicitly owned sibling. Killing
the empty workload leaves that same retained process alive for105.77ms. The
failure is frozen before the driver's exact pidfd kill and actual wait9. This
aggregate remains failed. No original PAM/helper behavior is inferred from the
authored process, and no unrelated process is moved.

The fixed Rust bootstrap now supplies a private mount namespace and a protected
minimal root before NSS/credential drop. It makes propagation private, binds only
the actual worker task directory and real read-only suid_dumpable/cap_last_cap,
pivots and detaches the former root, resets cwd and closes descriptors>=3. Its
inherited narrow syscall filter restricts direct outside descriptor acquisition
and mount/namespace entry while preserving NNP0 for the original set-ID helper.
It is compiled only through an explicit fixture feature; production PAM and the
actor wire remain unchanged. No PID namespace is used: both matched Google
configurations disable PID namespaces, user namespaces, the PID controller and
Landlock. Their cgroup.kill support is independent of the PID controller.

The separately frozen contained control passes once on actual Arch ARM. The
same requested helper privilege profile is independently held, including all
four UID/GID values, every capability set, NNP0, CORE0, one thread and no tracer.
The actual selected probe inode is executed in a distinct mount namespace with
the same real PID namespace; root/cwd dev+inode equal the installed private root,
only FD0/1/2 remain, and one inherited filter is active. The fixed target open
returns ENOENT. The probe remains the sole workload member; the sibling is empty.
The outside anchored workload cgroup.kill then terminates it: real wait9,
pidfd-dead/proc-absent, EOF with no extra bytes, empty leaves and final ECHILD all
complete within the original15s deadline without a fallback pidfd kill.

The contained probe changes its proc-based preconditions to direct readbacks,
because the new root deliberately lacks full proc/status and membership files.
The outside retained-process oracle supplies those facts. The exact packet and
flags differ accordingly; this is not byte-identical probe execution. The
private-root proc link text actually renders `/`, so root/cwd are bound by
filesystem identity, not an assumed staging-path display string.

Both runs use disposable RAM overlays, networknone and no host/phone device.
Actual guest/outer waits, exact same-daemon container absence and all frozen
bindings close. Default and contained strict Clippy, formatting and ARM release
compilation pass. Initial formatting and local Rust capability-API compile
failures remain failed preparation selections, not runtime REDs.
[Bounded proof](evidence/native-auth-containment-provenance.json) records hashes
and selected observations; [authored drivers](../tools/native-auth-helper-profile/README.md)
are public. Private raw logs, binaries, NSS data and backing images are not.

## Scope and next integration

This qualifies the fixed direct cgroup pathname and helper-profile preservation.
It does not qualify every proc/FD alias, each filter denial, outbound signalling,
network/abstract UNIX access or arbitrary hostile PAM code. No network namespace
is introduced; fixed trusted PAM dependencies remain required. The outside
fixture cgroup kill is not native ScopeChild cleanup evidence. Earlier native
scope cleanup/setup controls remain separately recorded.

Next compose the unchanged original PAM stack with this view and the real owned
scope, including bounded prompt/response forwarding. Finish its actual library,
NSS/config/helper and selected device closure; preserve the existing faillock
backing across attempts. Wrong/correct outcomes alone do not qualify faillock:
observe credential-free failure progression and successful reset. Keep optional
homed present and distinguish inactive versus active daemon support. No general
bus, broad /usr,/etc,/run bind or new empty tally is a completion shortcut.

Then connect the separate NNP0 broker to the NNP1 actor's inert attempt handle,
require whole-scope closure before accepting a candidate, observe the genuine
exact-object unlock, sync durable pending-intent clearance, and reopen normal
Messages over existing history. Keep the original deadline and100ms Source lease.
Matched Google kernel/LSM/delegation and all physical-device gates remain open;
no secure-frame, authentication, restoration or phone completion is claimed.
