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

The first phone candidate is a separate RAM-resident probe before the full
Omarchy session. It still needs authenticated, structurally validated Pixel boot
composition, a selected kernel/module/page-size pairing where required, generic
ramdisk ownership of `/init`, a usable diagnostic path, and a concrete bootloader/
AVB acceptance and recovery plan. A later physical test must observe native PID1
and a recoverable diagnostic result. No physical phone is accessed here.

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
