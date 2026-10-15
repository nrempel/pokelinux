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
Google4K configuration disables it. Earlier separate RAM-only executions
qualified the retained Arch ARM7.1.6 kernel and exact authenticated Google4K
kernel on synthetic QEMU virt. Those selections used their then-frozen native
ELFs and candidate generic LZ4; four correlated ACKs, matching child wait37 and
QMP guest shutdown passed. See
[Google-kernel execution](../docs/native-boot-google-kernel.md). The new PMSG
payload has its own five-boot Google-kernel evidence and is now packaged in two
reproducible8MiB development bundles. A fixed metadata assertion first failed
with `MISSING_NATIVE_PMSG_PROFILE_METADATA`; only the producer's PMSG-profile
boolean changed for suite GREEN. This was metadata TDD, not a new format RED.
Two exact-current-package Arch attempts both failed the unchanged ACK checker:
kernel console interleaved into event2 in the first and event4 in the second.
Both still had normal child37, PowerOff, Docker CLI0 and CID/input closure.
Current Arch ACK qualification remains open, with no checker relaxation or
third retry. Earlier Arch success and full-ramdisk evidence retain historical
bindings; no new full-ramdisk runtime is claimed. See the
[current package boundary](../docs/native-boot-native-pmsg.md#current-bundle-and-arch-consumer).
No Pixel board, bootloader, vendor module or phone is qualified.
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
