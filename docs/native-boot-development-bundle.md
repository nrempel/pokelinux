# Offline development diagnostic boot bundle

2026-10-15: a separate host producer turns the pinned compact native
payload into a complete8,388,608-byte `init_boot` image and private restoration
bundle. A successful compact producer followed by an actual missing-full-image
RED precedes the producer repair. Two real builds pass the same fixed witness.
This closes an offline format gap; it does not qualify a Pixel boot or admit
full smartphone deployment. [Original format provenance](evidence/native-boot-development-bundle-provenance.json)
retains its earlier payload. The current image below contains the
[native PMSG init](native-boot-native-pmsg.md), with fresh two-build format checks
and metadata RED→GREEN in [preceding PMSG provenance](evidence/native-boot-native-pmsg-provenance.json).

## Artifact and exact scope

The [producer](../tools/native-boot-development-bundle.py) calls the unchanged
[current compact packager](../native-boot-package/README.md), then uses pinned
unchanged AOSP `avbtool.py` to add fresh unsigned metadata. It emits:

- `candidate/init_boot.img`: complete raw8MiB image, SHA256
  `9bf8d4ab99a9ef82fa6c82546517cf494d64855832a4f15422b898d9f82410e7`.
- `restore/init_boot.img`: independent exact stock copy, SHA256
  `8ce3679285604688cea9c358d449a31db5fb7c0f0b49ef16c0f8c2c807f8425d`.
- `unchanged/`: exact authenticated `boot`, `vendor_boot`, `vendor_kernel_boot`
  and `dtbo` images, with sizes/hashes in the manifest. These are reference
  inputs, not instructions to write those partitions.
- `compact/`: the current payload package and its original manifest; root
  `manifest.json` binds candidate, restoration, source, tools and unresolved gates.

The first1,466,368bytes are exactly the current compact image. Its new legacy-LZ4
ramdisk pins the PMSG-capable static native init and unchanged normal child.
Current native/bundle hashes are recorded in the PMSG provenance. Fresh
algorithmNONE metadata contains one salted SHA256 descriptor
covering that whole prefix. Flags and header rollback location are0, with no
key, signature, policy modification or copied OEM footer. The rollback field
1791158400 comes from retained factory metadata, not a device measurement.
VBMeta occupies512bytes at1466368; the new64-byte footer ends at8388608.
All unused gap/padding bytes are zero. Logical zero bytes do not establish how a
future physical write or erase behaves.

These are private local files. Original images, native binaries, raw logs and
audit receipts are not published. The producer and verifier are host Python
infrastructure; the diagnostic PID1 and its child are native Rust. Packaging
executes neither native ARM payload nor firmware, kernel, vendor service or VM.
The host Rust inspector does execute. All14 selected original inputs and48
bounded tool/input files are rehashed unchanged around each qualified host run.
Whole-factory-archive authentication is inherited, not repeated here.

## Original format red and green

The following format RED→GREEN retains its preceding native payload and witness.

The frozen baseline completes the real compact producer successfully, then
fails `MISSING_COMPLETE_INIT_BOOT_8MIB`:1462272bytes instead of8388608. Python
source compilation passes before this behavioral failure. Only the bundle
producer changes for GREEN; all five other selected files, including the whole
witness, stay identical. Actual suite wait0 and both output streams close.

The [fixed suite](../tools/native-boot-development-bundle-suite.py) independently
checks the complete image using original AOSP unpacking, the retained Rust
geometry inspector, explicit footer/header/descriptor decoding and legacy-LZ4/
newc inspection. Exact native ELF bytes, the entire compact prefix, zero gaps,
restoration identity and all four unchanged assets agree. Two complete images
and their manifests are byte-identical. Original `info_image` and local
`verify_image` accept both images. A protected-payload flip and missing footer
are each refused with actual original CLI exit1 in isolated directories whose
files are named `init_boot.img`; the original hash checker resolves that basename.
The existing eight compact refusal groups retain their earlier evidence and
were not rerun by this new suite.

An early witness incorrectly required offsets of zero-sized key fields to be0.
The pinned original producer places them after descriptors. That source-bound
oracle correction precedes a fresh unchanged-baseline RED and GREEN; the first
size RED is preserved. The final publication witness also clarifies host versus
native ELF execution and removes a duplicate comparison; its fresh RED/GREEN
runs are the published checkpoint. No setup error is counted as a production RED.

## Preceding native PMSG package and current ACK successor

The new native init first passes the separate Google-kernel retention RED→GREEN.
An unchanged bundle-suite replay passes format checks but still emits the old
false PMSG flag. A new fixed artifact assertion then fails with
`MISSING_NATIVE_PMSG_PROFILE_METADATA` after producing a complete8MiB image.
Only that producer flag changes to true; the same suite then exits0. It still
requires phone retrieval and synthetic-address-added flags to be false. Two
complete images/manifests agree; original unpack, Rust geometry, native archive,
NONE integrity, restoration/reference assets and both original AVB tamper
refusals pass with all48 bounded external inputs unchanged.

This host replay executes no ARM payload. Separate Google-kernel tests use the
exact new init with a read-only witness. Two then-current-package Arch runs with
the normal child fail ACK validation because kernel console output interleaves
inside an ACK; both actual child37/powerdown/Docker0 and owned/input closures
remain observed, without qualifying that payload's Arch ACKs. No observer is relaxed.
Earlier exact-generic/full-ramdisk runtime successes keep their old identities.

The current ACK successor uses unchanged producers/suite with a new init pin.
Two full images/manifests again reproduce. Its exact full-image generic passes
the unchanged Arch checker and the retained full selected vendor/native
Google-kernel consumer; five separate PMSG regression boots also pass. The full
stream preserves both vendor_boot fragments and all45,338,032 vendor/padding
prefix bytes. See the [current checkpoint](native-boot-ack-and-current-bundle.md)
and [current provenance](evidence/native-boot-current-bundle-provenance.json).
The earlier failed Arch checks remain failed; host packet controls already pass
before repair, so no packet RED, causal mechanism or serial atomicity is claimed.

## Trust and first-device experiment gates

Local NONE verification is unsigned payload integrity, not Google's signature,
root-chain trust or unlocked-device acceptance. The candidate's empty child key
cannot match the retained stock delegated key. In pinned
[AOSP libavb](https://android.googlesource.com/platform/external/avb/+/761178607206f4cb2af79ed9eec52d8cbd814adb/libavb/avb_slot_verify.c),
well-formed unsigned metadata is a verification error; continuation requires
an explicit platform policy. Malformed metadata is a different, fatal class.
This is source inspection, not execution of libavb C or Pixel firmware. Actual
Pixel policy remains unobserved. The earlier [compact stock-contract refusal](native-boot-avb.md)
remains valid; no verification setting was changed.

Before any separately authorized phone experiment:

1. Verify actual model, installed build, slot, partition geometry and stored
   rollback constraints against the restoration set. A retained factory member
   is not a verified backup of the phone's current state; no target slot is chosen.
2. Establish independent recovery access and matching restoration assets even if
   PID1 never starts or the requested shutdown fails. No device operation is
   performed or authorized by producing this bundle.
3. Establish actual loader admission and fragment selection, a retrievable
   execution-correlated success/failure record, and bounded board/shutdown,
   watchdog, thermal and charging observations.

Native PID1 now submits its records to optional PMSG, with whole-record synthetic
Google-kernel reset retrieval and an actual no-backend control. The bundled init
still requests PowerOff; a supported physical return/recovery path remains open.
No synthetic ramoops address or guessed reboot target enters the phone artifact.

The [retained Google-kernel generic](native-boot-google-kernel.md) and
[full selected ramdisk](native-boot-full-ramdisk.md) QEMU runs remain separate
prior evidence for their earlier native payload. The current image has a new
ramdisk identity; its separate current full-stream execution is linked above.
Offline packaging itself starts no kernel. Temporary RAM override remains
unestablished and is not a prerequisite of this scoped future development
partition experiment. The [full deployment validator](native-deployment.md),
all smartphone services and every physical phone qualification remain separate.

## Reproduce on Harbour without a phone

Use a fresh private output label and the retained authenticated tools/payloads:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/native-boot-development-bundle.py \
  --authenticated-selection /tmp/PRIVATE_INPUTS/authenticated-selection.json \
  --private-reference-root /tmp/PRIVATE_AOSP_MKBOOTIMG \
  --private-avbtool /tmp/PRIVATE_AOSP_AVB/avbtool.py \
  --inspector /tmp/PRIVATE_INSPECTOR/inspect-x86 \
  --native-init /tmp/PRIVATE_NATIVE/pokelinux-boot-probe \
  --native-child /tmp/PRIVATE_NATIVE/pokelinux-boot-child \
  --private-output-dir /tmp/UNIQUE_PRIVATE_DIAGNOSTIC_BUNDLE
```

The suite takes the same six input paths, `--repo` for the frozen source tree
and a fresh `--private-evidence-dir`; it builds both candidates itself.
Pinned [original avbtool](https://android.googlesource.com/platform/external/avb/+/761178607206f4cb2af79ed9eec52d8cbd814adb/avbtool.py)
and [original boot-format tools](https://android.googlesource.com/platform/system/tools/mkbootimg/+/d2bb0af5ba6d3198a3e99529c97eda1be0b5a093/)
remain unchanged and private reference inputs. No flash or unlock script is supplied.
