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

Entropy uses one nonblocking kernel RNG call and refuses short/unavailable
results. The readback deadline uses suspend-aware BOOTTIME. These bounds cover
userspace observation and retries; they cannot bound every kernel call, mount,
console write or shutdown. Same-boot kernel admission also establishes neither
PSTORE/USB delivery nor retention/retrieval across a phone reset.

The explicit tmpfs/nodes path also avoids assuming DEVTMPFS support: the selected
Google4K configuration disables it. Separate RAM-only executions qualify both
the retained Arch ARM7.1.6 kernel and the exact authenticated Google4K kernel on
synthetic QEMU virt. The latter uses unchanged native ELFs and the exact current
candidate generic LZ4; four correlated ACKs, matching child wait37 and QMP guest
shutdown pass. See [Google-kernel execution](../docs/native-boot-google-kernel.md).
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
