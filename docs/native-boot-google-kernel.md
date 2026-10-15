# Native RAM probe on the retained Google kernel

This page preserves the preceding kmsg-only init and exact generic archive. The
[current native PMSG checkpoint](native-boot-native-pmsg.md) has its own execution
and package identities; earlier successes are not reruns of that payload.

2026-10-14UTC: the exact authenticated Pixel10 Google4K kernel executes the
unchanged native Rust RAM-only PID1 and diagnostic child on synthetic QEMU virt.
Four correlated kernel-readback ACKs, matching child execution/wait37 and QMP
guest shutdown pass. This is a new kernel/runtime evidence class, separate from
the earlier Arch ARM kernel tests and from physical Pixel qualification.
See [bounded provenance](evidence/native-boot-google-kernel-provenance.json).

The original frankelCP3A.261005.005 factory archive authentication and selected
`boot.img` identity come from the [input checkpoint](../native-boot-inputs/README.md).
Independent pinned AOSP unpacking and public liblz4 decoding bind the actual
17,315,533-byte compressed kernel to the36,866,560-byte raw Image. Its SHA256 is
`0226c3be4224b02884d4f9b332aee58ec06c65b2263265e80f2498332ef15c06`.
The embedded config is unchanged. This is execution of the retained binary,
not a reproduced source build or a claim of upstream binary equivalence.

The exact current unsigned candidate remains
`b2ba46ec4eaf8c4398ae7eb6033fed179c1cc030f4bdbcb753f30717738359a8`,
1,462,272bytes. Its extracted generic LZ4 is unchanged, SHA256
`0896081a31d739a466b7cd124335808c6641c70ca6df8ce20d0b1a7101ecd0f8`.
The native init and child are the same qualified static AArch64 ELFs as the
[diagnostic checkpoint](evidence/native-boot-diagnostics-provenance.json).
No userspace rebuild or implementation change was needed for this experiment.

## Actual execution

QEMU10.0.13 runs `virt-10.0,gic-version=3,virtualization=on`, CPUmax and explicit
multi-thread TCG, with2 guest CPUs and256MiB RAM. The network-disabled, unprivileged
container has2 CPUs,512MiB RAM and a64-PID limit, all capabilities dropped and
no-new-privileges. There is no root disk, NIC, host device, vendor fragment,
module or vendor executable. The private Unix QMP observer establishes the
paused peer's credentials against the owned live container init before `cont`.
Only capabilities negotiation, status query and continue are sent.

EL2 allows the original compiled protected-mode configuration to initialize;
the kernel and its security-mode arguments are unchanged. The actual command
line contains both its original defaults and these explicit test arguments:

```text
console=ttyAMA0,115200n8 earlycon=pl011,0x9000000 rdinit=/init panic=1
```

These arguments configure synthetic PL011 observation. They do not describe
the effective Pixel bootloader command line. No bootconfig was supplied. The
actual protected-nVHE initialization succeeds, while a no-IOMMU warning and
synthetic watchdog limitations remain in the private console. No confidential
workload or hardware watchdog qualification follows.

The native ACKs identify root PID1 and actual uname
`6.6.127-android15-8-g91d825611104-ab16252310-4k`. Events1–4 have one execution
epoch and kernel sequence IDs284–287. The actual child-exec PID56/PPID1 matches
the acknowledged actual wait exit37. COMPLETE precedes kernel powerdown and a
QMP `SHUTDOWN` event with `guest:true` and reason`guest-shutdown`.
The child wait is a source-bound producer report, without independent guest
syscall tracing. Kernel echoes and raw post-readback stdout ACKs are both retained.

Actual Docker CLI and outer-runner waits return0. The owned live container
name/image/init identity, same-daemon terminal absence and all52 frozen inputs
pass. Root independently rehashes those52 inputs, parses the raw ACKs and QMP
records, compares the retained live peer identity and performs a fresh exact-CID
absence check. The original45-second outer and5-second QMP setup budgets remain;
the native2-second child and100ms readback budgets are unchanged.

An initial data-only preflight incorrectly required the ARM64 header's effective
memory extent to equal the raw file length. The original boot protocol permits
the larger effective extent; the failed preflight remains preserved and preceded
any kernel run. A later audit-only case-sensitive Docker error assertion also
remains preserved. Neither caused a kernel retry or a runtime-input/predicate
change. The one frozen runtime passed on its first execution.

## Remaining phone gates

This proves that these native early-userspace paths execute on the selected
Google kernel in a synthetic machine. It does not qualify Pixel board/DTB/DTBO,
bootloader image and fragment selection, AVB/trust/rollback admission, vendor
module dependencies/signatures/KMI, or a real device's entropy/kmsg/console policy.
The separate [original AVB review](native-boot-avb.md) refuses the current unsigned
candidate under the retained stock contract.

No USB enumeration, warm-pstore retention/retrieval, independent reset/recovery,
thermal/charging/watchdog envelope, full signal/stuck-kernel-task policy, graphics,
native services or phone UI was exercised. The [full deployment profile](native-deployment.md)
stays refused; its Pixel runtime requirements must not be satisfied by this QEMU
result. No physical phone was accessed.

Original images, kernel bytes, native ELFs and raw console/QMP/audit records stay
private. Public provenance contains bounded facts, hashes and limitations.

References: [matched kernel boot protocol](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/Documentation/arch/arm64/booting.rst),
[QEMU Arm virt machine](https://www.qemu.org/docs/master/system/arm/virt.html).

## Later separate kernel experiments

The [warm-reset result](native-boot-warm-reset.md) qualifies actual console/PMSG
retrieval across a controlled synthetic reset with a separate authored child.
The [native service prerequisite result](native-google-kernel-services.md)
qualifies14 existing identity and three standalone lifecycle controls using a
separate test PID1. Neither result repins this original single-boot proof or
qualifies Pixel board, admission or physical recovery.

A later [complete four-frame counterfactual](native-boot-full-ramdisk.md) also
reaches native init under this kernel with512MiB guest RAM. The256MiB partial
unpack/vendor-panic failure is retained. The original single-generic run's
no-vendor-execution scope remains specific to its own selection.
