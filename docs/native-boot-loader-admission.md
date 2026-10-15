# Pixel native-init bootloader admission

Update2026-10-15: the [complete offline development bundle](native-boot-development-bundle.md)
now has fresh unsigned metadata, an exact factory restoration image and explicit
unknown device/slot/recovery gates. It establishes no loader acceptance. The
conditional temporary-load findings below remain unchanged. A temporary RAM
override is not required for the selected future development partition experiment;
no physical access or partition operation is performed or authorized here.

The unsigned native `init_boot` is a RAM payload candidate, not an established
temporary boot request. It contains no kernel. A focused static recovery of the
authenticated factory ABL also identifies a reviewed v4 path that requests
`init_boot` using a current-slot token and can prefer its loaded generic ramdisk. Combining a kernel
and the native generic archive in one downloaded v4 image therefore does not
establish that native init will run.

These are conditional static findings, without bootloader or phone execution.
They concern the recovered ordinary boot callback, shared loading path and valid
present nonempty slot `init_boot`. Other modes, skip conditions, header branches,
OEM staging paths and board selectors remain open. See
[bounded provenance](evidence/native-boot-loader-provenance.json).

## Bound input and static method

The official frankelCP3A.261005.005 factory archive was streamed again. Its entire
11,378,664,978bytes match SHA256
`3721e6accc2e87d851ee7966d794318078ec74c7cc32343daa10968264d6bb62`;
six new ZIP metadata reads are rebound to that stream. Only the required
20,043,164-byte bootloader member is retained, SHA256
`ecf3ac959f64a1a82bbda70eed4382ee3e3a797268a941398d65f7e90c6fb39a`.
Its recorded version is deepspace-17.4-16119390. This does not identify the
firmware currently installed on a phone.

Only the3,219,456-byte ABL component is extracted, SHA256
`bb64a515c65825da6423a133882835f6839482e51a83f1cffa75e48fd1441eba`.
The unchanged Ghidra12.1.4 FBPK library independently agrees with30 directory
entry extents and first-NUL filenames. REA6.4.0 is available; this investigation
uses Ghidra's static analysis directly. Address mapping and recovered function
boundaries remain analysis inferences. No original firmware is executed.

Two tool prerequisite failures remain failed: padded filename comparison and a
missing isolated Java preferences directory. Three later analysis passes
complete; the final selection recovers33 selected functions and85 cross-references.
Fourteen analysis-driver waits are retained:13 exit0 and the known prerequisite
exit1. A separate read-only peer review exports766 instructions in12 focused
windows, checks their actual bytes and seven direct-call words against the pinned
ABL, and challenges the field/branch chain. It finds no contradiction in the
narrow conditional contract. Its one tool child and outer CLI exit0; its exact
owned container is separately absent. Root independently rehashes the338 prior
private records and15 original/candidate bindings, checks all30 FBPK extents,
and verifies the focused instruction bytes and fresh exact-container absence.
This is bounded static review, not whole-path or firmware execution. Raw binaries,
decompilation, addresses, xrefs and private audits stay private.

## Recovered conditional contract

| Boundary | Bounded finding | Still unknown |
| --- | --- | --- |
| Host request | Pinned AOSP host source supplies one downloaded boot buffer. Its additional ramdisk argument cannot modify an already formed Android image. | A separate temporary `init_boot` replacement contract |
| Dispatch | The recovered boot entry selects its callback through a dispatcher with a locked-state gate; a failed state query also refuses that path. | Actual device state, available authorization and firmware version |
| Context | The callback validates a downloaded Android image and supplies optional staged vendor/DTB/DTBO/vendor-kernel resources. Its initial `init_boot` fields are absent. | Other staging commands, modes, complete input validation and selectors |
| Loading | The shared path can build an `init_boot` lookup/request with a current-slot token; loaded-image mapping can populate that same context field. | Suffix/partition-I/O/AVB callback implementations, actual external trust anchor, stored rollback state and runtime policy |
| Generic selection | The reviewed v4 branch prefers valid present nonempty context `init_boot` over the downloaded image's generic ramdisk. | Reachability through every mode, skip condition or alternative header |
| Vendor assembly | The reviewed branch copies vendor-kernel and declared vendor-boot spans before generic data. The selected vendor-boot span contains both recorded fragments. | All board/page-mode/fragment choices and actual extraction on a phone |
| Handoff | DTB/DTBO, fixup, initrd bounds and bootconfig paths are present. | The selected tree/overlay, effective arguments and hardware handoff |

The current [three-frame overlay calculation](../native-boot-package/README.md)
deliberately chooses two4K platform fragments and the native generic archive.
It is an offline calculation, separate from this inferred full-span loader path.
Neither establishes actual fragment selection. For an aligned initial destination,
the recovered copy helper projects17 zero-prefix bytes before the whole selected
vendor-boot span and4 before generic data. The next data-only control must include
both fragments and this padding; the earlier calculation did not. The generic-last
calculation does not qualify module compatibility, driver initialization or
retained firmware. Suffix construction, partition-I/O and AVB callback
implementations remain untraced; current-slot behavior is a token-flow inference.

Boot/init_boot v4 image alignment is4096 bytes; both selected vendor image
headers declare2048-byte alignment. Kernel4K pages are a third independent
property. The selected DTBs reside in `vendor_kernel_boot`; `vendor_boot` has
no DTB and contains77 bootconfig bytes. These facts must not be conflated with
observed loader selection.

## Next admission boundary

The required result is a concrete accepted request/header/context that selects
the authored generic init while retaining required board resources. The reviewed
v4 path with valid present stock `init_boot` is an explicit negative-selection
case, not a proven native-init procedure. Alternative layouts or a RAM override
remain hypotheses; this checkpoint adds no old-header compatibility or alternate
production runtime. No temporary generic override has been established.

Original public packing/unpacking and [unchanged-Google-kernel QEMU execution](native-boot-google-kernel.md)
can qualify a proposed container and its native payload. They cannot certify the
closed loader's acceptance. The [partial original AVB checks](native-boot-avb.md)
also do not establish device trust, rollback acceptance or unlocked error policy;
the current unsigned candidate still fails the retained stock contract.

Recoverable early diagnostics, independent reset, module admission, watchdog/
thermal behavior and all physical phone gates remain separate. The full
[deployment profile](native-deployment.md) stays refused. No phone, flash/unlock
command, signing change or device operation is performed or proposed here.

References: [pinned host request implementation](https://android.googlesource.com/platform/system/core/+/a3b721a32242006b59cb12bd62c9133632af3a2d/fastboot/fastboot.cpp),
[pinned transport implementation](https://android.googlesource.com/platform/system/core/+/a3b721a32242006b59cb12bd62c9133632af3a2d/fastboot/fastboot_driver.cpp),
[generic boot](https://source.android.com/docs/core/architecture/partitions/generic-boot),
[vendor boot](https://source.android.com/docs/core/architecture/partitions/vendor-boot-partitions).

## Later full-span control

The [four-frame data and original-kernel result](native-boot-full-ramdisk.md)
now includes both vendor_boot fragments and projected copy padding. The chosen
authored-generic stream reaches native init with512MiB guest RAM; its256MiB
unpack/vendor-panic failure remains separate. This qualifies neither a request
that realizes that selection nor a change to the static loader findings above.
