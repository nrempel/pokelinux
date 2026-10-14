# Native lifecycle authority

The actor owns app children, durable policy work, Messages reads and session
facts. Display-enabled callers supply `display::Factory` through
`service::run_with_services`; headless callers use the same app slot algorithm.
A trusted Factory creates one owned Context for a reserved app instance. Setup
uses an original one-second BOOTTIME deadline within the two-second client RPC.
No app starts before Ready and an exact-owning-connection EOF check.

Retirement revokes data authority, withdraws the listener before SDK signals and
retains app/context ownership until actual waits. Snapshot and Stop stay bounded
while initialization is held. Source observation and the history writer remain
independent of app context retirement. Metadata is not permission, pathname
withdrawal does not disconnect accepted clients, and local reap is not vendor
cleanup.

Owned locker facts are checked at the typed session Provider boundary. Repeated
sequence/revision values cannot hide changed locker ownership, and unlocked
snapshots cannot carry a locker. Inconsistency fails the established Source
terminally. The same cached reference or independently opened handle for the same
live task remains valid without a policy fence. Three actual RED failures precede
four focused GREEN controls and12 component passes on host and actual Arch ARM
kernels. This is separate from Source IPC, authentication and trusted unlock;
see [scope and provenance](../docs/native-session-owned-locker.md).

[Per-app ARM evidence](../docs/native-display-app-generations.md) qualifies
lifetime/relaunch and held-init Stop. Pending EOF passes its narrow boundary;
its successor aggregate is failed after Source InvalidData of unknown predicate.
The [native lock entry](../docs/native-lock-admission.md) now accepts only a fixed
UID1000 `{"op":"lock"}` request on the actor's `lock.sock`, using kernel credentials
on every packet. Memory authority is revoked before owned storage work; successful
ACK requires synced required-schema `Durable.lock_pending`. That intent is restored
before Ready and cannot be cleared by unlocked Source facts or root admin false.
`Gate::Lock` remains monotonic through irrelevant focus revisions. The fixed Rust
client verifies the connected root actor's reply within the original two-second
BOOTTIME budget before executing the exact retained original Omarchy action.

Initial 8 host/6 original ARM/8 component ingress groups and later 6 host/5 original
ARM/8 component persistence groups are separate frozen selections. The later actual
healthy actor Stop/wait and changed display/Source epoch preserve denial in the
same policy directory. SIGKILL establishes unclean-history refusal, not positive
crash recovery. A fresh unlocked session needs an explicit root-owned fresh storage
lifecycle. The full installed smartphone, trusted unlock/restore, PAM/secure frames,
camera plus new-lock runtime, incoming restricted-display composition, suspend/wake
and Pixel hardware gates remain open.
