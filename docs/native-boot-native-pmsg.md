# Native PID1 records retained through synthetic reset

This page retains the initial PMSG implementation, its bundle and two failed Arch
checks. The [current ACK/bundle checkpoint](native-boot-ack-and-current-bundle.md)
uses a later init and separately passes current Arch/full-stream consumers and
the same PMSG regression. Historical artifact hashes below are unchanged.

The native RAM probe now submits its own canonical diagnostics to an optional,
identity-checked PMSG device. Actual execution of the authenticated Google4K
kernel on synthetic QEMU virt recovers all four first-boot native records after
PowerOff followed by an owned reset. The successful-child and controlled-failure
cases both pass; an actual missing-backend case still completes ordinary native
child37, kernel-ring readback and PowerOff.

This is a native PID1 implementation result under a fixed synthetic kernel and
memory selection. The previous [canary experiment](native-boot-warm-reset.md)
remains separate historical evidence. Raw logs, retained bytes, original kernel
and generated archives remain private. No phone was accessed.

## What PID1 does

The reporter preserves its random execution epoch, consumed event ordinals,
required exact kmsg readback, child ownership/actual wait and PowerOff policy.
After tmpfs `/dev` and sysfs are mounted, it attempts PMSG discovery once. It
makes one bounded65-byte sysfs read, accepts at most64 bytes, validates the
filesystem and decimal dynamic major/minor, and creates a new0600 node relative to
its retained tmpfs `/dev`, and checks the opened descriptor's type, mode and
rdev. The descriptor uses CLOEXEC, NONBLOCK and NOFOLLOW. A conflicting path,
missing backend or failed identity check disables optional PMSG; there is no
regular-file fallback, guessed major/address, reopen or discovery retry.

Startup predates sysfs, so only its already validated canonical record is kept
until attachment. Attachment consumes that pending record before its attempt,
using the original epoch/event1. Later events submit directly. Canonical bytes
are `<message> epoch=<same epoch> event=<same event>\n`, without the kmsg priority
prefix. The existing512-byte kmsg bound includes that prefix and newline. PMSG
has a512-byte record ceiling and at most eight one-shot write attempts per
execution; each attempt is consumed before I/O. A short count or error disables
the descriptor without replay. There is no queue or extra native event.

`POKELINUX_PMSG_SUBMITTED` means the sole write returned the complete count;
it does not establish retention. Optional `POKELINUX_PMSG_UNAVAILABLE` reporting
cannot replace the required kmsg result or become a diagnostic-failure event.
PMSG submission and kmsg readback are separate observations: the independent
retention goal reconstructs canonical records from actual native ACKs and
compares the whole retrieved PMSG, rather than accepting a write count alone.

## Fixed RED and GREEN

The compiled baseline already had the optional wiring but discovery/submission
returned Unsupported. Three complete actual-kernel cases established healthy
ordinary native execution, real backend/console prerequisites, QMP ordering and
owned closure. Both retention cases then lacked native PMSG records. The fixed
positive-goal CLI actually exited1 with `MISSING_NATIVE_PID1_PMSG_RECORDS`.
This was a genuine missing-capability RED, distinct from earlier fixture errors.

Only `src/pmsg.rs` changed for repair. The same witness, observer, QMP profile,
receiver, positive-goal source and cases were used for GREEN; its positive-goal
CLI actually exited0. All three owned QEMU/Docker case CLIs exited0 in both the
qualified baseline and GREEN, with actual wait and merged stdout/stderr EOF.

| GREEN case | Actual child exits | Native ACKs | Retained first-boot PMSG |
| --- | --- | --- | --- |
| Success, then reset/retrieval | 37 →37 | 8 across two boots | Exact four records,445 bytes |
| Controlled child failure, then reset/retrieval | 3 →37 | 8 across two boots | Exact startup/kernel/wait3/failure,473 bytes |
| Actual backend absent, no reset | 37 | 4 in one boot | No submitted or retrieved PMSG |

The five GREEN boots produce20 ACKs with events1–4 in each boot. Each reset
case has distinct actual kernel boot UUIDs and native epochs, four exact prior
console markers, and eight matching SUBMITTED identities/counts. The retrieved
whole PMSG equals the first four ACK-derived canonical records byte-for-byte.
The read-only witness never writes PMSG or a canary in these profiles. Actual
retained console selects its second-boot branch, which exits37 even when PMSG
is missing; this makes missing records observable without faking native success.

The success and failure selections use `-no-shutdown`. After first actual child
wait and PowerOff, QMP reports guest SHUTDOWN and query-status confirms
`running=false,status=shutdown`. One authenticated reset succeeds and emits
RESET; actual post-reset status was `prelaunch,running=false`. Explicit cont
starts the second boot. Its actual wait37/PowerOff precedes the second guest
SHUTDOWN and owned QMP quit. The separate host-qmp-quit SHUTDOWN is recorded.
Terminal QMP EOF/ECONNRESET is permitted only after the successful quit reply
and required guest shutdowns, without reconnect; pipe EOF and CLI wait remain
independent requirements. The earlier receiver failures remain prerequisite
failures and were not relabeled RED.

The fixed original kernel is6.6.127-android15-8-g91d825611104-ab16252310-4k.
The previous whole-boot-blob memory checks and synthetic4MiB reservation remain:
2MiB console,2MiB PMSG, no panic-record buffer. Guest2CPU/256MiB and container
2CPU/512MiB, original45s outer/5s QMP setup bounds, dropped capabilities,
no-new-privileges, no network/root disk/host devices, exact CID/same-daemon
closure and before/after source/input hashes remain required. The three GREEN
cases retain28 direct helper waits plus three owned case CLI waits; all known
participant waits, EOF and original cutoffs were checked. The independent GREEN
review receipt is SHA256
`98ffd05d89ac1ddc451ee121dd1ea8eb3f37375daf991e27789784da3d04a373`.

## Remaining boundaries

The selected scope covers startup and child success/failure after epoch creation,
successful mounts and backend registration. Earlier RNG, mount or discovery
failures have no promised retained record. Missing PMSG is optional and the
actual no-backend control preserves ordinary execution. Required kmsg admission
still refuses failure. NONBLOCK and bounded userspace attempts do not bound
every kernel call, console write, mount or shutdown on hardware; device latency,
short/error writes and stuck kernel tasks need separate evidence.

This result adds no Pixel memory-map, bootloader/AVB admission, physical reset or
power-loss retention, panic capture, USB retrieval, independent hardware recovery,
watchdog/thermal policy, or authentication against other privileged writers.
PowerOff stays the requested action; no restart string or phone recovery path
was introduced. The full [deployment profile](native-deployment.md) remains
refused.

## Bundle and Arch consumer at the initial PMSG checkpoint

The current PMSG payload is repacked into two reproducible complete8MiB
[development diagnostic bundles](native-boot-development-bundle.md). An initial
unchanged bundle-suite replay passed. An independent artifact assertion then
found stale `diagnostic_profile.persistent_message_reporting_integrated=false`
metadata: the fixed suite actually exited1 with
`MISSING_NATIVE_PMSG_PROFILE_METADATA`. Changing only that producer boolean to
true made the same suite exit0. This is a metadata RED/GREEN, not another image
format RED. Original AOSP unpack/AVB checks, independent Rust geometry/payload
checks, exact stock restore/unchanged assets and two tamper refusals pass for
the current payload. NONE remains unsigned and does not admit deployment.

| Current artifact | Bytes | SHA256 |
| --- | --- | --- |
| Complete candidate | 8,388,608 | `80cd815edf5205679b15f253552cf060928aaa3c038f57e36807a8182432923e` |
| Compact intermediate | 1,466,368 | `01e004e27e569d3b28bf2e500ca0828ed0de12181d402970465f814b2f33567f` |
| Native init | 1,190,352 | `75e38222781c8b7ae9ab950b0caf697acb04c318419cd6c4e2f2f0f6bfd04115` |

The reproduced complete-bundle manifest SHA256 is
`546e12676b9fc1a0c4ce057490e042cb0d591686dc19bc4416696ddbd0bac990`.
The original-unpacked current generic LZ4 is
`56baaf87543a415025d7b48699b12127b7c4b1d36f6326d561f17b8aeba30592`.

Two actual Arch-kernel consumer attempts using that exact package ramdisk both
**failed** the unchanged serial ACK checker. Each had normal child37, PowerOff,
actual Docker CLI wait0, exact CID absence and unchanged inputs. In attempt1,
kernel console output interleaved inside the event2 raw ACK, so its kernel
message was refused. In attempt2, interleaving corrupted event4's completion
ACK, so completion was refused. Neither result qualifies the current package's
Arch ACK stream; successful child/shutdown/closure cannot replace intact ACKs.
No checker relaxation or third retry was used. Current Arch ACK qualification
remains open.

Earlier successful Arch/generic-LZ4 and full vendor-ramdisk executions keep their
original binary/archive bindings. The new five-boot Google native-PMSG GREEN is
separate from these failed current-package Arch consumers and establishes no
new full vendor-ramdisk runtime. See the [probe](../native-boot-probe/README.md)
and [independent witness sources](../tools/native-boot-pstore-witness/README.md).
