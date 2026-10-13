# Unsigned native Pixel boot probe candidate

This offline tool builds one explicitly
`UNQUALIFIED_PIXEL_NATIVE_BOOT_PROBE_CANDIDATE`. It replaces only the generic
ramdisk in `init_boot`, using the exact authored Rust PID1 and diagnostic-child
ELFs. Its new CPIO contains `init`, `probe-child`, and the empty `dev`, `proc`,
`sys` mount points, owned by root with deterministic modes and timestamps.
The PID1 creates RAM tmpfs and console/null nodes itself; it starts no Android
init, service, APK, module loader or phone backend.

All 14 original selected files are checked against frozen sizes and SHA256 pins
before and after packaging. The Google `boot`, `vendor_kernel_boot`,
`vendor_boot`, DTBO and module images remain unchanged and are referenced by hash.
The authenticated whole 11,378,664,978-byte factory archive proof is inherited
from the prior download and 38 rebound index reads; this tool rehashes the
selected files, rather than downloading or rehashing that whole archive again.
Originals, upstream sources, native ELFs and generated images remain private.
Only authored code and clean metadata belong in this directory.

The exact unchanged AOSP producer and unpacker at
`d2bb0af5ba6d3198a3e99529c97eda1be0b5a093` are verified privately with the existing
authored reference wrapper. The bounded Rust inspector is also pinned. The
producer emits header v4 with 4096-byte container pages, preserving the original
`init_boot` header except for the generic ramdisk size. Host open-source liblz4
produces the actual Google legacy framing: 8 MiB blocks, compressed sizes and an
implicit EOF. There is no modern-frame descriptor or zero end marker.

The resulting compact image is 1,454,080 bytes, below the original 8 MiB partition
capacity. Its SHA256 is
`588cabd9399e1ece30cd35373c07d5de8ee5c6654bae692879aeb912c9ac1faf`.
It contains only its newly produced container page padding. No original
signature, AVB metadata or partition footer is copied. It is unsigned,
unverified and local only; this size and format result establishes no bootloader
acceptance or temporary RAM-boot capability.

## Executed evidence

Before the packager implementation, an independent decoder checked the actual
authenticated stock `init_boot` and failed the required native `/init` hash.
The stock regular ELF is 2,942,024 bytes with SHA256
`67e58e501941ff52f7c1b409af6c1a29e4872763d1b32940db3bde7739ed3b41`;
the required native probe SHA256 is
`cbf938d20127b02dc84afe2bd0e1fc4c66bd6f2d33d33c436e7a8c9fe37d205c`.
The verifier first rejected repeated paths in the real factory CPIO; that
verifier error was retained separately, then corrected before the actual hash
failure and implementation. Repeated paths are accepted for original archives;
the newly authored generic archive must have unique paths.

Two actual candidate builds produce identical bytes. Unchanged original AOSP
unpacking extracts the exact authored compressed ramdisk, and the frozen Rust
inspector agrees on all candidate extents, signature absence and lack of opaque
trailing bytes. The independent verifier does not import the packager's encoder
or compressor. It decodes the actual concatenated vendor-kernel platform,
vendor-boot platform and native generic LZ4 frames, then reads their CPIO records.
The actual vendor `/init` symlink to `/system/bin/init` is replaced by the exact
native regular ELF in the final overlay. The native child also matches exactly.
This is a format/overlay calculation, not a live Pixel kernel extraction test.

Eight real source/file-boundary refusal groups cover changed native init,
changed child, the actual stock Android ELF supplied as native init, an existing
output label, output inside a Git worktree, unauthenticated selection metadata,
changed original AOSP producer source and a changed Rust inspector binary.
All 14 original selected files remain unchanged. Existing labels are refused
without overwriting earlier evidence; new artifact directories use mode 0700.
The tool assumes cooperative immutable inputs; it is not a hostile concurrent
filesystem snapshot authority.

The exact candidate now also has a narrower actual-kernel consumer observation.
The [consumer](../tools/native-boot-probe-package-runtime.py) extracts the produced
`init_boot` with unchanged pinned AOSP code, compares its compressed extent with
the pinned Rust inspector, then gives those unaltered legacy LZ4 bytes directly
to the retained Arch ARM7.1.6 kernel. Native root PID1 executes the child and
actually waits exit37, then powers down. Actual Docker wait0 and no timeout are
observed; elapsed wrapper time is1.987s, not a phone deadline.

This run originally tried to reproduce an early startup abort because the
generic CPIO and both selected vendor platform fragments omit `dev/console` and
`dev/null`. That RED expectation was falsified: initial stdio was usable on this
Arch kernel/console combination and the complete native probe ran. The executed
consumer recorded `passed=false`, `expected_red=false`, and exit1 because its
selected mode expected a failure. Those original fields remain unchanged in the
[runtime record](evidence/runtime-verification.json), alongside the actual
runtime GREEN observation. This is not a genuine runtime RED and demonstrates
no Pixel startup bug or need for a console-node repair. The test does not isolate
which kernel mechanism supplied initial stdio.

Only the authored generic ramdisk is passed to this VM. No vendor ramdisk,
vendor executable, root disk or NIC is attached. This standalone runtime is
narrower than the offline three-frame overlay; neither the selected Pixel
overlay runtime nor Google's kernel is qualified. The retained Arch kernel
initializes devtmpfs, while authored PID1 explicitly mounts tmpfs and creates
its own nodes. A kernel build with DEVTMPFS disabled was not executed. The
[selected vendor metadata](evidence/vendor-startup-metadata.json) was inspected
without executing original userspace. The producer, generic CPIO and candidate
remain exactly the original frozen package; no node enhancement is published.

## Fragment choice and remaining gates

For this offline candidate only, select the unchanged 4 KiB kernel in `boot`,
`vendor_kernel_boot` platform fragment 00, and `vendor_boot` platform fragment 00.
Exclude the separate `16K` fragment. The independent calculation concatenates
the two vendor frames with generic last and no intervening padding. The
bootloader's actual fragment selection and ordering remain unqualified. This
tool does not assume that temporary `fastboot boot` can substitute all needed
partitions, and it provides no deployment commands.

AVB/signing and bootloader admission, multi-partition RAM boot, Pixel kernel
runtime behavior, kernel/module KMI and signed-module admission, console/USB/
pstore observability, watchdog/thermal/charging behavior, complete native init,
SELinux and installed services, and later authorized physical-device validation
remain gates. The restricted probe's separate Arch ARM7.1.6 kernel runtime proof
is not evidence for Google's kernel. The full smartphone deployment profile
remains refused. No original vendor executable was loaded, no Google kernel was
booted, and no phone or privileged host runtime was accessed.

## Reproduction

Use existing private authenticated selection, pinned sources and original
comparison outputs. The input pins select one current component set, with no
older-format readers or general ramdisk-content options.

```sh
python3 tools/native-boot-probe-package.py \
  --authenticated-selection /tmp/PRIVATE_INPUTS/authenticated-selection.json \
  --private-reference-root /tmp/PRIVATE_REFERENCE \
  --inspector /tmp/PRIVATE_BUILD/inspect-x86 \
  --native-init /tmp/PRIVATE_NATIVE/pokelinux-boot-probe \
  --native-child /tmp/PRIVATE_NATIVE/pokelinux-boot-child \
  --private-output-dir /dev/shm/NEW_PRIVATE_LABEL
```

Exit 0 reports a successfully produced unqualified candidate. Exit 2 refuses an
input, output or comparison requirement. A candidate emitted by this tool is
never a deployment admission result. The verifier's `factory-red` action is
expected to fail; its `green` action requires the retained actual red log and
pre-implementation verifier snapshot as private provenance inputs.

The clean [candidate manifest](evidence/candidate-manifest.json) records exact
immutable component hashes, root-init ownership and all refused qualifications.
The separate [verification result](evidence/verification.json) binds original red,
reproduction, original-code and independent overlay observations.

Reproduce the narrower actual Arch consumer with a new private output label:

```sh
python3 tools/native-boot-probe-package-runtime.py green \
  --candidate-image /tmp/PRIVATE_CANDIDATE/init_boot.img \
  --private-reference-root /tmp/PRIVATE_REFERENCE \
  --inspector /tmp/PRIVATE_BUILD/inspect-x86 \
  --private-output-dir /tmp/NEW_PRIVATE_RUNTIME_LABEL
```

The consumer requires this exact candidate SHA256. Its `red` mode retains the
falsified startup-abort expectation and exits1 even when the historical probe
completes. Read `observed_native_runtime_green` separately from `passed` and
`expected_red`. Neither mode admits a Pixel deployment.

References: [AOSP vendor ramdisk concatenation and fragment selection](https://source.android.com/docs/core/architecture/partitions/vendor-boot-partitions),
[boot image header v4](https://source.android.com/docs/core/architecture/bootloader/boot-image-header),
[Linux initramfs buffer format](https://docs.kernel.org/driver-api/early-userspace/buffer-format.html),
[LZ4 legacy frame specification](https://github.com/lz4/lz4/blob/v1.10.0/doc/lz4_Frame_format.md#legacy-frame).
