# Native boot input inspection

This crate inspects current Android v4 `boot`/`init_boot` and vendor-boot container
geometry without reading payloads or modifying an image. It has no earlier-format
reader, image writer, flash operation or deployment admission. The read-only CLI
returns JSON geometry; it does not authenticate its input or verify AVB.

The implementation forbids unsafe Rust. It reads one bounded header and at most
4096 fixed 108-byte vendor table entries. Images are bounded to 512 MiB. Declared
aligned payload extents must fit the supplied immutable input. Header versions,
header sizes, reserved boot words, supported vendor container pages, table size,
fragment type/name/coverage and terminated strings are checked. The current
profile requires table fragments in contiguous producer order with unique names.
Trailing partition bytes are reported as opaque. Their presence proves no footer,
signature, verified-boot or bootloader acceptance property.

Boot v4 uses 4096-byte container pages. Vendor v4 accepts 2048/4096/8192/16384-byte
container pages. These alignments are independent of the ARM64 kernel page size.
The library does not choose runtime fragments from names or board identifiers.

## Independent references and reproduction

The authored wrapper downloads unchanged public AOSP files at commit
`d2bb0af5ba6d3198a3e99529c97eda1be0b5a093`, checks exact SHA256 pins and retains their
embedded Apache 2.0/BSD 3-Clause notices privately. Original sources are not
redistributed here. Original `mkbootimg.py` produces three authored fixtures;
original `unpack_bootimg.py` separately extracts components. The comparison
checks each Rust extent's exact bytes against the original extraction.

```sh
python3 tools/native-boot-inputs-reference.py prepare /tmp/PRIVATE_REFERENCE
BOOT_REFERENCE_ROOT=/tmp/PRIVATE_REFERENCE \
BOOT_BUILD_OUTPUT=/tmp/PRIVATE_BUILD \
bash tools/build-native-boot-inputs.sh
python3 tools/native-boot-inputs-reference.py compare /tmp/PRIVATE_REFERENCE \
  --inspector /tmp/PRIVATE_BUILD/inspect-x86
```

The build uses the pinned existing Rust image, network-disabled Docker, 8 Cargo
jobs, 8 CPU quota, RAM-backed targets and GNU bfd linking. A prior lld reproduction
failed with thread creation EAGAIN under the 128-pid budget and remains preserved.
The GNU bfd correction changes the build resource behavior; parser assertions
and source behavior stay unchanged. ARM64 tests execute under user-mode QEMU,
which supplies no guest kernel or Pixel hardware proof.

The private authenticated workflow additionally supplies
`--authenticated-selection /tmp/PRIVATE_INPUTS/authenticated-selection.json` to
`compare`. It requires the exact whole official 11,378,664,978-byte factory archive
SHA256 and 38 rebound ZIP metadata reads, then rehashes each selected original.
`tools/native-boot-inputs-audit.py` rechecks all selected files and binds original
unpacked components to their authenticated image bytes before bounded LZ4,
embedded-config and CPIO metadata inspection. It uses host open-source liblz4
and read-only debugfs; it never executes a factory payload. Raw originals remain
private. Its metadata result still does not authenticate AVB or qualify KMI.

## Observed factory composition

Authenticated frankel `CP3A.261005.005` has the following static layout:

| Image | Format/container page | Payload |
| --- | --- | --- |
| boot | v4/4096 | 17,315,533-byte LZ4 kernel, no generic ramdisk |
| init_boot | v4/4096 | 2,685,572-byte LZ4 generic ramdisk, no kernel |
| vendor_boot | vendor v4/2048 | 24,702,895-byte platform fragment, 15,291,325-byte unspecified-type `16K` fragment, 77-byte bootconfig, no DTB |
| vendor_kernel_boot | vendor v4/2048 | 5,343,791-byte platform fragment plus 1,447,123-byte concatenated DTB payload |

The generic CPIO `/init` is a regular ARM64 ELF. The vendor platform CPIO `/init`
is a symlink to `/system/bin/init`. AOSP specifies generic ramdisk extraction last;
a native generic `/init` must replace that symlink. The factory fragment selection
and the behavior of a RAM-boot command across these partitions remain untested
at runtime. A later [bounded ABL static review](../docs/native-boot-loader-admission.md)
identifies a conditional path that requests init_boot using a current-slot token
and prefers qualifying loaded generic data. Its vendor_boot copy uses the full
declared span, including both fragments. This is neither observed phone selection
nor a demonstrated temporary override.
The external `ramdisk_16k.img` equals the `16K` fragment byte-for-byte. The default
kernel is 4 KiB and the external alternative 16 KiB; both ARM64 Image flags and
embedded configs agree. This does not establish bootloader selection policy.

The actual 4 KiB config enables tmpfs, procfs, sysfs, Binder/Binderfs, TTY, early
serial console, pstore/ramoops, USB configfs/ACM/FunctionFS and DWC3 dual-role. It
disables devtmpfs, PID namespaces and VT. A restricted native PID1 must create
its `/dev` nodes explicitly. Those static options do not prove phone console,
USB observability, crash retention or driver initialization.

The 4 KiB kernel release is
`6.6.127-android15-8-g91d825611104-ab16252310-4k`; the 16 KiB release omits `-4k`.
Original module metadata includes GKI and vendor build release strings and one
older 6.6.98 ramdisk module. Different vermagic strings alone do not establish
incompatibility: `CONFIG_MODVERSIONS` and `CONFIG_MODULE_SIG_PROTECT` are enabled.
Exact protected-symbol/CRC, signed-module admission and live module loading remain
unqualified. The full smartphone deployment profile remains refused.

## Evidence limits

The first 26 behavior tests failed against a retained permissive baseline. A
27th original-produced 2048-page positive check then failed. The first green run
passed 25 and exposed two fixture mistakes: an incorrect trailing-byte count and
a duplicate-name negative that did not actually duplicate the original name.
Both were corrected. One baseline rerun hit a missing Display implementation
introduced by the new CLI; that compilation failure is retained separately.
The API-neutral baseline correction then ran all 27 actual failures; the unchanged
parser passed all 27 on x86 and ARM64 user-QEMU. Formatting, Clippy with warnings
refused and both release builds passed. Exact reproduced x86 and ARM64 CLIs also returned
identical geometry for all four authenticated images. Seven exact original-code comparisons
cover three authored producer fixtures and four authenticated factory images.

This inspector checkpoint booted no kernel, loaded no vendor module, executed
no original vendor userspace and accessed no physical phone. A later independent
[Google-kernel/QEMU experiment](../docs/native-boot-google-kernel.md) executes the
exact authenticated4K kernel with the authored RAM-only init. Separate
[original AVB checks](../docs/native-boot-avb.md) do not change this geometry
inspector into a signature verifier. These are original static-format/config
observations and original public-format execution plus user-mode CPU/runtime tests.
The separate RAM-resident native PID1 guest probe has its own evidence.

References: [AOSP boot image header](https://source.android.com/docs/core/architecture/bootloader/boot-image-header),
[vendor boot partitions](https://source.android.com/docs/core/architecture/partitions/vendor-boot-partitions),
[generic boot partition](https://source.android.com/docs/core/architecture/partitions/generic-boot),
[pinned producer](https://android.googlesource.com/platform/system/tools/mkbootimg/+/d2bb0af5ba6d3198a3e99529c97eda1be0b5a093/mkbootimg.py),
[pinned unpacker](https://android.googlesource.com/platform/system/tools/mkbootimg/+/d2bb0af5ba6d3198a3e99529c97eda1be0b5a093/unpack_bootimg.py),
[pinned BSD header](https://android.googlesource.com/platform/system/tools/mkbootimg/+/d2bb0af5ba6d3198a3e99529c97eda1be0b5a093/include/bootimg/bootimg.h),
[kernel module protection](https://source.android.com/docs/core/architecture/kernel/convert-or-add#signed_modules).
