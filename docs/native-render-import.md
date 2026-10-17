# Rust-owned immutable SHM import

The [retained CPU surface-content checkpoint](native-render-surface-content.md) now passes unchanged-consumer five-group C++/Rust integration after actual geometry RED. Source extent uses retained Rust pixels after producer overwrite/unmap/close; no-attach identity, transforms, detach/replacement and independent clones pass. The two-file patch changes CPU geometry only; core GL capture/upload, normal Wayland frames, Source7 and phone gates remain open.

The original [`native-render`](../native-render/README.md) library and
[connected compositor patch](../tools/native-render-import/README.md) now compile
as one native x86 Hyprland cohort. A real C++→Rust integration helper also runs
under the actual Arch x86 KVM kernel and passes five groups. This is a CPU payload
ownership prerequisite. GL still runs on the original compositor owner, and the
[seven-case rendering-stall baseline remains RED](native-render-authority.md).

## Actual implementation boundary

`commitState` calls `updateSynchronousTexture`, which captures the client's exact
pixel extent into bounded immutable Rust storage and ends the client's data view
before texture creation/update. Const texture inputs consume those owned bytes.
The surface-state wrapper explicitly clones, moves and releases Rust handles;
normal SHM buffer release leaves the snapshot available. Cursor CPU reads use the
same snapshot instead of opening the client's memory again. The C ABI carries
format, width, height, stride, bytes per pixel and monotonic import identities.
These identities identify content imports, not Wayland resources or authority.

Distinct live pixel allocations share one atomic byte budget across clones.
Validation and capacity refusal publish no partial snapshot. Bulk pixel reserve
is fallible; `Box`/`Arc` metadata allocation can still abort on exhaustion. There
is no full process out-of-memory recovery guarantee.

The public patch is complete against pinned upstream Hyprland
`efb50993780079460b0cbed1363e2166a2de1d9f`. Its33-file before/after map preserves
the original observations, immutable imports, Cairo repair and scene prerequisites,
then adds the bounded frame codec and owned snapshot adoption. It contains no
proprietary source or unpublished patch prerequisite. Every plugin must use the
complete matching rebuilt SDK. Earlier18-file import and21-file cursor cohorts
retain their own evidence; their no-attach/Cairo oracles were not rerun on31.

The later [immutable frame CPU checkpoint](native-render-frame.md) keeps the
historical FULL33/private-header native build separate from the new public-header
source selection. It qualifies real value/snapshot custody, without executing the
live scene producer, GL primitives or a production render worker. A separate
fresh public-header native build now passes all16 affected units, its matching
507-file SDK, five snapshot FFI and four codec CPU groups.

## Executed builds and C++ lifetime boundary

The std-only Rust library passes formatting, Clippy with warnings denied and a
Release x86 staticlib build. Its original driver later fails an artifact-location
assertion: Cargo actually writes under `/build`, whereas the driver assumes
`/build/target`. A separate Root review qualifies the actual Cargo artifact,
compiled source and owned closure without rerunning the compiler. That failed
driver result remains preserved. Earlier Python/toolchain/formatting setup
failures supply no behavioral RED.

A separate native guest compiles the actual C++ helper against that exact Rust
archive/header. Five groups pass:

- Native ABI layout and store admission.
- A real shared-memfd producer mutates, unmaps/closes and exits after capture;
  the snapshot retains every original byte, including row padding.
- An explicitly cloned handle is transferred to a real C++ thread; it remains
  valid after the original handle and store are released, then is released and
  the actual exited thread is joined.
- Clones charge one allocation, refusal preserves quota and final release
  returns it for reuse.
- Invalid identity, reserved fields, shape or byte length produce no snapshot
  and consume no quota.

The helper emits five behavior groups plus two custody records, not seven
production rendering tests. The producer has actual wait0, pidfd death, process
absence, ECHILD and PIPE EOF; the worker has observed task disappearance, actual
join and PIPE EOF under its original5s work/8s close cutoffs.

The same guest builds full Hyprland and all generated protocols with explicit
Rust archive, header and actual target native libraries. All nine changed C++
translation units occur in the fresh successful build. The complete matching SDK
contains501 regular files and the exact C ABI header. This175.325546004s build
closes33 Scope waits/33 PIPE EOFs, counting the early runtime mask once; the helper's
producer wait and thread join remain separate. Same-peer QMP KVM, guest shutdown,
QEMU0, foreground CLI0, exact same-daemon container absence and unchanged locked
inputs close independently. No compositor, Source, UI or PCM executes in this
build-only epoch. [Hash-only provenance](evidence/native-render-import-provenance.json)
binds the Rust, C++/cohort and closure results separately.

## Actual no-attach identity RED

One actual native x86 compositor case now qualifies RED on the initial cohort.
The real UID1000 client creates an unassigned64×64 XRGB8888 surface, attaches SHM
and completes `wl_display_sync`. A test witness retains a separate Rust clone.
A second `U` issues damage/frame/commit without attaching another buffer, then
completes another real sync before the live witness check. Import resource,
generation and commit identities change from `(7,7,7)` to `(8,8,8)` while the
16,384-byte original pattern remains identical. The stable-identity expectation
is unchanged; this is the observed recapture defect, without a measured quota-
exhaustion or upload failure claim.

Controller1 is the expected property failure; outer0 qualifies that RED and owned
closure. All42 actual waits/49 PIPE EOFs meet original cutoffs, counting the early
mask once; watched descendants are dead/absent and ECHILD, same-peer QMP KVM/guest
shutdown/EOF/QEMU0, CLI0, exact same-daemon container absence and unchanged locked
inputs close separately. After the client's actual exit, the witness's separate
Rust clone still retains the original bytes. This does not establish server-side
surface/resource retirement. Source and Observer are not selected in this case.
[Hash-only provenance](evidence/native-render-import-provenance.json) binds the
actual result, source actors and physical closure.

The [exact public actors and oracle](../tools/native-render-import/noattach/README.md)
match selected RED4 bytes. Earlier attempts remain separate: old SDK pins fail
setup; a test-only cursor borrow triggers a compile warning; a third attempt
observes component identity RED but fails the outer tuple/list epoch join. None
is relabeled the fully qualified RED4 epoch. The semantic identity oracle remains
byte-identical.

## Rebuilt repair and no-attach GREEN

The repair changes only `Compositor.cpp`: synchronous capture now requires the
incoming `state.updated.bits.buffer` as well as a synchronous current buffer.
The other17 cohort payloads are unchanged. That complete18-file patch
contains this repair. Its separate full native rebuild and the same five FFI
groups pass in173.836398179s with33 Scope waits/33 PIPE EOFs and a matching501-file
SDK; producer wait and thread join remain separate. Compositor/Source/UI/PCM do
not execute in that build.

A later actual native no-attach run against that repaired ELF/SDK passes the same
one-case contract: current identity remains `(7,7,7,16384)` before and after the
no-attach commit, with the full original pattern and retained client buffer.
Client, witness, controller, semantic oracle and epoch join are byte-identical to
qualified RED4. Controller0 and outer0 qualify this CPU identity GREEN; all42 waits/
49 PIPE EOFs and the same serial/QMP/KVM/VM/container/locked-input closure pass.
The separately retained clone still verifies after actual client exit, then close
releases one handle and leaves zero. Server-resource retirement remains unqualified.
The initial RED/build and repaired build/GREEN retain separate
[hash-only provenance](evidence/native-render-import-provenance.json).

`texture->ok()` checks returned texture-object validity rather than GL upload
success. Actual upload failure, completion and output pixels remain unqualified.

## Actual padded cursor consumer RED

A later behavior-preserving baseline extracts the existing Cairo input constructor
into `Pointer::cursorCpuInputSurface`. The production `PointerManager` callsite
passes the retained source stride, while the baseline helper still uses `width*4`.
Its separate full native build and the same five FFI groups pass with a matching
502-file SDK, including `CursorImage.hpp`. All11 changed C++ units have actual
build lines;33 Scope waits/33 PIPE EOFs and the original owned closure pass. This
174.777423764s build executes no compositor or Source and is a local source cohort,
not a reproduction of the signed Arch package recipe.

One later actual native case qualifies consumer RED. The real UID1000 Wayland
client creates a public virtual pointer, receives a server-issued pointer-enter
serial and selects its actual cursor role. Its assigned369×881 XRGB surface
retains all1,300,356 original bytes after real commit/release/sync and a protocol
frame callback. Its32×32 ARGB cursor retains every original pixel and row-padding
byte: stride144,4,608 bytes, with the actual cursor CPU cache matching the snapshot.

The [test witness](../tools/native-render-import/consumer/README.md) calls the
linked production helper and paints its Cairo source into an owned CPU image.
Both live observations use input stride128 and produce992 wrong pixels out of
1,024. The first mismatch at `(0,1)` reads original padding `0xa5a5a5a5` rather
than the expected row pixel. The frozen oracle always requires original rows;
controller1 is the actual property failure and outer0 qualifies that RED. All42
waits/49 PIPE EOFs meet original cutoffs with the same QMP/KVM/VM/container/input
closure. After actual client wait0, both retained clones still hold their original
bytes; explicit close releases two handles and leaves zero.

Two prior attempts remain setup failures: obsolete focus-dispatch syntax, then a
single center-motion request that yields no pointer enter before the original deadline.
Correct current Lua focus syntax and a fixed real virtual-pointer zero→center
motion sequence repair setup only. The semantic oracle and witness remain
byte-identical. No cursor behavior is inferred from either failed setup.

## Rebuilt stride repair and cursor CPU GREEN

After that qualified RED, a one-helper repair uses the supplied stride; the other
20 cohort payloads are unchanged. The complete cursor cohort has21 files. The
matching repaired full native build and same five FFI groups pass in173.902010707s
with a502-file SDK, all11 changed C++ units and33 Scope waits/33 PIPE EOFs. Only
the helper differs from the baseline; compositor/Source/UI/PCM remain unexecuted
in that build.

A later actual native run passes the unchanged one-case contract and all five
actor/controller/oracle/join source hashes from RED3. Both live pre-client-exit
Cairo comparisons check1,024 pixels with zero mismatches: the source and input
stride are144, and the owned CPU destination stride is128. Assigned SHM, real
virtual-pointer/enter/selected-role and cache/padding controls still pass.
Controller0 and outer0 qualify this CPU-consumer GREEN; all42 waits/49 PIPE EOFs,
original cutoffs and the same QMP/KVM/VM/container/locked-input closure pass.

After actual `Q` and client wait0, two retained Rust clones verify their original
raw pixels/padding, then close releases two handles and leaves zero. This latter
check invokes no Cairo helper and makes no live server-surface query. Separate
[hash-only provenance](evidence/native-render-import-provenance.json) binds both
builds, actual RED/GREEN and independent reviews.

This checks the actual Cairo CPU row consumer used when preparing a hardware
cursor buffer. It does not execute or qualify the complete hardware-cursor branch
or software-cursor display path. Snapshot bytes, returned texture validity,
protocol callbacks and this CPU image establish no GPU upload, output readback,
presentation or server-resource retirement.

## Qualified scene31 checkpoint

The complete public31-file patch now passes a native x86 source build with its
matching505-file SDK. All15 affected C++ units appear in actual compilation;
453 `compile_commands` entries describe configuration, not453 executed units.
The five existing C++/Rust snapshot lifetime groups pass separately within this
build. Its175.441011001s epoch closes33 waits/33 PIPE EOFs, same-peer QMP guest
shutdown/QEMU0, foreground CLI0, exact same-daemon container absence and original
locked-input/cutoff closure. This build executes no compositor.

An earlier29-path native build fails C++ namespace/type/private-access compilation;
it supplies no behavioral RED. Four narrowly corrected files produce the31-file
candidate; the other27 prior payloads remain identical. The complete upstream
patch and every before/after hash match the qualified source manifest. The
corrected final `FramePlan.hpp` carries `securityGeneration`, but integer shape
alone supplies no authentic control authority.

The factored order, CPU frame-description producer and owner-only GL primitives
are prerequisites. The new producer/primitives are not called and no render owner
starts in this selected runtime. The31 ELF uses the old snapshot-only Rust archive
`69b81281…` and unchanged C ABI `4a7a9a60…`; the separate eight-test permit archive
`fe5aac70…` is not linked. These artifacts have separate source/build qualifications.

A fresh native replay then rebuilds the original helpers against the complete
505-file SDK and executes the unchanged seven Source cases. All setups qualify.
No-stall focus/death/lock facts arrive after129,074/370,623/71,485ns. Their three
entered EGL holds last200.617/200.452/200.649ms and still produce terminal timeout
with positive state false before release. The full post-operation response window
exceeds100ms inside each hold; actual focus reply, client exit and lock protocol
acceptance remain mandatory. The stopped-owner negative also passes. Controller1
and outer0 qualify expected RED, with four controls passing and three rendering
failures; they do not qualify feature GREEN.

This36.805508919s replay closes77 waits/84 PIPE EOFs, counting the early two once.
All seven watched descendants are dead and absent, and the subreaper reaches
ECHILD before final closure. Same-peer QMP shutdown/EOF/QEMU0, CLI0, exact-CID
absence, same daemon and original locked inputs close separately. Hash-only
[provenance](evidence/native-render-import-provenance.json) binds both qualifications
and the independent reviews. The earlier CPU identity/Cairo results retain their
original cohorts. No connected worker, AQ/GL execution migration, frame completion,
Source GREEN or phone acceptance follows from this checkpoint.

## Rust commit-permit component

A separate original Rust component now orders an individual attempt against
accepted control intent and Stop. It retains a full nonzero u64 security/output/
scene/session policy, checks serial exhaustion, and seals a move-only ticket
with one CAS and real CLOCK_BOOTTIME reads before/after. New admitted work or
security intent invalidates unused tickets; `invalidate_current` also revokes
unsealed work without changing the security epoch or granting a replacement.
Already sealed old-attempt records survive later intent/Stop without authorizing
another attempt. Integer policy shape does not authenticate output/session/
scene/device bindings; authentic policy and reserved outcome custody remain
requirements of the future connected owner. The deadline bounds the grant-time
check only: preemption may put native entry or completion after it.

Six initial native Linux x86 thread/clock/admission cases pass. The earlier
missing-implementation compile attempt runs no semantic test and is not behavioral
RED. A seventh case then executes and exposes control-owner loss incorrectly
allowing an unused ticket to seal: six pass/one fails, Cargo/CLI101. Adding the
control owner's `Drop` Stop fence gives seven passes. The drop expectation is
unchanged; one existing boolean assertion changes to equivalent syntax, so the
whole test file is not claimed byte-identical. An eighth case adds same-epoch
invalidation after a separate missing-API compile failure, also not semantic RED.
Final formatted-source QA passes all eight tests, rustfmt, Clippy with warnings
denied and the x86 release build. The new API case uses authored policy; actual
device-pause integration is unqualified. The existing snapshot C ABI/header is
unchanged, and its C++ FFI was not re-run against the new release archive.
[Hash-only provenance](evidence/native-render-import-provenance.json) keeps those
epochs separate.

This is a Rust component on the host Linux kernel, separate from the earlier
C++/Cairo native VM evidence. It implements no production worker or JoinHandle
custody, FD queue, C ABI, AQ/render dispatch, native call or Source integration.
Neither an issued ticket nor a sealed record establishes commit, GPU completion
or presentation; the eight Rust cases do not turn the seven rendering cases GREEN.

Complete supported scene/frame integration, a Rust-owned executor, exclusive
AQ/session/GL/output state, real fence/presentation completion and responsive terminal ownership
are still required. Queue acceptance or copied pixels cannot substitute for
those results. Keep the frozen seven rendering cases and original cold owner,
permissions, focus, Pixel and Sounds/PCM guards unchanged. ARM, the matched
Google kernel, complete Omarchy shell/startup, secure frames, physical hardware,
carrier and full smartphone matrix remain open. No physical phone was accessed.
