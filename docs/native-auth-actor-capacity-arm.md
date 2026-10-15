# Actor capacity controls on an ARM64 kernel

The unchanged actor capacity code from `8296587` passes its22 selected tests
under an actual generic Arch ARM kernel7.1.6-1-aarch64-ARCH. The full40-test
library inventory is enumerated only;18 other authority/storage tests are
explicitly filtered out. Both ignored-only listings are empty. The full and
selected inventories match the frozen host lists exactly.
[Bounded provenance](evidence/native-auth-actor-capacity-arm-provenance.json)
binds the source, reused ELF, adapters, logs and independent Root review.

The existing cross-compiled ELF is reused without rebuilding or changing source,
tests or deadlines. The guest directly drops groups/GID/UID in Python, recording
UID/GID1000, groups[] and CapEff0 before each exec. It uses no runuser/PAM session.
Only loopback is exposed; QEMU has no NIC. The existing generic VM image is bound
read-only and QEMU uses snapshot=on; there is no clone copy, rewrite or mutation.

The selected tests exercise actual Authority methods, typed Provider facts and
real locker pidfds. Capacity stamps are still cfg-test authored closure facts.
Passing final assertions bound to source and ELF do not prove actual scope
closure, an authentication result or a production stamp producer. The separate
host40 root-storage profile remains host evidence; this selection does not run
those storage tests on ARM.

All five listing/component processes actually wait0 with stdout EOF and proc
absence. QEMU naturally waits0 after26.763s within its unchanged360s bound; CLI
and container wait0. Root independently rehashes209 unique paths, including31
source pins in four frozen copies,30 current pins, the actual AArch64 ELF,
kernel/initrd, unchanged VM image and execution artifacts. The sole current
document difference is the README updated after source sealing. Root freshly
checks guest and both upstream host exact CIDs absent on the same daemon.

This is system QEMU with a real ARM kernel, separate from cross-compilation,
user emulation, matched Google images and physical-device evidence. No broker,
original PAM, authentication worker/cgroup, actor result/armed receipt, exact
unlock, synced durable clear or normal Messages reopening runs here. Those
gates remain open, with Source100ms and no renewal/revival. No phone is accessed.
