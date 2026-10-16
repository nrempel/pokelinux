# Native GL executor lifecycle checkpoint

A prepared native x86 KVM run now qualifies a missing public executor handshake
RED. All eight existing GL cases pass with 42 authenticated packets and six real
`sync_file` completion facts before six exact 192-byte RGBA readbacks. Only then
`pokelinux-owner-gl stop` returns `unknown request`. No finalize call, plugin
unload, Rust join or release occurs. [Hash-only provenance](evidence/native-render-executor-gl-provenance.json)
binds this checkpoint separately from the earlier SHM/fence RED/GREEN pairs.
The [exact executed classifier](../tools/native-render-owner/executor-handshake-oracle.py)
retains both the RED and required future GREEN predicates.

This run retains the historical Frame ELF91848cff, 507-file SDKc1775 and
Rust3cf0a142 archive. The unchanged sampler actor and four local native helper
sources compile into a test plugin. It selects no new Rust executor or rebuilt
compositor cohort. Observed local GL cleanup, DONE, socket EOF and C++ task exit
do not establish a Rust join. Retained compositor SIGKILL and actual wait−9
qualify process disposal only.

Root1 remains aggregate-failed on a copied/generated classifier-bytecode input
hash. Root2 excludes cached bytecode and disables its generation before local
imports in six entrypoints. The classifier, actor, GL oracles and runtime
requirements remain unchanged; original deadlines are preserved.

Each attempt retains 50 direct waits, 57 PIPE EOFs and one adopted-child wait.
QMP binds actual KVM, guest SHUTDOWN, same-socket EOF and QEMU0; foreground CLI0
precedes exact container absence on the same daemon. Input checks run before
and after execution under the shared lock. Fresh authenticated Source samples
bound the Stop window under the unchanged 100ms lease; a later disconnect follows
intentional teardown.

The matching AQ15/current-ABI build and actor work is authored/reviewed source
only in this checkpoint. Its build, selection and execution remain pending.
Native Source GREEN, actual AQ output/scanout/presentation, device retirement,
blocked-driver recovery, normal app/PCM, ARM/matched Google kernel and every
physical-phone/full-smartphone gate remain open. No phone was accessed.
