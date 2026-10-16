# Rust-owned immutable SHM import

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
`efb50993780079460b0cbed1363e2166a2de1d9f`. Its18-file before/after map combines the
six preserved original render observations with13 import files; GLRenderer
belongs to both. It contains no proprietary source or unpublished patch
prerequisite. Every plugin must use the rebuilt SDK, including the exact C header.

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
The other17 cohort payloads are unchanged. The public complete18-file patch now
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
success. The existing pointer consumer also assumes tight AR24 rows despite
retained snapshot stride. Neither source observation is a new behavioral RED;
actual upload failure and padded-cursor output remain unqualified.

A complete immutable scene/frame, Rust-owned executor, exclusive AQ/session/GL/
output state, real fence/presentation completion and responsive terminal ownership
are still required. Queue acceptance or copied pixels cannot substitute for
those results. Keep the frozen seven rendering cases and original cold owner,
permissions, focus, Pixel and Sounds/PCM guards unchanged. ARM, the matched
Google kernel, complete Omarchy shell/startup, secure frames, physical hardware,
carrier and full smartphone matrix remain open. No physical phone was accessed.
