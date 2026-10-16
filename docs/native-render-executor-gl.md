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

A later, separate matching native build now passes compilation and CPU/loader
checks. It does not turn this handshake RED into GREEN.

## Separate AQ15/current-ABI build and startup failure

The new full33 main and matching SDK select Rust archive 246a6680, all three current C
headers and AQ15 library222d4c55. All 16 affected C++ units compile. The SDK has 530
regular files and two exact library links. Main ELF ab8458c2 explicitly retains
and exports all 23 public Rust C functions; SDK ec484015 carries matching headers.
Its relative RUNPATH and actual loader checks select only AQ15, with no missing
or undefined relocations. [Hash-only build and failure evidence](evidence/native-render-executor-aq15-build-provenance.json)
keeps this 169.011s build distinct from every GL execution.

The build reruns five Snapshot FFI and four codec CPU groups, the C ABI lifecycle
test and five authored-owner C ABI groups, with four separately observed worker
task absences. It does not rerun the earlier 62 Rust cases. Native Scope receipts
retain 46 waits/46 PIPE EOFs; the embedded Snapshot producer adds one wait and the
producer/thread add two EOFs, for 47/48 totals, plus an actual thread join. Original
cutoffs, KVM/QMP closure, readonly input lock, exact container absence and daemon
identity remain qualified.

The exact [new executor actor](../tools/native-render-owner/executor-actor.cpp)
compiles as a shared plugin in both later attempts. The first then fails the
relocation gate on two Wayland-server symbols. Adding that explicit dependency
closes the second attempt's installer and plugin-linkage checks, but its compositor then aborts with signal 6 before
Source or the actor starts. It records zero GL cases and the message
CBackend::create() failed. That message covers both a null creation result and
start returning false; internal AQ logs were not retained, so the exact branch
and abort cause remain unresolved. Unexpected compositor exit also prevents a
normal component-cleanup qualification. Both attempts remain failed.

The second plugin has an explicit Wayland-server dependency and resolves Rust
against the matching main; it has no direct AQ DT_NEEDED entry. AQ15 is verified
in the main loader closure. The actor's source supplies a zero-output offscreen
bootstrap and worker-created native holder, with no output grant. Its controlled
unload requires Stop, nonblocking finalize/actual join, both watch removals and
release before plugin exit; unexpected unload aborts. These are compile/link-
checked contracts here, not executed GL or lifecycle GREEN.

The [separate incremental Hyprland source patch](../tools/native-render-owner/hyprland-executor-aq15.patch)
and [exact AQ15 source patch](../tools/native-render-owner/aquamarine.patch) supply
this compile-only successor. The existing Frame507 import patch remains a frozen
historical selection, never a runtime fallback. Source inspection still requires
legacy startup to integrate the control broker and render-session mirror/native
owner; it does not prove that omission caused the observed abort.

Matched worker GL execution and public Stop/finalize/join/release, ordered session
ingress and out-of-band revocation, genuine output/routing and separate startup/
Stop-retirement authority remain open. So do Source7 GREEN, AQ output/scanout/
presentation, device retirement, blocked-driver recovery, normal app/PCM, ARM/
matched Google kernel and every physical-phone/full-smartphone gate. No phone
was accessed.
