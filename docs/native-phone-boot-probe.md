# First native boot probe

2026-10-14UTC checkpoint: the restricted native PID1 now requires kernel-ring
readback for its diagnostics. An actual-kernel absence RED against the original
init precedes the first reporter. A stronger, separately frozen witness then
observes the intermediate static messages but fails their missing execution
epoch/event correlation. After repair, that exact unchanged witness passes:
native event1 and kernel event2 share an epoch, followed by its own canary in a
complete sequence0–311 stream. The deliberately quiet `console=ttynull` guest
uses a separate actual PL011 observer and owned QMP shutdown. Root independently
rehashes all55 selected inputs and verifies exact-CID absence after actual wait0.
This witness declares its final child-exit intention; it does not independently
observe the final exit or parent wait, or the stdout acknowledgement.

Separate ordinary serial RAM runs qualify four ordered readback ACKs and actual
child execution/wait37, plus a missing-child run with three ACKs and explicit
NotFound/ENOENT failure before powerdown. Each observed wrapper takes1.744s.
An unchanged-init control with `printk.devkmsg=off` observes diagnostic-open
EPERM, no ACK/child/completion, and controlled powerdown; its ordinary healthy
aggregate remains failed. These are actual generic-kernel executions. A separate
component replay fixes the old console consumer's acceptance of preserved actual
PID1-panic output; that replay is not a fresh kernel or missing-child measurement.
See [diagnostic provenance](evidence/native-boot-diagnostics-provenance.json).

The reporter retains verified character1:11 descriptors across the new `/dev`
mount, uses nonblocking entropy for a128-bit epoch, consumes each event ordinal,
and writes at most512 bytes including framing. Exact readback has a100ms BOOTTIME
and128-record observation bound. Only then does a distinct raw stdout ACK carry
the observed kernel sequence. The release-matched
[original kernel writer](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/kernel/printk/printk.c)
can return a full write count while discarding a rate-limited or disabled message.
This is same-boot occurrence correlation with trusted restricted writers;
privileged-writer authentication, every kernel/console call's latency, silent
same-FD drop/overflow controls, cross-reset retention and retrieval remain open.

The current unsigned candidate is1,462,272 bytes and binds the repaired init.
Its independently decoded generic archive supplies `/dev/kmsg` as a0600
character1:11 node; the selected two vendor fragments plus generic frame retain
the final native `/init`. Two actual builds, original unpacking/Rust geometry and
eight refusal groups pass. Its exact extracted LZ4 executes four ACKs, matching
child wait37 and powerdown under Arch ARM7.1.6, with separate live owned-CID
identity and terminal absence checks. No selected vendor userspace or Google
kernel executes. Earlier images and their evidence below remain historical.

The effective Pixel console is unknown: compiled `ttynull` with EXTEND and no
FORCE does not establish bootloader-supplied arguments. Empty INITRAMFS_SOURCE
selects the kernel's built-in default archive, which supplies `/dev/console`;
DEVTMPFS being disabled alone does not establish its absence. Actual base DTBs
describe ramoops memory, but selected DTBOs, backend registration, warm retention
and retrieval are unqualified. Same-boot kmsg, console, PMSG and retained crash
logs are separate paths. A useful phone experiment still needs observable early
success and failure, bootloader/AVB admission and a recoverable shutdown/reset.

2026-10-13. A native Rust `/init` now runs as root PID1 under the actual retained
Arch ARM7.1.6 guest kernel. It creates a RAM runtime, starts and actually waits an
authored native child with exit37, then powers down. A separate missing-helper
case records failure and powers down. Each guest/container exits0; elapsed
guest-wrapper times are1.945s and1.995s. These are observed test times, not hardware
deadlines or a Pixel boot estimate.
Exact source, binary and preserved runtime bindings are recorded in
[provenance](evidence/native-boot-probe-provenance.json).

The whole official frankelCP3A.261005.005 archive is now independently SHA256
authenticated against Google's published checksum. All38 ZIP index reads are
rebound to the same11,378,664,978-byte stream;14 selected boot/module members are
size/CRC/hash checked. The [Rust input inspector](../native-boot-inputs/README.md)
adds bounded v4 geometry and seven exact-byte comparisons with pinned unchanged
AOSP producers/unpacker. Actual factory metadata identifies separate4K/16K
kernels and ramdisks, and puts DTBs in `vendor_kernel_boot`. The4K base kernel
banner is `6.6.127-android15-8-g91d825611104-ab16252310-4k`; many vendor module
vermagic strings differ. With modversions/protected signatures present, strings
alone establish neither KMI compatibility nor incompatibility. No Google kernel
or vendor module was executed. See [clean metadata](../native-boot-inputs/evidence/factory-metadata.json).

A genuine missing-`/init` ARM RED precedes implementation: the kernel reports
ENOENT for `rdinit=/init`, falls through to unavailable root storage and panics.
An earlier default-NIC ROM prerequisite failure and an incorrect consumer panic
signature remain separately failed. The corrected RED keeps the same no-init
image and recognizes its actual missing-init/root-fallback signature. The test
does not replace the Google kernel with the Arch kernel in a Pixel package.

The [Rust probe](../native-boot-probe/README.md) mounts tmpfs `/dev` and creates
standard console/null character nodes rather than relying on DEVTMPFS. It mounts
proc/sysfs with readonly/nodev/nosuid/noexec flags. It attaches no persistent root
filesystem and starts no phone backend. The child BOOTTIME deadline and retained
wait implementation are present, but permanently held-child/error/shutdown-signal
integration is not qualified by the two successful groups. Console/device IO,
module loading and persistent boot-log/USB diagnostics also remain work.

Three later actual-kernel controls execute the unchanged init ELF with separate
authored RAM fixtures. A permanently held child reaches failure and powerdown
in3.969s; child exit38 is actually waited and rejected before powerdown in1.716s.
A bootstrap removing `CAP_SYS_BOOT` produces real kernel EPERM on poweroff,
followed by visible PID1 panic in4.769s. These are observed wrapper times.
The shutdown consumer initially expected the wrong error/abort representation;
its failure remains preserved, and the corrected classifier checks the same
frozen console without another VM run. The held-child cleanup branch is supported
by exact source and observed poweroff, with no independent reap syscall trace:
the kernel rejected the requested tracing options. Uninterruptible kernel-task
cleanup and full PID1 signal policy remain unqualified. There was no meaningful
implementation RED and no probe change. See
[failure provenance](evidence/native-boot-probe-failure-provenance.json).
These fixture ramdisks are separate from the Pixel candidate below.

The first experimental Pixel candidate is now built offline. The
[packager](../native-boot-package/README.md) produces a reproducible1,454,080-byte
unsigned v4 `init_boot` containing the exact native init and diagnostic child.
Pinned unchanged AOSP unpacking and the Rust inspector agree on its extents.
Independent legacy-LZ4/newc decoding of the actual two selected4K vendor fragments
with generic last confirms that the native regular `/init` replaces the vendor
symlink. Two actual builds are byte-identical; eight input/output refusal groups
pass. The images and original payloads stay private.

The exact candidate's extracted generic LZ4 also runs under the retained Arch
ARM7.1.6 kernel: root PID1 executes and actually waits child exit37, then powers
down in1.987s with actual container wait0. This observation uses only the authored
generic ramdisk. Google's kernel and the selected vendor overlay are not executed.
The original consumer expected startup failure from missing initial device nodes;
that expectation was falsified. Its recorded aggregate exit1 and `passed=false`
remain unchanged beside the successful native execution observation. No runtime
RED or device-node repair is claimed. See the
[runtime record](../native-boot-package/evidence/runtime-verification.json).

The restricted RAM probe precedes the full Omarchy session. Pixel kernel startup,
bootloader fragment selection, AVB acceptance and recovery, a usable diagnostic
path, and watchdog/thermal behavior remain unresolved. Module KMI/loading is a
separate gate where modules are required. A later authorized physical test must
observe native PID1 and a recoverable diagnostic result. There is no reliable
phone-boot date from the Arch result. No physical phone is accessed here.

For modern Android devices the generic ramdisk normally resides in `init_boot`,
while `boot` contains the kernel and vendor boot images supply board resources.
A single modified `boot.img` is not an established Pixel10 procedure.
[AOSP generic boot](https://source.android.com/docs/core/architecture/partitions/generic-boot)
and [vendor ramdisks](https://source.android.com/docs/core/architecture/partitions/vendor-boot-partitions)
describe this split and concatenation order. Actual Pixel inputs must determine
the package; those documents alone do not qualify temporary `fastboot boot`.

The full [Pixel deployment profile](native-deployment.md) stays refused. Full IMS,
camera and PowerVR/Hyprland are outside the first console probe's critical path;
they remain required for the smartphone goal. A visible Omarchy session separately
needs working Pixel display/buffers/input and the native Arch service arrangement.

Reproduce on Harbour without a phone:

```sh
export NATIVE_BOOT_PROBE_TARGET=/dev/shm/pokelinux-native-boot-probe-target
export NATIVE_BOOT_PROBE_CARGO=/dev/shm/pokelinux-native-boot-probe-cargo
export NATIVE_BOOT_PROBE_REGISTRY=/home/ubuntu/Projects/pokelinux/sim/rust-cargo/registry
bash tools/native-boot-probe/build.sh
python3 tools/native-boot-probe/run.py green UNIQUE_NATIVE_PID1_LABEL
python3 tools/native-boot-probe/run.py missing-child UNIQUE_MISSING_HELPER_LABEL
```

The root-init proof remains separate from [virtual Omarchy graphics](native-virtual-graphics.md),
[original Binder evidence](native-arm64-binder.md) and any later phone measurement.
