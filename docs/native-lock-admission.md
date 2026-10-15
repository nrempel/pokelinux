# Native lock admission and durable pending intent

The same Rust lifecycle actor now accepts fixed session UID1000 lock intent before
the original Omarchy locker starts. Native data authority remains denied across a
healthy actor replacement using the same root policy directory and a changed
original compositor/display/Source epoch. Trusted unlock, native owned PAM
authentication and secure presentation are still unqualified. A separate
[Original PAM reference prerequisite](native-original-pam-prerequisite.md)
passes typed wrong/correct outcomes without clearing native lock intent.

## Runtime order

The actor owns `lock.sock` independently of a Messages provider. Its installed
path is `/run/pokelinux/messages/lock.sock`. Each bounded sequence packet must carry
kernel `SCM_CREDENTIALS` for UID1000 and parse exactly `{"op":"lock"}`. Other UIDs,
unknown fields, effect parameters, grants, launch and unlock are denied; inherited
socket possession cannot bypass per-packet credentials.

The installed `omarchy-system-lock` wrapper invokes the fixed Rust `lifecycle-lock`
client as real/effective UID1000 with no arguments. One absolute two-second
`CLOCK_BOOTTIME` budget covers admission, authenticated reply and the final pre-exec
check. The client requires the connected root actor's exact PID/UID/GID on its ACK.
Only success executes `/usr/libexec/pokelinux/omarchy-system-lock-original` as the
same UID. The retained original action has SHA256
`f1953110658d819597d3c9d8df616350c362ee4c257227fcbdefac8275bbd596`.
Installer-owned executable/parent modes are checked; shell startup injection is
removed and fixed lookup paths are used. Original action/QML/PAM and the 500 ms
stabilization timer keep their bytes. Missing, denied, delayed or uncertain ACK
refuses the original action, which is an availability limitation.

The actor revokes current memory authority first. Required-schema
`Durable.lock_pending` then goes through the sole owned storage child, with an
original one-second storage deadline: exclusive pending file, write, file sync,
rename and directory sync. Success ACK requires confirmed durable completion.
`Gate::Lock` is monotonic, supplies no effect stamp and tolerates irrelevant focus
revisions during sync. Already durable pending intent can ACK again without a new
write only when no actor storage operation is unresolved at planning time. The
[storage barrier](native-lock-storage-barrier.md) now refuses cached successful
ACKs while busy, after a genuine actual-daemon/socket RED. Refused or uncertain
admission retains denial. The later [queued-reply repair](native-lock-queued-storage.md)
also downgrades queued success when storage becomes busy before send, after a
separate actual socket RED. EAGAIN, a complete intervening write and complete
authenticated-clear serialization remain separately unqualified.

Pending intent is restored before Ready. Fresh Source `locked:false`, root admin
`locked:false`, locker/UI death and display/Source replacement cannot clear it.
Missing-field old schemas are refused; there is no compatibility fallback, unlock
or reset API. A fresh unlocked session requires an explicit root-owned fresh
storage lifecycle, never an inference from unlocked compositor facts or removal
of admitted policy files. Existing owner/pending/clean-seal/hash checks refuse
uncertain or corrupt recovery.

## Separate executed selections

| Selection | Actual evidence | Limit |
| --- | --- | --- |
| [Initial ingress proof](../tools/native-session-lock-admission/integration-proof.json) | Ready/unlocked unchanged actor RED exposes UID1000 `lock.sock` ENOENT. 8 host groups, 6 original ARM groups and 8 existing components pass after ingress. Native ACK precedes original action; normal Qt reads deny while actual original QML is T-held, before its timer. Fresh unlocked Source and shell death retain the fence | Frozen pre-persistence source selection; no current durable restart claim |
| [Durable restart proof](../tools/native-session-lock-admission/restart-proof.json) | Genuine original ARM RED releases a fresh unchanged normal Qt Messages page after healthy Stop/wait, clean seal and a changed display/Source epoch in the same directory with the original QML T-held. Required pending intent precedes 6 host groups, 5 original ARM groups and 8 components; the fresh authenticated page now denies | Healthy replacement, separate from uncertain-history SIGKILL refusal |

The current restart build selection is
`08580b69240a5a640b7520c214b23ee575eeff9bac5fbe2fc12f6c9221096293`;
its public proof retains every selected source and executable hash. Initial ingress
build selection is
`0474b4387e648bc03779ab30cc6b7a5ccceabefa77cddbebfdaf54303d3692ad`.
Neither record is repinned after later source or documentation changes. Recorded
format/Clippy pass, and restart components use the exact Cargo-selected test ELF.
Two dependent Cargo.lock graphs include the already selected native-camera
component; no camera plus new-lock merged runtime is qualified.

The six later host groups exercise healthy restore/repeated ACK, actual actor
SIGKILL after synced lock before ACK, actual owned-storage death before ACK,
exclusive pending-file collision, corrupt sealed policy and held initial load.
The actual killed actor's successor refuses unclean history; the authored original
effect never executes. This is uncertain-history refusal, not positive crash
recovery. Storage controls do not emulate power loss or uninterruptible physical IO.

The original ARM runtime uses Arch `7.1.6-1-aarch64-ARCH`, unchanged normal Qt and
selected original Hyprland/Omarchy. The100ms Source lease remains unchanged. The
restart epoch uses an explicit private overlay over the readonly shared image,
with 266,539,008 actual bytes within its 256 MiB bound. Both ARM selections record all
tracked processes waited or finally absent. Root waits its owned diagnostic
`strace` parent separately from native actor ownership; actor pidfd death/proc
absence is observed, and native parent wait syscalls are not independently traced.
The proofs record 681 private bindings for ingress and 1,375 for restart; Root
independently verified the latter. Only authored
metadata, hashes and tools are public; packages, ELFs, traces, logs and frames stay
private.

The later [unchanged-Google-kernel prerequisites](native-google-kernel-services.md)
use this current standalone lifecycle build for three permission/storage controls.
They qualify no original Omarchy locker/PAM runtime or trusted unlock. Trusted
lock→authenticated unlock→reopen and positive crash recovery remain implementation
work; permanent refusal is not the finished phone lifecycle.

## Retained failures and remaining gates

Ingress `arm-green-2` lacked `strace`; `arm-green-3` failed the bounded original
lock-entry prerequisite. Restart `red-build-1` failed with Cargo101;
`arm-red-1` failed the prerequisite; `arm-green-1` did not qualify its intended
boundary; `host-green-1` timed out; component epochs 1/2/3 failed with 126/127/127.
These exact epochs remain recorded as failures and are not counted as successful
behavioral REDs. The positive restart RED is `pokelinux-lock-restart-arm-red-2`.

Same-Source continuation and fresh-endpoint reload on the old compositor remain
unavailable. New native pixels 1080×2424 and DPR 2.625 are qualified metadata in the
restart epoch; no new capture or secure frames are claimed. Trusted locker/restore
identity, PAM, external-transition ordering, production installation, camera plus
new-lock runtime, notifications/wake, matched Pixel kernel and physical hardware
remain gates. No physical phone or proprietary runtime was accessed.
