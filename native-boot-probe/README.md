# Native RAM boot probe

This restricted Rust `/init` is the first early-boot execution probe, separate
from the full Arch/Omarchy phone system and its refused Pixel deployment profile.
It runs only as root PID1, creates tmpfs `/dev` and standard console/null nodes,
mounts restricted proc/sysfs, starts one authored diagnostic child, actually
waits its termination, then powers down. It mounts no persistent storage or
starts phone backends. A missing helper produces failure and controlled shutdown.

The authored initial archive must provide `/dev/kmsg` (character device1:11).
PID1 verifies and retains separate nonblocking read/write descriptors before
replacing `/dev`. Each bounded record carries a random128-bit execution epoch
and a consumed event ordinal. A full write count is insufficient: Linux can
silently discard it. The reporter requires exact matching kernel-ring readback
within100ms and128 records, then writes a distinct stdout acknowledgement with
the observed kernel sequence. Console consumers require the ordered ACKs, a
matching executed/waited child, and controlled powerdown; kernel console text
alone cannot establish readback. This is trusted-writer occurrence correlation,
not authenticated PID provenance or protection against another privileged writer.
The whole kernel write, including priority and newline, is at most512 bytes.

PMSG is an optional second sink after mounts. PID1 discovers the actual dynamic
sysfs major/minor, validates the sysfs/tmpfs/opened character-device identities,
and retains one nonblocking descriptor. Only the startup record is deferred
until attachment; it keeps its existing epoch/event. Later canonical records
submit directly, with at most eight one-shot attempts and no replay after a
short write or error. Missing PMSG never changes required kmsg admission, child
ownership or PowerOff. SUBMITTED means a full write count, not retention.
[Actual native-PMSG proof](../docs/native-boot-native-pmsg.md) separately compares
whole retrieved first-boot records after post-PowerOff reset: success37→37,
controlled failure3→37, and an actual no-backend37 control. The fixed Unsupported
baseline goal exited1; changing only pmsg.rs made the same goal exit0.

Entropy uses one nonblocking kernel RNG call and refuses short/unavailable
results. The readback deadline uses suspend-aware BOOTTIME. These bounds cover
userspace observation and retries; they cannot bound every kernel call, mount,
console write or shutdown. Same-boot kernel admission also establishes neither
PSTORE/USB delivery nor retention/retrieval across a phone reset.

The explicit tmpfs/nodes path also avoids assuming DEVTMPFS support: the selected
Google4K configuration disables it. Earlier Arch/Google RAM-only, full-ramdisk
and canary experiments retain their original binary/archive bindings. The two
previous PMSG-package Arch failures remain the preserved integration RED:
kernel console interleaved inside event2/event4 ACKs and the unchanged checker
refused them, despite normal child37 and shutdown/closure.

ACK output now formats the whole line before one direct write, refusing short
counts/errors without replay. The extracted baseline helper preserved the old
formatted-write semantics. Host packet controls and host strace each passed
before and after, with one write for the traced control: there was no packet
RED, and no syscall-causality or general TTY atomicity claim follows.

The new current reproducible8MiB bundle's exact original-extracted generic now
passes the unchanged Arch consumer: four intact ACKs, matching actual child
wait37, powerdown and owned closure. The same current generic, joined to the
complete selected vendor-kernel and both vendor_boot fragments/padding, passes
the retained Google4K full-stream consumer at512MiB guest RAM. All151 historical
external pins remain; only the runner ROOT literal changes, with no acceptance/
ROM/QMP/deadline relaxation. The current native init also passes the separate
five-boot PMSG regression; those retrieval fixtures do not establish phone
retention or replace the normal packaged child. See the
[current-bundle checkpoint](../docs/native-boot-ack-and-current-bundle.md) and
[bounded proof](../docs/evidence/native-boot-current-bundle-provenance.json).

These are synthetic actual-kernel results. Physical loader/AVB/slot/DT selection,
observable diagnostics, independent recovery and current-state restoration
remain open. Real PowerOff may lose DRAM; the first compatible recovery boot's
snapshot must be exported before another initialization replaces old records.
The current payload has no qualified RAM-preserving hardware recovery transition.
No Pixel board, vendor module or phone is qualified.
The native executable has no ELF interpreter or dynamic section. The static GNU
libc build emits std-library NSS link warnings; the exercised no-network paths
execute in the RAM image with no loader/shared libraries. This is not a universal
static-libc/runtime-closure claim.

This is not a general init/service manager, diagnostic USB gadget, module loader,
SELinux installer, watchdog/thermal policy, hardware recovery mechanism or Omarchy
session. Console/device IO and mount latency on actual hardware remain unqualified.
Controlled shutdown is tested; complete PID1 signal handling and stuck-kernel-task
recovery remain work. Never execute this binary as a privileged host service.

Build and run only the isolated guest fixture in
[tools/native-boot-probe](../tools/native-boot-probe/build.sh). Both Cargo directories
must be private scratch space; preserve frozen binaries separately. The runner
attaches only the retained non-Pixel kernel and authored initramfs, no root disk,
NIC, vendor userspace or physical device. The separately frozen Google-kernel
experiment has its own observer and provenance; it does not change this runner
or its release-matched Arch checker. Output labels are never reused.
