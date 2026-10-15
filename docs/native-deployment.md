# Native deployment admission checkpoint

A separate [offline development diagnostic bundle](native-boot-development-bundle.md)
now contains a reproducible complete8MiB `init_boot`, fresh unsigned NONE metadata,
exact stock restoration asset and four unchanged boot/vendor/DTBO reference
images. A real successful-compact/missing-full-image RED precedes fixed-witness
GREEN with original AOSP unpack/AVB, Rust geometry and exact native payload
checks. Local NONE integrity is not stock trust or Pixel loader acceptance.
The retained stock file is not a current-phone backup; slot, actual policy,
retrievable diagnostics and independent recovery remain unqualified. No phone
operation or new kernel run occurs. The next first-boot work is native PID1 PMSG
reporting and synthetic reset retrieval; full IMS/graphics are not prerequisites
for that diagnostic experiment. Full smartphone deployment remains refused.

2026-10-12, corrected after independent review. The `native-deployment` Rust component checks a reviewed device
profile against an actual immutable staging tree and its current inventory.
The selected Pixel profile refuses deployment. Nine offline requirements and
one later physical-device requirement remain unresolved. No installed service,
phone boot or privileged host configuration is changed by this work.

## Current device and matching boundary

Update2026-10-13: the whole original frankelCP3A.261005.005 factory archive now
matches Google's published SHA256. All38 ZIP index reads are rebound to the same
authenticated stream and14 selected boot/module members are size/CRC/hash checked.
The [new v4 Rust inspector](../native-boot-inputs/README.md) adds seven exact-byte
comparisons with pinned unchanged AOSP tools; the [clean metadata](../native-boot-inputs/evidence/factory-metadata.json)
records the actual4K/16K kernels, configs, ramdisks, DTBs and module inventories.
Original payloads remain private. That input inspection executes none of them;
the separate retained-kernel experiment below has its own evidence class.

The live profile now pins the exact legacy-LZ4 kernel payload extracted from
authenticated `boot.img`, SHA256`e43d13d5b8e2657e27437bc59d8d5abd2131f255dfca26354d96e02ce43115d1`,
and selects its observed base release
`6.6.127-android15-8-g91d825611104-ab16252310-4k`. The earlier graphics/vendor
module release below is a distinct observed build. MODVERSIONS and protected
module signatures are enabled; different release strings prove neither KMI
compatibility nor incompatibility. Full dependency/symbol-CRC/signature/module
loading and complete firmware qualification remain unresolved. Complete module
and firmware artifact pins remain null. Complete AVB/bootloader admission
remains unverified; [partial original AVB checks](native-boot-avb.md) now identify
the signed chain and unsigned-candidate refusal. A later
[bounded static loader review](native-boot-loader-admission.md) finds that a
reviewed v4 path can prefer qualifying loaded init_boot generic data over the
downloaded archive. It establishes no temporary native-init procedure or runtime
admission. All nine offline
requirements and the later physical gate are still unresolved; deployment is
still refused. The separate [RAM-only native PID1 guest probe](native-phone-boot-probe.md)
does not qualify Pixel boot or installed Arch supervision.

An [experimental unsigned `init_boot` candidate](../native-boot-package/README.md)
now packages the repaired native diagnostic probe in a reproducible1,462,272-byte
container. Fresh original AOSP unpacking, Rust geometry, actual vendor/generic
overlay and eight refusal groups qualify its offline format. Its exact generic
LZ4 executes four correlated readback ACKs, matching child wait37 and controlled
powerdown under the Arch guest kernel, with separate owned-CID closure. A
separate [Google-kernel/QEMU run](native-boot-google-kernel.md) executes that same
generic LZ4 under the unchanged authenticated4K kernel, with four ACKs, matching
child wait37, QMP guest shutdown and independent exact-CID closure. No Pixel
board, vendor overlay, module or original vendor userspace executes. The earlier1,454,080-byte candidate and its falsified
missing-console expectation remain historical evidence. See
[current diagnostic verification](../native-boot-package/evidence/diagnostic-verification.json).
These observations leave the full profile, all nine offline requirements and
the physical gate refused; neither format checks nor either kernel/QEMU result
establishes Google's bootloader/AVB acceptance or recoverable phone diagnostics.
Later separate [synthetic console/PMSG warm-reset retrieval](native-boot-warm-reset.md)
and [17 native service prerequisite groups](native-google-kernel-services.md)
use the unchanged Google kernel. They add neither physical retention/retrieval
nor installed persistent service/desktop closure; the full profile remains refused.

The following paragraphs retain the original2026-10-12 audit checkpoint; their
missing kernel/whole-ZIP facts are superseded by the update above, while runtime,
KMI, AVB and hardware limits remain open.

`native-deployment/profiles/pixel10-frankel.json` selects the requested first
Pixel target and the recorded frankel `CP3A.261005.005` factory set. This checkpoint
independently checked the [authoritative AOSP device table](https://source.android.com/docs/setup/reference/build-numbers)
on 2026-10-12: Pixel 10 has codename frankel. That does not authenticate the selected
`CP3A.261005.005` build/set. Selected vendor ZIP member
size/CRC was checked, but whole outer ZIP SHA256 and AVB verification were not.

The recorded graphics module release is
`6.6.127-android15-8-gee206ffe8752-ab16254410-4k`. That is an observed module
identity, not a verified selected kernel image or KMI match. The retained factory
partition index also records a `kernel_16k` member; selecting the exact kernel,
configuration, page size, source manifest and compatible module set remains work.
The kernel, complete module set and complete firmware set have null digest pins.
They cannot be qualified by inserting an ARM VM kernel or reusing a release label.

One retained RGX firmware file has a recorded SHA256 pin. Six original audit
inputs were independently rehashed: that firmware, Shannon IMS APK, the camera
and power init files, board init, and `libsitril.so`. File identity provides no
runtime ownership, factory signature, KMI, driver/firmware matching or hardware
execution evidence. Original payloads and raw audit records remain private;
only clean paths, sizes, hashes and limits are included in this checkpoint.

## Admission rules

The current schema is version 1. It has no old-format reader or migration path.
An inventory binds exactly one profile, device, architecture and component set.
A profile declares required factory artifacts, resource owners, exact service
dependencies, capability availability and mandatory qualification requirements.
Package records declare one exact name/version/architecture/set and dependencies;
`.PKGINFO` in the actual staging tree must agree, including exact versioned
`depend = name=version` records. This is a scoped current package contract;
existing unpinned package dependencies do not qualify its closure.

The validator refuses missing or extra files, digest or mode changes, unsafe
relative paths, symlinks, hard-linked payloads, special files and duplicate
bindings. Each native executable must have a structurally checked ELF64 little-endian target
header and the selected glibc interpreter. Every `PT_LOAD` file extent must lie
inside the actual file, its memory size must cover its file bytes, and its virtual
extent must not overflow. Alignment must be zero, one, or a power of two with
congruent virtual address/file offset; load entries must have ascending addresses.
The declared entry must fall within file-backed executable load bytes. Interpreter
placement and bounded header/table/extent reads are also checked. These geometry
rules follow the [ELF ABI program-loading specification](https://gabi.xinuos.com/elf/07-pheader.html),
checked 2026-10-12. Wrong architecture, malformed checked geometry and Bionic
interpreter substitutions fail even when the inventory digest matches the changed bytes.
This is structural inspection, not proof that a system loader can execute the file. This does
not resolve ELF `DT_NEEDED`, `dlopen`, Binder, Android property or APEX dependencies;
Bionic and transitive userspace closure remain a separate unresolved requirement.

Required artifact digests must be pinned by the reviewed profile. Artifact and
package component-set labels and artifact kernel-release declarations must agree.
These checks reject a mismatched declaration/file set; they do not establish
kernel ABI compatibility. The original factory set still needs complete,
authenticated inventories and actual integration evidence.

Every required resource has exactly one declared native Rust service owner.
Service executables must belong to declared packages, all service dependencies
must exist, and cycles fail. The inventory must match the profile's exact owner
and dependency declarations; deleting a required dependency cannot make a
candidate conform. Android/Java and retained Bionic vendor service declarations
are refused. These are declarations checked against file/package identity, not
proof that the compiled program implements Rust policy or owns a live resource.

Unavailable or unknown capabilities cannot be enabled. All twelve Pixel
capability entries currently report unavailable, including MMS, eSIM and advanced
camera features. Component and synthetic installed tests do not turn a device
capability into hardware-verified availability. There is no fallback to a Pixel
backend for an unknown device, and no second device is qualified here.

The CLI bounds each JSON input to 1 MiB and denies unknown fields. Collections
are bounded to 4096 entries, staging traversal to 32 levels and each payload to
128 MiB. Hashing streams through a fixed 64 KiB chunk. ELF inspection reads bounded
headers/program entries/interpreter bytes, not the whole binary. One package
metadata body of at most 1 MiB is parsed and dropped; only its validation boolean
is retained. Ordinary payload bytes are never accumulated across files. These
bounds do not claim a universal process-RSS limit or bounded storage latency.
Reports distinguish `inventory_valid` from `deployment_admitted`.
An authored fixture can have a valid file/package inventory and still cannot
admit deployment. Omitting any mandatory requirement fails. A requirement cannot
be reclassified from offline work to physical-device work.

The report applies to a reviewed profile and cooperative immutable staging
snapshot. It is a read-only consistency result, not a reusable launch permission,
a signature verifier, runtime permission authority or installer. No installed
launch path consumes it. Concurrent hostile directory replacement and transitive
runtime closure are not qualified by this checkpoint.

## Native ownership and unresolved deployment work

The Pixel profile declares these desired service relationships. All remain
subject to the native-service integration, permission and init requirements.

| Declared owner | Resource | Required service dependencies |
| --- | --- | --- |
| permissions | Authorization | none |
| durable-storage | Durable common storage | permissions |
| radio-ims | Radio/IMS | permissions, suspend-power |
| calls | Call state | permissions, radio-ims, audio, notifications, contacts |
| messages | Messaging state | permissions, durable-storage, radio-ims, notifications |
| notifications | Notifications | permissions, suspend-power |
| contacts | Contacts | permissions, durable-storage |
| camera | Camera | permissions, durable-storage, suspend-power |
| suspend-power | Suspend/power authority | permissions |
| audio | Audio routing | permissions, suspend-power |
| connectivity | Connectivity | permissions, suspend-power |
| sensors-location | Sensors/location | permissions, suspend-power |
| display-input | Display/input | permissions, suspend-power |

Nine offline requirements remain explicit: factory integrity; exact kernel/KMI;
vendor userspace reduction; Bionic/transitive ABI closure; native boot/init and
supervision; matched SELinux policy; backend discovery and identity; installed
permissions; and native service integration. Exact factory camera interface hash,
modem producer/reset/ACK policy, native IMS/calls, AoC/DSP routes, source sensor
wake obligations, calibration and shared notification/media/contacts integration
are part of that offline work. Their unresolved status is not hardware-only.

A separate physical-device requirement retains later authorized phone boot,
carrier, GPU/display, input, camera/DMA, audio, sensors, charging/thermal and
suspend/wake measurements. No physical phone was accessed.

## Executed evidence and reproduction

Nineteen actual red tests failed against a permissive baseline before the first
implementation. One authored Bionic negative-fixture slice-length error was
corrected and the complete nineteen behavior failures were rerun; both logs are
retained. Two further red tests demonstrated omitted owner/dependency contracts
before enforcing the profile. A final actual CLI red demonstrated that oversized
JSON was parsed rather than refused; bounded loading then fixed that defect.

The original frozen 21-test checkpoint passed on x86 and ARM64 user-QEMU.
Independent review later found that it accepted malformed load geometry and
retained whole payload bodies across the inventory. Its source snapshot,
provenance, binaries and index remain retained as historical evidence. The
following correction supersedes those two claims without relabeling that proof.

Seven new red groups mutated actual compiled ELF load/file/virtual/alignment/entry
geometry while refreshing inventory digests; all incorrectly remained conformant.
One further red used syntactically valid excessive package metadata. A separate
actual x86 CLI red loaded twelve 16 MiB ordinary authored resources (192 MiB)
under a 96 MiB address-space limit. The retained-buffer implementation returned
an invalid inventory with ten `unreadable-file` issues, at peak RSS 51,712 KiB.

The corrected 29-test suite passes on x86 and ARM64 user-QEMU. The same 192 MiB
inventory then conforms under the same child limit, with observed peak RSS
11,272 KiB and only the mandatory authored-fixture deployment refusal. All staged
bytes remain unchanged. The RSS value is an observed x86 process result, not a
promise for other inventories, architectures, allocators or installed services.
 Positive fixtures
copy actual compiled Rust ELF binaries for their target. The final build uses
small stripped Rust probe executables to avoid repeatedly hashing debug metadata;
the earlier first 19-test pass used the actual compiled validator executables.
The two temporary package metadata files, kernel bytes, device/set labels,
qualification assertions and service declarations are authored fixture inputs.
They are not original kernel images or operational phone services. Malformed,
architecture and interpreter negative cases explicitly mutate temporary copies.

Formatting, Clippy with warnings denied and both target release builds pass.
The actual x86 CLI separately refuses the Pixel profile, retains all nine offline
and one physical requirement, rejects unknown fields and oversized inputs, and
leaves its temporary staging directory empty. Original reference rehashing is
separate from the authored fixture tests. ARM64 evidence is user-mode CPU/runtime
and filesystem evidence; it is not an ARM guest-kernel boot, Binder, device
userspace, Pixel KMI or physical-device result.

```sh
bash tools/build-native-deployment-correction.sh build
python3 tools/native-deployment-resident-suite.py green
python3 tools/native-deployment-evidence.py
python3 tools/index-native-deployment.py
```

Read-only CLI usage after the release build:

```sh
artifacts/native-deployment/correction-elf-resident/x86_64-unknown-linux-gnu/release/pokelinux-deployment \
  native-deployment/profiles/pixel10-frankel.json INVENTORY.json STAGING_ROOT
```

Exit 0 means the supplied reviewed declarations and staging check meet all
requirements; the current Pixel profile cannot produce that outcome. Exit 1
returns a JSON refusal report. Exit 2 rejects invocation or manifest input with a
bounded diagnostic. There is no boot, install, mutation or credential operation.

Clean hashes, tests and execution limits are recorded in
[correction provenance](evidence/native-deployment-correction-provenance.json).
The [original 21-test provenance](evidence/native-deployment-provenance.json) remains
unchanged and is explicitly superseded for ELF geometry/resident payload handling.
Existing native components remain documented in [smartphone-goal.md](smartphone-goal.md),
[device-portability.md](device-portability.md),
[hardware-boundary-decision.md](hardware-boundary-decision.md) and
[native-device-contracts.md](native-device-contracts.md).
