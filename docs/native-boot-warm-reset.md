# Retained diagnostics across synthetic warm reset

This page retains the original canary experiment. The later
[native PID1 record checkpoint](native-boot-native-pmsg.md) separately qualifies
whole native success/failure records after actual child wait and PowerOff.

The unchanged authenticated Google4K kernel and unchanged production native init
recover the exact162-byte first-boot PMSG canary and21,362-byte prior console
after one controlled QEMU reset. Both previous native startup markers survive.
The second boot has distinct kernel boot and native execution identities, then
matching child wait37, completion and guest shutdown. Only the second child is
observed waited; the first is reset. [Bounded proof](evidence/native-boot-warm-reset-provenance.json).

This is actual original-kernel execution on synthetic QEMU virt. It qualifies a
console/PMSG retention mechanism under explicit RAM parameters, separate from
Pixel loader admission or recoverable phone diagnostics. The
[authored Rust witness and Python observer](../tools/native-boot-pstore-witness/README.md)
are published unchanged from the executed selection. Raw logs, retained bytes,
original kernel and generated archives remain private.

## Actual boundary

The independent witness runs as the production probe's child. It verifies the
selected kernel and pstore/procfs/sysfs identities, discovers PMSG's dynamic
major/minor from sysfs, creates a0600 character node and validates the opened
node's type/rdev. On the first boot it generates a128-bit nonblocking random
canary, writes one whole record and reports READY. The owned host observer then
requests one authenticated QMP system_reset. READY or a successful write count
alone does not qualify retention.

On the second boot the witness reads the actual pstore PMSG and prior console,
reports their bounded whole bytes and exits37. The observer requires exact
canary equality, different boot UUIDs/native epochs, the two prior native markers,
six native ACKs in two boot-scoped sequences, the second matching wait37 and
COMPLETE, a successful reset reply and RESET before final guest SHUTDOWN. Root
independently compares the retrieved whole files and raw output, rehashes the
selected bindings and freshly checks exact-container absence on the same daemon.

QEMU provides256MiB physical RAM; Linux's explicit252MiB limit reserves the last
4MiB for2MiB console plus2MiB PMSG. record_size0 supplies no panic-record buffer.
These are synthetic addresses, not a selected Pixel memory-map qualification.
Before cont, read-only info roms bounds all four actual RAM boot blobs, including
the kernel's effective memory extent, below that region. The three exact
addressless fw_cfg table regions are separately classified against original
QEMU loader source; Linux mem= alone would not constrain QEMU reset writes.

Compiled protected-mode kernel arguments, production init, two-second child
budget and100ms readback bound remain unchanged. The outer45s and QMP setup5s
bounds,2CPU/256MiB guest,2CPU/512MiB container, dropped capabilities, no-new-privs,
no network, root disk or host devices remain. The positive runtime wrapper takes
2.190s; this is an observed synthetic runtime, not a hardware deadline.

## Refusals and evidence classes

| Selection | Actual result | Scope |
| --- | --- | --- |
| Warm epoch1 | info-roms parser refuses an addressless firmware-table row while paused; no cont or reset. Exact owned container is killed, CLI waited137, absence/byte closure pass | Prerequisite failure; no guest/backend inference |
| Fresh warm epoch2 | Exact canary and prior console recovered after one reset; second child wait37, guest shutdown, CLI0 and all90 bindings unchanged | Original kernel plus synthetic warm reset |
| Fresh no-backend | Same witness/init/archive without ramoops parameters: actual NotFound/errno2, child wait3, native failure ACK, no READY/retrieval/reset/COMPLETE. Positive observer fails; separate negative classifier passes. CLI0 and92 bindings unchanged | Actual kernel negative, not a production defect RED |
| Five saved-output controls | Saved positive accepted; stale canary, stale prior native epoch, missing successful reset reply and reversed RESET/SHUTDOWN rejected | Component replay; no new kernel execution |

An optional host readelf prerequisite was unavailable; exact ELF bytes were
independently parsed before freezing. Earlier failed epochs remain unchanged.
No production code was repaired by this experiment.

## Remaining recovery gates

The selected Pixel DTB/address, real reset retention, physical retrieval/USB,
bootloader/AVB request, independent recovery, watchdog/thermal safety and failures
before backend registration remain unqualified. RAM retention does not establish
power-loss persistence. Console/PMSG do not establish panic-record capture, and
canary correlation does not authenticate privileged writers. The
[deployment profile](native-deployment.md) remains refused.

Primary sources: [matched common-kernel ramoops documentation](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/Documentation/admin-guide/ramoops.rst),
[QEMU10.0.13 loader](https://gitlab.com/qemu-project/qemu/-/blob/v10.0.13/hw/core/loader.c).
The selected original binary was not rebuilt from those public sources.
