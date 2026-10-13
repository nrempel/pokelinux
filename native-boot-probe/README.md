# Native RAM boot probe

This restricted Rust `/init` is the first early-boot execution probe, separate
from the full Arch/Omarchy phone system and its refused Pixel deployment profile.
It runs only as root PID1, creates tmpfs `/dev` and standard console/null nodes,
mounts restricted proc/sysfs, starts one authored diagnostic child, actually
waits its termination, then powers down. It mounts no persistent storage or
starts phone backends. A missing helper produces failure and controlled shutdown.

The explicit tmpfs/nodes path also avoids assuming DEVTMPFS support: the selected
Google4K configuration disables it. Execution is qualified only on the retained
Arch ARM7.1.6 kernel, not Google's kernel or a phone. No Pixel image has booted.
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
NIC, vendor userspace or physical device. Output labels are never reused.
