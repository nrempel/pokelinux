# Full vendor/native ramdisk under the original kernel

The [current diagnostic bundle](native-boot-ack-and-current-bundle.md) separately
passes this existing full-stream consumer with its exact new generic and normal
child, retaining the complete vendor prefix and both vendor_boot fragments. Its
46,799,452-byte stream has distinct current source/artifact provenance. The
earlier full-stream checkpoint below retains its recorded init and bytes.

## Earlier initial full-stream checkpoint

The unchanged authenticated Google4K kernel now unpacks the complete padded
four-frame native-generic counterfactual and reaches native Rust PID1. Four
correlated diagnostic ACKs, the selected uname, matching child wait37, completion
and QMP guest shutdown pass. This uses512MiB guest RAM and a1GiB container; the
prior256MiB run failed. See [bounded proof](evidence/native-boot-full-ramdisk-provenance.json).

This is a chosen RAM-only experiment on synthetic QEMU virt. It supplies no
Pixel loader request that selects authored generic data. The
[reviewed loader path](native-boot-loader-admission.md) can still prefer qualifying
loaded stock init_boot. Pixel board/module/firmware qualification remains open.

## Data selection and actual kernel result

The independent offline control retains the complete5,343,791-byte vendor-kernel
span and39,994,220-byte vendor_boot span, including both recorded fragments.
The recovered helper's aligned-start projection inserts17 zero bytes before
vendor_boot and4 before generic. Original liblz41.9.4 and libarchive3.7.2 agree
with every decoded block, record and payload; the original AOSP unpacker checks
the kernel-bearing downloaded-image data fixture. The namespace model begins
with a root directory only, not an executed kernel/VFS.

| Data case | Bytes / frames / records | Projected final state |
| --- | --- | --- |
| Qualifying present stock init_boot |48,023,604 /4 /1,166 | Stock init, root character kmsg1:11 mode0755; no native child |
| Explicit authored-generic counterfactual |46,794,223 /4 /1,135 | Exact native init/child; root character kmsg1:11 mode0600 |

Seven separate source-model vectors cover overwrite/hardlink cases. The real
archives contain zero regular hardlink records; their hardlink behavior is not
claimed exercised. These data cases execute no firmware or ELF. Only the native
counterfactual is then supplied to the original kernel in the runs below.

The first kernel run retains the previous256MiB guest/512MiB container envelope.
It reports initramfs write error and fallback initrd.image ENOSPC(-28), starts a
partial vendor init, loads vendor modules and panics in a vendor scheduler module
on synthetic QEMU. Native init emits no ACK. The failed run remains failed;
actual Docker wait0 and container absence do not turn a kernel panic into success.

Matched public source selects tmpfs rootfs for these parameters and defaults its
block limit to half totalram_pages. A fresh run changes only guest RAM to512MiB
and container memory to1GiB, plus the physical-RAM bounds assertion. Exact kernel,
full archive, native ELFs, compiled/test arguments, CPU/PID limits,45s outer,
5s QMP setup,2s native child and100ms native readback bounds remain. There is no
module removal or altered filesystem/security mode. The successful memory-only
control supports the capacity diagnosis; the original failing xwrite is not
independently traced.

Before cont, the owned authenticated QMP observer bounds all four actual boot
blobs and the kernel's effective extent inside the selected physical RAM. The
512MiB run has no unpack-error, vendor-init or panic log and observes native
ACKs1–4 in one epoch with increasing kernel sequences, matching child wait37,
COMPLETE and actual guest shutdown. Root rehashes151 positive and143 predecessor
external bindings, checks whole serial/QMP output and freshly verifies both exact
containers absent on the same daemon. A separate first-label name mismatch was
refused before any container/kernel execution and remains preserved.

## Remaining gates

This qualifies actual kernel unpacking and the source-bound native payload for
this supplied stream. It proves neither the closed loader's generic selection
nor a complete module/KMI/firmware or Pixel board path. The failed partial vendor
runtime is reference evidence, separate from native production architecture.
Not every kernel usermode helper/module invocation was independently traced.
Usable phone diagnostics/recovery, watchdog/thermal behavior, display/input,
full native init and installed persistent services remain required. No physical
phone is accessed.

Primary sources: [matched rootfs selection](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/init/do_mounts.c),
[matched tmpfs limits](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/mm/shmem.c).
