# Unsigned native Pixel boot probe candidate

The separate [development bundle](../docs/native-boot-development-bundle.md)
now consumes this unchanged compact producer with current native pins and emits a full8MiB unsigned
`init_boot`, exact stock restore image and unchanged boot/vendor/DTBO reference
assets. Original-tool format/hash checks and two-build reproducibility pass after
a real missing-full-image RED. Pixel admission and recovery remain unqualified;
this is not full smartphone deployment. The compact format contract below is unchanged; current runtime limits are explicit.

This offline producer authors only the generic ramdisk in `init_boot`, using
one exact pinned native Rust PID1 and diagnostic child. The current archive
contains `dev`, `dev/kmsg`, `proc`, `sys`, `init`, and `probe-child`. The initial
`dev/kmsg` entry is a root-owned character device, mode0600, rdev1:11, with no
payload bytes. Directories are mode0755; both static ARM64 ELFs are mode0755.
All entries have deterministic ownership, timestamps and distinct inodes.
There is no Android init, vendor service, module loader or phone backend.

The current compact image is1,466,368bytes with SHA256
`b9b3be959ac35c2de1290877a8acb8bc1a856f02b841e28195dd963fc55eefdd`.
Its PMSG-capable native init SHA256 is
`c47752349f7df514708d6046904bac66782bfecc5cb7d7fce8ec69f5b49dd7ad`;
the normal child remains
`3b15eb4a9fd289e6664c917861f98bbe385e117e26b28abb149b3021e4b19e63`.
Two fresh full-bundle builds contain the exact same compact prefix and manifest.
Original AOSP unpack/Rust geometry, independent native archive bytes and unsigned
integrity/restoration/reference checks pass. See
[current bundle provenance](../docs/evidence/native-boot-current-bundle-provenance.json).

The exact current original-extracted generic passes the unchanged Arch consumer
with four intact ACKs, matching normal child37, powerdown and owned/input closure.
That same full-bundle generic joined to the unchanged complete vendor prefix
passes the existing Google4K full-stream consumer at512MiB. Both vendor_boot
fragments and all padding remain. The current init separately passes five PMSG
regression boots with a read-only witness. See the
[current checkpoint](../docs/native-boot-ack-and-current-bundle.md).

Two Arch ACK failures from the preceding PMSG-only payload remain preserved.
Its host packet controls already pass; the ACK change and current kernel passes
do not establish universal serial atomicity or a proven corruption mechanism.

The preceding1,462,272-byte compact image and earlier native init retain their
successful two-build/original-tool/three-frame/eight-refusal and Arch/Google-kernel
executions. [Earlier package/Arch verification](evidence/diagnostic-verification.json)
and [Google-kernel proof](../docs/native-boot-google-kernel.md) keep those exact
bindings. This revision does not rewrite historical evidence or accept old
native pins as current. The current candidate remains
`UNQUALIFIED_PIXEL_NATIVE_BOOT_PROBE_CANDIDATE`, unsigned and local only.

## Current verification contract

All14 authenticated selected originals are checked before and after packaging.
Their whole11,378,664,978-byte official factory archive authentication and38
rebound index reads are inherited; the packager does not rehash that entire
archive. Packaging leaves Google kernel, vendor ramdisks, DTB, DTBO and module
images unchanged and executes none of them. The earlier separate Google-kernel
experiment does not execute vendor fragments/modules or a Pixel board.
Original payloads, native ELFs and generated images
remain private; only authored tools and bounded metadata are public.

Unchanged AOSP mkbootimg/unpack_bootimg at
`d2bb0af5ba6d3198a3e99529c97eda1be0b5a093` and the pinned Rust inspector must
agree on header-v4 geometry and actual compressed bytes. The original header
is preserved except for ramdisk size. Host liblz4 emits the actual legacy
8MiB-block framing with implicit EOF. New page padding is zero; no signature,
AVB footer or opaque trailing bytes are copied. The image must fit the original
8MiB partition capacity.

The schema2 manifest lists all six paths and the character-node metadata. The
independent archive decoder retains actual newc device/rdev fields and requires
`dev/kmsg` to be a character device1:11, not a regular file. It verifies both
native ELF contents against the current `input-pins.json`, then compares the
actual AOSP extraction and Rust extents. It also decodes the three actual
vendor-kernel platform, vendor-boot platform and generic LZ4 frames with generic
last and no intervening padding. Final `/init` must be the exact native regular
ELF, replacing the vendor symlink; final `dev/kmsg` must come from the authored
generic frame. This is an offline overlay calculation, not Pixel extraction
or execution. The later [bounded static loader review](../docs/native-boot-loader-admission.md)
identifies a conditional full-span path including both vendor_boot fragments,
with projected helper padding; this chosen-three-frame check does not qualify
that path. It also finds that qualifying loaded stock init_boot can supersede a
downloaded v4 generic ramdisk. This package alone establishes no temporary
native-init request.

The suite retains two-build byte reproducibility and all eight real refusal
groups: changed init, changed child, actual stock Android init supplied as
native init, an existing output label, output inside an actual private Git
fixture, unauthenticated selection metadata, changed original AOSP source and
changed Rust inspector. No output is overwritten. Inputs are cooperatively
immutable; this is not a hostile concurrent-filesystem snapshot authority.
The current native diagnostic RED is established separately by the diagnostic
harness; a historical stock-init mismatch does not prove that new boundary.

## Standalone Arch consumer and retained prior success

The [runtime consumer](../tools/native-boot-probe-package-runtime.py) accepts
only a schema2 candidate whose image, manifest, actual archive and native hashes
agree with the current pins. It has no hardcoded historical image digest and no
startup-console RED mode. It uses unchanged original AOSP extraction and gives
the exact unaltered legacy-LZ4 extent directly to retained Arch ARM7.1.6.
Only the authored generic archive is attached: no vendor archive/executable,
root disk, NIC, host device or phone.

Acceptance requires the shared
[diagnostic checker](../tools/native-boot-probe/diagnostics.py). Four raw-stdout
`POKELINUX_DIAGNOSTIC_ACK` lines must acknowledge actual correlated kernel-ring
readback: one32-lowercase-hex epoch, canonical event IDs1–4 and strictly
increasing bounded kernel sequence IDs. Their original messages must identify
native root PID1, the actual kernel, the matching child wait37 and completion.
The separate actual child-exec PID must match that acknowledged wait. Panic,
refusal, failure, missing/malformed/reordered ACKs or missing powerdown cannot
pass. A kernel console echo or successful `/dev/kmsg` write count is insufficient.
Same-boot readback does not qualify pstore retention or phone diagnostics.

The successful preceding-payload run observed events1–4 with kernel sequence IDs312–315 in
one epoch. Its actual child-exec PID matched the acknowledged wait37, followed
by COMPLETE and controlled powerdown. No panic, refusal or probe/diagnostic
failure was present. Actual Docker and consumer CLI waits were0; live exact-CID
identity, final exact-CID absence and same-daemon checks passed. Runtime elapsed
was1.923s, a wrapper measurement rather than a phone deadline. All15 runtime
input/source hashes were unchanged; the separate freeze also retained all7
selected source files and32 external inputs. The final shared checker SHA256 is
`e5021893640110dde0a80b82967f9760ec4bbe46dd3711a04bc3e24e8ecd6ffd`.

The owned Docker CLI uses a fresh exclusive output label and CID file. The
observer tolerates bounded registration delay only for that same published CID,
checks live name/image/identity, actually waits the CLI, and independently checks
exact-CID absence against the same daemon. Timeout cleanup targets only the
owned CID. Killing or waiting the CLI does not establish container cleanup;
any unresolved owner remains explicit and prevents a successor. Selected
candidate, manifest, kernel, extracted ramdisk, current pins, checker, authored
consumer/decoder and original tool-source hashes are rechecked after execution.
A current runtime pass requires all of those closures as well as actual ACKs.

## Preserved historical evidence

The original compact candidate is1,454,080bytes with SHA256
`588cabd9399e1ece30cd35373c07d5de8ee5c6654bae692879aeb912c9ac1faf`.
Its original five-path CPIO and native pins differ from this successor. Two
actual builds, independent AOSP/Rust/three-frame format checks and eight refusal
groups qualified its offline format. Its standalone Arch consumer executed the
child, actually waited exit37 and powered down. Those observations remain in the
historical [candidate](evidence/candidate-manifest.json),
[verification](evidence/verification.json) and
[runtime](evidence/runtime-verification.json) records; this revision does not
rewrite them or accept that old manifest as current output.

The original attempted missing-console RED was falsified: its complete native
probe ran. The historical consumer therefore recorded `passed=false`,
`expected_red=false`, exit1, while separately observing native completion.
It was not a genuine runtime RED or proof of a missing-node bug. The new kmsg
node serves the explicit diagnostic interface and is not a revival or repair
claim for that historical hypothesis. Static common-kernel source also supplies
a default initial console node for the selected empty initramfs-source setting;
that source observation is not actual Pixel startup evidence.

The retained stock-init RED predates the original packager and is bound to its
own frozen verifier and original required native hash. The successor suite
labels it as historical packager provenance, independently of the new native
reporter/correlation chronology. Earlier verifier failures and all failed
runtime records remain preserved.

## Fragment choice and open phone gates

The offline calculation selects the unchanged4KiB `boot` kernel and platform
fragment00 from `vendor_kernel_boot` and `vendor_boot`, excluding the16K fragment.
Actual bootloader selection, DTBO application and vendor-plus-generic runtime
are unqualified. The tool provides no deployment commands and does not establish
that temporary RAM boot can replace all required partitions.

The [original AVB review](../docs/native-boot-avb.md) confirms partial stock
metadata/descriptor consistency and refuses this unsigned compact candidate.
Complete AVB/signing and bootloader admission, multi-partition temporary boot,
actual Pixel board runtime, module KMI/signature/dependency admission, USB or warm-pstore
retrieval, bounded shutdown and watchdog/thermal/charging behavior, complete
native init/SELinux/services and authorized physical validation remain open.
A generic-kernel same-boot marker readback cannot establish those properties.
Full-phone deployment remains refused.

## Reproduction

Use a fresh private label and exact authenticated selection, original references,
Rust inspector and the exact current native binaries. Existing labels are
refused without replacing earlier evidence.

```sh
python3 tools/native-boot-probe-package.py \
  --authenticated-selection /tmp/PRIVATE_INPUTS/authenticated-selection.json \
  --private-reference-root /tmp/PRIVATE_REFERENCE \
  --inspector /tmp/PRIVATE_BUILD/inspect-x86 \
  --native-init /tmp/PRIVATE_NATIVE/pokelinux-boot-probe \
  --native-child /tmp/PRIVATE_NATIVE/pokelinux-boot-child \
  --private-output-dir /dev/shm/NEW_PRIVATE_CANDIDATE

python3 tools/native-boot-probe-package-runtime.py green \
  --candidate-image /dev/shm/NEW_PRIVATE_CANDIDATE/init_boot.img \
  --private-reference-root /tmp/PRIVATE_REFERENCE \
  --inspector /tmp/PRIVATE_BUILD/inspect-x86 \
  --private-output-dir /tmp/NEW_PRIVATE_RUNTIME_LABEL
```

Producer exit0 means an unqualified candidate was produced. Runtime exit0 means
only its narrow actual Arch/ACK/owned-closure contract passed. Neither admits a
Pixel deployment. The independent `green` suite requires the retained historical
factory RED log and its frozen pre-implementation verifier as provenance inputs.

References: [AOSP vendor ramdisk concatenation and selection](https://source.android.com/docs/core/architecture/partitions/vendor-boot-partitions),
[boot header v4](https://source.android.com/docs/core/architecture/bootloader/boot-image-header),
[Linux initramfs format](https://docs.kernel.org/driver-api/early-userspace/buffer-format.html),
[LZ4 legacy framing](https://github.com/lz4/lz4/blob/v1.10.0/doc/lz4_Frame_format.md#legacy-frame).

## Later complete-span kernel control

The [full four-frame experiment](../docs/native-boot-full-ramdisk.md) separately
retains both vendor_boot fragments and projected helper padding. Its explicit
authored-generic counterfactual reaches native init under the unchanged Google
kernel with512MiB guest RAM; the256MiB partial-unpack/vendor-panic failure is
preserved. This does not modify this package's chosen-three-frame calculation
or establish the loader request needed to select authored generic data.
