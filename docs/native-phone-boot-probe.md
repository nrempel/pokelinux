# First native boot probe

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
